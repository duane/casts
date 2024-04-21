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
    FeedEntryListenedTo {
        /// The stream in which the feed updates were seen.
        stream_id: Uuid,

        /// The feed that changed.
        feed_id: Uuid,

        /// The entry that was listened to.
        entry_guid: String,

        /// The time the entry was listened to.
        elapsed_seconds: i64,

        /// The playtime of the entry.
        elapsed_nanos: i32,
    },
}

impl Event for FeedAggregateEvent {}
