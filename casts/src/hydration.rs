//! Rebuilding `FeedAggregate` from the commit store, using snapshots.
//!
//! commit-store stores a snapshot as an ordinary commit flagged `snapshot`
//! whose `events` payload is a materialized aggregate rather than events to
//! apply. It does not interpret that payload -- the encoding is ours, and
//! this module is the only place that knows it.
//!
//! Why this exists: every read RPC used to replay the stream from version
//! zero. At 507 commits that measured 66 seconds cold, which is past the
//! client's own deadline -- the app would report "could not reach the casts
//! server" against a server that was merely slow. Snapshots turn that into
//! one indexed lookup plus the tail.

use anyhow::{Context, Result};
use commit_store::commit::{CommitMetadata, StreamMetadata};
use commit_store::store::Hydration;
use commit_store::CommitAttempt;
use uuid::Uuid;

use crate::aggregate::FeedAggregate;
use crate::applicable::EventApplicable;
use crate::events::FeedAggregateEvent;

/// Write a new snapshot once the tail above the last one reaches this many
/// commits. Small enough that a cold read stays fast, large enough that the
/// snapshotter is not writing a full aggregate on every poll.
pub const SNAPSHOT_AFTER_COMMITS: usize = 200;

/// Rebuild the aggregate from a snapshot (if any) plus the commits above it.
///
/// The version is taken from the hydration rather than accumulated:
/// `apply_many` bumps `version` once per call, which only tracks the stream
/// while every commit is replayed. Starting from a snapshot skips commits, so
/// the counter would be short by exactly the number the snapshot stands in
/// for -- and since writers derive the next stream version from
/// `aggregate.version + 1`, an under-counted version means every subsequent
/// write collides with history.
pub fn aggregate_from_hydration(hydration: Hydration) -> Result<FeedAggregate> {
    let mut aggregate = match hydration.snapshot {
        Some(ref commit) => serde_json::from_value(commit.events.clone())
            .context("stored snapshot failed to deserialize")?,
        None => FeedAggregate::default(),
    };

    for commit in &hydration.commits {
        let events: Vec<FeedAggregateEvent> = serde_json::from_value(commit.events.clone())
            .context("stored events failed to deserialize")?;
        aggregate = aggregate.apply_many(events);
    }

    aggregate.version = hydration.stream_version().unwrap_or(0);
    Ok(aggregate)
}

/// `aggregate_from_hydration`, off the async runtime's worker threads.
///
/// The fold is CPU-bound and unbounded in the size of the library: it
/// deserializes every commit's payload and folds it, which measured 66
/// seconds at 507 commits. An `async fn` that spends 66 seconds computing
/// never yields, so it does not merely make itself slow -- it occupies a
/// runtime worker and starves everything sharing it. That is what took
/// production down: the archival worker's hydration blocked the gRPC health
/// endpoint past the 5-second probe timeout, ECS killed the task for failing
/// its health check, and the replacement did the same thing.
///
/// `spawn_blocking` puts the work on the blocking pool, where a thread that
/// never yields is expected and harmless.
pub async fn aggregate_from_hydration_off_thread(hydration: Hydration) -> Result<FeedAggregate> {
    tokio::task::spawn_blocking(move || aggregate_from_hydration(hydration))
        .await
        .context("hydration task panicked")?
}

/// `snapshot_attempt`, off the async runtime's worker threads.
///
/// Serializing the whole aggregate is the same class of CPU-bound work as
/// the fold, and runs on the write path.
pub async fn snapshot_attempt_off_thread(
    stream_id: Uuid,
    aggregate: FeedAggregate,
) -> Result<CommitAttempt> {
    tokio::task::spawn_blocking(move || snapshot_attempt(stream_id, &aggregate))
        .await
        .context("snapshot task panicked")?
}

