#![deny(missing_docs)]
#![deny(missing_doc_code_examples)]

//! casts is a tool to manage and listen to podcasts from the command line.

extern crate rusqlite;

extern crate base64;
extern crate bytesize;
extern crate casts;
extern crate chrono;
extern crate clap;
extern crate colored;
extern crate futures;
extern crate html2text;
extern crate htmlescape;
extern crate magic;
extern crate notify_rust;
extern crate regex;
extern crate reqwest;
extern crate tempfile;
extern crate url;
extern crate uuid;
extern crate xdg;

use bytesize::ByteSize;
use chrono::{Duration, Utc};
use std::io::{stdout, Write};

mod app;
mod cache;
mod model;
mod net;
mod view;

use casts::{blocking_download_feed_rs, parse_feed_rs};
use clap::Clap;
use url::Url;

#[derive(Clap, Debug)]
#[clap(version = "1.0", author = "Duane R Bailey <duane@duane.cc>")]
struct Opts {
  #[clap(subcommand)]
  cmd: SubCommand,
}

#[derive(Clap, Debug)]
enum SubCommand {
  Subscribe(Subscribe),
  Dump(Dump),
  Download(Download),
  Feeds(Feeds),
  Feed(Feed),
  Fresh(Fresh),
  Poll(Poll),
}

#[derive(Clap, Debug)]
struct Subscribe {
  feed: String,
}

#[derive(Clap, Debug)]
struct Dump {
  feed: String,
}

#[derive(Clap, Debug)]
struct Download {}

#[derive(Clap, Debug)]
struct Feeds {}

#[derive(Clap, Debug)]
struct Poll {
  feed: Option<String>,
}

#[derive(Clap, Debug)]
struct Feed {
  arg: String,
}

#[derive(Clap, Debug)]
struct Fresh {}

fn main() {
  let opts: Opts = Opts::parse();

  match opts.cmd {
    SubCommand::Subscribe(s) => {
      let url = Url::parse(&s.feed).unwrap();
      app::subscribe(&url);
    }
    SubCommand::Dump(d) => {
      let url = Url::parse(&d.feed).unwrap();
      let result = blocking_download_feed_rs(&url).unwrap();
      let feed = parse_feed_rs(&result).unwrap();

      println!("{:?}", feed);
    }
    SubCommand::Poll(p) => {
      let conn = model::open();
      let feeds = match p.feed {
        Some(feed_name) => match model::get_feed_by_name(&conn, &feed_name) {
          Some(feed) => vec![feed],
          None => {
            println!("No such feed: {}", feed_name);
            vec![]
          }
        },
        None => model::slurp_feeds(&conn),
      };
      for feed in feeds.into_iter() {
        app::poll(&conn, &feed);
      }
    }
    SubCommand::Fresh(_) => {
      let conn = model::open();
      let entries = model::slurp_entries_since(
        &conn,
        &Utc::now().checked_sub_signed(Duration::weeks(1)).unwrap(),
      );
      for entry in entries.into_iter() {
        let feed = model::get_feed_by_id(&conn, entry.feed_id).unwrap();
        view::show_feed_and_entry(&feed, &entry, false);
      }
    }
    SubCommand::Download(_) => {
      let conn = model::open();
      let entries = model::slurp_entries_since(
        &conn,
        &Utc::now().checked_sub_signed(Duration::weeks(1)).unwrap(),
      );
      for entry in entries.into_iter() {
        let feed = model::get_feed_by_id(&conn, entry.feed_id).unwrap();
        if cache::should_download(&feed, &entry) {
          let feed = model::get_feed_by_id(&conn, entry.feed_id).unwrap();
          let content_url: Url = entry.content_url.clone();
          print!(
            "Downloading {} - {}...",
            &feed.title.clone(),
            &entry.guid.clone()
          );
          stdout().flush().unwrap();
          let size =
            net::blocking_stream_to_file(&content_url, &cache::cache_location(&feed, &entry));
          println!("done, {:}", ByteSize::b(size as u64).to_string());
        } else {
          println!("Skipping {:}, already downloaded.", &entry.guid);
        }
      }
    }
    SubCommand::Feeds(_) => {
      let conn = model::open();

      let feeds = model::slurp_feeds(&conn);
      for feed in feeds {
        view::show_feed(&feed, false);
      }
    }
    SubCommand::Feed(l) => {
      let conn = model::open();

      // Direct titles
      let feed = model::get_feed_by_name(&conn, &l.arg).unwrap();
      view::show_feed(&feed, true);
    }
  }
}
