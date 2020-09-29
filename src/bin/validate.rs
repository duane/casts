extern crate casts;

use casts::{blocking_download_feed_rs, parse_feed_rs};
use std::env;
use std::process::exit;
use url::Url;

fn main() {
  let mut args: Vec<String> = env::args().collect();
  if args.len() != 2 {
    println!("USAGE: validate PODCAST_URL");
    exit(1);
  }
  let arg: String = args.pop().unwrap();
  let body = match blocking_download_feed_rs(&Url::parse(&arg).unwrap()) {
    Ok(body) => body,
    Err(err) => {
      println!("{:?}", err);
      exit(1);
    }
  };
  let _parsed_result = match parse_feed_rs(&body) {
    Ok(feed_body) => feed_body,
    Err(err) => {
      println!("{:?}", err);
      exit(1);
    }
  };
  println!("valid feed!");
}
