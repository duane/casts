// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

use super::Command;
use crate::aggregate::FeedAggregate;
use crate::events::FeedAggregateEvent;
use async_trait::async_trait;
use immutable_feed::{Entry, Feed, FeedMutation};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
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

        /// Skip this feed in the archival worker from the moment it is
        /// added. `None`/`Some(false)` archive normally; only `Some(true)`
        /// opts out.
        do_not_archive: Option<bool>,
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
    /// Record that an entry's audio has been copied to durable storage.
    ///
    /// Issued by the archival worker *after* the upload is confirmed.
    ArchiveEntry {
        stream_id: Uuid,
        feed_id: Uuid,
        entry_guid: String,
        bucket: String,
        key: String,
        content_type: Option<String>,
        size_bytes: i64,
    },

    /// Add a batch of entries to an existing feed directly, without diffing
    /// against the feed's full current state.
    ///
    /// Exists because `AddFeed`'s `Added` event embeds every entry it is
    /// given in one commit -- fine for a small feed, but a large one (Crime
    /// Junkie: 7.16 MB of entries, observed live) blows commit-store's 2 MB
    /// message limit in a single commit. `ObserveFeed` cannot be paginated
    /// instead: `Feed::diff` computes a full comparison between two
    /// complete feed snapshots, so handing it a partial slice would make
    /// every entry from an earlier, un-included batch look removed
    /// (`(Some(existing), None) => EntryRemoved`). This command sidesteps
    /// diffing: it reuses `ObservedChanges`/`FeedMutation::EntryAdded`,
    /// whose `apply_diff` handling only ever inserts by guid and needs no
    /// full-feed context, so a caller (see `poller::add_feed_paginated`)
    /// can add a large feed's entries across many commits with no risk of
    /// an earlier batch being read as deleted.
    ///
    /// Idempotent per entry: a guid already on the feed is silently
    /// skipped rather than duplicated, so retrying a batch after a partial
    /// failure is safe.
    AppendEntries {
        stream_id: Uuid,
        feed_id: Uuid,
        entries: Vec<Entry>,
    },

    /// Apply a pre-computed batch of feed changes.
    ///
    /// `ObserveFeed` computes a whole-feed diff and emits it as one
    /// `ObservedChanges` event, which blows commit-store's 2 MB message
    /// limit when a feed has drifted far from what is stored (observed
    /// live: a 7.97 MB diff for a feed whose import stopped after its first
    /// batch). This takes the mutations already computed by the caller, so
    /// they can be split across commits.
    ///
    /// Safe to chunk because `FeedMutation`s are independent per-entry
    /// operations -- unlike `ObserveFeed`, nothing here is derived from
    /// comparing against the feed's complete state, so a partial batch
    /// cannot make un-included entries look removed.
    ApplyFeedChanges {
        stream_id: Uuid,
        feed_id: Uuid,
        changes: Vec<FeedMutation>,
    },

    /// Set or clear a feed's "do not archive" preference. Errors if the
    /// feed is unknown; otherwise always produces an event, even if the
    /// value is unchanged -- unlike `ArchiveEntry`'s no-op-when-redundant
    /// rule, there is no retry/race this needs to protect against, and a
    /// caller re-asserting the same preference is ordinary use, not a
    /// symptom of a crash to recover from.
    SetArchivePreference {
        stream_id: Uuid,
        feed_id: Uuid,
        do_not_archive: bool,
    },

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
            FeedAggregateCommand::ArchiveEntry { ref stream_id, .. } => stream_id,
            FeedAggregateCommand::AppendEntries { ref stream_id, .. } => stream_id,
            FeedAggregateCommand::ApplyFeedChanges { ref stream_id, .. } => stream_id,
            FeedAggregateCommand::SetArchivePreference { ref stream_id, .. } => stream_id,
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
                do_not_archive,
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
                    do_not_archive: *do_not_archive,
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
            FeedAggregateCommand::ArchiveEntry {
                stream_id,
                feed_id,
                ref entry_guid,
                ref bucket,
                ref key,
                ref content_type,
                size_bytes,
            } => {
                let feed = aggregate
                    .feeds_by_feed_id
                    .get(feed_id)
                    .ok_or(FeedAggregateError::UnknownFeedError)?;
                if !feed.entries.iter().any(|e| &e.guid == entry_guid) {
                    return Err(FeedAggregateError::UnknownEntryError);
                }
                // Already archived is a no-op, not a conflict. Two workers
                // can race on the same entry, and a retry after a crash
                // between upload and commit re-runs this path; emitting no
                // events makes both cases harmless instead of producing a
                // second, contradictory record.
                if aggregate
                    .archived_by_feed_and_guid
                    .contains_key(&(feed_id.to_string(), entry_guid.clone()))
                {
                    return Ok(vec![]);
                }
                Ok(vec![FeedAggregateEvent::EntryArchived {
                    stream_id: *stream_id,
                    feed_id: *feed_id,
                    entry_guid: entry_guid.clone(),
                    bucket: bucket.clone(),
                    key: key.clone(),
                    content_type: content_type.clone(),
                    size_bytes: *size_bytes,
                }])
            }

            FeedAggregateCommand::AppendEntries {
                stream_id,
                feed_id,
                entries,
            } => {
                let Some(existing) = aggregate.feeds_by_feed_id.get(feed_id) else {
                    return Err(FeedAggregateError::UnknownFeedError);
                };
                let existing_guids: HashSet<&str> =
                    existing.entries.iter().map(|e| e.guid.as_str()).collect();
                let changes: Vec<FeedMutation> = entries
                    .iter()
                    .filter(|e| !existing_guids.contains(e.guid.as_str()))
                    .cloned()
                    .map(FeedMutation::EntryAdded)
                    .collect();
                if changes.is_empty() {
                    return Ok(vec![]);
                }
                Ok(vec![FeedAggregateEvent::ObservedChanges {
                    stream_id: *stream_id,
                    feed_id: *feed_id,
                    changes,
                }])
            }
            FeedAggregateCommand::ApplyFeedChanges {
                stream_id,
                feed_id,
                changes,
            } => {
                if !aggregate.feeds_by_feed_id.contains_key(feed_id) {
                    return Err(FeedAggregateError::UnknownFeedError);
                }
                if changes.is_empty() {
                    return Ok(vec![]);
                }
                Ok(vec![FeedAggregateEvent::ObservedChanges {
                    stream_id: *stream_id,
                    feed_id: *feed_id,
                    changes: changes.clone(),
                }])
            }
            FeedAggregateCommand::SetArchivePreference {
                stream_id,
                feed_id,
                do_not_archive,
            } => {
                if !aggregate.feeds_by_feed_id.contains_key(feed_id) {
                    return Err(FeedAggregateError::UnknownFeedError);
                }
                Ok(vec![FeedAggregateEvent::ArchivePreferenceSet {
                    stream_id: *stream_id,
                    feed_id: *feed_id,
                    do_not_archive: *do_not_archive,
                }])
            }
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
            do_not_archive: None,
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
            updated_at: None,
            published_at: None,
            content_digest: None,
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
            do_not_archive: None,
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

    /// Archiving is recorded only for entries that exist, and only once.
    #[tokio::test]
    async fn archive_entry_is_recorded_once_and_ignores_repeats() {
        let feed_id = Uuid::new_v4();
        let aggregate = aggregate_with_entry(feed_id, "guid-1").await;
        let stream_id = aggregate.stream_id;

        let cmd = FeedAggregateCommand::ArchiveEntry {
            stream_id,
            feed_id,
            entry_guid: "guid-1".to_string(),
            bucket: "casts-archive".to_string(),
            key: "a/b/c.mp3".to_string(),
            content_type: Some("audio/mpeg".to_string()),
            size_bytes: 1234,
        };
        let events = cmd.async_apply(&aggregate).await.unwrap();
        assert_eq!(events.len(), 1);

        // Applying it, then re-issuing, must produce nothing: two workers can
        // race on one entry, and a crash between upload and commit replays
        // this path.
        let applied = aggregate.apply_many(events);
        assert!(applied
            .archived_by_feed_and_guid
            .contains_key(&(feed_id.to_string(), "guid-1".to_string())));
        assert!(
            cmd.async_apply(&applied).await.unwrap().is_empty(),
            "re-archiving an already-archived entry must be a no-op"
        );
    }

    #[tokio::test]
    async fn archive_entry_rejects_unknown_feed_and_entry() {
        let feed_id = Uuid::new_v4();
        let aggregate = aggregate_with_entry(feed_id, "guid-1").await;
        let stream_id = aggregate.stream_id;

        let unknown_entry = FeedAggregateCommand::ArchiveEntry {
            stream_id,
            feed_id,
            entry_guid: "nope".to_string(),
            bucket: "b".to_string(),
            key: "k".to_string(),
            content_type: None,
            size_bytes: 1,
        };
        assert!(unknown_entry.async_apply(&aggregate).await.is_err());

        let unknown_feed = FeedAggregateCommand::ArchiveEntry {
            stream_id,
            feed_id: Uuid::new_v4(),
            entry_guid: "guid-1".to_string(),
            bucket: "b".to_string(),
            key: "k".to_string(),
            content_type: None,
            size_bytes: 1,
        };
        assert!(unknown_feed.async_apply(&aggregate).await.is_err());
    }

    #[tokio::test]
    async fn set_archive_preference_rejects_unknown_feed() {
        let feed_id = Uuid::new_v4();
        let aggregate = aggregate_with_entry(feed_id, "guid-1").await;
        let cmd = FeedAggregateCommand::SetArchivePreference {
            stream_id: aggregate.stream_id,
            feed_id: Uuid::new_v4(),
            do_not_archive: true,
        };
        assert!(cmd.async_apply(&aggregate).await.is_err());
    }

    #[tokio::test]
    async fn set_archive_preference_emits_for_a_known_feed_either_direction() {
        let feed_id = Uuid::new_v4();
        let aggregate = aggregate_with_entry(feed_id, "guid-1").await;
        let stream_id = aggregate.stream_id;

        let opt_out = FeedAggregateCommand::SetArchivePreference {
            stream_id,
            feed_id,
            do_not_archive: true,
        };
        let events = opt_out.async_apply(&aggregate).await.unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            FeedAggregateEvent::ArchivePreferenceSet {
                do_not_archive: true,
                ..
            }
        ));

        // Re-enabling is not treated as a redundant no-op the way an
        // already-archived ArchiveEntry is: this command has no retry/race
        // to protect against, so re-asserting -- or reversing -- the
        // preference always produces an event.
        let opt_in = FeedAggregateCommand::SetArchivePreference {
            stream_id,
            feed_id,
            do_not_archive: false,
        };
        let events = opt_in.async_apply(&aggregate).await.unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            FeedAggregateEvent::ArchivePreferenceSet {
                do_not_archive: false,
                ..
            }
        ));
    }

    #[tokio::test]
    async fn append_entries_rejects_unknown_feed() {
        let feed_id = Uuid::new_v4();
        let aggregate = aggregate_with_entry(feed_id, "existing").await;
        let cmd = FeedAggregateCommand::AppendEntries {
            stream_id: aggregate.stream_id,
            feed_id: Uuid::new_v4(),
            entries: vec![test_entry("new")],
        };
        assert!(cmd.async_apply(&aggregate).await.is_err());
    }

    /// The whole reason this command exists: a batch must add only what is
    /// genuinely new, and must never look at (or touch) entries outside the
    /// batch -- unlike `ObserveFeed`'s full-feed diff, which would flag a
    /// prior batch as removed.
    #[tokio::test]
    async fn append_entries_adds_new_and_skips_existing_by_guid() {
        let feed_id = Uuid::new_v4();
        let aggregate = aggregate_with_entry(feed_id, "already-here").await;
        let stream_id = aggregate.stream_id;

        let cmd = FeedAggregateCommand::AppendEntries {
            stream_id,
            feed_id,
            // One duplicate (must be skipped), one genuinely new.
            entries: vec![test_entry("already-here"), test_entry("brand-new")],
        };
        let events = cmd.async_apply(&aggregate).await.unwrap();
        assert_eq!(events.len(), 1);
        let FeedAggregateEvent::ObservedChanges { ref changes, .. } = events[0] else {
            panic!("expected ObservedChanges");
        };
        assert_eq!(changes.len(), 1, "the duplicate guid must not reappear");
        assert!(matches!(
            changes[0],
            immutable_feed::FeedMutation::EntryAdded(ref e) if e.guid == "brand-new"
        ));

        let applied = aggregate.apply_many(events);
        let guids: Vec<&str> = applied.feeds_by_feed_id[&feed_id]
            .entries
            .iter()
            .map(|e| e.guid.as_str())
            .collect();
        assert!(guids.contains(&"already-here"));
        assert!(guids.contains(&"brand-new"));
        assert_eq!(guids.len(), 2, "no duplicate entry, and nothing lost");
    }

    /// A batch that adds nothing new (a retried duplicate batch) must be a
    /// no-op, not an empty-but-real commit -- same idiom as `ArchiveEntry`.
    #[tokio::test]
    async fn append_entries_with_only_duplicates_is_a_noop() {
        let feed_id = Uuid::new_v4();
        let aggregate = aggregate_with_entry(feed_id, "already-here").await;
        let cmd = FeedAggregateCommand::AppendEntries {
            stream_id: aggregate.stream_id,
            feed_id,
            entries: vec![test_entry("already-here")],
        };
        let events = cmd.async_apply(&aggregate).await.unwrap();
        assert!(events.is_empty());
    }
}
