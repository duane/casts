// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

use super::Command;
use crate::aggregate::FeedAggregate;
use crate::events::FeedAggregateEvent;
use async_trait::async_trait;
use immutable_feed::Feed;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};
use url::Url;
use uuid::Uuid;

#[allow(clippy::large_enum_variant)]
/// The commands available for the aggregate.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum FeedAggregateCommand {
    /// Subscribe to a new feed.
    ///
    /// The caller fetches and parses the feed BEFORE issuing the command
    /// (`immutable_feed::fetch_feed`); command application itself is pure
    /// validation against the aggregate and never performs I/O.
    AddFeed {
        /// The id of the aggregate.
        stream_id: Uuid,

        /// The feed id to represent this new feed.
        feed_id: Uuid,

        /// The url where the feed is located.
        url: Url,

        /// The feed as fetched from `url` by the caller.
        feed: Feed,
    },

    /// The first event for each feed. Used to initialize entries and metadata attributes.
    ObserveFeed {
        /// The stream in which the feed was observed.
        stream_id: Uuid,

        /// The feed that was observed
        feed_id: Uuid,

        /// The initial feed seen.
        feed: Feed,
    },

    /// Record the playback position reached for one entry. Last write wins.
    MarkEntryListened {
        /// The stream in which the playback was recorded.
        stream_id: Uuid,

        /// The feed holding the entry.
        feed_id: Uuid,

        /// The entry that was listened to.
        entry_guid: String,

        /// Whole seconds of playback position reached.
        elapsed_seconds: i64,

        /// Sub-second remainder of the playback position, in nanoseconds.
        elapsed_nanos: i32,
    },
}

/// The error associated with running commands against a feed aggregate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeedAggregateError {
    /// A feed with this id or url already associates with the aggregate.
    DuplicateFeedError,

    /// The feed is not well-formed.
    BadFeedError,

    /// The command names a feed id the aggregate does not hold.
    UnknownFeedError,

    /// The command names an entry guid its feed does not hold.
    UnknownEntryError,
}

impl Error for FeedAggregateError {
    fn cause(&self) -> Option<&dyn Error> {
        None
    }

    fn description(&self) -> &str {
        match self {
            FeedAggregateError::DuplicateFeedError => "This feed has already been added.",
            FeedAggregateError::BadFeedError => "The url does not point to a feed.",
            FeedAggregateError::UnknownFeedError => "No feed with this id exists.",
            FeedAggregateError::UnknownEntryError => "The feed holds no entry with this guid.",
        }
    }
}

impl Display for FeedAggregateError {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{:?}", self)
    }
}

#[async_trait]
impl Command for FeedAggregateCommand {
    type Aggregate = FeedAggregate;
    type Error = FeedAggregateError;

    fn id(&self) -> &Uuid {
        match self {
            FeedAggregateCommand::AddFeed { ref stream_id, .. } => stream_id,
            FeedAggregateCommand::ObserveFeed { ref stream_id, .. } => stream_id,
            FeedAggregateCommand::MarkEntryListened { ref stream_id, .. } => stream_id,
        }
    }

