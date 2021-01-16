use crate::model::{Entry, Feed};

use chrono::{Duration, Utc};
use std::path::{Path, PathBuf};

//const FRESH_DURATION: Duration = Duration::weeks(1);

/// Generate a string to store the content associated with the entry.
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

/// Generate a string to store the content associated with the entry.
pub fn create_cache_location(title: &str, guid: &str) -> PathBuf {
  let xdg_dirs = xdg::BaseDirectories::with_prefix("casts").unwrap();
  let encoded_guid = base64::encode(String::from(guid).into_bytes());
  let path = Path::new(title)
    .join(&encoded_guid)
    .with_extension("mp3");
  xdg_dirs
    .place_cache_file(path)
    .expect("could not create cache directory")
}

/// Whether or not the feed entry is still "fresh"
pub fn is_fresh_entry(entry: &Entry) -> bool {
  Utc::now().signed_duration_since(entry.published.unwrap()) < Duration::weeks(1)
}

/// whether or not the entry is eligible for downloading.
pub fn should_download(feed: &Feed, entry: &Entry) -> bool {
  is_fresh_entry(&entry) && !cache_location(&feed, &entry).exists()
}
