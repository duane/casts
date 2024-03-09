use crate::commit::{Commit, CommitAttempt};
use crate::CommitStream;
use crate::StoreError;
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait DispatchingAsyncClientStore: Send + AsyncClientStore {
    /// Receive updates with each new commit to the stream.
    async fn subscribe(&mut self) -> CommitStream;
    async fn subscribe_stream(&mut self, stream_id: &Uuid) -> CommitStream;
}

#[async_trait]
pub trait AsyncClientStore: Send {
    /// Write a commit to a store.
    ///
    /// If the commit succeeds, this will return an immutable commit representing a set of events that have occurred.
    async fn commit(
        &mut self,
        commit_attempt: &CommitAttempt,
    ) -> Result<Commit, Box<dyn StoreError>>;

    /// Get a sequence of events specific to the stream id.
    ///
    /// Because the minimum and maximum bounds are passed, you may get zero commits.
    /// Getting all commits between 0 and the latest commit id should retrieve all events associated with the aggregate.
    async fn get_stream_range(
        &mut self,
        stream_id: Uuid,
        min_version: i64,
        max_version: i64,
    ) -> Result<Vec<Commit>, Box<dyn StoreError>>;

    /// Get a sequence of events specific to the stream id.
    ///
    /// Because the minimum and maximum bounds are passed, you may get zero commits.
    /// Getting all commits between 0 and the latest commit id should retrieve all events associated with the aggregate.
    async fn get_range(
        &mut self,
        min_commit_number: i64,
        max_commit_number: i64,
    ) -> Result<Vec<Commit>, Box<dyn StoreError>>;

    /// Get a single commit.
    async fn get_commit(&mut self, commit_it: &Uuid) -> Result<Commit, Box<dyn StoreError>>;
}
