// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

#[macro_use]
extern crate log;

use std::{env, sync::Arc};

use casts::{
    aggregate::FeedAggregate,
    applicable::EventApplicable,
    events::FeedAggregateEvent,
    grpc::Grpc2Store,
    grpc_interface::{
        casts_server::CastsServer, Feed as GrpcFeed, Feeds as GrpcFeeds, GetFeedRequest,
        ListFeedsRequest,
    },
    util::aretry3,
};
use commit_store::grpc_interface::commit_store_client::CommitStoreClient;

use commit_store::store::DispatchingAsyncReadWriteStore;
use commit_store::Client as GrpcClient;
// use futures::Stream;
// use tokio_stream::{Stream, StreamExt};
use anyhow::Result;
use futures::lock::Mutex;
use immutable_feed::Feed;
use tonic::{transport::Server, Request, Response, Status};
use uuid::Uuid;

pub struct CastsImpl {
    backing_store: Arc<Mutex<Grpc2Store>>,
}

async fn hydrate_aggregate(
    client: &mut dyn DispatchingAsyncReadWriteStore,
    stream_id: &str,
) -> FeedAggregate {
    let stream_id: Uuid = Uuid::parse_str(stream_id).unwrap();
    let mut aggregate = FeedAggregate::new();
    println!("stream_id {}", stream_id);
    println!("empty aggregate");
    let commits = client
        .get_stream_range(stream_id, 0, i64::MAX)
        .await
        .expect("failed to make upstream grpc call");
    println!("commits in hand");
    for commit in commits.into_iter() {
        let events: Vec<FeedAggregateEvent> = serde_json::from_value(commit.events)
            .expect("unable to deserialize events to FeedAggregateEvent");
        for event in events.into_iter() {
            aggregate = aggregate.apply(event);
            println!("applied event");
        }
    }
    println!("returning aggregate");
    aggregate
}

#[tonic::async_trait]
impl casts::grpc_interface::casts_server::Casts for CastsImpl {
    async fn list_feeds(
        &self,
        request: Request<ListFeedsRequest>,
    ) -> Result<Response<GrpcFeeds>, Status> {
        info!("list_feeds");
        let mut lock = self.backing_store.lock().await;
        let client = lock.store();

        println!("inside feed accessor");
        println!("client");
        let aggregate = hydrate_aggregate(client, &request.get_ref().stream_id).await;
        println!("aggregate");
        let feed_result = aggregate_to_feeds(aggregate);
        println!("returning result...");

        Ok(Response::new(feed_result))
    }

    async fn get_feed(
        &self,
        _request: Request<GetFeedRequest>,
    ) -> Result<Response<GrpcFeed>, Status> {
        unimplemented!("get_feed");
    }
}

async fn commit_store_connect(commit_store_addr: String) -> Result<GrpcClient> {
    Ok(GrpcClient::new(
        CommitStoreClient::connect(commit_store_addr)
            .await
            .expect("Failed to connect to backend commit store"),
    ))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let addr = ([127, 0, 0, 1], 8243).into();

    let commit_store_addr =
        env::var("COMMIT_STORE_ADDR").unwrap_or_else(|_| "http://127.0.0.1:5059".to_string());
    log::info!("Connecting to commit store at {}", commit_store_addr);

    let grpc_client = aretry3(|| commit_store_connect(commit_store_addr.clone()), 3)
        .await
        .unwrap();

    let commit_store_impl = CastsImpl {
        backing_store: Arc::new(Mutex::new(Grpc2Store::Grpc(grpc_client))),
    };

    info!("Started server at {:?}", addr);
    Server::builder()
        .add_service(CastsServer::new(commit_store_impl))
        .serve(addr)
        .await?;

    info!("Server stopped.");
    Ok(())
}

// Conversion stuff

fn feed_to_grpc_feed(feed: Feed, feed_id: Uuid) -> GrpcFeed {
    GrpcFeed {
        feed_id: feed_id.to_string(),
        title: feed.metadata.title,
        authors: feed.metadata.authors,
        description: feed.metadata.description.unwrap_or_default(),
        logo: None,
    }
}

fn aggregate_to_feeds(aggregate: FeedAggregate) -> GrpcFeeds {
    let feeds = aggregate
        .feeds_by_feed_id
        .into_iter()
        .map(|(feed_id, feed)| feed_to_grpc_feed(feed, feed_id))
        .collect();
    GrpcFeeds { feeds }
}
