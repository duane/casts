extern crate rusqlite;

extern crate clap;
extern crate casts_rs;
extern crate url;

mod model;

use clap::Clap;
use casts_rs::{blocking_download_feed_rs, parse_feed_rs};
use url::Url;

#[derive(Clap,Debug)]
#[clap(version = "1.0", author = "Duane R Bailey <duane@duane.cc>")]
struct Opts {

  #[clap(subcommand)]
  cmd: SubCommand,
}

#[derive(Clap,Debug)]
enum SubCommand {
  Subscribe(Subscribe),
  Dump(Dump),
}

#[derive(Clap,Debug)]
struct Subscribe {
  feed: String,
}

#[derive(Clap,Debug)]
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

      let feed_id = model::add_feed(&conn, Url::parse(&s.feed).unwrap());

      let (title, title_content_type) = match feed.title {
        Some(t) => (Some(t.content.clone()), Some(t.content_type.to_string())),
        None => (None, None),
      };
      let (description, description_content_type) = match feed.description {
        Some(d) => (Some(d.content.clone()), Some(d.content_type.to_string())),
        None => (None, None),
      };

      let update = model::SetFeedMetadata{
        feed_id, title, title_content_type, description, description_content_type
      };

      model::set_feed_metadata(&conn, &update);

      conn.close().expect("could not close db");
    },
    SubCommand::Dump(d) => {
      let result = blocking_download_feed_rs(d.feed).unwrap();
      let feed = parse_feed_rs(&result).unwrap();
      let (title, title_content_type) = match feed.title {
        Some(t) => (Some(t.content.clone()), Some(t.content_type.to_string())),
        None => (None, None),
      };
      let (description, description_content_type) = match feed.description {
        Some(d) => (Some(d.content.clone()), Some(d.content_type.to_string())),
        None => (None, None),
      };

      println!("{:?}", title);
      println!("{:?}", title_content_type);

      println!("{:?}", description);
      println!("{:?}", description_content_type);
    }
  }

  
}