    async fn async_apply(
        &self,
        aggregate: &FeedAggregate,
    ) -> Result<Vec<FeedAggregateEvent>, FeedAggregateError> {
        match self {
            FeedAggregateCommand::AddFeed {
                ref stream_id,
                ref feed_id,
                ref url,
                ref feed,
            } => {
                if aggregate.feeds_by_feed_id.contains_key(feed_id) {
                    return Err(FeedAggregateError::DuplicateFeedError);
                }

                // does the url already exist in the platform?
                if aggregate
                    .feeds_by_feed_id
                    .values()
                    .any(|f| f.url.as_ref() == Some(url))
                {
                    return Err(FeedAggregateError::DuplicateFeedError);
                }

                Ok(vec![FeedAggregateEvent::Added {
                    stream_id: stream_id.to_owned(),
                    url: url.to_owned(),
                    feed_id: feed_id.to_owned(),
                    metadata: feed.metadata.clone(),
                    entries: feed.entries.clone(),
                }])
            }
            FeedAggregateCommand::ObserveFeed {
                stream_id,
                feed_id,
                ref feed,
            } => match aggregate.feeds_by_feed_id.get(feed_id) {
                Some(existing_feed) => {
                    log::debug!(
                        "comparing feeds: {} existing entries vs {} observed",
                        existing_feed.entries.len(),
                        feed.entries.len()
                    );
                    // `existing.diff(observed)` produces the mutations that
                    // move the stored feed toward the observation.
                    let changes = existing_feed.diff(feed);
                    if changes.is_empty() {
                        return Ok(vec![]);
                    }
                    Ok(vec![FeedAggregateEvent::ObservedChanges {
                        stream_id: *stream_id,
                        feed_id: *feed_id,
                        changes,
                    }])
                }
                None => Err(FeedAggregateError::UnknownFeedError),
            },
            FeedAggregateCommand::MarkEntryListened {
                stream_id,
                feed_id,
                ref entry_guid,
                elapsed_seconds,
                elapsed_nanos,
            } => {
                let feed = aggregate
                    .feeds_by_feed_id
                    .get(feed_id)
                    .ok_or(FeedAggregateError::UnknownFeedError)?;
                if !feed.entries.iter().any(|entry| &entry.guid == entry_guid) {
                    return Err(FeedAggregateError::UnknownEntryError);
                }
                // The same position twice is not a change; emit nothing so
                // callers skip the commit entirely.
                let key = (feed_id.to_string(), entry_guid.clone());
                if aggregate.playtime_by_feed_and_guid.get(&key)
                    == Some(&(*elapsed_seconds, *elapsed_nanos))
                {
                    return Ok(vec![]);
                }
                Ok(vec![FeedAggregateEvent::FeedEntryListenedTo {
                    stream_id: *stream_id,
                    feed_id: *feed_id,
                    entry_guid: entry_guid.clone(),
                    elapsed_seconds: *elapsed_seconds,
                    elapsed_nanos: *elapsed_nanos,
                }])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::applicable::EventApplicable;
    use immutable_feed::FeedMetadata;

    fn test_feed(url: &str, title: &str) -> Feed {
        let url = Url::parse(url).unwrap();
        Feed {
            url: Some(url.clone()),
            raw_url: url.to_string(),
            metadata: FeedMetadata {
                title: String::from(title),
                authors: vec![],
                logo: None,
                description: None,
            },
            entries: vec![],
        }
    }

    fn add_feed(stream_id: Uuid, feed_id: Uuid, url: &str) -> FeedAggregateCommand {
        FeedAggregateCommand::AddFeed {
            stream_id,
            feed_id,
            url: Url::parse(url).unwrap(),
            feed: test_feed(url, "Examplecast"),
        }
    }

    #[tokio::test]
    async fn add_feed_emits_added_without_io() {
        let aggregate = FeedAggregate::new();
        let feed_id = Uuid::new_v4();
        let events = add_feed(aggregate.stream_id, feed_id, "http://example.com/a.xml")
            .async_apply(&aggregate)
            .await
            .unwrap();
        assert_eq!(events.len(), 1);
        let applied = aggregate.apply_many(events);
        assert_eq!(
            applied.feeds_by_feed_id[&feed_id].metadata.title,
            "Examplecast"
        );
    }

    #[tokio::test]
    async fn add_feed_rejects_duplicate_id() {
        let aggregate = FeedAggregate::new();
        let feed_id = Uuid::new_v4();
        let first = add_feed(aggregate.stream_id, feed_id, "http://example.com/a.xml");
        let events = first.async_apply(&aggregate).await.unwrap();
        let aggregate = aggregate.apply_many(events);

        let dup = add_feed(aggregate.stream_id, feed_id, "http://example.com/b.xml");
        assert!(matches!(
            dup.async_apply(&aggregate).await,
            Err(FeedAggregateError::DuplicateFeedError)
        ));
    }

    #[tokio::test]
    async fn add_feed_rejects_duplicate_url() {
        let aggregate = FeedAggregate::new();
        let first = add_feed(
            aggregate.stream_id,
            Uuid::new_v4(),
            "http://example.com/a.xml",
        );
        let events = first.async_apply(&aggregate).await.unwrap();
        let aggregate = aggregate.apply_many(events);

        let dup = add_feed(
            aggregate.stream_id,
            Uuid::new_v4(),
            "http://example.com/a.xml",
        );
        assert!(matches!(
            dup.async_apply(&aggregate).await,
            Err(FeedAggregateError::DuplicateFeedError)
        ));
    }

    #[tokio::test]
    async fn observe_feed_with_no_changes_emits_nothing() {
        let aggregate = FeedAggregate::new();
        let feed_id = Uuid::new_v4();
        let add = add_feed(aggregate.stream_id, feed_id, "http://example.com/a.xml");
        let events = add.async_apply(&aggregate).await.unwrap();
        let aggregate = aggregate.apply_many(events);

        let observe = FeedAggregateCommand::ObserveFeed {
            stream_id: aggregate.stream_id,
            feed_id,
            feed: test_feed("http://example.com/a.xml", "Examplecast"),
        };
        assert!(observe.async_apply(&aggregate).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn observe_feed_for_unknown_feed_errors() {
        let aggregate = FeedAggregate::new();
        let observe = FeedAggregateCommand::ObserveFeed {
            stream_id: aggregate.stream_id,
            feed_id: Uuid::new_v4(),
            feed: test_feed("http://example.com/a.xml", "Examplecast"),
        };
        assert!(matches!(
            observe.async_apply(&aggregate).await,
            Err(FeedAggregateError::UnknownFeedError)
        ));
    }

    fn test_entry(guid: &str) -> immutable_feed::Entry {
        immutable_feed::Entry {
            guid: String::from(guid),
            url: Url::parse("http://example.com/episode-1.mp3").unwrap(),
            title: Some(String::from("Episode 1")),
            summary: None,
            updated_at: None,
            published_at: None,
            content: None,
            links: vec![],
            media: vec![],
        }
    }

    async fn aggregate_with_entry(feed_id: Uuid, guid: &str) -> FeedAggregate {
        let aggregate = FeedAggregate::new();
        let mut feed = test_feed("http://example.com/a.xml", "Examplecast");
        feed.entries.push(test_entry(guid));
        let add = FeedAggregateCommand::AddFeed {
            stream_id: aggregate.stream_id,
            feed_id,
            url: Url::parse("http://example.com/a.xml").unwrap(),
            feed,
        };
        let events = add.async_apply(&aggregate).await.unwrap();
        aggregate.apply_many(events)
    }

    #[tokio::test]
    async fn mark_entry_listened_records_playtime() {
        let feed_id = Uuid::new_v4();
        let aggregate = aggregate_with_entry(feed_id, "guid-1").await;
        let mark = FeedAggregateCommand::MarkEntryListened {
            stream_id: aggregate.stream_id,
            feed_id,
            entry_guid: String::from("guid-1"),
            elapsed_seconds: 90,
            elapsed_nanos: 500,
        };
        let events = mark.async_apply(&aggregate).await.unwrap();
        assert_eq!(events.len(), 1);
        let applied = aggregate.apply_many(events);
        assert_eq!(
            applied.playtime_by_feed_and_guid[&(feed_id.to_string(), String::from("guid-1"))],
            (90, 500)
        );
    }

    #[tokio::test]
    async fn mark_entry_listened_with_unchanged_position_emits_nothing() {
        let feed_id = Uuid::new_v4();
        let aggregate = aggregate_with_entry(feed_id, "guid-1").await;
        let mark = |aggregate: &FeedAggregate| FeedAggregateCommand::MarkEntryListened {
            stream_id: aggregate.stream_id,
            feed_id,
            entry_guid: String::from("guid-1"),
            elapsed_seconds: 90,
            elapsed_nanos: 500,
        };
        let events = mark(&aggregate).async_apply(&aggregate).await.unwrap();
        let aggregate = aggregate.apply_many(events);
        assert!(mark(&aggregate)
            .async_apply(&aggregate)
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn mark_entry_listened_rejects_unknown_feed() {
        let aggregate = aggregate_with_entry(Uuid::new_v4(), "guid-1").await;
        let mark = FeedAggregateCommand::MarkEntryListened {
            stream_id: aggregate.stream_id,
            feed_id: Uuid::new_v4(),
            entry_guid: String::from("guid-1"),
            elapsed_seconds: 1,
            elapsed_nanos: 0,
        };
        assert!(matches!(
            mark.async_apply(&aggregate).await,
            Err(FeedAggregateError::UnknownFeedError)
        ));
    }

    #[tokio::test]
    async fn mark_entry_listened_rejects_unknown_entry() {
        let feed_id = Uuid::new_v4();
        let aggregate = aggregate_with_entry(feed_id, "guid-1").await;
        let mark = FeedAggregateCommand::MarkEntryListened {
            stream_id: aggregate.stream_id,
            feed_id,
            entry_guid: String::from("guid-does-not-exist"),
            elapsed_seconds: 1,
            elapsed_nanos: 0,
        };
        assert!(matches!(
            mark.async_apply(&aggregate).await,
            Err(FeedAggregateError::UnknownEntryError)
        ));
    }

    #[tokio::test]
    async fn observe_feed_diffs_metadata() {
        let aggregate = FeedAggregate::new();
        let feed_id = Uuid::new_v4();
        let add = add_feed(aggregate.stream_id, feed_id, "http://example.com/a.xml");
        let events = add.async_apply(&aggregate).await.unwrap();
        let aggregate = aggregate.apply_many(events);

        let observe = FeedAggregateCommand::ObserveFeed {
            stream_id: aggregate.stream_id,
            feed_id,
            feed: test_feed("http://example.com/a.xml", "Renamedcast"),
        };
        let events = observe.async_apply(&aggregate).await.unwrap();
        assert_eq!(events.len(), 1);
        let applied = aggregate.apply_many(events);
        assert_eq!(
            applied.feeds_by_feed_id[&feed_id].metadata.title,
            "Renamedcast"
        );
    }
}
