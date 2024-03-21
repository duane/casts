// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

#[macro_use]
extern crate log;

use chrono::Utc;
use commit_store::{
    dispatch::DispatchedStore,
    domain_to_grpc_ts,
    grpc_interface::{commit_store_server::CommitStoreServer, CommitCursor, Commits, StreamCursor},
    postgres::PostgresStoreAsync,
    AsyncReadWriteStore, CommitStream, DispatchingAsyncReadWriteStore, StoreErrorType,
};
// use futures::Stream;
use std::pin::Pin;

use serde_json::Value;
use tokio_stream::{Stream, StreamExt};
use tonic::{transport::Server, Code, Request, Response, Status};
use uuid::Uuid;

pub struct CommitStoreImpl {
    backing_store: DispatchedStore<PostgresStoreAsync>,
}

fn domain_to_rpc_commit(
    domain_commit: commit_store::commit::Commit,
) -> commit_store::grpc_interface::Commit {
    commit_store::grpc_interface::Commit {
        stream_id: domain_commit.stream_id.to_string(),
        stream_version: domain_commit.stream_version,
        commit_id: domain_commit.commit_id.to_string(),
        commit_timestamp: Some(domain_to_grpc_ts(domain_commit.commit_timestamp)),
        commit_sequence: domain_commit.commit_sequence,
        metadata: serde_json::to_vec(&domain_commit.metadata).unwrap(),
        events: serde_json::to_vec(&domain_commit.events).unwrap(),
        events_count: domain_commit.events_count,
        dispatch: 0,
    }
}

#[tonic::async_trait]
impl commit_store::grpc_interface::commit_store_server::CommitStore for CommitStoreImpl {
    type StreamStreamRangeStream =
        Pin<Box<dyn Stream<Item = Result<commit_store::grpc_interface::Commit, Status>> + Send>>;

    type StreamCommitRangeStream =
        Pin<Box<dyn Stream<Item = Result<commit_store::grpc_interface::Commit, Status>> + Send>>;

    async fn stream_stream_range(
        &self,
        request: Request<StreamCursor>,
    ) -> Result<Response<Self::StreamStreamRangeStream>, Status> {
        info!("stream_stream_range");
        let stream_id = match Uuid::parse_str(request.get_ref().stream_id.as_str()) {
            Ok(id) => id,
            Err(_) => {
                return Err(Status::new(
                    Code::InvalidArgument,
                    "(stream_stream_range) not a valid stream id",
                ))
            }
        };
        let result = self
            .backing_store
            .clone()
            .get_stream_range(
                stream_id,
                request.get_ref().min_version,
                request.get_ref().max_version,
            )
            .await;
        let existing_commits = match result {
            Ok(domain_commits) => domain_commits,
            Err(err) => return Err(Status::new(Code::Internal, err.to_string())),
        };

        let existing_stream = tokio_stream::iter(existing_commits.into_iter());
        let receiver = match self
            .backing_store
            .clone()
            .subscribe_stream(&stream_id)
            .await
        {
            CommitStream::ReceiverBacked(receiver) => receiver,
            _ => panic!("expected receiver backed commit stream"),
        };
        let both = existing_stream
            .chain(receiver)
            .map(|e| Ok(domain_to_rpc_commit(e)));
        let output_stream = Box::pin(both);
        Ok(Response::new(
            output_stream as Self::StreamStreamRangeStream,
        ))
    }

    async fn stream_commit_range(
        &self,
        request: Request<CommitCursor>,
    ) -> Result<Response<Self::StreamCommitRangeStream>, Status> {
        info!("stream_commit_range");
        let existing_commit_vec = match self
            .backing_store
            .clone()
            .get_range(request.get_ref().min_commit, request.get_ref().max_commit)
            .await
        {
            Ok(domain_commits) => domain_commits,
            Err(err) => return Err(Status::new(Code::Internal, err.to_string())),
        };

        let existing_commit_stream = tokio_stream::iter(existing_commit_vec.into_iter());

        let ongoing_commit_receiver = match self.backing_store.clone().subscribe_all_commits().await
        {
            CommitStream::ReceiverBacked(receiver) => receiver,
            _ => panic!("expected receiver backed commit stream"),
        };
        let existing_and_ongoing = existing_commit_stream
            .chain(ongoing_commit_receiver)
            .map(|e| Ok(domain_to_rpc_commit(e)));
        let output_stream = Box::pin(existing_and_ongoing);
        Ok(Response::new(
            output_stream as Self::StreamCommitRangeStream,
        ))
    }

