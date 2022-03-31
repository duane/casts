/**
 * TODO:
 *  - GQL serving from event source aggregate.
 *  - Caching of most recent aggregate version.
 *  - Caching of aggregates per-version.
 *  - Async updates via pipe: update most recent version.2
 */

extern crate juniper;

extern crate anyhow;
extern crate event_source;
extern crate feed_rs;
extern crate futures;
extern crate libc;
extern crate quick_xml;
extern crate reqwest;
extern crate serde;
extern crate serde_derive;
extern crate serde_json;
extern crate uuid;
extern crate chrono;

use std::path::Path;
use event_source::client::ClientBuilder;
use event_source::dispatch::DispatchDelegate;

use uuid::Uuid;
use casts::aggregate::FeedAggregate;

use chrono::{DateTime, Utc};

use std::{convert::Infallible, sync::Arc};
use anyhow::Result;

use url::Url;

use tokio::sync::broadcast::{channel, Sender};

use event_source::commit::Commit;
use event_source::store::Store;
use event_source::store::sqlite::SqliteStore;

// EventSource shit

struct InMemoryDispatcher {
  sender: Sender<Commit>,
}

impl InMemoryDispatcher {
  fn new() -> Self {
    let (sender, _receiver) = channel(16);
    InMemoryDispatcher { sender }
  }
}

impl DispatchDelegate for InMemoryDispatcher {
  fn dispatch(&mut self, commit: &Commit) -> Result<(), String> {
    match self.sender.send(commit.to_owned()) {
      Ok(_) => Ok(()),
      Err(err) => Err(err.to_string()),
    }
  }
}

// hyper shit

use hyper::{
  server::Server,
  service::{make_service_fn, service_fn},
  Body, Method, Response, StatusCode,
};

// graphql shit

use juniper::{
  graphql_object, EmptyMutation, EmptySubscription,
  RootNode, 
};

/// graphql state
#[derive(Clone, Debug)]
pub struct Context {
  feeds: Vec<Feed>,
}
impl juniper::Context for Context {}

// graphql model

/// The feed itself.
#[derive(Clone, Debug)]
pub struct Feed {
  /// The title of the feed.
  pub title: String,

  /// The url where the feed may be found.
  pub url: Url,

  /// The description of the feed.
  pub description: Option<String>,

  /// When the feed was last updated.
  pub updated_at: Option<DateTime<Utc>>,

  /// When the feed was first published.
  pub published_at: Option<DateTime<Utc>>,

  // // The entries associated with the feed.
  pub entries: Vec<Entry>,
}

#[graphql_object(context = Context)]
impl Feed {
  /// Title of the feed
  fn title(&self) -> String {
    self.title.clone()
  }

  fn url(&self) -> Url {
    self.url.clone()
  }

  fn description(&self) -> Option<String> {
    self.description.clone()
  }

  fn updated_at(&self) -> Option<DateTime<Utc>> {
    self.updated_at
  }

  fn published_at(&self) -> Option<DateTime<Utc>> {
    self.published_at
  }

  fn entries(&self) -> Vec<Entry> {
    self.entries.clone()
  }
}

impl From<casts::aggregate::Feed> for Feed {
  fn from(feed: casts::aggregate::Feed) -> Feed {
    let title: String = feed.title;
    println!("{:?}", feed.entries);
    Feed{
      title: title,
      url: Url::parse(&feed.url).unwrap(),
      description: None,
      updated_at: None,
      published_at: None,
      entries: feed.entries.values().map(|v| Entry::from(v.clone())).collect()
    }
  }
}

impl From<immutable_feed::Feed> for Feed {
  fn from(feed: immutable_feed::Feed) -> Feed {
    Feed {
      title: feed.title,
      url: feed.url,
      description: feed.description,
      updated_at: feed.updated_at,
      published_at: feed.published_at,
      entries: feed.entries.into_iter().map(Entry::from).collect(),
    }
  }
}


// ENTRY

/// An entry associated with the feed.
#[derive(Clone, Debug)]
pub struct Entry {
  /// The self-identified guid of the feed entry.
  pub guid: String,

  /// The url where the entry content may be found.
  pub url: Url,

  /// The title of the feed entry.
  pub title: Option<String>,

  /// A summary of the entry content.
  pub summary: Option<String>,

  /// When the entry was last updated.
  pub updated_at: Option<DateTime<Utc>>,

