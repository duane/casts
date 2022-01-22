use crate::util::download_queue::DownloadTraitObject;
use chrono::{DateTime, Utc};
use std::cmp::Ordering;
use std::fmt;
use std::path::{Path, PathBuf};
use url::Url;

/// The actual state useful to describe an episode download.
#[derive(Debug, Eq, Clone)]
pub struct FeedEntryDownloadObject {
  /// The id of the feed to which this episode belongs. Used to identify the download in callbacks later.
  pub feed_id: i64,

  /// The guid of the episode. Used to identify the download in callbacks later.
  pub guid: String,

  /// The download path of the episode.
  pub path: PathBuf,

  /// The url of the content of the episode.
  pub content_url: Url,

  /// A date representing the episode's first known observation. Used to order the downoads by recency.
  pub effective_date: DateTime<Utc>,
}

impl PartialEq for FeedEntryDownloadObject {
  fn eq(&self, other: &Self) -> bool {
    self.feed_id == other.feed_id && self.guid == other.guid
  }
}

impl PartialOrd for FeedEntryDownloadObject {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for FeedEntryDownloadObject {
  fn cmp(&self, other: &Self) -> Ordering {
    (self.effective_date, self.feed_id).cmp(&(other.effective_date, other.feed_id))
  }
}

impl fmt::Display for FeedEntryDownloadObject {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}/{}", self.feed_id, self.guid)
  }
}

impl DownloadTraitObject for FeedEntryDownloadObject {
  fn url(&self) -> &Url {
    &self.content_url
  }

  fn path(&self) -> &Path {
    &self.path
  }
}
