use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// An attempt at a commit. This potential commit has not been written yet to the store.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CommitAttempt {
    /// The new stream_id.
    pub stream_id: Uuid,

    /// The new stream version.
    pub stream_version: i64,

    /// The new commit id.
    pub commit_id: Uuid,

    /// The new commit timestamp.
    ///
    /// May be overwritten by a more recent timestamp before or at write time.
    pub commit_timestamp: DateTime<Utc>,

    /// The new commit sequence.
    pub commit_sequence: i64,

    /// The metadata of the commit.
    pub metadata: serde_json::Value,

    /// The events to commit.
    pub events: serde_json::Value,

    /// The number of events serialized.
    pub events_count: i64,
}

impl CommitAttempt {
    pub fn into_commit(self, commit_number: i64) -> Commit {
        Commit {
            stream_id: self.stream_id,
            stream_version: self.stream_version,
            commit_id: self.commit_id,
            commit_timestamp: self.commit_timestamp,
            commit_sequence: self.commit_sequence,
            commit_number,
            metadata: self.metadata,
            events: self.events,
            events_count: self.events_count,
            dispatched: false,
        }
    }
}

/// A commit with the events deserialized.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Commit {
    /// Stream id.
    pub stream_id: Uuid,

    /// Stream version.
    pub stream_version: i64,

    /// commit id.
    pub commit_id: Uuid,

    /// Commit timestamp
    pub commit_timestamp: DateTime<Utc>,

    /// Commit sequence number.
    pub commit_sequence: i64,

    /// Commit number.
    pub commit_number: i64,

    /// Events serialized.
    pub events: serde_json::Value,

    /// Metadata serialized
    pub metadata: serde_json::Value,

    /// Number of deserialized events.
    pub events_count: i64,

    /// Whether or not the commit has been dispatched.
    pub dispatched: bool,
}
