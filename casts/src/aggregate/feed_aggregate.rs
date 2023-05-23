// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

use immutable_feed::Feed;

use super::Aggregate;
use crate::applicable::EventApplicable;
use crate::events::FeedAggregateEvent;
use serde::Serialize;
use std::collections::HashMap;
use uuid::Uuid;

/// The feed aggregate, representing the total result of events in-memory.
#[derive(Default, Serialize, Clone, Debug)]
pub struct FeedAggregate {
    /// The id of the aggregate
    pub stream_id: Uuid,

    /// The version of the aggregate.
    pub version: i64,

    /// The feeds associated with the aggregate by feed id.
    pub feeds_by_feed_id: HashMap<Uuid, Feed>,
}

impl FeedAggregate {
    /// Make a new feed aggregate.
    pub fn new() -> FeedAggregate {
        FeedAggregate::with_id(Uuid::new_v4())
    }
}

impl Aggregate for FeedAggregate {
    fn with_id(uuid: Uuid) -> FeedAggregate {
        FeedAggregate {
            stream_id: uuid,
            version: 0,
            feeds_by_feed_id: HashMap::new(),
        }
    }

    fn version(&self) -> i64 {
        self.version
    }

    fn id(&self) -> Uuid {
        self.stream_id
    }
}

impl EventApplicable for FeedAggregate {
    type Event = FeedAggregateEvent;

    fn apply(self, event: FeedAggregateEvent) -> FeedAggregate {
        self.apply_many(vec![event])
    }

    fn apply_many(self, events: Vec<FeedAggregateEvent>) -> FeedAggregate {
        let mut applied = self.clone();
        for event in events.into_iter() {
            match event {
                FeedAggregateEvent::Added {
                    stream_id: _,
                    feed_id,
                    url,
                    metadata,
                    entries,
                } => {
                    // TODO: also look up by url if feed_id fails.
                    let found_feed = applied.feeds_by_feed_id.get(&feed_id);
                    match found_feed {
                        Some(_) => unreachable!(),
                        None => {
                            let raw_url = url.to_string();
                            let new_feed = Feed {
                                url: Some(url),
                                raw_url,
                                entries,
                                metadata,
                            };
                            applied.feeds_by_feed_id.insert(feed_id.clone(), new_feed);
                        }
                    };
                }
                FeedAggregateEvent::ObservedChanges {
                    stream_id: _,
                    feed_id,
                    changes,
                } => {
                    let feed = applied.feeds_by_feed_id.get_mut(&feed_id).unwrap();
                    *feed = feed.apply_diff(changes);
                }
            }
        }
        applied.version += 1;
        applied
    }
}

#[cfg(test)]
mod tests {
    use immutable_feed::FeedMetadata;
    use url::Url;

    use super::*;
    #[test]
    fn creates_new_aggregate() {
        let uuid = Uuid::new_v4();
        let aggregate = FeedAggregate::with_id(uuid.clone());
        assert_eq!(uuid, aggregate.stream_id);
        assert_eq!(0, aggregate.version);
    }

    #[test]
    fn works_with_sentinel_uuid() {
        let uuid = Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap();
        let aggregate = FeedAggregate::with_id(uuid.clone());
        assert_eq!(uuid, aggregate.stream_id);
        assert_eq!(0, aggregate.version);
    }

    #[test]
    fn adds_item() {
        let aggregate = FeedAggregate::new();
        let feed_id = Uuid::new_v4();
        let event = FeedAggregateEvent::Added {
            stream_id: aggregate.stream_id,
            feed_id: feed_id.clone(),
            url: Url::parse("http://example.com/examplecast.xml").unwrap(),
            metadata: FeedMetadata {
                title: String::from("Examplecast"),
                authors: vec!["Cindy Yoo-Hoo".to_string()],
                logo: None,
                description: None,
                updated_at: None,
                published_at: None,
            },
            entries: vec![],
        };
        let aggregate_after_application = aggregate.apply(event);
        assert_eq!(aggregate_after_application.feeds_by_feed_id.len(), 1);
        assert_eq!(aggregate_after_application.version(), 1);
        assert_eq!(
            aggregate_after_application.feeds_by_feed_id[&feed_id]
                .metadata
                .title,
            String::from("Examplecast")
        );
    }
}
