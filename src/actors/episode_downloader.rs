use actix::prelude::*;
use crate::events::FeedEvent;
use crate::actors::event_bus::BusMessage;
use crate::download_object::FeedEntryDownloadObject;
use crate::util::download_queue::DownloadQueue;
use log::debug;
use chrono::{Duration, Utc};
#[derive(Default,Debug)]
/// The episode downloader.
pub struct EpisodeDownloader {
  queue: DownloadQueue<FeedEntryDownloadObject>,
}

impl Handler<BusMessage> for EpisodeDownloader {
  type Result = ();
  
  fn handle(&mut self, msg: BusMessage, _ctx: &mut Context<Self>) -> Self::Result {
    match msg {
      BusMessage(event) => {
        match event {
          FeedEvent::FeedEntryAdded{
            feed_id,
            guid,
            title: _title,
            content_url,
            content_type: _content_type,
            summary: _summary,  
            published,
            updated: _updated,
          } => {
            let path = crate::cache::create_cache_location(feed_id.to_string().as_str(), guid.as_str());
            let effective_date = published.unwrap();
            
            if Utc::now().signed_duration_since(effective_date) > Duration::weeks(1) {
              let download_object = FeedEntryDownloadObject{
                feed_id,
                guid,
                path,
                content_url: content_url.clone(),
                effective_date: published.unwrap(),
              };
              debug!("enqueuing {}", download_object);
              self.queue.push(download_object);
            }
          }
          _ => ()
        }
      }
    }
  }
}

impl Actor for EpisodeDownloader {
  type Context = Context<Self>;

  fn started(&mut self, ctx: &mut Self::Context) {
    self.queue.start()
  }
}
