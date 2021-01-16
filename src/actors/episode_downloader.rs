use actix::prelude::*;
use crate::events::FeedEvent;
use std::collections::BTreeMap;
use crate::actors::event_bus::BusMessage;
use url::Url;
use chrono::{DateTime, Utc};
//use futures::stream::futures_unordered::FuturesUnordered;

/*enum DownloadState {
  Ready,
  InProgress(usize),
  Complete,
}*/

#[derive(Debug)]
struct DownloadObject {
  feed_id: i64,
  guid: String,
  content_url: Url,
  effective_date: DateTime<Utc>,
}

#[derive(Default)]
/// The episode downloader.
pub struct EpisodeDownloader {
  queue: BTreeMap<DateTime<Utc>, Vec<DownloadObject>>
//  in_flight: FuturesUnordered<_>
}

/// Download an object.
/*async fn download_object(_object: &DownloadObject) -> () {
  ()
}*/

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
            let effective_date = published.unwrap();
            let download_object = DownloadObject{
              feed_id,
              guid,
              content_url: content_url.clone(),
              effective_date
            };
            match self.queue.get_mut(&effective_date) {
              Some(existing_list) => existing_list.push(download_object),
              None => { self.queue.insert(effective_date, vec![download_object]);}
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
}

impl EpisodeDownloader {
  
}

/*
/// Whether or not the feed entry is still "fresh"
pub fn is_fresh_entry(entry: &) -> bool {
  Utc::now().signed_duration_since(entry.published.unwrap()) < Duration::weeks(1)
}

/// whether or not the entry is eligible for downloading.
pub fn should_download(feed: &Feed, entry: &Entry) -> bool {
  is_fresh_entry(&entry) && !cache_location(&feed, &entry).exists()
}
*/
