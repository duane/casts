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

use anyhow::Result;
use casts::events::FeedAggregateEvent;
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

// The task to kick off feed polling.
async fn cron_task() -> Result<()> {
  task::spawn(async {
    let mut interval = interval(Duration::from_secs(2000));
    let feeds = vec![Feed{
      url: Url::parse("https://fans.fm/feed/e0dc46be-6f5b-453e-bfc7-2a2ac5ced97c.xml")?,
      title: Default::default(),
      description: Default::default(),
      logo: Default::default(),
      entries: Default::default(),
    }];
    loop {
      interval.tick().await;
      for feed in &feeds {
        let body = fetch_feed(&feed.url).await?;
        let remote_feed = parse_doc(&feed.url, body.as_slice())?;
        for mutation in feed.diff(&remote_feed) {
          match mutation {
            FeedMutation::AddedEntry(entry) => {
              let href = entry.media.unwrap().url.unwrap().to_string();
              let event = FeedAggregateEvent::EntryObserved{
                id: uuid::Uuid::new_v4(),

                /// The feed in which the entry was observed.
                feed_id: feed.url.to_string(),

                /// The unique identifier of the entry.
                guid: entry.id.to_owned(),

                /// The url where the entry content may be found.
                url: href,

                /// The title of the entry.
                title: entry.title,

                /// A summary of the entry.
                summary: entry.summary,

                /// When the feed was updated (according to the feed).
                updated_at: None,

                /// When the feed was published (according to the feed).
                published_at: None,
              };
              println!("{:?}", event);
            }
            FeedMutation::ReplacedTitle(_) => {}
            FeedMutation::ReplacedDescription(_) => {}
            FeedMutation::ReplacedLogo(_) => {}
            _ => panic!()
          }
        }
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
  cron_task().await
}
