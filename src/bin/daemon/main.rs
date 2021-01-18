extern crate anyhow;
extern crate base64;
extern crate bytesize;
extern crate casts;
extern crate chrono;
extern crate clap;
extern crate colored;
extern crate env_logger;
extern crate futures;
extern crate html2text;
extern crate htmlescape;
extern crate hyper;
extern crate log;
extern crate quick_xml;
extern crate regex;
extern crate reqwest;
extern crate rusqlite;
extern crate tempfile;
extern crate url;
extern crate uuid;
extern crate xdg;

use std::time::Duration;

use actix::prelude::*;
use casts::actors::{event_bus::{EventBus, Subscribe}, feed_poller::FeedPoller};
use casts::actors::event_logger::EventLogger;
use casts::actors::episode_downloader::EpisodeDownloader;
use casts::console::Console;

async fn whatever(local_tasks: &tokio::task::LocalSet) {
  let system_fut = actix_rt::System::run_in_tokio("main", local_tasks);

  let event_bus = EventBus::default().start();

  let event_logger = EventLogger::default().start();
  event_bus.try_send(Subscribe(event_logger.recipient().clone())).unwrap();

  let episode_downloader = EpisodeDownloader::default().start();
  event_bus.try_send(Subscribe(episode_downloader.recipient().clone())).unwrap();

  FeedPoller{
    period: Duration::new(60, 0),
    event_bus,
  }.start();

  system_fut.await.unwrap();
}

fn main() {
  env_logger::init();

  let mut tok_runtime = tokio::runtime::Runtime::new().unwrap();
  let local_tasks = tokio::task::LocalSet::new();

  let _console = Console::spawn();

  local_tasks.block_on(&mut tok_runtime, whatever(&local_tasks))
}