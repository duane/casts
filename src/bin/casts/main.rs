extern crate rusqlite;

extern crate casts;
extern crate clap;
extern crate url;
extern crate xdg;

mod model;

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
}

#[derive(Clap, Debug)]
struct Subscribe {
  feed: String,
}

#[derive(Clap, Debug)]
struct Dump {
  feed: String,
}

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
        feed: Url::parse(&s.feed).unwrap(),
        title,
        title_content_type,
        description,
        description_content_type,
      };

      let feed_id = model::add_feed(&conn, &add_feed);
      for entry in feed.entries.into_iter() {
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
          feed_id,
          title,
          title_content_type,
          summary,
          summary_content_type,
          content_url,
          content_content_type,
        };
        let _ = model::add_entry(&conn, &add_entry);
      }

      conn.close().expect("could not close db");
    }
    SubCommand::Dump(d) => {
      let result = blocking_download_feed_rs(d.feed).unwrap();
      let feed = parse_feed_rs(&result).unwrap();

      println!("{:?}", feed);
    }
  }
}
