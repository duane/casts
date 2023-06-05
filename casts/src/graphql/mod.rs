// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

use std::pin::Pin;

use chrono::Utc;
use commit_store::{
    commit::CommitAttempt, AsyncClientStore, DispatchingAsyncClientStore, GrpcClient,
};
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
pub struct GraphQLStore<T: AsyncClientStore + Clone> {
    store: T,
}

impl<T: AsyncClientStore + Clone> GraphQLStore<T> {
    pub fn new(store: T) -> Self {
        GraphQLStore { store }
    }
}

impl<T: AsyncClientStore + Clone> juniper::Context for GraphQLStore<T> {}

pub type GrpcStore = GraphQLStore<GrpcClient>;

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
    /// Duration of the object
    pub duration: Option<[i32; 2]>,
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
        MediaObject {
            title: value.title,
            content: value.content.into_iter().map(MediaContent::from).collect(),
            duration: value.duration.map(|d| [d.0 as i32, d.1 as i32]),
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
}

impl From<immutable_feed::Entry> for Entry {
    fn from(entry: immutable_feed::Entry) -> Self {
        Entry {
            title: entry.title.unwrap_or_default(),
            guid: entry.guid,
            summary: entry.summary.unwrap_or_default(),
            links: entry.links.into_iter().map(Link::from).collect(),
            content: entry.content.map(Content::from),
            media: entry.media.into_iter().map(MediaObject::from).collect(),
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

impl Feed {
    fn with_feed_id(feed: immutable_feed::Feed, feed_id: Uuid) -> Self {
        Feed {
            feed_id,
            title: feed.metadata.title,
            authors: feed.metadata.authors.clone(),
            description: feed.metadata.description.unwrap_or_default(),
            entries: feed.entries.into_iter().map(Entry::from).collect(),
            logo: feed.metadata.logo.map(Image::from),
        }
    }
}

pub struct Mutation;

async fn hydrate_aggregate(client: &mut GrpcClient, stream_id: Uuid) -> FeedAggregate {
    let mut aggregate = FeedAggregate::new();
    let commits = client
        .get_stream_range(stream_id, 0, i64::MAX)
        .await
        .expect("failed to make upstream grpc call");
    for commit in commits.into_iter() {
        let events: Vec<FeedAggregateEvent> = serde_json::from_value(commit.events)
            .expect("unable to deserialize events to FeedAggregateEvent");
        for event in events.into_iter() {
            aggregate = aggregate.apply(event);
        }
    }
    aggregate
}

#[graphql_object(context = GrpcStore)]
impl Mutation {
    async fn add_feed<'db>(
        stream_id: Uuid,
        feed_id: Uuid,
        url: Url,
        context: &'db GrpcStore,
    ) -> FieldResult<Uuid> {
        let command = FeedAggregateCommand::AddFeed {
            stream_id,
            feed_id,
            url,
        };
        let mut client: GrpcClient = context.store.clone();
        let aggregate = hydrate_aggregate(&mut client, stream_id).await;

        let events = command
            .async_apply(&aggregate)
            .await
            .expect("failed to execute addFeed command");
        let events_count = events.len() as i64;
        let events: serde_json::Value =
            serde_json::to_value(events).expect("could not serialize events to json");

        let commit_attempt = CommitAttempt {
            stream_id,
            stream_version: aggregate.version + 1,
            commit_id: Uuid::new_v4(),
            commit_sequence: 0,
            commit_timestamp: Utc::now(),
            metadata: serde_json::to_value(command)
                .expect("could not serialize command to metadata"),
            events,
            events_count,
        };

        let committed = client
            .commit(&commit_attempt)
            .await
            .expect("failed to commit addFeed event");

        FieldResult::Ok(committed.commit_id)
    }
}

pub struct Query;

#[graphql_object(context = GrpcStore)]
impl Query {
    async fn feeds(stream_id: Uuid, context: &GrpcStore) -> Vec<Feed> {
        let mut client: GrpcClient = context.store.clone();
        let aggregate = hydrate_aggregate(&mut client, stream_id).await;
        aggregate
            .feeds_by_feed_id
            .into_iter()
            .map(|(feed_id, feed)| Feed::with_feed_id(feed, feed_id))
            .collect()
    }
}

type FeedAggregateStream = Pin<Box<dyn Stream<Item = Result<Vec<Feed>, FieldError>> + Send>>;
use futures::StreamExt;

pub struct Subscription;

#[graphql_subscription]
impl Subscription {
    async fn watch_feeds(stream_id: Uuid, context: &GrpcStore) -> FeedAggregateStream {
        let aggregate_stream = ApplicableStream::subscribe_aggregate(
            FeedAggregate::default(),
            context.store.clone().subscribe(&stream_id).await,
        );
        let gql_stream = aggregate_stream.map(|commit_result| match commit_result {
            Ok((_, agg)) => Ok(agg
                .feeds_by_feed_id
                .into_iter()
                .map(|(feed_id, immutable_feed)| Feed::with_feed_id(immutable_feed, feed_id))
                .collect()),
            Err(_) => unimplemented!(),
        });
        Box::pin(gql_stream)
    }
}

pub type Schema = RootNode<'static, Query, Mutation, Subscription>;

pub async fn create_store() -> GrpcStore {
    let grpc_client =
        commit_store::grpc_interface::commit_store_client::CommitStoreClient::connect(
            "http://127.0.0.1:5059",
        )
        .await
        .expect("Failed to connect to backend commit store");
    let async_wrapper_client = GrpcClient::new(grpc_client);
    GrpcStore::new(async_wrapper_client)
}

pub fn create_schema() -> Schema {
    RootNode::new(Query, Mutation, Subscription)
}