    async fn append_commit(
        &self,
        request: Request<commit_store::grpc_interface::CommitAttempt>,
    ) -> Result<Response<commit_store::grpc_interface::Commit>, Status> {
        let store = self.backing_store.clone();
        let serialized_request = request.get_ref();
        let stream_id = match Uuid::parse_str(serialized_request.stream_id.as_str()) {
            Ok(uuid) => uuid,
            Err(_err) => return Err(Status::new(Code::InvalidArgument, "invalid stream id")),
        };
        let stream_version = serialized_request.stream_version;
        let commit_id = match Uuid::parse_str(serialized_request.commit_id.as_str()) {
            Ok(uuid) => uuid,
            Err(_err) => return Err(Status::new(Code::InvalidArgument, "invalid commit id")),
        };
        let metadata = serde_json::from_slice(serialized_request.metadata.as_slice())
            .expect("failed to deserialize metadata from grpc");
        let events: Value = serde_json::from_slice(serialized_request.events.as_slice())
            .expect("failed to deserialize metadata from grpc");
        let events_count = serialized_request.events_count;
        let domain_attempt = commit_store::commit::CommitAttempt {
            stream_id,
            stream_version,
            commit_id,
            commit_timestamp: Utc::now(),
            commit_sequence: serialized_request.commit_sequence,
            metadata,
            events,
            events_count,
        };


        let commit = match store.clone().commit(&domain_attempt).await {
            Ok(commit) => commit,
            Err(err) => {
                match err.error_type() {
                    StoreErrorType::UnknownError => {
                        return Err(Status::new(Code::Internal, err.to_string()))
                    }
                    err @ _ => return Err(Status::new(Code::Internal, err.to_string())),
                };
            }
        };


        Ok(Response::new(domain_to_rpc_commit(commit)))
    }

    async fn get_stream_range(
        &self,
        cursor: Request<StreamCursor>,
    ) -> Result<Response<Commits>, Status> {
        let stream_id = match Uuid::parse_str(cursor.get_ref().stream_id.as_str()) {
            Ok(id) => id,
            Err(_) => {
                return Err(Status::new(
                    Code::InvalidArgument,
                    "(get_stream_range) not a valid stream id",
                ))
            }
        };
        let result = self
            .backing_store
            .clone()
            .get_stream_range(
                stream_id,
                cursor.get_ref().min_version,
                cursor.get_ref().max_version,
            )
            .await;
        match result {
            Ok(domain_commits) => {
                let grpc_commits = domain_commits
                    .into_iter()
                    .map(domain_to_rpc_commit)
                    .collect();
                Ok(Response::new(Commits {
                    commits: grpc_commits,
                }))
            }
            Err(err) => Err(Status::new(Code::Internal, err.to_string())),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let port = i32::from_str_radix(
        std::env::var("PORT")
            .unwrap_or_else(|_| "5059".to_string())
            .as_str(),
        10,
    )
    .unwrap_or(5059);
    let addr = format!("0.0.0.0:{}", port).parse()?;
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://duane@localhost:5432/commit_store".into());
    let store = PostgresStoreAsync::with_connection(database_url).await;
    let dispatched_store = DispatchedStore::new(store);
    let commit_store_impl = CommitStoreImpl {
        backing_store: dispatched_store,
    };

    info!("Started server at {:?}", addr);
    Server::builder()
        .add_service(CommitStoreServer::new(commit_store_impl))
        .serve(addr)
        .await?;

    info!("Server stopped.");
    Ok(())
}
