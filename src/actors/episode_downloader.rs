use actix::prelude::*;
use crate::events::FeedEvent;
use std::{collections::BTreeMap, path::PathBuf};
use crate::actors::event_bus::BusMessage;
use url::Url;
use chrono::{DateTime, Utc};
use futures::{future::ready, stream::{FuturesOrdered, StreamExt}};
use std::rc::Rc;
use std::cell::RefCell;
use log::debug;
use futures::FutureExt;
use std::future::Future;
use std::pin::Pin;
use futures::future::Map;
use hyper::{Client, Request};
use std::time::Duration;

#[derive(Debug)]
enum DownloadState {
  Ready,
  InProgress(usize)
}

type DownloadKey = (DateTime<Utc>, i64);

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
  fn sort_key(&self) -> DownloadKey {
    (self.effective_date, self.feed_id).clone()
  }

  fn is_ready(&self) -> bool {
    match self.state {
      DownloadState::Ready => true,
      DownloadState::InProgress(_) => false
    }
  }
}

/// The episode downloader.
pub struct EpisodeDownloader {
  n: usize,
  queue: BTreeMap<DownloadKey, Rc<RefCell<DownloadObject>>>,
  drain: Pin<Box<dyn Future<Output=()>>>,
}

impl Default for EpisodeDownloader {
  fn default() -> Self {
    EpisodeDownloader{
      n: 3,
      queue: Default::default(),
      drain: Box::pin(ready(())),
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
            self.enqueue_object(DownloadObject{
              feed_id,
              guid,
              title,
              content_url: content_url.clone(),
              effective_date: published.unwrap(),
              state: DownloadState::Ready,
            });
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
  fn enqueue_object(&mut self, download_object: DownloadObject) {
    self.queue.insert(download_object.sort_key(), Rc::new(RefCell::new(download_object)));
  }

  fn trigger_finished(&mut self, key: DownloadKey) {
    self.queue.remove(&key);
    debug!("Download object finished: {:?}", key);
  }

  async fn drain_queue(&mut self) {
    let mut ready_iter = self.queue.values().filter(|obj| (*obj).borrow().is_ready());
    let mut in_flight = FuturesOrdered::new();

    // spin up n futures
    while in_flight.len() < self.n  {
      match ready_iter.next() {
        Some(obj) => {
          let future = download_object(obj.clone());
          in_flight.push(future);
          println!("Downloading {:?}", (*obj).borrow().sort_key());
        },
        _ => break
      };
    }

    // remove the objects from the queue as this finishes
    loop {
      match in_flight.next().await {
        Some(result) => {
          self.trigger_finished(result);
          self.queue.values().filter(|obj|(*obj).borrow().is_ready()).
            next(). // pop another download object off the queue
            and_then(|f| {
              Some(in_flight.push(download_object(f.clone()))) // and download it using this worker slot.
          });
        },
        None => {
          // and fully wind down when there's nothing left to drain.
          debug!("queue drained");
          return;
        }
      }
    }
  }
}

async fn download_object(obj: Rc<RefCell<DownloadObject>>) -> DownloadKey {
  (*obj).borrow_mut().state = DownloadState::InProgress(0);
  let key = (*obj).borrow().sort_key();
  let content_url = (*obj).borrow().content_url.clone();
  let target_path = crate::cache::create_cache_location((*obj).borrow().title.as_str(), (*obj).borrow().guid.as_str());
  /*crate::net::stream_to_file_owned(content_url, target_path).map(move |result| {
    let _result = result.unwrap();
    key
  })*/
  key
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
