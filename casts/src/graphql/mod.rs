// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

use std::pin::Pin;

use chrono::Utc;
use commit_store::commit::{CommitAttempt, CommitMetadata, StreamMetadata};
use commit_store::dispatch::{DispatchedStore, DispatchingAsyncReadWriteStore};
use commit_store::sqlite::SqliteStore;
use commit_store::Client as GrpcClient;
use futures::stream::StreamExt;
use futures_core::Stream;
use juniper::graphql_subscription;
use juniper::{graphql_object, FieldError, FieldResult, RootNode};
use url::Url;
use uuid::Uuid;

use crate::{
    aggregate::FeedAggregate,
    applicable::{ApplicableStream, EventApplicable},
    commands::{Command, FeedAggregateCommand},
    events::FeedAggregateEvent,
};

// Store

#[derive(Clone, Debug)]
pub enum GraphQLStore {
    Sqlite(DispatchedStore<SqliteStore>),
    // Postgres(DispatchedStore<PostgresStoreAsync>),
    Grpc(GrpcClient),
}

impl juniper::Context for GraphQLStore {}

impl GraphQLStore {
    fn store(&mut self) -> &mut dyn DispatchingAsyncReadWriteStore {
        match self {
            GraphQLStore::Sqlite(ref mut sqlite) => sqlite,
            // GraphQLStore::Postgres(ref mut postgres) => postgres,
            GraphQLStore::Grpc(ref mut client) => client,
        }
    }
}

// Model

#[derive(juniper::GraphQLObject)]
pub struct Link {
    pub href: Url,
    pub rel: Option<String>,
    pub media_type: Option<String>,
    pub href_lang: Option<String>,
    pub title: Option<String>,
    pub length: Option<i32>,
}

impl From<immutable_feed::Link> for Link {
    fn from(link: immutable_feed::Link) -> Link {
        Link {
            href: Url::parse(&link.href).unwrap(),
            rel: link.rel,
            media_type: link.media_type,
            href_lang: link.href_lang,
            title: link.title,
            length: link.length.map(|length| length as i32),
        }
    }
}

#[derive(juniper::GraphQLObject)]
pub struct Image {
    pub url: Option<Url>,
    pub raw_url: String,
    pub title: Option<String>,
    pub link: Option<Link>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub description: Option<String>,
}

impl From<immutable_feed::Image> for Image {
    fn from(img: immutable_feed::Image) -> Image {
        // MUNGE ALERT
        // TODO: replace with more sane way of handling.
        // ATS complains because of lack of security.
        let url = img.url.map(|url| {
            if url.scheme() != "http" {
                url
            } else {
                let mut new_url = url.clone();
                new_url
                    .set_scheme("https")
                    .expect("could not set scheme of url to https");
                new_url
            }
        });
        Image {
            url,
            raw_url: img.raw_url.clone(),
            title: img.title.clone(),
            link: img.link.map(Link::from),
            width: img.width.map(|i| i as i32),
            height: img.height.map(|i| i as i32),
            description: img.description.clone(),
        }
    }
}

#[derive(juniper::GraphQLObject)]
pub struct Content {
    pub body: Option<String>,
    pub content_type: String,
    pub length: Option<i32>,
    pub src: Option<Link>,
}

impl From<immutable_feed::Content> for Content {
    fn from(value: immutable_feed::Content) -> Self {
        Content {
            body: value.body,
            content_type: value.content_type.to_string(),
            length: value.length.map(|i| i as i32),
            src: value.src.map(Link::from),
        }
    }
}

#[derive(juniper::GraphQLObject)]
pub struct MediaContent {
    pub url: Option<Url>,
    pub content_type: Option<String>,
    pub height: Option<i32>,
    pub width: Option<i32>,
    // pub duration: Option<(u32, u32)>,
    pub size: Option<i32>,
    // pub rating: Option<MediaRating>,
}

impl From<immutable_feed::MediaContent> for MediaContent {
    fn from(value: immutable_feed::MediaContent) -> MediaContent {
        MediaContent {
            url: value.url,
            content_type: value.content_type,
            height: value.height.map(|i| i as i32),
            width: value.width.map(|i| i as i32),
            size: value.size.map(|i| i as i32),
        }
    }
}

#[derive(juniper::GraphQLObject)]
pub struct MediaObject {
    /// Title of the object (from the media:title element)
    pub title: Option<String>,
    /// Collection of the media content elements
    pub content: Vec<MediaContent>,
    /// Duration of the object in seconds
    pub duration_s: Option<i32>,
    /// Duration of the object in milliseconds
    pub duration_ms: Option<i32>,
    // Representative images for the object (from media:thumbnail elements)
    // pub thumbnails: Vec<MediaThumbnail>,
    // A text transcript, closed captioning or lyrics of the media content.
    // pub texts: Vec<MediaText>,
    /// Short description of the media object (from the media:description element)
    pub description: Option<String>,
    // Community info (from the media:community element)
    // pub community: Option<MediaCommunity>,
    // Credits
    // pub credits: Vec<MediaCredit>,
}

