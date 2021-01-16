use actix::prelude::*;
use crate::events::FeedEvent;
use std::collections::BTreeMap;
use crate::actors::event_bus::BusMessage;
use url::Url;
use chrono::{DateTime, Utc};
use futures::stream::futures_unordered::FuturesUnordered;
use std::rc::Rc;
use std::cell::RefCell;

#[derive(Debug)]
enum DownloadState {
  Ready,
  InProgress(usize),
  Complete(usize),
}

#[derive(Debug)]
struct DownloadObject {
  feed_id: i64,
  guid: String,
  title: String,
  content_url: Url,
  effective_date: DateTime<Utc>,
  state: DownloadState
}

impl DownloadObject {
  fn sort_key(&self) -> (DateTime<Utc>, i64) {
    (self.effective_date, self.feed_id).clone()
  }
}

/// The episode downloader.
pub struct EpisodeDownloader {
  n: usize,
  queue: BTreeMap<(DateTime<Utc>, i64), Rc<RefCell<DownloadObject>>>,
  in_flight: FuturesUnordered<()>
}

impl Default for EpisodeDownloader {
  fn default() -> Self {
    EpisodeDownloader{
      n: 3,
      queue: Default::default(),
      in_flight: Default::default(),
    }
  }
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
            title,
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
              title,
              content_url: content_url.clone(),
              effective_date,
              state: DownloadState::Ready,
            };
            self.queue.insert(download_object.sort_key(), Rc::new(RefCell::new(download_object)));
            self.trigger_downloads();
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
  fn trigger_downloads(&mut self) {
    let mut ready_iter = self.queue.values().filter(|obj| {
      match (*obj).borrow().state {
        DownloadState::Ready => true,
        _ => false
      }
    });
    while self.in_flight.len() < self.n  {
      match ready_iter.next() {
        Some(obj) => {
          let _future = self.download_object(obj.clone());
          self.in_flight.push(());
          println!("Downloading {}", (*obj).borrow().content_url);
        },
        _ => break
      };
    }
  }

  async fn download_object(&self, obj: Rc<RefCell<DownloadObject>>) {
    (*obj).borrow_mut().state = DownloadState::InProgress(0);
    let target_path = crate::cache::create_cache_location((*obj).borrow().title.as_str(), (*obj).borrow().guid.as_str());
    (*obj).borrow_mut().state = DownloadState::Complete(crate::net::stream_to_file(&(*obj).borrow().content_url, &target_path).await.unwrap());
  }
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
