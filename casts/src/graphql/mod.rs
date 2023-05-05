use chrono::Utc;
use commit_store::{commit::CommitAttempt, AsyncClientStore, GrpcClient};
use juniper::{graphql_object, EmptySubscription, FieldResult, RootNode};
use url::Url;
use uuid::Uuid;

use crate::{
    aggregate::FeedAggregate,
    applicable::EventApplicable,
    commands::{Command, FeedAggregateCommand},
    events::FeedAggregateEvent,
};

// Store

#[derive(Clone, Debug)]
pub struct Store<T: AsyncClientStore + Clone> {
    store: T,
}

impl<T: AsyncClientStore + Clone> Store<T> {
    pub fn new(store: T) -> Self {
        Store { store: store }
    }
}

impl<T: AsyncClientStore + Clone> juniper::Context for Store<T> {}

// type PostgresStore = Store<PostgresStoreAsync>;
pub type GrpcStore = Store<GrpcClient>;

// Model

#[derive(juniper::GraphQLObject)]
pub struct Link {
    pub href: String,
    pub rel: Option<String>,
    pub media_type: Option<String>,
    pub href_lang: Option<String>,
    pub title: Option<String>,
    pub length: Option<i32>,
}

#[derive(juniper::GraphQLObject)]
pub struct Image {
    pub uri: String,
    pub title: Option<String>,
    pub link: Option<Link>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub description: Option<String>,
}

#[derive(juniper::GraphQLObject)]
pub struct Content {
    pub body: Option<String>,
    pub content_type: String,
    pub length: Option<i32>,
    pub src: Option<Link>,
}

#[derive(juniper::GraphQLObject)]
pub struct Entry {
    pub title: String,
    pub summary: String,
    pub links: Vec<Link>,
    pub content: Option<Content>,
}

#[derive(juniper::GraphQLObject)]
pub struct FeedsAggregate {
    pub id: String,
    pub version: i32,
    pub feeds: Vec<Feed>,
}

impl FeedsAggregate {
    pub fn new() -> Self {
        FeedsAggregate {
            id: Uuid::new_v4().to_string(),
            version: 0,
            feeds: vec![],
        }
    }
}

#[derive(juniper::GraphQLObject)]
pub struct Feed {
    pub title: String,
    pub description: String,
    pub entries: Vec<Entry>,
    pub logo: Option<Image>,
}

pub struct Mutation;

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
        let mut aggregate = FeedAggregate::new();
        let mut client: GrpcClient = context.store.clone();
        let commits = client
            .get_stream_range(stream_id, 0, i64::MAX)
            .await
            .expect("failed to make upstream grpc call");
        for commit in commits.into_iter() {
            let events: Vec<FeedAggregateEvent> = serde_json::from_value(commit.events)
                .expect("unable to deserialize events to FeedAggregateEvent");
            for event in events.into_iter() {
                aggregate = aggregate.apply(&event);
            }
        }

        let events = command
            .async_apply(&aggregate)
            .await
            .expect("failed to execute addFeed command");
        let events_count = events.len() as i64;
        let events: serde_json::Value =
            serde_json::to_value(events).expect("could not serialize events to json");
        println!("events_json: {:?}", events);

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
    fn feeds(_ctx: &GrpcStore) -> Vec<String> {
        vec![]
    }
}

impl Query {}

pub type Schema = RootNode<'static, Query, Mutation, EmptySubscription<GrpcStore>>;

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
    RootNode::new(Query, Mutation, EmptySubscription::new())
}