impl From<immutable_feed::MediaObject> for MediaObject {
    fn from(value: immutable_feed::MediaObject) -> Self {
        let duration = value.duration.map(|d| [d.0 as i32, d.1 as i32]);
        MediaObject {
            title: value.title,
            content: value.content.into_iter().map(MediaContent::from).collect(),
            duration_s: duration.map(|d| d[0]),
            duration_ms: duration.map(|d| d[1]),
            description: value.description,
        }
    }
}

#[derive(juniper::GraphQLObject)]
pub struct Entry {
    pub title: String,
    pub guid: String,
    pub summary: String,
    pub links: Vec<Link>,
    pub content: Option<Content>,
    pub media: Vec<MediaObject>,
    /// Last recorded playback position in whole seconds; absent when the
    /// entry has never been listened to.
    pub elapsed_seconds: Option<i32>,
}

impl Entry {
    fn with_playtime(entry: immutable_feed::Entry, elapsed: Option<(i64, i32)>) -> Self {
        Entry {
            title: entry.title.unwrap_or_default(),
            guid: entry.guid,
            summary: entry.summary.unwrap_or_default(),
            links: entry.links.into_iter().map(Link::from).collect(),
            content: entry.content.map(Content::from),
            media: entry.media.into_iter().map(MediaObject::from).collect(),
            elapsed_seconds: elapsed
                .map(|(seconds, _nanos)| i32::try_from(seconds).unwrap_or(i32::MAX)),
        }
    }
}

#[derive(juniper::GraphQLObject)]
pub struct FeedsAggregate {
    pub stream_id: Uuid,
    pub version: i32,
    pub feeds: Vec<Feed>,
}

#[derive(juniper::GraphQLObject)]
pub struct Feed {
    /// The ID of the feed.
    pub feed_id: Uuid,

    // TODO: URL
    /// The title of the feed.
    pub title: String,

    /// Authors of the feed
    pub authors: Vec<String>,

    /// A string describing the feed.
    pub description: String,

    /// A list of entries to which the feed links.
    pub entries: Vec<Entry>,

    /// A link to an image with which to represent the feed.
    pub logo: Option<Image>,
}

type PlaytimeByFeedAndGuid = std::collections::HashMap<(String, String), (i64, i32)>;

impl Feed {
    fn with_feed_id(
        feed: immutable_feed::Feed,
        feed_id: Uuid,
        playtimes: &PlaytimeByFeedAndGuid,
    ) -> Self {
        Feed {
            feed_id,
            title: feed.metadata.title,
            authors: feed.metadata.authors.clone(),
            description: feed.metadata.description.unwrap_or_default(),
            entries: feed
                .entries
                .into_iter()
                .map(|entry| {
                    let elapsed = playtimes
                        .get(&(feed_id.to_string(), entry.guid.clone()))
                        .copied();
                    Entry::with_playtime(entry, elapsed)
                })
                .collect(),
            logo: feed.metadata.logo.map(Image::from),
        }
    }
}

pub struct Mutation;

/// One page of commits per round trip; the server clamps to its own maximum
/// page size anyway, so this only bounds client-side memory.
const HYDRATE_PAGE_SIZE: i64 = 1000;

async fn hydrate_aggregate(
    client: &mut dyn DispatchingAsyncReadWriteStore,
    stream_id: Uuid,
) -> FeedAggregate {
    let mut aggregate = FeedAggregate::new();
    let mut cursor = Some(commit_store::store::ScanStreamCursor {
        stream_id,
        min_version: 0,
        max_version: None,
        reverse: false,
    });
    while let Some(current) = cursor {
        let (commits, next) = client
            .get_stream_range(current, HYDRATE_PAGE_SIZE)
            .await
            .expect("failed to make upstream grpc call");
        for commit in commits {
            let events: Vec<FeedAggregateEvent> = serde_json::from_value(commit.events)
                .expect("unable to deserialize events to FeedAggregateEvent");
            // One commit advances the aggregate by exactly one version, no
            // matter how many events it carries; `apply_many` is the only
            // application path so every hydration agrees on the version.
            aggregate = aggregate.apply_many(events);
        }
        cursor = next;
    }
    aggregate
}

