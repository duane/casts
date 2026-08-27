// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

use immutable_feed::Feed;

use super::Aggregate;
use crate::applicable::EventApplicable;
use crate::events::FeedAggregateEvent;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// The feed aggregate, representing the total result of events in-memory.
// Deserialize is required, not decorative: a snapshot commit stores the
// materialized aggregate and hydration reads it straight back.
#[derive(Default, Serialize, Deserialize, Clone, Debug)]
pub struct FeedAggregate {
    /// The id of the aggregate
    pub stream_id: Uuid,

    /// The version of the aggregate.
    pub version: i64,

    /// The feeds associated with the aggregate by feed id.
    pub feeds_by_feed_id: HashMap<Uuid, Feed>,

    /// Playtime
    pub playtime_by_feed_and_guid: HashMap<(String, String), (i64, i32)>,

    /// Where each entry's audio was archived, keyed like
    /// `playtime_by_feed_and_guid`. Absence means "not archived", which is
    /// what the archival worker selects on.
    pub archived_by_feed_and_guid: HashMap<(String, String), ArchivedAudio>,

    /// Feeds the archival worker must skip. Presence means "do not
    /// archive"; there is no separate `false` state to represent, since
    /// absence already means "archive normally".
    pub do_not_archive_feed_ids: HashSet<Uuid>,
}

