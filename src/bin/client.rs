extern crate anyhow;
extern crate event_source;
extern crate feed_rs;
extern crate libc;
extern crate quick_xml;
extern crate reqwest;
extern crate serde;
extern crate serde_derive;
extern crate serde_json;
extern crate uuid;

use casts::{aggregate::FeedAggregate, commands::FeedAggregateCommand};
use event_source::commit::Commit;
use event_source::dispatch::DispatchDelegate;
use event_source::store::sqlite::SqliteStore;
use event_source::store::Store;
use event_source::{client::*, store::ClientStore};
use std::env;
use std::path::Path;
use uuid::Uuid;

use std::io::{stdin, stdout, Write};
use tokio::sync::broadcast::{channel, Sender};
// use tokio::task;
// use std::error::Error;

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
  let stdin = stdin();
  let mut stdout = stdout();
  let mut buf = String::new();
  loop {
    print!("casts- ");
    stdout.flush().unwrap();

    match stdin.read_line(&mut buf) {
      Ok(0) => break,
      Err(err) => panic!("{}", err),
      _ => {
        if buf.starts_with("subscribe ") {
          let uri = buf.strip_prefix("subscribe ").unwrap().trim();
          let command = FeedAggregateCommand::AddFeed {
            id: uuid,
            feed_id: uri.to_owned(),
            url: uri.to_owned(),
          };
          let result = futures::executor::block_on(client.issue_command_async(
            &aggregate,
            &command.clone(),
            &command,
          ))
          .unwrap();
          println!("{:#?}", result);
        } else if buf.starts_with("dump") {
          let mut dump_store = SqliteStore::with_new_connection_at_path(Path::new(&String::from(
            "./event_store.sqlite",
          )));
          let commits = dump_store.get_range(0, i64::MAX).unwrap();
          println!("{}", commits.len());
          for commit in commits {
            println!("{:?}", commit);
          }
        }
      }
    }
    buf = String::new();
  }
}