#[graphql_object(context = GraphQLStore)]
impl Mutation {
    async fn add_feed(
        stream_id: Uuid,
        feed_id: Uuid,
        url: Url,
        context: &GraphQLStore,
    ) -> FieldResult<Uuid> {
        // Fetch and parse before issuing the command; command application
        // itself is pure validation and performs no I/O.
        let feed = immutable_feed::fetch_feed(&url)
            .await
            .map_err(|err| FieldError::from(format!("failed to fetch feed: {err}")))?;
        // KNOWN GAP, deliberately not fixed here: unlike service.rs's
        // add_feed (the deployed gRPC RPC) and poller::add_feed_paginated
        // (the CLI), this single-commit AddFeed still embeds every entry in
        // one commit and will blow commit-store's 2 MB message limit for a
        // large feed (Crime Junkie: 7.16 MB, observed live). Left as-is
        // because `casts-server`, the only binary that serves this GraphQL
        // API, is not deployed by any ECS service -- confirmed, not
        // assumed. Apply the same pagination here if that ever changes.
        let command = FeedAggregateCommand::AddFeed {
            stream_id,
            feed_id,
            url,
            feed,
            // No mutation argument sets this yet -- always start archiving normally.
            do_not_archive: None,
        };
        let mut mutable_ctx = context.clone();
        let client: &mut dyn DispatchingAsyncReadWriteStore = mutable_ctx.store();
        let aggregate = hydrate_aggregate(client, stream_id).await;

        let events = command
            .async_apply(&aggregate)
            .await
            .map_err(|err| FieldError::from(format!("addFeed was rejected: {err}")))?;
        let events_count = events.len() as i64;
        let events: serde_json::Value =
            serde_json::to_value(events).expect("could not serialize events to json");

        let commit_attempt = CommitAttempt {
            stream_metadata: StreamMetadata {
                stream_id,
                stream_version: aggregate.version + 1,
            },
            commit_metadata: CommitMetadata {
                commit_id: Uuid::new_v4(),
                commit_sequence: aggregate.version + 1,
                commit_timestamp: Utc::now(),
            },
            metadata: serde_json::to_value(command)
                .expect("could not serialize command to metadata"),
            events,
            events_count,
        };

        let committed = client
            .commit(&commit_attempt)
            .await
            .map_err(|err| FieldError::from(format!("failed to commit addFeed event: {err}")))?;

        FieldResult::Ok(committed.commit_metadata.commit_id)
    }
}

pub struct Query;

#[graphql_object(context = GraphQLStore)]
impl Query {
    async fn feeds(stream_id: Uuid, context: &GraphQLStore) -> Vec<Feed> {
        let mut mutable_ctx = context.clone();
        let client: &mut dyn DispatchingAsyncReadWriteStore = mutable_ctx.store();
        let aggregate = hydrate_aggregate(client, stream_id).await;
        let playtimes = &aggregate.playtime_by_feed_and_guid;
        let feed_result = aggregate
            .feeds_by_feed_id
            .iter()
            .map(|(feed_id, feed)| Feed::with_feed_id(feed.clone(), *feed_id, playtimes))
            .collect();
        feed_result
    }
}

type FeedAggregateStream = Pin<Box<dyn Stream<Item = Result<Vec<Feed>, FieldError>> + Send>>;

pub struct Subscription;

#[graphql_subscription]
impl Subscription {
    async fn watch_feeds(stream_id: Uuid, context: &GraphQLStore) -> FeedAggregateStream {
        let mut mutable_ctx = context.clone();
        let aggregate_stream = ApplicableStream::subscribe_aggregate(
            FeedAggregate::default(),
            mutable_ctx.store().subscribe_stream(&stream_id).await,
        );
        let gql_stream = aggregate_stream.map(|commit_result| match commit_result {
            Ok((_, agg)) => {
                let playtimes = &agg.playtime_by_feed_and_guid;
                Ok(agg
                    .feeds_by_feed_id
                    .iter()
                    .map(|(feed_id, feed)| Feed::with_feed_id(feed.clone(), *feed_id, playtimes))
                    .collect())
            }
            Err(err) => Err(FieldError::from(format!(
                "subscription stream error: {err}"
            ))),
        });
        Box::pin(gql_stream)
    }
}

pub type Schema = RootNode<'static, Query, Mutation, Subscription>;

pub async fn create_grpc_store() -> GraphQLStore {
    // One config mechanism for every commit-store client: the address comes
    // from COMMIT_STORE_ADDR (default http://localhost:7392), not a literal.
    let commit_store_cfg = commit_store::config::ClientConfigBuilder::default().build();
    let grpc_client =
        commit_store::grpc_interface::commit_store_client::CommitStoreClient::connect(
            tonic::transport::Uri::try_from(commit_store_cfg.addr.clone())
                .expect("COMMIT_STORE_ADDR is not a valid URI"),
        )
        .await
        .expect("Failed to connect to backend commit store");
    let async_wrapper_client = GrpcClient::new(grpc_client);
    GraphQLStore::Grpc(async_wrapper_client)
}

pub fn create_sqlite_store() -> GraphQLStore {
    GraphQLStore::Sqlite(DispatchedStore::new(
        SqliteStore::with_new_in_memory_connection(),
    ))
}

pub fn create_schema() -> Schema {
    RootNode::new(Query, Mutation, Subscription)
}
