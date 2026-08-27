// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

//! Durable storage for entry show notes, held outside the event stream.
//!
//! `summary` and `content` were 86% of every entry (1,920 + 1,841 of 4,398
//! bytes, measured on one commit) and no command has ever read them:
//! deduplication is by guid, ordering by publication date, archival by media
//! URL. Carrying them made the unit that every commit, aggregate and
//! snapshot is built from about seven times larger than the decisions
//! required, which is what put commits past the size limits repeatedly.
//!
//! So an event carries a digest and the notes live here. Objects are
//! content-addressed, which makes the store idempotent for free: writing
//! unchanged notes writes identical bytes to the same key, and two entries
//! with the same notes share one object.
//!
//! ORDERING: store the notes, THEN commit the entry that references them.
//! The reverse leaves a committed digest that resolves to nothing, and a
//! commit cannot be taken back. A stored object that never gets referenced
//! is inert -- it is addressed by its own content, so a later write of the
//! same notes simply finds it already there. Same rule the archival worker
//! follows for audio: upload, then record.

use anyhow::{Context, Result};
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client as S3Client;
use immutable_feed::EntryContent;

/// Key prefix for stored notes, keeping them clear of archived audio in the
/// same bucket.
pub const CONTENT_PREFIX: &str = "content";

/// Show notes in durable storage, addressed by digest.
#[derive(Clone, Debug)]
pub struct ContentStore {
    s3: S3Client,
    bucket: String,
}

impl ContentStore {
    #[must_use]
    pub fn new(s3: S3Client, bucket: String) -> Self {
        ContentStore { s3, bucket }
    }

    /// The object key for a digest.
    #[must_use]
    pub fn key(digest: &str) -> String {
        format!("{CONTENT_PREFIX}/{digest}")
    }

    /// Store one entry's notes, returning the digest that identifies them.
    ///
    /// `Ok(None)` when there is nothing to store; the matching entry then
    /// carries no digest, so absence stays consistent on both sides.
    pub async fn put(&self, content: &EntryContent) -> Result<Option<String>> {
        let Some(digest) = content.digest() else {
            return Ok(None);
        };
        let body = serde_json::to_vec(content).context("entry content failed to serialize")?;
        self.s3
            .put_object()
            .bucket(&self.bucket)
            .key(Self::key(&digest))
            .content_type("application/json")
            .body(ByteStream::from(body))
            .send()
            .await
            .with_context(|| format!("could not store entry content {digest}"))?;
        Ok(Some(digest))
    }

    /// Store every non-empty item, returning how many objects were written.
    ///
    /// One failure fails the batch: the caller is about to commit entries
    /// that reference these digests, and committing after a partial store
    /// is what produces a dangling reference.
    pub async fn put_all(&self, contents: &[EntryContent]) -> Result<usize> {
        let mut stored = 0usize;
        for content in contents {
            if self.put(content).await?.is_some() {
                stored += 1;
            }
        }
        Ok(stored)
    }

    /// Read back the notes for a digest, or `None` when nothing is stored.
    ///
    /// A missing object is not an error: an entry committed before the
    /// content split carries no digest at all, and one whose backfill has
    /// not run yet resolves to nothing. Callers render an entry without
    /// notes rather than failing the read.
    pub async fn get(&self, digest: &str) -> Result<Option<EntryContent>> {
        let response = match self
            .s3
            .get_object()
            .bucket(&self.bucket)
            .key(Self::key(digest))
            .send()
            .await
        {
            Ok(response) => response,
            Err(err) => {
                // Distinguish "not stored" from "could not reach storage":
                // the first is normal, the second must not look like an
                // entry that simply has no notes.
                if err
                    .as_service_error()
                    .is_some_and(aws_sdk_s3::operation::get_object::GetObjectError::is_no_such_key)
                {
                    return Ok(None);
                }
                return Err(err).with_context(|| format!("could not read entry content {digest}"));
            }
        };
        let bytes = response
            .body
            .collect()
            .await
            .with_context(|| format!("could not read entry content body {digest}"))?
            .into_bytes();
        let content: EntryContent = serde_json::from_slice(&bytes)
            .with_context(|| format!("stored entry content {digest} failed to deserialize"))?;
        Ok(Some(content))
    }
}

/// Prepare a freshly parsed feed for a write that has never been committed
/// before: store every entry's notes, or strip every digest when there is
/// nowhere to put them.
///
/// Used where every entry in `fetched` is new (`AddFeed`, and the CLI import
/// paths that have no content store configured at all). `refresh_feeds`
/// does its own, narrower version of this: only entries that actually
/// changed need a decision, not the whole feed.
///
/// Stripping rather than committing a dangling digest is the point: an
/// entry with a digest that resolves to nothing is a bug a reader cannot
/// recover from, whereas an entry that plainly has no notes is normal and
/// self-heals -- the next `refresh_feeds` that runs with a content store
/// will see the digest go from absent to present and store it then.
pub async fn store_or_strip(
    fetched: &mut immutable_feed::FetchedFeed,
    content_store: Option<&ContentStore>,
) -> Result<()> {
    match content_store {
        Some(store) => {
            store.put_all(&fetched.contents).await?;
        }
        None => {
            for entry in &mut fetched.feed.entries {
                entry.content_digest = None;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use immutable_feed::Content;

    fn content(summary: Option<&str>) -> EntryContent {
        EntryContent {
            guid: String::from("guid-1"),
            summary: summary.map(String::from),
            content: None,
        }
    }

    #[test]
    fn the_key_is_the_digest_under_one_prefix() {
        assert_eq!(ContentStore::key("abc123"), "content/abc123");
    }

    /// Content addressing is what makes storing idempotent, so identical
    /// notes must land on identical keys regardless of which entry carried
    /// them -- the guid is deliberately not part of the digest.
    #[test]
    fn identical_notes_from_different_entries_share_a_key() {
        let mut a = content(Some("the same notes"));
        let mut b = content(Some("the same notes"));
        a.guid = String::from("guid-a");
        b.guid = String::from("guid-b");
        assert_eq!(a.digest(), b.digest());
        assert!(a.digest().is_some());
    }

    #[test]
    fn different_notes_get_different_keys() {
        assert_ne!(content(Some("one")).digest(), content(Some("two")).digest());
    }

    /// An entry with no notes must carry no digest, so that "no notes" and
    /// "notes not yet stored" cannot be confused on either side.
    #[test]
    fn empty_notes_have_no_digest() {
        assert_eq!(content(None).digest(), None);
        assert!(content(None).is_empty());
    }

    /// The `content` half counts too: an entry whose summary is empty but
    /// whose body is not still has notes worth storing.
    #[test]
    fn content_alone_still_produces_a_digest() {
        let mut only_body = content(None);
        only_body.content = Some(Content {
            body: Some(String::from("<p>notes</p>")),
            content_type: String::from("text/html"),
            length: None,
            src: None,
        });
        assert!(!only_body.is_empty());
        assert!(only_body.digest().is_some());
    }
}