/// Build the commit that snapshots `aggregate` at the next stream version.
///
/// A snapshot takes a version of its own, so it goes through the same
/// optimistic-concurrency path as any other write: a racing writer gets the
/// usual conflict and rebases -- off the snapshot it just lost to, which is
/// cheaper than what it was doing before.
pub fn snapshot_attempt(stream_id: Uuid, aggregate: &FeedAggregate) -> Result<CommitAttempt> {
    let next_version = aggregate.version + 1;
    Ok(CommitAttempt {
        stream_metadata: StreamMetadata {
            stream_id,
            stream_version: next_version,
        },
        commit_metadata: CommitMetadata {
            commit_id: Uuid::new_v4(),
            commit_sequence: next_version,
            commit_timestamp: chrono::Utc::now(),
        },
        metadata: serde_json::json!({ "Snapshot": { "through_version": aggregate.version } }),
        // The payload IS the aggregate; commit-store stores it opaquely.
        events: serde_json::to_value(aggregate).context("aggregate failed to serialize")?,
        // Not an event count: a snapshot carries no events to apply.
        events_count: 0,
        snapshot: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use commit_store::Commit;
    use immutable_feed::{Feed, FeedMetadata};
    use url::Url;

    use immutable_feed::Entry;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    fn entry(guid: &str) -> Entry {
        Entry {
            guid: String::from(guid),
            url: Url::parse("http://example.com/e.mp3").unwrap(),
            // Bulk, so the fold costs what a real one costs: the payload is
            // dominated by per-entry text, not by the number of commits.
            title: Some(format!("Episode {guid} with a reasonably long title")),
            updated_at: None,
            published_at: None,
            content_digest: Some("d".repeat(64)),
            links: vec![],
            media: vec![],
        }
    }

    /// A hydration whose fold takes long enough to observe.
    fn slow_hydration(stream_id: Uuid, commits: usize, entries_each: usize) -> Hydration {
        let made: Vec<Commit> = (0..commits)
            .map(|i| {
                let events = serde_json::json!([{
                    "Added": {
                        "stream_id": stream_id,
                        "feed_id": Uuid::new_v4(),
                        "url": format!("http://example.com/{i}.xml"),
                        "metadata": feed("F").metadata,
                        "entries": (0..entries_each)
                            .map(|j| entry(&format!("{i}-{j}")))
                            .collect::<Vec<_>>(),
                        "do_not_archive": null,
                    }
                }]);
                commit_with(stream_id, i as i64 + 1, events, false)
            })
            .collect();
        Hydration {
            snapshot: None,
            commits: made,
        }
    }

    fn feed(title: &str) -> Feed {
        let url = Url::parse("http://example.com/f.xml").unwrap();
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

    fn commit_with(
        stream_id: Uuid,
        version: i64,
        events: serde_json::Value,
        snapshot: bool,
    ) -> Commit {
        let mut attempt = CommitAttempt {
            stream_metadata: StreamMetadata {
                stream_id,
                stream_version: version,
            },
            commit_metadata: CommitMetadata {
                commit_id: Uuid::new_v4(),
                commit_sequence: version,
                commit_timestamp: chrono::Utc::now(),
            },
            metadata: serde_json::Value::Null,
            events,
            events_count: 1,
            snapshot: false,
        };
        attempt.snapshot = snapshot;
        attempt.into_commit(version).unwrap()
    }

    fn added_event(stream_id: Uuid, feed_id: Uuid, title: &str) -> serde_json::Value {
        serde_json::to_value(vec![FeedAggregateEvent::Added {
            stream_id,
            feed_id,
            url: Url::parse("http://example.com/f.xml").unwrap(),
            metadata: feed(title).metadata,
            entries: vec![],
            do_not_archive: None,
        }])
        .unwrap()
    }

    /// With no snapshot the behaviour must match the old full replay.
    #[test]
    fn hydrates_from_events_when_there_is_no_snapshot() {
        let stream_id = Uuid::new_v4();
        let feed_id = Uuid::new_v4();
        let hydration = Hydration {
            snapshot: None,
            commits: vec![commit_with(
                stream_id,
                1,
                added_event(stream_id, feed_id, "A"),
                false,
            )],
        };
        let aggregate = aggregate_from_hydration(hydration).unwrap();
        assert_eq!(aggregate.feeds_by_feed_id.len(), 1);
        assert_eq!(aggregate.version, 1);
    }

    /// The snapshot payload stands in for everything below it: the feed it
    /// records must be present even though no event for it was replayed.
    #[test]
    fn hydrates_from_a_snapshot_without_replaying_its_events() {
        let stream_id = Uuid::new_v4();
        let feed_id = Uuid::new_v4();

        let mut snapshotted = FeedAggregate::default();
        snapshotted
            .feeds_by_feed_id
            .insert(feed_id, feed("From snapshot"));
        snapshotted.version = 90;

        let hydration = Hydration {
            snapshot: Some(commit_with(
                stream_id,
                90,
                serde_json::to_value(&snapshotted).unwrap(),
                true,
            )),
            commits: vec![],
        };
        let aggregate = aggregate_from_hydration(hydration).unwrap();
        assert_eq!(aggregate.feeds_by_feed_id.len(), 1);
        assert_eq!(
            aggregate.feeds_by_feed_id[&feed_id].metadata.title,
            "From snapshot"
        );
        assert_eq!(aggregate.version, 90);
    }

    /// The version must come from the stream, not from counting `apply_many`
    /// calls -- otherwise a snapshot-based hydration reports a version far
    /// below the stream's real one and every later write collides.
    #[test]
    fn version_tracks_the_stream_not_the_number_of_applies() {
        let stream_id = Uuid::new_v4();
        let snapshot_feed = Uuid::new_v4();
        let later_feed = Uuid::new_v4();

        let mut snapshotted = FeedAggregate::default();
        snapshotted
            .feeds_by_feed_id
            .insert(snapshot_feed, feed("Old"));
        snapshotted.version = 90;

        let hydration = Hydration {
            snapshot: Some(commit_with(
                stream_id,
                90,
                serde_json::to_value(&snapshotted).unwrap(),
                true,
            )),
            // Two commits above the snapshot: the stream is at 92.
            commits: vec![
                commit_with(
                    stream_id,
                    91,
                    added_event(stream_id, later_feed, "New"),
                    false,
                ),
                commit_with(
                    stream_id,
                    92,
                    serde_json::to_value(Vec::<FeedAggregateEvent>::new()).unwrap(),
                    false,
                ),
            ],
        };
        let aggregate = aggregate_from_hydration(hydration).unwrap();

        assert_eq!(aggregate.feeds_by_feed_id.len(), 2, "snapshot + tail");
        assert_eq!(
            aggregate.version, 92,
            "version must be the stream's, so the next write is 93"
        );
    }

    /// A snapshot must round-trip: what `snapshot_attempt` writes is what
    /// `aggregate_from_hydration` reads back.
    #[test]
    fn snapshot_attempt_round_trips() {
        let stream_id = Uuid::new_v4();
        let feed_id = Uuid::new_v4();
        let mut original = FeedAggregate::default();
        original
            .feeds_by_feed_id
            .insert(feed_id, feed("Round trip"));
        original.version = 41;

        let attempt = snapshot_attempt(stream_id, &original).unwrap();
        assert!(attempt.snapshot, "must be flagged, or hydrate ignores it");
        assert_eq!(attempt.stream_metadata.stream_version, 42);

        let hydration = Hydration {
            snapshot: Some(attempt.clone().into_commit(42).unwrap()),
            commits: vec![],
        };
        let restored = aggregate_from_hydration(hydration).unwrap();
        assert_eq!(restored.feeds_by_feed_id.len(), 1);
        assert_eq!(
            restored.feeds_by_feed_id[&feed_id].metadata.title,
            "Round trip"
        );
        assert_eq!(restored.version, 42, "the snapshot occupies version 42");
    }

    /// The fold must not occupy an async runtime worker.
    ///
    /// This is the defect that took production down, so it is asserted
    /// directly rather than through the behaviour it caused. The runtime has
    /// exactly one worker thread and BOTH the fold and the heartbeat are
    /// spawned as tasks onto it -- that pairing is what makes starvation
    /// observable. Running the fold inside `block_on` instead would prove
    /// nothing: `block_on` executes on the calling thread, so it would never
    /// contend with the worker the heartbeat lives on.
    ///
    /// The real casualty was the gRPC health endpoint, which ECS polls with
    /// a 5-second timeout and kills the task after three failures.
    ///
    /// Falsifiable: swap `aggregate_from_hydration_off_thread` for the plain
    /// `aggregate_from_hydration` and the heartbeat count collapses to 1.
    #[test]
    fn hydration_does_not_starve_the_runtime() {
        let stream_id = Uuid::new_v4();
        let hydration = slow_hydration(stream_id, 120, 120);

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("test runtime");

        let (ticks, elapsed, feeds) = runtime.block_on(async move {
            let ticks = Arc::new(AtomicUsize::new(0));
            let counter = Arc::clone(&ticks);
            let beating = Arc::new(AtomicUsize::new(1));
            let still_beating = Arc::clone(&beating);
            let heartbeat = tokio::spawn(async move {
                while still_beating.load(Ordering::Relaxed) == 1 {
                    counter.fetch_add(1, Ordering::Relaxed);
                    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
                }
            });
            // Let the heartbeat reach its first await, so the count below
            // measures the fold's effect and not the order tasks started in.
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            let before = ticks.load(Ordering::Relaxed);

            // Spawned, not awaited inline: the fold has to be a task on the
            // same single worker as the heartbeat for this to test anything.
            let start = std::time::Instant::now();
            let folding =
                tokio::spawn(async move { aggregate_from_hydration_off_thread(hydration).await });
            let aggregate = folding.await.expect("fold task").expect("hydration");
            let elapsed = start.elapsed();

            beating.store(0, Ordering::Relaxed);
            let _ = heartbeat.await;
            (
                ticks.load(Ordering::Relaxed) - before,
                elapsed,
                aggregate.feeds_by_feed_id.len(),
            )
        });

        assert_eq!(feeds, 120);
        // The fold must actually have been slow, or this proves nothing.
        assert!(
            elapsed >= std::time::Duration::from_millis(50),
            "fold finished in {elapsed:?}; too fast for this test to mean anything"
        );
        // A free worker at 2ms per tick manages far more than this. Starved,
        // it manages none: the heartbeat cannot run again until the fold ends.
        assert!(
            ticks >= 8,
            "heartbeat ticked {ticks} times during a {elapsed:?} fold; \
             the fold is occupying the runtime worker"
        );
    }
}
