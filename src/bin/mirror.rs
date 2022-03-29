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

#[derive(Clone, Copy, Debug)]
struct Context;
impl juniper::Context for Context {}

#[derive(Clone, Debug)]
struct Feed {
  url: Url,
}

#[graphql_object(context = Context)]
impl Feed {
  fn url(&self) -> Url {
    self.url.clone()
  }
}


#[derive(Clone, Copy, Debug)]
struct Query;

#[graphql_object(context = Context)]
impl Query {
  async fn feeds() -> Vec<Feed> {
    vec![Feed {
      url: Url::parse("https://fans.fm/feed/e0dc46be-6f5b-453e-bfc7-2a2ac5ced97c.xml").unwrap(),
    }]
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