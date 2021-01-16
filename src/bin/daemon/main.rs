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
use casts::actors::feed_poller::FeedPoller;
use casts::actors::event_bus::{EventBus, Subscribe};
use casts::actors::event_logger::EventLogger;
use casts::actors::episode_downloader::EpisodeDownloader;

fn main() {
  let system = System::new("casts");
  env_logger::init();

  let event_bus_actor: EventBus = Default::default();
  let event_bus = event_bus_actor.start();
  
  let event_logger = EventLogger{}.start();
  event_bus.try_send(Subscribe(event_logger.recipient().clone())).unwrap();

  let episode_downloader: EpisodeDownloader = Default::default();
  let episode_downloader_addr = episode_downloader.start();
  event_bus.try_send(Subscribe(episode_downloader_addr.recipient().clone())).unwrap();

  FeedPoller {
    period: Duration::new(60, 0),
    event_bus
  }.start();

  system.run().unwrap();
}
