// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

use crate::{
    grpc_interface::{
        commit_store_client::CommitStoreClient, Commit as GrpcCommit,
        CommitAttempt as GrpcCommitAttempt, StreamCursor,
    },
    AsyncClientStore, CommitStream, DispatchingAsyncClientStore, StoreError,
};
use crate::{Commit, CommitAttempt};
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use prost_types::Timestamp as GrpcTimestamp;
use uuid::Uuid;

// Grpc Async Client

type InnerGrpcClient = CommitStoreClient<tonic::transport::Channel>;

/// A store that works over GRPC.
#[derive(Clone, Debug)]
pub struct GrpcClient {
    client: InnerGrpcClient,
}

impl GrpcClient {
    pub fn new(client: InnerGrpcClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl AsyncClientStore for GrpcClient {
    /// Write a commit to a store.
    ///
    /// If the commit succeeds, this will return an immutable commit representing a set of events that have occurred.
    async fn commit(
        &mut self,
        commit_attempt: &CommitAttempt,
    ) -> Result<Commit, Box<dyn StoreError>> {
        let grpc_commit_attempt = domain_to_grpc_commit_attempt(commit_attempt);
        let domain_commit = grpc_to_domain_commit(
            self.client
                .append_commit(grpc_commit_attempt)
                .await
                .unwrap()
                .get_ref(),
        );
        Ok(domain_commit)
    }

    /// Get a sequence of events specific to the stream id.
    ///
    /// Because the minimum and maximum bounds are passed, you may get zero commits.
    /// Getting all commits between 0 and the latest commit id should retrieve all events associated with the aggregate.
    async fn get_stream_range(
        &mut self,
        stream_id: Uuid,
        min_version: i64,
        max_version: i64,
    ) -> Result<Vec<Commit>, Box<dyn StoreError>> {
        let cursor = StreamCursor {
            stream_id: stream_id.to_string(),
            min_version: min_version,
            max_version: max_version,
        };
        let response = self
            .client
            .get_stream_range(cursor)
            .await
            .expect("get_stream_range failed");
        let domain_commits: Vec<Commit> = response
            .get_ref()
            .commits
            .iter()
            .map(grpc_to_domain_commit)
            .collect();
        Ok(domain_commits)
    }

    /// Get a sequence of events specific to the stream id.
    ///
    /// Because the minimum and maximum bounds are passed, you may get zero commits.
    /// Getting all commits between 0 and the latest commit id should retrieve all events associated with the aggregate.
    async fn get_range(
        &mut self,
        _min_commit_number: i64,
        _max_commit_number: i64,
    ) -> Result<Vec<Commit>, Box<dyn StoreError>> {
        unimplemented!("need to expose this service stub before we can call through it.")
    }

    /// Get a single commit.
    async fn get_commit(&mut self, _commit_id: &Uuid) -> Result<Commit, Box<dyn StoreError>> {
        unimplemented!("need to expose this service stub before we can call through it.")
    }
}

#[async_trait]
impl DispatchingAsyncClientStore for GrpcClient {
    async fn subscribe(&mut self) -> CommitStream {
        let cursor = StreamCursor {
            stream_id: "".to_string(),
            min_version: 0,
            max_version: i64::MAX,
        };
        CommitStream::GrpcBacked(
            self.client
                .stream_stream_range(cursor)
                .await
                .unwrap()
                .into_inner(),
        )
    }

    async fn subscribe_stream(&mut self, stream_id: &Uuid) -> CommitStream {
        let cursor = StreamCursor {
            stream_id: stream_id.to_string(),
            min_version: 0,
            max_version: i64::MAX,
        };
        CommitStream::GrpcBacked(
            self.client
                .stream_stream_range(cursor)
                .await
                .unwrap()
                .into_inner(),
        )
    }
}

//
// Conversion utilities
//

pub fn domain_to_grpc_ts(datetime: DateTime<Utc>) -> GrpcTimestamp {
    let seconds = datetime.timestamp();
    let onanos_i64 = datetime.timestamp_nanos_opt().expect("cannot fetch nanos");
    let nanos_i64 = onanos_i64 - seconds * 1_000_000_000;
    let nanos = nanos_i64 as i32;
    GrpcTimestamp { seconds, nanos }
}

pub fn grpc_to_domain_ts(timestamp: &GrpcTimestamp) -> DateTime<Utc> {
    Utc.timestamp_opt(timestamp.seconds, timestamp.nanos as u32)
        .unwrap()
}

pub fn grpc_to_domain_commit(commit: &GrpcCommit) -> Commit {
    Commit {
        stream_id: Uuid::parse_str(&commit.stream_id).expect("failed to parse stream_id"),
        stream_version: commit.stream_version,
        commit_id: Uuid::parse_str({
            let ref this = commit.commit_id;
            this
        })
        .expect("failed to parse commit_id"),
        commit_timestamp: grpc_to_domain_ts(
            &commit
                .commit_timestamp
                .clone()
                .expect("commit_timestamp is required"),
        ),
        commit_sequence: commit.commit_sequence,
        metadata: serde_json::from_slice(commit.metadata.as_slice())
            .expect("failed to deserialize metadata json"),
        events: serde_json::from_slice(commit.events.as_slice())
            .expect("failed to deserialize events json"),
        events_count: commit.events_count,
        commit_number: 0,
        dispatched: commit.dispatch != 0,
    }
}

fn domain_to_grpc_commit_attempt(commit_attempt: &CommitAttempt) -> GrpcCommitAttempt {
    GrpcCommitAttempt {
        stream_id: commit_attempt.stream_id.to_string(),
        stream_version: commit_attempt.stream_version,
        commit_id: commit_attempt.commit_id.to_string(),
        commit_timestamp: Some(domain_to_grpc_ts(commit_attempt.commit_timestamp)),
        commit_sequence: commit_attempt.commit_sequence,
        metadata: serde_json::to_vec(&commit_attempt.metadata)
            .expect("failed to serialize commit metadata"),
        events: serde_json::to_vec(&commit_attempt.events)
            .expect("failed to serialize commit events"),
        events_count: commit_attempt.events_count,
    }
}
