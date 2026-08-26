// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

use super::Event;
use immutable_feed::{Entry, FeedMetadata, FeedMutation};
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

#[allow(clippy::large_enum_variant)]
/// Aggregate events!
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum FeedAggregateEvent {
    /// A feed has been added to the database.
    Added {
        /// The aggregate ID.
        stream_id: Uuid,

        /// The feed ID.
        feed_id: Uuid,

        /// The url of the feed.
        url: Url,

        /// initial metadata found by the command.
        metadata: FeedMetadata,

        /// initial entries found by the command.
        entries: Vec<Entry>,
    },

    /// An event noting changes from a previously observed (or just added) feed.
    ObservedChanges {
        /// The stream in which the feed updates were seen.
        stream_id: Uuid,

        /// The feed that changed.
        feed_id: Uuid,

        /// The changes that occurred to the feed since last observation.
        changes: Vec<FeedMutation>,
    },
    /// Playback progress was recorded for one entry. Last write wins.
    FeedEntryListenedTo {
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

    /// An entry's audio was copied to durable storage.
    ///
    /// Emitted by the archival worker only after the upload is confirmed, so
    /// the presence of this event means the object exists. Records the bucket
    /// and key rather than a URL: an immutable event should not carry a
    /// presigned URL that expires, and bucket/key survives the archive later
    /// being fronted by a CDN or moved.
    EntryArchived {
        /// The stream holding the feed.
        stream_id: Uuid,

        /// The feed holding the entry.
        feed_id: Uuid,

        /// The entry whose audio was archived.
        entry_guid: String,

        /// The bucket the object was written to.
        bucket: String,

        /// The object key within that bucket.
        key: String,

        /// The content type reported by the origin, when it gave one.
        content_type: Option<String>,

        /// Size of the stored object in bytes.
        size_bytes: i64,
    },
}

impl Event for FeedAggregateEvent {}
