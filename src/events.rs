use url::Url;
use chrono::{DateTime, Utc};

/// The main event type driving the app logic of polling and downloading feeds.
#[derive(Debug,Clone)]
pub enum FeedEvent {
  /// Event send when feed is subscribed to.
  FeedAdded(Url),

  /// Event sent when a new entry is observed.
  FeedEntryAdded {
    /// The primary key of the feed.
    feed_id: i64,
  
    /// The guid of the entry.
    guid: String,
  
    /// The title of the entry.
    title: String,
  
    /// The url of the content (likely audio).
    content_url: Url,
  
    /// The content type of the content (unreliable).
    content_type: String,
  
    /// The summary of the content.
    summary: Option<String>,
  
    /// The published date.
    published: Option<DateTime<Utc>>,
  
    /// The last-modified date.
    updated: Option<DateTime<Utc>>,
  },
}