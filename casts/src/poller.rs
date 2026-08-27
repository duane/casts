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

/// Entries per commit when adding a feed. `Added` and `AppendEntries` each
/// embed every entry they are given in one commit; commit-store's message
/// limit is 2 MB, and a large feed's full entry list can exceed that on its
/// own (Crime Junkie: 7.16 MB, observed live -- this constant exists
/// because that import failed before it did).
pub const ADD_FEED_ENTRY_BATCH_SIZE: usize = 100;

/// Add a feed, paginating its entries into commits of at most
/// `ADD_FEED_ENTRY_BATCH_SIZE` instead of one commit holding all of them.
///
/// The first commit (`AddFeed`) carries the feed's metadata and its first
/// batch of entries; every subsequent batch is a separate `AppendEntries`
/// commit. `AppendEntries`, not another `AddFeed` or `ObserveFeed`, is what
/// makes this safe: see its doc comment for why paginating through
/// `ObserveFeed`'s full-feed diff would have made every earlier batch look
/// deleted.
///
/// Takes and returns `aggregate` rather than hydrating internally, like
/// `commit_with_rebase`: a caller adding many feeds in one run (bulk
/// import) threads state forward across all of them instead of re-reading
/// the whole stream from scratch before every single feed.
pub async fn add_feed_paginated(
    client: &mut CommitStoreClient<Channel>,
    stream_id: &Uuid,
    aggregate: FeedAggregate,
    feed_id: Uuid,
    url: url::Url,
    feed: immutable_feed::Feed,
    do_not_archive: Option<bool>,
) -> Result<FeedAggregate> {
    let mut remaining = feed.entries.clone();
    let first_batch_len = remaining.len().min(ADD_FEED_ENTRY_BATCH_SIZE);
    let first_batch: Vec<_> = remaining.drain(..first_batch_len).collect();
    let mut first_feed = feed;
    first_feed.entries = first_batch;

    let (mut aggregate, _) = commit_with_rebase(client, stream_id, aggregate, |_| {
        Some(FeedAggregateCommand::AddFeed {
            stream_id: *stream_id,
            feed_id,
            url: url.clone(),
            feed: first_feed.clone(),
            do_not_archive,
        })
    })
    .await?;

    for chunk in remaining.chunks(ADD_FEED_ENTRY_BATCH_SIZE) {
        let chunk = chunk.to_vec();
        let (next, _) = commit_with_rebase(client, stream_id, aggregate, |_| {
            Some(FeedAggregateCommand::AppendEntries {
                stream_id: *stream_id,
                feed_id,
                entries: chunk.clone(),
            })
        })
        .await?;
        aggregate = next;
    }
    Ok(aggregate)
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
        // Checked once per feed, not per entry: cheaper, and it is the
        // feed-level preference from `ArchivePreferenceSet`/`Added` being
        // respected, not a per-entry one.
        if aggregate.do_not_archive_feed_ids.contains(feed_id) {
            continue;
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::applicable::EventApplicable;
    use immutable_feed::{Entry, Feed, FeedMetadata, MediaContent, MediaObject};
    use url::Url;

    fn feed_with_audio_entry(title: &str) -> Feed {
        let entry = Entry {
            guid: format!("{title}-episode-1"),
            url: Url::parse("http://example.com/episode-1").unwrap(),
            title: Some(String::from("Episode 1")),
            summary: None,
            updated_at: None,
            published_at: None,
            content: None,
            links: vec![],
            media: vec![MediaObject {
                title: None,
                content: vec![MediaContent {
                    url: Url::parse("http://example.com/episode-1.mp3").ok(),
                    content_type: Some(String::from("audio/mpeg")),
                    height: None,
                    width: None,
                    size: None,
                }],
                duration: None,
                description: None,
            }],
        };
        Feed {
            url: Url::parse("http://example.com/feed.xml").ok(),
            raw_url: String::from("http://example.com/feed.xml"),
            metadata: FeedMetadata {
                title: String::from(title),
                authors: vec![],
                logo: None,
                description: None,
            },
            entries: vec![entry],
        }
    }

    /// The archiver must never touch a feed's audio once it is marked "do
    /// not archive" -- checked here at the selection step (`pending_archives`)
    /// rather than only trusting the aggregate flag exists, since a bug in
    /// the filter would silently archive opted-out feeds with no error
    /// anywhere.
    #[test]
    fn pending_archives_skips_do_not_archive_feeds() {
        let aggregate = FeedAggregate::new();
        let stream_id = aggregate.stream_id;
        let opted_out = Uuid::new_v4();
        let normal = Uuid::new_v4();

        let opted_out_feed = feed_with_audio_entry("Opted out");
        let normal_feed = feed_with_audio_entry("Normal");

        let aggregate = aggregate.apply_many(vec![
            FeedAggregateEvent::Added {
                stream_id,
                feed_id: opted_out,
                url: opted_out_feed.url.clone().unwrap(),
                metadata: opted_out_feed.metadata.clone(),
                entries: opted_out_feed.entries.clone(),
                do_not_archive: Some(true),
            },
            FeedAggregateEvent::Added {
                stream_id,
                feed_id: normal,
                url: normal_feed.url.clone().unwrap(),
                metadata: normal_feed.metadata.clone(),
                entries: normal_feed.entries.clone(),
                do_not_archive: None,
            },
        ]);

        let pending = pending_archives(&aggregate, 10);
        let pending_feed_ids: Vec<Uuid> = pending.iter().map(|p| p.feed_id).collect();

        assert!(
            !pending_feed_ids.contains(&opted_out),
            "an opted-out feed's audio must never be selected for archiving"
        );
        assert!(
            pending_feed_ids.contains(&normal),
            "a feed with no preference set must still be archived normally"
        );
    }

    /// Reversing the preference (`ArchivePreferenceSet { do_not_archive:
    /// false, .. }`) must make the feed's entries archivable again -- not
    /// leave it stuck opted out.
    #[test]
    fn pending_archives_resumes_after_the_preference_is_cleared() {
        let aggregate = FeedAggregate::new();
        let stream_id = aggregate.stream_id;
        let feed_id = Uuid::new_v4();
        let feed = feed_with_audio_entry("Resumed");

        let aggregate = aggregate.apply_many(vec![
            FeedAggregateEvent::Added {
                stream_id,
                feed_id,
                url: feed.url.clone().unwrap(),
                metadata: feed.metadata.clone(),
                entries: feed.entries.clone(),
                do_not_archive: Some(true),
            },
            FeedAggregateEvent::ArchivePreferenceSet {
                stream_id,
                feed_id,
                do_not_archive: false,
            },
        ]);

        let pending = pending_archives(&aggregate, 10);
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].feed_id, feed_id);
    }
}
