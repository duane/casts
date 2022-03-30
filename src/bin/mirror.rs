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


use chrono::{DateTime, Utc};

use std::{convert::Infallible, sync::Arc};
use anyhow::Result;
use casts::events::FeedAggregateEvent;
use std::time::Duration;
use tokio::task;
use tokio::time::interval;

use casts::model::*;

use url::Url;

// hyper shit

use hyper::{
  server::Server,
  service::{make_service_fn, service_fn},
  Body, Method, Response, StatusCode,
};

// graphql shit

use juniper::{
  graphql_object, graphql_subscription, http::GraphQLRequest, DefaultScalarValue, EmptyMutation, EmptySubscription,
  FieldError, RootNode, 
};

/// graphql state
#[derive(Clone, Copy, Debug)]
pub struct Context;
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
  // pub entries: HashMap<String, Entry>,
}

/// An entry associated with the feed.
// #[derive(Default, Serialize, Clone, Debug)]
// pub struct Entry {
//   /// The self-identified guid of the feed entry.
//   pub guid: String,

//   /// The url where the entry content may be found.
//   pub url: Url,

//   /// The title of the feed entry.
//   pub title: Option<String>,

//   /// A summary of the entry content.
//   pub summary: Option<String>,

//   /// When the entry was last updated.
//   pub updated_at: Option<DateTime<Utc>>,

//   /// When the entry was first published.
//   pub published_at: Option<DateTime<Utc>>,
// }

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
    let feed = Feed {
      url: immutable_feed.url,
      title: immutable_feed.title,
      description: immutable_feed.description,
      updated_at: None,
      published_at: None,
    };
    Some(feed)
  }

  async fn feeds() -> Vec<Feed> {
    vec![]
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


async fn mirror_task() -> Result<()> {
  let addr = ([127, 0, 0, 1], 3000).into();
  let root_node = schema();

  let new_service = make_service_fn(move |_| {
    let root_node = root_node.clone();
    let ctx = Arc::new(Context);
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
  mirror_task().await
}