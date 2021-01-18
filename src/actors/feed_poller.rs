use std::time::Duration;

use actix::prelude::*;
use rusqlite::Connection;

use log::{error, info, debug};
use anyhow::Result;
use futures::future::join_all;
use futures::FutureExt;

use crate::actors::event_bus::{EventBus, Commit, Subscribe, BusMessage};

/// This message is used to trigger scheduled polling intervals.
#[derive(Message)]
#[rtype(result = "anyhow::Result<()>")]
struct ClockTrigger();

/// The FeedPoller actor.
///
/// This polls every feed every `period`.
pub struct FeedPoller {
  /// The duration over which feeds should be polled.
  pub period: Duration,

  /// The event bus for app logic.
  pub event_bus: Addr<EventBus>,
}

impl Handler<ClockTrigger> for FeedPoller {
  type Result = ResponseFuture<anyhow::Result<()>>;

  fn handle(&mut self, _msg: ClockTrigger, _ctx: &mut Context<Self>) -> Self::Result {
    let event_bus_recipient = Box::new(self.event_bus.clone().recipient());
    Box::pin(async move {
      let event_bus = event_bus_recipient;
      let conn = crate::model::open();
      join_all(crate::model::slurp_feeds(&conn).iter().map(|feed| {
        poll(&conn, (*event_bus).clone(), feed).map(|result| {
          match result {
            Err(err) => error!("{}", err),
            Ok(_) => (),
          }
        })
      })).await;
      Ok(())
    })
  }
}

impl Handler<BusMessage> for FeedPoller {
  type Result = ();

  fn handle(&mut self, _msg: BusMessage, _ctx: &mut Context<Self>) -> Self::Result {}
}

impl Actor for FeedPoller {
  type Context = Context<Self>;

  fn started(&mut self, ctx: &mut Context<Self>) {
    self.event_bus.try_send(Subscribe(ctx.address().recipient())).unwrap();
    // send initial trigger
    ctx.address().try_send(ClockTrigger()).unwrap();
    ctx.run_interval(self.period, move |_act, ctx| {
      ctx.address().try_send(ClockTrigger()).unwrap();
    });
  }
}

/// Look for updates to the existing feed.
pub async fn poll(conn: &Connection, event_bus: Recipient<Commit>, feed: &crate::model::Feed) -> Result<()> {
  info!("Polling {} at {}....", feed.title, feed.url.as_ref());
  let download_result = reqwest::get(feed.url.clone()).await?.text().await?;
  let parsed_feed = crate::parse_feed_rs(&download_result).unwrap();
  crate::app::update_feed(&conn, feed, &parsed_feed);

  for entry in parsed_feed.entries.into_iter().rev() {
    match crate::model::find_entry_by_guid(&conn, feed.id, &entry.id) {
      Some(persisted) => crate::app::update_feed_item_if_changed(conn, &persisted, &entry),
      None => {
        debug!("Unknown episode {}, adding...", entry.id);
        let title = entry.title.as_ref().unwrap().content.clone();

        let (content_url, content_type) = match entry.content.as_ref() {
          Some(content) => (
            crate::feeds::convert_link_to_url(content.src.as_ref().unwrap()),
            content.content_type.to_string(),
          ),
          None => panic!("All feed items should have content."),
        };
        let summary = entry.summary.as_ref().map(|t| t.content.clone());
        let published = entry.published.as_ref().map(|d| d.clone());
        let updated = entry.updated.as_ref().map(|d| d.clone());

        let add_entry = crate::model::AddEntry {
          guid: entry.id.clone(),
          feed_id: feed.id.clone(),
          title: title.clone(),
          summary: summary.clone(),
          content_url: content_url.clone(),
          content_type: content_type.clone(),
          published: published.clone(),
          updated: updated.clone(),
        };

        let event = crate::events::FeedEvent::FeedEntryAdded {
          guid: entry.id.clone(),
          feed_id: feed.id,
          title,
          summary,
          content_url,
          content_type,
          published,
          updated
        };
        event_bus.send(Commit(event)).await.unwrap().unwrap();
        crate::model::add_entry(&conn, &add_entry);
        let _persisted_entry = crate::model::find_entry_by_guid(&conn, feed.id, &entry.id).unwrap();
      }
    }
  }
  Ok(())
}
