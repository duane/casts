use crate::model::{Entry, Feed};

use chrono::{Duration, Utc};
use std::path::{Path, PathBuf};

//const FRESH_DURATION: Duration = Duration::weeks(1);

pub fn cache_location(feed: &Feed, entry: &Entry) -> PathBuf {
  let xdg_dirs = xdg::BaseDirectories::with_prefix("casts").unwrap();
  let encoded_guid = base64::encode(entry.guid.clone().into_bytes());
  let path = Path::new(&feed.title.clone())
    .join(&encoded_guid)
    .with_extension("mp3");
  xdg_dirs
    .place_cache_file(path)
    .expect("could not create cache directory")
}

pub fn is_fresh_entry(entry: &Entry) -> bool {
  Utc::now().signed_duration_since(entry.published.unwrap()) < Duration::weeks(1)
}

pub fn should_download(feed: &Feed, entry: &Entry) -> bool {
  is_fresh_entry(&entry) && !cache_location(&feed, &entry).exists()
}
