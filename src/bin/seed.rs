extern crate anyhow;
extern crate event_source;
extern crate either;
extern crate feed_rs;
extern crate libc;
extern crate quick_xml;
extern crate reqwest;
extern crate serde;
extern crate serde_derive;
extern crate serde_json;
extern crate uuid;
extern crate url;

use casts::commands::FeedAggregateError;
use casts::{aggregate::FeedAggregate};
use event_source::commit::Commit;
use event_source::dispatch::DispatchDelegate;
use event_source::store::sqlite::SqliteStore;
use event_source::store::Store;
use event_source::{client::*};
use std::env;
use std::path::Path;
use uuid::Uuid;
use url::Url;

use tokio::sync::broadcast::{channel, Sender};

struct InMemoryDispatcher {
  sender: Sender<Commit>,
}

impl InMemoryDispatcher {
  fn new() -> Self {
    let (sender, _receiver) = channel(16);
    InMemoryDispatcher { sender }
  }

  // fn subscribe(&self) -> Receiver<Commit> {
  //   self.sender.subscribe()
  // }
}

impl DispatchDelegate for InMemoryDispatcher {
  fn dispatch(&mut self, commit: &Commit) -> Result<(), String> {
    match self.sender.send(commit.to_owned()) {
      Ok(_) => Ok(()),
      Err(err) => Err(err.to_string()),
    }
  }
}

#[tokio::main]
async fn main() {
  let args: Vec<String> = env::args().collect();
  if args.len() != 1 {
    panic!("USAGE: {}", args[0]);
  }

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
  let feeds = ["https://fans.fm/feed/e0dc46be-6f5b-453e-bfc7-2a2ac5ced97c.xml"];
  for url_str in feeds {
    let url = Url::parse(url_str).unwrap(); 
    let cmd = casts::commands::FeedAggregateCommand::AddFeed{
      id: uuid,
      feed_id: url.to_string(),
      url,
    };
    match client.issue_command_async(&aggregate, &cmd, &cmd).await {
      Ok(_) | Err(either::Either::Right(FeedAggregateError::DuplicateFeedError)) => {}
      Err(err) => panic!("error seeding feed: {:?}", err)
    }
  }
  
}
