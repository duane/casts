extern crate rusqlite;

extern crate casts;
extern crate chrono;
extern crate clap;
extern crate futures;
extern crate indicatif;
extern crate magic;
extern crate regex;
extern crate reqwest;
extern crate tempfile;
extern crate url;
extern crate uuid;
extern crate xdg;

use std::path::Path;
use uuid::Uuid;

mod model;
mod net;

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

fn main() {
  let opts: Opts = Opts::parse();

  match opts.cmd {
    SubCommand::Subscribe(s) => {
      let conn = model::open();
      model::initialize(&conn);

      let result = blocking_download_feed_rs(s.feed.clone()).unwrap();
      let feed = parse_feed_rs(&result).unwrap();

      let (title, title_content_type) = match feed.title {
        Some(t) => (Some(t.content.clone()), Some(t.content_type.to_string())),
        None => (None, None),
      };
      let (description, description_content_type) = match feed.description {
        Some(d) => (Some(d.content.clone()), Some(d.content_type.to_string())),
        None => (None, None),
      };

      let add_feed = model::AddFeed {
        id: Uuid::new_v4(),
        url: Url::parse(&s.feed).unwrap(),
        title,
        title_content_type,
        description,
        description_content_type,
      };

      let _ = model::add_feed(&conn, &add_feed);
      for entry in feed.entries.into_iter().rev() {
        let (title, title_content_type) = match entry.title {
          Some(t) => (Some(t.content.clone()), Some(t.content_type.to_string())),
          None => (None, None),
        };
        let (summary, summary_content_type) = match entry.summary {
          Some(s) => (Some(s.content.clone()), Some(s.content_type.to_string())),
          None => (None, None),
        };
        let (content_url, content_content_type) = match entry.content {
          Some(c) => (
            c.src.map(|l| Url::parse(&l.href).unwrap()),
            Some(c.content_type.to_string()),
          ),
          None => (None, None),
        };

        let add_entry = model::AddEntry {
          id: Uuid::new_v4(),
          feed_id: add_feed.id.clone(),
          title,
          title_content_type,
          summary,
          summary_content_type,
          content_url,
          content_content_type,
          published: entry.published,
          updated: entry.updated,
        };
        let _ = model::add_entry(&conn, &add_entry);
      }

      conn.close().expect("could rm not close db");
    }
    SubCommand::Dump(d) => {
      let result = blocking_download_feed_rs(d.feed).unwrap();
      let feed = parse_feed_rs(&result).unwrap();

      println!("{:?}", feed);
    }
    SubCommand::Download(_) => {
      let xdg_dirs = xdg::BaseDirectories::with_prefix("casts").unwrap();
      let conn = model::open();
      match model::fetch_latest_entry(&conn) {
        None => println!("Nothing to download...."),
        Some(e) => {
          let feed = model::get_feed(&conn, &e.feed_id).unwrap();
          let mp3_path = xdg_dirs
            .place_cache_file(Path::new(&e.feed_id.to_string()).with_extension("mp3"))
            .expect("could not create cache directory");
          println!(
            "Dowloading {:} -  {:} to {:}!",
            feed.title.clone().unwrap_or("n/a".to_string()),
            e.title.clone().unwrap_or("n/a".to_string()),
            mp3_path.to_str().unwrap_or("")
          );
          e.content_url
            .clone()
            .map(|url| net::blocking_stream_to_file(&url, &mp3_path));
          println!(
            "Dowloaded {:} -  {:} to {:}!",
            feed.title.unwrap_or("n/a".to_string()),
            e.title.unwrap_or("n/a".to_string()),
            mp3_path.to_str().unwrap_or("")
          );
        }
      };
    }
  }
}
