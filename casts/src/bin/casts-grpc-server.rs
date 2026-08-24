// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

#[macro_use]
extern crate log;

use chrono::{DateTime, Utc};
use commit_store::{commit, store::StreamCursor};
use prost_types::Timestamp as GrpcTimestamp;
use std::{pin::pin, str::FromStr, sync::Arc};
use url::Url;

use casts::{
    aggregate::FeedAggregate,
    applicable::EventApplicable,
    events::FeedAggregateEvent,
    grpc::Grpc2Store,
    grpc_interface::{
        casts_server::CastsServer, Content as GrpcContent, Entry as GrpcEntry, Feed as GrpcFeed,
        Feeds as GrpcFeeds, GetFeedRequest, Image as GrpcImage, Link as GrpcLink, ListFeedsRequest,
        MediaContent as GrpcMediaContent, MediaObject as GrpcMediaObject,
    },
};

use commit_store::store::DispatchingAsyncReadWriteStore;
use commit_store::{grpc_interface::commit_store_client::CommitStoreClient, Client};
// use futures::Stream;
// use tokio_stream::{Stream, StreamExt};
use anyhow::Result;
use futures::{lock::Mutex, StreamExt};
use immutable_feed::{Content, Entry, Feed, Image, Link, MediaContent, MediaObject};
use tonic::{transport::Server, Request, Response, Status};
use uuid::Uuid;

pub struct CastsImpl {
    backing_store: Arc<Mutex<Grpc2Store>>,
}

async fn hydrate_aggregate<S: DispatchingAsyncReadWriteStore>(
    client: &mut S,
    stream_id: &str,
) -> FeedAggregate
where
    S: Clone + 'static,
{
    let stream_id: Uuid = Uuid::parse_str(stream_id).unwrap();
    let mut aggregate = FeedAggregate::new();
    let cursor = StreamCursor {
        stream_id,
        min_version: 0,
        max_version: None,
    };
    let mut commits = pin!(client.stream_commits(cursor));
    while let Some(commit) = commits.next().await {
        let events: Vec<FeedAggregateEvent> = serde_json::from_value(commit.events)
            .expect("unable to deserialize events to FeedAggregateEvent");
        for event in events.into_iter() {
            aggregate = aggregate.apply(event);
        }
    }
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
        let client: &mut Grpc2Store = &mut lock;

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

// const CA_PEM: &str = include_str!("../../data/tls/ca.pem");

pub async fn serve() -> Result<()> {
    let server_config = casts::config::ServerConfigBuilder::default().build()?;

    let commit_store_cfg = commit_store::config::ClientConfigBuilder::default().build();

    println!("Connecting to address {:?}", commit_store_cfg.addr);

    log::info!("Connecting to commit store at {}", commit_store_cfg.addr);
    let commit_store_addr: Url = commit_store_cfg.addr;

    let endpoint = tonic::transport::Uri::from_str(&commit_store_addr.clone().to_string()).unwrap();

    let client: CommitStoreClient<tonic::transport::Channel> =
        CommitStoreClient::connect(endpoint).await?;

    // let addr = hyper::Uri::from_str(&commit_store_cfg.addr.as_str()).unwrap();
    // let host: String = addr.host().unwrap().to_string();
    // let tls_config: ClientTlsConfig = ClientTlsConfig::new()
    // .ca_certificate(Certificate::from_pem(CA_PEM))
    // .domain_name(host);
    // let channel = tonic::transport::Channel::from_shared(addr.to_string())
    // .unwrap()
    // .tls_config(tls_config.clone())
    // .unwrap()
    // .connect()
    // .await
    // .unwrap();
    let grpc_client = Client::new(client);

    let commit_store_impl = CastsImpl {
        backing_store: Arc::new(Mutex::new(Grpc2Store::Grpc(grpc_client))),
    };

    let service = CastsServer::new(commit_store_impl);

    // println!("tls identity is {:?}", tls_config);

    info!("Started server at {:?}", server_config.listen);
    let mut builder = Server::builder();

    builder
        .add_service(service)
        .serve(server_config.listen)
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
    let mut feeds: Vec<GrpcFeed> = Vec::new();
    for feed in aggregate.feeds_by_feed_id.values() {
        let feed_id = uuid::Uuid::new_v4();
        feeds.push(feed_to_grpc_feed(feed.clone(), feed_id));
    }
    GrpcFeeds { feeds }
}
