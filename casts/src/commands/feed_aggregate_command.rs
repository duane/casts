use super::Command;
use crate::aggregate::FeedAggregate;
use crate::events::FeedAggregateEvent;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};
use url::Url;
use uuid::Uuid;

/// The commands available for the aggregate.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum FeedAggregateCommand {
    /// Subscribe to a new feed.
    AddFeed {
        /// The id of the aggregate.
        stream_id: Uuid,

        /// The feed id to represent this new feed.
        feed_id: Uuid,

        /// The url where the feed is located.
        url: Url,
    },

    /// Observe an entry for a feed.
    FeedEntryObserved {
        /// aggregate id
        stream_id: Uuid,

        /// Entry title
        title: String,

        /// The feed whose entry is observed.
        feed_id: Uuid,

        /// (theoretically) unique guid of the feed entry
        guid: String,

        /// Url of the primary media content attached to the entry.
        url: Url,

        /// Text summary of the entry.
        summary: Option<String>,

        /// When the entry was last updated.
        updated_at: Option<DateTime<Utc>>,

        /// When the entry was first published.
        published_at: Option<DateTime<Utc>>,
    },
}

/// The error associated with running commands against a feed aggregate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeedAggregateError {
    /// A feed with this id already exists associated with the aggregate.
    DuplicateFeedError,

    /// The feed is not well-formed.
    BadFeedError,
}

impl Error for FeedAggregateError {
    fn cause(&self) -> Option<&dyn Error> {
        None
    }

    fn description(&self) -> &str {
        match self {
            &FeedAggregateError::DuplicateFeedError => "This feed has already been added.",
            &FeedAggregateError::BadFeedError => "The url does not point to a feed.",
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
            &FeedAggregateCommand::AddFeed {
                stream_id: ref inner_id,
                feed_id: _,
                url: _,
            } => inner_id,
            &FeedAggregateCommand::FeedEntryObserved {
                stream_id: ref inner_id,
                title: _,
                feed_id: _,
                guid: _,
                url: _,
                summary: _,
                updated_at: _,
                published_at: _,
            } => inner_id,
        }
    }

    fn apply(
        &self,
        _aggregate: &FeedAggregate,
    ) -> Result<Vec<FeedAggregateEvent>, FeedAggregateError> {
        unimplemented!("do not use sync apply for commands")
    }

    async fn async_apply(
        &self,
        aggregate: &FeedAggregate,
    ) -> Result<Vec<FeedAggregateEvent>, FeedAggregateError> {
        match self {
            &FeedAggregateCommand::AddFeed {
                ref stream_id,
                ref feed_id,
                ref url,
            } => {
                if aggregate.feeds_by_feed_id.get(feed_id).is_some() {
                    return Err(FeedAggregateError::DuplicateFeedError);
                }
                let download_result = reqwest::get(url.to_string())
                    .await
                    .unwrap()
                    .bytes()
                    .await
                    .unwrap();
                let parsed_feed =
                    immutable_feed::parse_doc(url, &download_result).expect("could not parse feed");
                let mut events = vec![FeedAggregateEvent::Added {
                    stream_id: stream_id.to_owned(),
                    url: url.to_owned(),
                    feed_id: feed_id.to_owned(),
                    title: parsed_feed.title,
                    description: parsed_feed.description,
                    updated_at: parsed_feed.updated_at,
                    published_at: parsed_feed.published_at,
                }];
                for entry in parsed_feed.entries.iter() {
                    events.push(FeedAggregateEvent::EntryObserved {
                        stream_id: stream_id.clone(),
                        feed_id: feed_id.clone(),
                        guid: entry.guid.clone(),
                        title: entry.title.clone(),
                        url: entry.url.clone(),
                        summary: entry.summary.clone(),
                        updated_at: entry.updated_at.clone(),
                        published_at: entry.published_at.clone(),
                    });
                }
                Ok(events)
            }
            &FeedAggregateCommand::FeedEntryObserved {
                ref stream_id,
                ref title,
                ref feed_id,
                ref guid,
                ref url,
                ref summary,
                ref updated_at,
                ref published_at,
            } => match aggregate.feeds_by_feed_id.get(feed_id) {
                Some(feed) => match feed.entries.iter().find(|e| e.guid.as_str() == guid) {
                    Some(existing_entry) => {
                        let observed_entry = immutable_feed::Entry {
                            guid: guid.to_owned(),
                            title: Some(title.to_owned()),
                            url: url.to_owned(),
                            summary: summary.to_owned(),
                            updated_at: updated_at.to_owned(),
                            published_at: published_at.to_owned(),
                        };
                        let diff = observed_entry.diff(&existing_entry);
                        for mutation in diff.iter() {
                            todo!()
                        }
                        Ok(vec![])
                    }
                    None => {
                        return Ok(vec![FeedAggregateEvent::EntryObserved {
                            stream_id: stream_id.to_owned(),
                            title: Some(title.to_owned()),
                            feed_id: feed_id.to_owned(),
                            guid: guid.to_owned(),
                            url: url.to_owned(),
                            summary: summary.to_owned(),
                            updated_at: updated_at.to_owned(),
                            published_at: published_at.to_owned(),
                        }]);
                    }
                },
                None => panic!("unknown feed: {:?}", feed_id),
            },
        }
    }
}
