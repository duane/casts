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
use clap::{App, Arg, SubCommand};
use url::Url;

fn main() {
  let matches = App::new("casts")
    .version("0.1")
    .author("Duane R Bailey <duane@duane.cc>")
    .subcommand(SubCommand::with_name("subscribe").arg(Arg::with_name("INPUT").required(true)))
    .subcommand(SubCommand::with_name("dump").arg(Arg::with_name("INPUT").required(true)))
    .subcommand(SubCommand::with_name("poll").arg(Arg::with_name("FEED").required(false)))
    .subcommand(SubCommand::with_name("download"))
    .subcommand(SubCommand::with_name("feeds"))
    .subcommand(SubCommand::with_name("fresh"))
    .get_matches();
  if let Some(matches) = matches.subcommand_matches("subscribe") {
    let url = Url::parse(&matches.value_of("INPUT").unwrap()).unwrap();
    app::subscribe(&url);
  } else if let Some(matches) = matches.subcommand_matches("dump") {
    let url = Url::parse(&matches.value_of("INPUT").unwrap()).unwrap();
    let result = blocking_download_feed_rs(&url).unwrap();
    let feed = parse_feed_rs(&result).unwrap();

    println!("{:?}", feed);
  } else if let Some(matches) = matches.subcommand_matches("poll") {
    let conn = model::open();
      let feeds = match matches.value_of("FEED") {
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
  } else if let Some(_) = matches.subcommand_matches("fresh") {
    let conn = model::open();
    let entries = model::slurp_entries_since(
      &conn,
      &Utc::now().checked_sub_signed(Duration::weeks(1)).unwrap(),
    );
    for entry in entries.into_iter() {
      let feed = model::get_feed_by_id(&conn, entry.feed_id).unwrap();
      view::show_feed_and_entry(&feed, &entry, false);
    }
  } else if let Some(_) = matches.subcommand_matches("download") {
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
  } else if let Some(_) = matches.subcommand_matches("feeds") {
    let conn = model::open();

    let feeds = model::slurp_feeds(&conn);
    for feed in feeds {
      view::show_feed(&feed, false);
    }
  } else if let Some(matches) = matches.subcommand_matches("feed") {
    let conn = model::open();

    // Direct titles
    let feed = model::get_feed_by_name(&conn, matches.value_of("FEED").unwrap()).unwrap();
    view::show_feed(&feed, true);
  }
}
