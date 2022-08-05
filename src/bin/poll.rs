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
extern crate pretty_env_logger;
#[macro_use] extern crate log;

use anyhow::Result;
use casts::aggregate::FeedAggregate;
use casts::commands::FeedAggregateCommand;
use casts::events::FeedAggregateEvent;
use event_source::client::ClientBuilder;
use event_source::client::applicable::EventApplicable;
use event_source::commit::Commit;
use event_source::dispatch::DispatchDelegate;
use event_source::store::Store;
use event_source::store::sqlite::SqliteStore;
use tokio::sync::broadcast::{Sender, channel};
use uuid::Uuid;
use std::path::Path;
use std::time::Duration;
use tokio::task;
use tokio::time::interval;

use casts::model::*;

use url::Url;

async fn fetch_feed(url: &Url) -> Result<Vec<u8>> {
  let response = reqwest::get(url.as_str()).await?;
  let body = response.text().await?;
  Ok(body.as_bytes().into())
}

fn parse_doc(url: &Url, doc: &[u8]) -> Result<Feed> {
  Ok(immutable_feed::parse_doc(url, doc)?.into())
}

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

fn vend_command(cmd: FeedAggregateCommand) -> Result<Commit> {
  let dispatcher = InMemoryDispatcher::new();
  let uuid = Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap();

  let mut sqlite_store =
    SqliteStore::with_new_connection_at_path(Path::new(&String::from("./event_store.sqlite")));
  sqlite_store.initialize().unwrap();
  let mut client = ClientBuilder::default()
    .with_store(Box::new(sqlite_store))
    .with_dispatch_delegate(dispatcher)
    .finish()
    .unwrap();

  let aggregate: FeedAggregate = client.fetch_latest(uuid).unwrap();
  Ok(client.issue_command(&aggregate, &cmd, &cmd).unwrap())
}

fn fetch_feeds() -> Result<Vec<Feed>> {
  let dispatcher = InMemoryDispatcher::new();
  let uuid = Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap();

  let mut sqlite_store =
    SqliteStore::with_new_connection_at_path(Path::new(&String::from("./event_store.sqlite")));
  sqlite_store.initialize().unwrap();
  let mut client = ClientBuilder::default()
    .with_store(Box::new(sqlite_store))
    .with_dispatch_delegate(dispatcher)
    .finish()
    .unwrap();

  let aggregate: FeedAggregate = client.fetch_latest(uuid).unwrap();
  Ok(aggregate.feeds_by_feed_id.values().collect())
}

// The task to kick off feed polling.
async fn cron_task() -> Result<()> {
  task::spawn(async {
    let mut interval = interval(Duration::from_secs(2000));
    let feeds = vec![Feed{
      url: Url::parse("https://fans.fm/feed/e0dc46be-6f5b-453e-bfc7-2a2ac5ced97c.xml")?,
      title: "Majority Report".into(),
      description: Some("Only you can support an unbought and unbossed voice, beholden to no party insiders and one which will never shill for anyone or anything.\r\r\n\r\r\n[Support independent media today by becoming a Majority Report member for less than 50 cents a show!](/majority/join) Not only will you make the Majority Report possible but you’ll get guaranteed access to all Majority Report bonus content, you’ll be able to access the program 5 days a week in all formats commercial free!".into()),
      logo: Default::default(),
      entries: Default::default(),
    }];
    loop {
      interval.tick().await;
      for feed in &feeds {
        println!("fetching {}", &feed.url);
        let body = fetch_feed(&feed.url).await?;
        let remote_feed = parse_doc(&feed.url, body.as_slice())?;
        let mut cmd_buf = vec![];
        for mutation in feed.diff(&remote_feed) {
          match mutation {
            FeedMutation::AddedEntry(entry) => {
              let cmd = FeedAggregateCommand::FeedEntryObserved {
                id: uuid::Uuid::new_v4(),

                /// The feed in which the entry was observed.
                feed_id: feed.url.to_string(),

                /// The unique identifier of the entry.
                guid: entry.id.to_owned(),

                /// The url where the entry content may be found.
                url: entry.media.unwrap().url.unwrap(),

                /// The title of the entry.
                title: entry.title.expect("entry titles are required"),

                /// A summary of the entry.
                summary: entry.summary,

                /// When the feed was updated (according to the feed).
                updated_at: None,

                /// When the feed was published (according to the feed).
                published_at: None,
              };
              cmd_buf.push(cmd);
            }
            FeedMutation::ReplacedTitle(new_title) => {
              info!("ReplacedTitle: {} with {}", feed.title, new_title)
            }
            FeedMutation::ReplacedDescription(new_description) => {
              info!("ReplacedDescription: {:?} with {:?}", feed.description, new_description)
            }
            FeedMutation::ReplacedLogo(_) => {
              info!("ReplacedLogo")
            }
            _ => panic!()
          }
        }
        info!("found {} cmds", cmd_buf.len());
      }
    }
  })
  .await?
}

/*
  let's do some talking here

  discrete processes that need to communicate:
  1. cron: continually loops through subscribed feeds and polls them for new events. If there are new events, this process commits them as well.
  2. Mirror: re-serve the feed from the process on given host, port, etc.
  3. input poller: waits for commands and executes them, delivering some result, continually. Not necessary if commands are given outside lifetime of process.

  1 must influence 2.
  3 must influence 1.

  Fresh aggregate -> Poller
  Poller -> Command
  Command -> Event Bus
  Event Bus -> Query
*/

#[tokio::main]
async fn main() -> Result<()> {
  pretty_env_logger::init();
  cron_task().await
}
