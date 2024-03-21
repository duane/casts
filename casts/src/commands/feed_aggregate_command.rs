// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

use super::Command;
use crate::aggregate::FeedAggregate;
use crate::events::FeedAggregateEvent;
use async_trait::async_trait;
use immutable_feed::{Entry, Feed};
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
    AddFeed {
        /// The id of the aggregate.
        stream_id: Uuid,

        /// The feed id to represent this new feed.
        feed_id: Uuid,

        /// The url where the feed is located.
        url: Url,
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
}

/// The error associated with running commands against a feed aggregate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeedAggregateError {
    /// A feed with this id or url already associates with the aggregate.
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
            FeedAggregateError::DuplicateFeedError => "This feed has already been added.",
            FeedAggregateError::BadFeedError => "The url does not point to a feed.",
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
            // add vs subscribe
            // TODO:
            // add -> observe
            // subscribe command/event for added feed
            FeedAggregateCommand::AddFeed {
                stream_id: ref inner_id,
                feed_id: _,
                url: _,
            } => inner_id,
            FeedAggregateCommand::ObserveFeed {
                stream_id: ref inner_id,
                feed_id: _,
                feed: _,
            } => inner_id,
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
            } => {
                if aggregate.feeds_by_feed_id.get(feed_id).is_some() {
                    return Err(FeedAggregateError::DuplicateFeedError);
                }

                // does the url already exist in the platform?
                if aggregate
                    .feeds_by_feed_id
                    .values()
                    .any(|f| f.url.is_some() && f.url.as_ref().unwrap() == url)
                {
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

                Ok(vec![FeedAggregateEvent::Added {
                    stream_id: stream_id.to_owned(),
                    url: url.to_owned(),
                    feed_id: feed_id.to_owned(),
                    metadata: parsed_feed.metadata,
                    entries: parsed_feed.entries.into_iter().map(Entry::from).collect(),
                }])
            }
            FeedAggregateCommand::ObserveFeed {
                stream_id,
                feed_id,
                ref feed,
            } => match aggregate.feeds_by_feed_id.get(feed_id) {
                Some(existing_feed) => {
                    let changes = feed.diff(existing_feed);
                    Ok(vec![FeedAggregateEvent::ObservedChanges {
                        stream_id: *stream_id,
                        feed_id: *feed_id,
                        changes,
                    }])
                }
                None => panic!("found bad feed id"),
            },
        }
    }
}