/// The durable-storage location of one entry's audio.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchivedAudio {
    pub bucket: String,
    pub key: String,
    pub content_type: Option<String>,
    pub size_bytes: i64,
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
            playtime_by_feed_and_guid: HashMap::new(),
            archived_by_feed_and_guid: HashMap::new(),
            do_not_archive_feed_ids: HashSet::new(),
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
                    do_not_archive,
                } => {
                    // A duplicate Added is a defect in the history, not in
                    // this fold: skip the one event and keep going. The old
                    // `return self` here silently discarded every event
                    // already applied in this batch and the version bump.
                    if applied.feeds_by_feed_id.contains_key(&feed_id) {
                        log::warn!("feed {feed_id} already added; skipping duplicate Added event");
                        continue;
                    }
                    let raw_url = url.to_string();
                    let new_feed = Feed {
                        url: Some(url),
                        raw_url,
                        entries,
                        metadata,
                    };

                    let _ = applied.feeds_by_feed_id.insert(feed_id, new_feed);
                    if do_not_archive == Some(true) {
                        applied.do_not_archive_feed_ids.insert(feed_id);
                    }
                }
                FeedAggregateEvent::ObservedChanges {
                    stream_id: _,
                    feed_id,
                    changes,
                } => {
                    // Same policy as the duplicate Added above: corrupt
                    // history must not panic a read path.
                    let Some(feed) = applied.feeds_by_feed_id.get_mut(&feed_id) else {
                        log::warn!(
                            "ObservedChanges for unknown feed {feed_id}; skipping {} changes",
                            changes.len()
                        );
                        continue;
                    };
                    *feed = feed.clone().apply_diff(changes);
                }
                FeedAggregateEvent::EntryArchived {
                    stream_id: _,
                    feed_id,
                    entry_guid,
                    bucket,
                    key,
                    content_type,
                    size_bytes,
                } => {
                    applied.archived_by_feed_and_guid.insert(
                        (feed_id.to_string(), entry_guid),
                        ArchivedAudio {
                            bucket,
                            key,
                            content_type,
                            size_bytes,
                        },
                    );
                }
                FeedAggregateEvent::ArchivePreferenceSet {
                    stream_id: _,
                    feed_id,
                    do_not_archive,
                } => {
                    if do_not_archive {
                        applied.do_not_archive_feed_ids.insert(feed_id);
                    } else {
                        applied.do_not_archive_feed_ids.remove(&feed_id);
                    }
                }
                FeedAggregateEvent::FeedEntryListenedTo {
                    stream_id: _,
                    feed_id,
                    entry_guid,
                    elapsed_seconds,
                    elapsed_nanos,
                } => {
                    applied.playtime_by_feed_and_guid.insert(
                        (feed_id.to_string(), entry_guid),
                        (elapsed_seconds, elapsed_nanos),
                    );
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
        let aggregate = FeedAggregate::with_id(uuid);
        assert_eq!(uuid, aggregate.stream_id);
        assert_eq!(0, aggregate.version);
    }

    #[test]
    fn works_with_sentinel_uuid() {
        let uuid = Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap();
        let aggregate = FeedAggregate::with_id(uuid);
        assert_eq!(uuid, aggregate.stream_id);
        assert_eq!(0, aggregate.version);
    }

    fn added_event(stream_id: Uuid, feed_id: Uuid, title: &str) -> FeedAggregateEvent {
        FeedAggregateEvent::Added {
            stream_id,
            feed_id,
            url: Url::parse("http://example.com/examplecast.xml").unwrap(),
            metadata: FeedMetadata {
                title: String::from(title),
                authors: vec![],
                logo: None,
                description: None,
            },
            entries: vec![],
            do_not_archive: None,
        }
    }

    #[test]
    fn duplicate_added_does_not_abort_the_batch() {
        let aggregate = FeedAggregate::new();
        let stream_id = aggregate.stream_id;
        let feed_a = Uuid::new_v4();
        let feed_b = Uuid::new_v4();
        // The duplicate sits between two good events; both must apply.
        let applied = aggregate.apply_many(vec![
            added_event(stream_id, feed_a, "A"),
            added_event(stream_id, feed_a, "A duplicate"),
            added_event(stream_id, feed_b, "B"),
        ]);
        assert_eq!(applied.feeds_by_feed_id.len(), 2);
        assert_eq!(applied.feeds_by_feed_id[&feed_a].metadata.title, "A");
        assert_eq!(applied.feeds_by_feed_id[&feed_b].metadata.title, "B");
        assert_eq!(applied.version(), 1);
    }

    #[test]
    fn one_commit_advances_the_version_by_one() {
        let aggregate = FeedAggregate::new();
        let stream_id = aggregate.stream_id;
        // A commit carrying many events still advances the version by exactly
        // one; per-event application would disagree with this and corrupt the
        // optimistic-concurrency expectation derived from `version`.
        let applied = aggregate.apply_many(vec![
            added_event(stream_id, Uuid::new_v4(), "A"),
            added_event(stream_id, Uuid::new_v4(), "B"),
            added_event(stream_id, Uuid::new_v4(), "C"),
        ]);
        assert_eq!(applied.version(), 1);
        assert_eq!(applied.feeds_by_feed_id.len(), 3);
    }

    #[test]
    fn observed_changes_for_unknown_feed_is_skipped() {
        let aggregate = FeedAggregate::new();
        let stream_id = aggregate.stream_id;
        let feed_a = Uuid::new_v4();
        let applied = aggregate.apply_many(vec![
            FeedAggregateEvent::ObservedChanges {
                stream_id,
                feed_id: Uuid::new_v4(),
                changes: vec![],
            },
            added_event(stream_id, feed_a, "A"),
        ]);
        assert_eq!(applied.feeds_by_feed_id.len(), 1);
        assert_eq!(applied.version(), 1);
    }

    #[test]
    fn listened_to_keeps_the_last_write() {
        let aggregate = FeedAggregate::new();
        let stream_id = aggregate.stream_id;
        let feed_id = Uuid::new_v4();
        let listened = |seconds, nanos| FeedAggregateEvent::FeedEntryListenedTo {
            stream_id,
            feed_id,
            entry_guid: String::from("guid-1"),
            elapsed_seconds: seconds,
            elapsed_nanos: nanos,
        };
        let applied = aggregate
            .apply_many(vec![added_event(stream_id, feed_id, "A"), listened(10, 0)])
            .apply(listened(90, 500));
        assert_eq!(
            applied.playtime_by_feed_and_guid[&(feed_id.to_string(), String::from("guid-1"))],
            (90, 500)
        );
        assert_eq!(applied.version(), 2);
    }

    #[test]
    fn adds_item() {
        let aggregate = FeedAggregate::new();
        let feed_id = Uuid::new_v4();
        let event = FeedAggregateEvent::Added {
            stream_id: aggregate.stream_id,
            feed_id,
            url: Url::parse("http://example.com/examplecast.xml").unwrap(),
            metadata: FeedMetadata {
                title: String::from("Examplecast"),
                authors: vec!["Cindy Yoo-Hoo".to_string()],
                logo: None,
                description: None,
                // updated_at: None,
                // published_at: None,
            },
            entries: vec![],
            do_not_archive: None,
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

    /// `Added { do_not_archive: Some(true), .. }` must mark the feed from
    /// the moment it exists, and a plain `Added` (the shape of every event
    /// already committed before this field existed) must not.
    #[test]
    fn added_do_not_archive_sets_the_initial_preference() {
        let aggregate = FeedAggregate::new();
        let stream_id = aggregate.stream_id;
        let opted_out = Uuid::new_v4();
        let normal = Uuid::new_v4();

        let mut opted_out_event = added_event(stream_id, opted_out, "Opted out");
        if let FeedAggregateEvent::Added {
            ref mut do_not_archive,
            ..
        } = opted_out_event
        {
            *do_not_archive = Some(true);
        }

        let applied = aggregate.apply_many(vec![
            opted_out_event,
            added_event(stream_id, normal, "Normal"),
        ]);

        assert!(applied.do_not_archive_feed_ids.contains(&opted_out));
        assert!(!applied.do_not_archive_feed_ids.contains(&normal));
    }

    /// The preference must be reversible: setting it, then clearing it,
    /// must leave the feed archivable again -- not stuck in whatever the
    /// first event said.
    #[test]
    fn archive_preference_set_toggles_in_both_directions() {
        let aggregate = FeedAggregate::new();
        let feed_id = Uuid::new_v4();
        let stream_id = aggregate.stream_id;

        let applied = aggregate.apply_many(vec![
            added_event(stream_id, feed_id, "A"),
            FeedAggregateEvent::ArchivePreferenceSet {
                stream_id,
                feed_id,
                do_not_archive: true,
            },
        ]);
        assert!(applied.do_not_archive_feed_ids.contains(&feed_id));

        let applied = applied.apply(FeedAggregateEvent::ArchivePreferenceSet {
            stream_id,
            feed_id,
            do_not_archive: false,
        });
        assert!(!applied.do_not_archive_feed_ids.contains(&feed_id));
    }
}
