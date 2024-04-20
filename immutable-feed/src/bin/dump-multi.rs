// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

extern crate url;

use std::io::BufRead;

use feed_rs::parser::parse;
use url::Url;

/// What is parsed from an RSS feed.
pub struct Feed {
    pub url: Url,
    pub name: String,
    pub title: String,
    pub image_url: Option<Url>,
    pub description: Option<Vec<u8>>,
}

pub fn list() {}
// const PODCAST: &str = include_str!("../../MajorityReport");
const PODCAST: &str = include_str!("../../history-of-rome");

fn main() {
    let feed = parse(PODCAST.as_bytes()).unwrap();
    println!("{:?}", feed);
}
