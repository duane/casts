//! Feed reconciliation and audio archival, shared by the `poll` CLI and the
//! background worker that runs inside `casts-grpc-server`.
//!
//! This lives in the library rather than in a binary because both callers
//! need it and the previous copy in `bin/casts.rs` could not be reused --
//! the same reason `CastsImpl` and `commit_store_client` were moved here.

use anyhow::{anyhow, Context, Result};
use commit_store::{
    commit::{CommitMetadata, StreamMetadata},
    domain_to_grpc_commit_attempt,
    grpc_interface::{commit_store_client::CommitStoreClient, StreamCursor},
    grpc_to_domain_commit, CommitAttempt,
};
use futures::StreamExt;
use immutable_feed::{fetch_feed, Entry};
use tonic::transport::Channel;
use uuid::Uuid;

use crate::aggregate::FeedAggregate;
use crate::applicable::EventApplicable;
use crate::commands::{Command, FeedAggregateCommand};
use crate::events::FeedAggregateEvent;

/// How many times a commit is retried against a moved-on stream before the
/// item is abandoned for this pass.
const MAX_REBASE_ATTEMPTS: usize = 3;

/// Outcome of one reconciliation pass.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct RefreshReport {
    /// Feeds that produced a commit.
    pub changed: usize,
    /// Feeds that were already up to date.
    pub unchanged: usize,
    /// Feeds that could not be refreshed, with the reason.
    pub failures: Vec<(Uuid, String)>,
}

impl RefreshReport {
    #[must_use]
    pub fn ok(&self) -> bool {
        self.failures.is_empty()
    }
}

/// Rebuild the aggregate from the stream's whole history.
///
/// `limit: i64::MAX` is the *page* size, which the server clamps to its own
/// maximum and then pages through with a cursor -- so this reads everything,
/// not the first page.
pub async fn hydrate(
    client: &mut CommitStoreClient<Channel>,
    stream_id: &Uuid,
) -> Result<FeedAggregate> {
    let mut commits = client
        .stream_stream_range(StreamCursor {
            stream_id: stream_id.to_string(),
            min_version: 0,
            max_version: i64::MAX,
            limit: i64::MAX,
            reverse: false,
        })
        .await?
        .into_inner();

    let mut aggregate = FeedAggregate::default();
    while let Some(commit) = commits.next().await {
        let commit = commit?;
        aggregate = aggregate.apply_many(serde_json::from_slice(commit.events.as_slice())?);
    }
    Ok(aggregate)
}

/// Commit `command`'s events, re-hydrating and recomputing if the stream has
/// moved underneath us.
///
/// Optimistic concurrency means a competing writer makes this attempt stale;
/// the store rejects it rather than corrupting the stream. Retrying against
/// fresh state is the intended remedy -- the previous code called `unwrap()`
/// here and panicked instead.
async fn commit_with_rebase(
    client: &mut CommitStoreClient<Channel>,
    stream_id: &Uuid,
    mut aggregate: FeedAggregate,
    build: impl Fn(&FeedAggregate) -> Option<FeedAggregateCommand>,
) -> Result<(FeedAggregate, bool)> {
    for attempt in 1..=MAX_REBASE_ATTEMPTS {
        let Some(command) = build(&aggregate) else {
            return Ok((aggregate, false));
        };
        let events = command
            .async_apply(&aggregate)
            .await
            .map_err(|err| anyhow!("command rejected: {err:?}"))?;
        if events.is_empty() {
            return Ok((aggregate, false));
        }

        let attempt_commit = CommitAttempt {
            stream_metadata: StreamMetadata {
                stream_id: *stream_id,
                stream_version: aggregate.version + 1,
            },
            commit_metadata: CommitMetadata {
                commit_id: Uuid::new_v4(),
                commit_sequence: aggregate.version + 1,
                commit_timestamp: chrono::Utc::now(),
            },
            metadata: serde_json::to_value(&command)?,
            events: serde_json::to_value(&events)?,
            events_count: events.len() as i64,
        };

        match client
            .append_commit(domain_to_grpc_commit_attempt(&attempt_commit))
            .await
        {
            Ok(response) => {
                let committed = grpc_to_domain_commit(response.get_ref());
                let applied: Vec<FeedAggregateEvent> = serde_json::from_value(committed.events)?;
                return Ok((aggregate.apply_many(applied), true));
            }
            // AlreadyExists is the store's typed conflict signal: someone
            // else wrote first, so rebuild and recompute rather than fail.
            Err(status) if status.code() == tonic::Code::AlreadyExists => {
                log::info!(
                    "commit conflict on attempt {attempt}/{MAX_REBASE_ATTEMPTS} ({}); rebasing",
                    status.message()
                );
                aggregate = hydrate(client, stream_id).await?;
            }
            Err(status) => return Err(anyhow!("append_commit failed: {status}")),
        }
    }
    Err(anyhow!(
        "gave up after {MAX_REBASE_ATTEMPTS} rebase attempts"
    ))
}