  /// When the entry was first published.
  pub published_at: Option<DateTime<Utc>>,
}

impl From<casts::aggregate::Entry> for Entry {
  fn from(entry: casts::aggregate::Entry) -> Entry {
    Entry {
      guid: entry.guid,
      url: Url::parse(entry.url.as_str()).unwrap(),
      title: entry.title,
      summary: entry.summary,
      updated_at: entry.updated_at,
      published_at: entry.published_at,
    }
  }
}

impl From<immutable_feed::Entry> for Entry {
  fn from(entry: immutable_feed::Entry) -> Entry {
    Entry {
      guid: entry.guid,
      url: entry.url,
      title: entry.title,
      summary: entry.summary,
      updated_at: entry.updated_at,
      published_at: entry.published_at,
    }
  }
}

#[graphql_object(context = Context)]
impl Entry {
  fn id(&self) -> String {
    self.guid.clone()
  }

  fn url(&self) -> Url {
    self.url.clone()
  }

  fn title(&self) -> Option<String> {
    self.title.clone()
  }

  fn summary(&self) -> Option<String> {
    self.summary.clone()
  }

  fn updated_at(&self) -> Option<DateTime<Utc>> {
    self.updated_at.clone()
  }

  fn published_at(&self) -> Option<DateTime<Utc>> {
    self.published_at.clone()
  }
}

/// Use I/O to retrieve the feed object from the given url.
async fn fetch_feed(url: &Url) -> Result<Vec<u8>> {
  let response = reqwest::get(url.as_str()).await?;
  let body = response.text().await?;
  Ok(body.as_bytes().into())
}

#[derive(Clone, Copy, Debug)]
struct Query;

#[graphql_object(context = Context)]
impl Query {
  async fn preview_feed(url: Url) -> Option<Feed> {
    let body = fetch_feed(&url).await.unwrap();
    let immutable_feed: immutable_feed::Feed = immutable_feed::parse_doc(&url, &body).unwrap();
    Some(Feed::from(immutable_feed))
  }

  async fn feeds<'ctx>(&self, ctx: &'ctx Context) -> Vec<Feed> {
    ctx.feeds.clone()
  }
}

type Schema = RootNode<'static, Query, EmptyMutation<Context>, EmptySubscription<Context>>;

fn schema() -> Arc<Schema> {
  Arc::new(
    Schema::new(
      Query,
      EmptyMutation::<Context>::new(),
      EmptySubscription::<Context>::new(),
    )
  )
}

async fn mirror_task(feeds: Vec<Feed>) -> Result<()> {
  let addr = ([127, 0, 0, 1], 3000).into();
  let root_node = schema();

  let new_service = make_service_fn(move |_| {
    let root_node = root_node.clone();
    let ctx = Arc::new(Context{
      feeds: feeds.clone()
    });
    async {
      Ok::<_, hyper::Error>(service_fn(move |req| {
        let root_node = root_node.clone();
        let ctx = ctx.clone();
        async {
          Ok::<_, Infallible>(match (req.method(), req.uri().path()) {
            (&Method::GET, "/") => juniper_hyper::graphiql("/graphql", None).await,
            (&Method::GET, "/graphql") | (&Method::POST, "/graphql") => {
              juniper_hyper::graphql(root_node, ctx, req).await
            }
            _ => {
              let mut response = Response::new(Body::empty());
              *response.status_mut() = StatusCode::NOT_FOUND;
              response
            }
          })
        }
      }))
    }
  });

  let server = Server::bind(&addr).serve(new_service);
  println!("Listening on http://{}", addr);

  Ok(server.await?)
}

#[tokio::main]
async fn main() -> Result<()> {
  let uuid = Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap();

  let dispatcher = InMemoryDispatcher::new();

  let mut sqlite_store =
    SqliteStore::with_new_connection_at_path(Path::new(&String::from("./event_store.sqlite")));
  sqlite_store.initialize().unwrap();
  let mut client = ClientBuilder::default()
    .with_store(Box::new(sqlite_store))
    .with_dispatch_delegate(dispatcher)
    .finish()
    .unwrap();

  let aggregate: FeedAggregate = client.fetch_latest(uuid).unwrap();
  let feeds: Vec<Feed> = aggregate.feeds.values().map(|e|e.to_owned().into()).collect();
  
  mirror_task(feeds).await
}