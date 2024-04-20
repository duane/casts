// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

#[macro_use]
extern crate log;

use chrono::{DateTime, Utc};
use prost_types::Timestamp as GrpcTimestamp;
use std::sync::Arc;

use casts::{
    aggregate::FeedAggregate,
    applicable::EventApplicable,
    events::FeedAggregateEvent,
    grpc_interface::{
        casts_server::CastsServer, Content as GrpcContent, Entry as GrpcEntry, Feed as GrpcFeed,
        Feeds as GrpcFeeds, GetFeedRequest, Image as GrpcImage, Link as GrpcLink, ListFeedsRequest,
        MediaContent as GrpcMediaContent, MediaObject as GrpcMediaObject,
    },
    server::Grpc2Store,
    util::aretry3,
};

use commit_store::grpc_interface::commit_store_client::CommitStoreClient;
use commit_store::store::DispatchingAsyncReadWriteStore;
use commit_store::Client as GrpcClient;
// use futures::Stream;
// use tokio_stream::{Stream, StreamExt};
use anyhow::Result;
use futures::lock::Mutex;
use immutable_feed::{Content, Entry, Feed, Image, Link, MediaContent, MediaObject};
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

pub async fn serve() -> Result<()> {
    let config = casts::config::ServerConfig::load()?;

    let grpc_client = aretry3(
        || commit_store_connect(config.commit_store_addr.to_string()),
        3,
    )
    .await?;

    let commit_store_impl = CastsImpl {
        backing_store: Arc::new(Mutex::new(Grpc2Store::Grpc(grpc_client))),
    };
    info!("Started server at {:?}", config.listen);
    Server::builder()
        .add_service(CastsServer::new(commit_store_impl))
        .serve(config.listen)
        .await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    serve().await?;
    Ok(())
}

// Conversion stuff

fn media_content_to_grpc_media_content(media_content: MediaContent) -> GrpcMediaContent {
    GrpcMediaContent {
        url: media_content.url.map(|s| s.to_string()).unwrap_or_default(),
        content_type: media_content.content_type.unwrap_or_default(),
        width: media_content.width.map(|i| i as i32).unwrap_or_default(),
        height: media_content.height.map(|i| i as i32).unwrap_or_default(),
        size: media_content.size.map(|i| i as i32).unwrap_or_default(),
    }
}

fn media_object_to_grpc_media_object(media_object: MediaObject) -> GrpcMediaObject {
    GrpcMediaObject {
        title: media_object.title.unwrap_or_default(),
        content: media_object
            .content
            .into_iter()
            .map(media_content_to_grpc_media_content)
            .collect(),
        duration_secs: media_object
            .duration
            .map(|i| i.0 as i32)
            .unwrap_or_default(),
        duration_nsecs: media_object
            .duration
            .map(|i| i.1 as i32)
            .unwrap_or_default(),
        description: media_object.description.unwrap_or_default(),
    }
}

fn link_to_grpc_link(link: Link) -> GrpcLink {
    GrpcLink {
        title: link.title.unwrap_or_default(),
        href: link.href,
        href_lang: link.href_lang.unwrap_or_default(),
        length: link.length.map(|i| i as i32).unwrap_or_default(),
        media_type: link.media_type.unwrap_or_default(),
        rel: link.rel.unwrap_or_default(),
    }
}

fn image_to_grpc_image(image: Image) -> GrpcImage {
    GrpcImage {
        title: image.title.unwrap_or_default(),
        url: image.raw_url,
        link: image.link.map(link_to_grpc_link),
        width: image.width.map(|w| w as i32).unwrap_or_default(),
        height: image.height.map(|h| h as i32).unwrap_or_default(),
        description: image.description.unwrap_or_default(),
    }
}

fn content_to_grpc_content(content: Content) -> GrpcContent {
    GrpcContent {
        body: content.body.unwrap_or_default(),
        content_type: content.content_type,
        length: content.length.map(|i| i as i32).unwrap_or_default(),
        src: content.src.map(link_to_grpc_link),
    }
}

fn entry_to_grpc_entry(entry: Entry) -> GrpcEntry {
    GrpcEntry {
        guid: entry.guid,
        title: entry.title.unwrap_or_default(),
        summary: entry.summary.unwrap_or_default(),
        links: entry.links.into_iter().map(link_to_grpc_link).collect(),
        media: entry
            .media
            .into_iter()
            .map(media_object_to_grpc_media_object)
            .collect(),
        published: entry.published_at.map(domain_to_grpc_ts),
        content: entry.content.map(content_to_grpc_content),
    }
}

fn domain_to_grpc_ts(datetime: DateTime<Utc>) -> GrpcTimestamp {
    let seconds = datetime.timestamp();
    let onanos_i64 = datetime.timestamp_nanos_opt().expect("cannot fetch nanos");
    let nanos_i64 = onanos_i64 - seconds * 1_000_000_000;
    let nanos = i32::try_from(nanos_i64).expect("nanos is too large to downcast to i32");
    GrpcTimestamp { seconds, nanos }
}

fn feed_to_grpc_feed(feed: Feed, feed_id: Uuid) -> GrpcFeed {
    let mut entries: Vec<GrpcEntry> = feed
        .entries
        .into_iter()
        .map(|entry| entry_to_grpc_entry(entry))
        .collect();
    entries.sort_by(|a, b| match (&a.published, &b.published) {
        (Some(a_ts), Some(b_ts)) => b_ts.seconds.cmp(&a_ts.seconds),
        (None, Some(_)) => std::cmp::Ordering::Less,
        (Some(_), None) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
    GrpcFeed {
        feed_id: feed_id.to_string(),
        title: feed.metadata.title,
        authors: feed.metadata.authors,
        description: feed.metadata.description.unwrap_or_default(),
        logo: feed.metadata.logo.map(image_to_grpc_image),
        entries,
    }
}

fn aggregate_to_feeds(aggregate: FeedAggregate) -> GrpcFeeds {
    let mut feeds: Vec<GrpcFeed> = aggregate
        .feeds_by_feed_id
        .into_iter()
        .map(|(feed_id, feed)| feed_to_grpc_feed(feed, feed_id))
        .collect();
    feeds.sort_by(|a, b| a.title.cmp(&b.title));
    GrpcFeeds { feeds }
}