/// Fetch every feed in the stream and commit whatever changed.
///
/// A feed that cannot be fetched or committed is recorded and skipped; one
/// unreachable podcast host must not stop the rest from updating.
pub async fn refresh_feeds(
    client: &mut CommitStoreClient<Channel>,
    stream_id: &Uuid,
) -> Result<RefreshReport> {
    let mut aggregate = hydrate(client, stream_id).await?;
    let mut report = RefreshReport::default();

    for (feed_id, existing) in aggregate.feeds_by_feed_id.clone() {
        // `Feed.url` is optional precisely because a feed's URL may fail to
        // parse; `raw_url` keeps the original. Skipping beats panicking.
        let Some(url) = existing.url.clone() else {
            report.failures.push((
                feed_id,
                format!("no parseable url (raw: {})", existing.raw_url),
            ));
            continue;
        };

        let observed = match fetch_feed(&url).await {
            Ok(feed) => feed,
            Err(err) => {
                report
                    .failures
                    .push((feed_id, format!("fetch failed: {err}")));
                continue;
            }
        };

        // No pre-diff here: `ObserveFeed` computes exactly this comparison to
        // build its event, and doing it twice both wasted the work and
        // reported a number that came from a different computation than the
        // one committed.
        let result = commit_with_rebase(client, stream_id, aggregate.clone(), |current| {
            // After a rebase the feed may be gone from the stream; building
            // no command tells `commit_with_rebase` to stop rather than
            // committing against state that no longer holds.
            if !current.feeds_by_feed_id.contains_key(&feed_id) {
                return None;
            }
            Some(FeedAggregateCommand::ObserveFeed {
                stream_id: *stream_id,
                feed_id,
                feed: observed.clone(),
            })
        })
        .await;

        match result {
            Ok((next, committed)) => {
                aggregate = next;
                if committed {
                    report.changed += 1;
                } else {
                    report.unchanged += 1;
                }
            }
            Err(err) => report.failures.push((feed_id, err.to_string())),
        }
    }
    Ok(report)
}

/// An entry with downloadable audio that has not been archived yet.
#[derive(Debug, Clone)]
pub struct PendingArchive {
    pub feed_id: Uuid,
    pub entry_guid: String,
    pub audio_url: url::Url,
    pub content_type: Option<String>,
}

/// The first audio media URL on an entry, if it has one.
fn audio_source(entry: &Entry) -> Option<(url::Url, Option<String>)> {
    entry
        .media
        .iter()
        .flat_map(|m| m.content.iter())
        .find_map(|c| {
            let url = c.url.clone()?;
            let content_type = c.content_type.clone();
            match content_type {
                Some(ref t) if t.starts_with("audio/") => Some((url, content_type)),
                // Some feeds omit the type on an enclosure that is plainly audio;
                // an entry's media object is audio by convention in a podcast
                // feed, so a missing type is accepted rather than skipped.
                None => Some((url, None)),
                Some(_) => None,
            }
        })
}

/// Entries with audio and no archive record, oldest feed order, capped.
#[must_use]
pub fn pending_archives(aggregate: &FeedAggregate, limit: usize) -> Vec<PendingArchive> {
    let mut pending = Vec::new();
    for (feed_id, feed) in &aggregate.feeds_by_feed_id {
        for entry in &feed.entries {
            if pending.len() >= limit {
                return pending;
            }
            if aggregate
                .archived_by_feed_and_guid
                .contains_key(&(feed_id.to_string(), entry.guid.clone()))
            {
                continue;
            }
            if let Some((audio_url, content_type)) = audio_source(entry) {
                pending.push(PendingArchive {
                    feed_id: *feed_id,
                    entry_guid: entry.guid.clone(),
                    audio_url,
                    content_type,
                });
            }
        }
    }
    pending
}

/// Deterministic object key for one entry's audio.
///
/// The guid is hashed because it is arbitrary publisher text -- frequently a
/// URL, sometimes containing characters that make hostile S3 keys. Being
/// deterministic is what makes a retried upload overwrite rather than
/// duplicate, and therefore what makes the upload step idempotent.
#[must_use]
pub fn archive_key(stream_id: &Uuid, feed_id: &Uuid, entry_guid: &str) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(entry_guid.as_bytes());
    format!("{stream_id}/{feed_id}/{digest:x}.audio")
}

/// Record an archived entry in the commit store.
pub async fn record_archived(
    client: &mut CommitStoreClient<Channel>,
    stream_id: &Uuid,
    pending: &PendingArchive,
    bucket: &str,
    key: &str,
    size_bytes: i64,
) -> Result<bool> {
    let aggregate = hydrate(client, stream_id).await?;
    let (_, committed) = commit_with_rebase(client, stream_id, aggregate, |_| {
        Some(FeedAggregateCommand::ArchiveEntry {
            stream_id: *stream_id,
            feed_id: pending.feed_id,
            entry_guid: pending.entry_guid.clone(),
            bucket: bucket.to_string(),
            key: key.to_string(),
            content_type: pending.content_type.clone(),
            size_bytes,
        })
    })
    .await
    .with_context(|| format!("recording archive for {}", pending.entry_guid))?;
    Ok(committed)
}
