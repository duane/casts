use super::events::FeedAggregateEvent;
use chrono::{DateTime, Utc};
use event_source::aggregate::Aggregate;
use event_source::client::applicable::EventApplicable;
use std::collections::HashMap;
use uuid::Uuid;
use url::Url;

/// The feed aggregate, representing the total result of events in-memory.
#[derive(Default, Serialize, Clone, Debug)]
pub struct FeedAggregate {
  /// The id of the aggregate
  pub id: Uuid,

  /// The version of the aggregate.
  pub version: i64,

  /// The feeds associated with the aggregate by feed id.
  pub feeds_by_feed_id: HashMap<String, Feed>,
}

/// The feed itself.
#[derive(Serialize, Clone, Debug)]
pub struct Feed {
  /// The id of the feed
  pub feed_id: String,

  /// The title of the feed.
  pub title: String,

  /// The url where the feed may be found.
  pub url: Url,

  /// The description of the feed.
  pub description: Option<String>,

  /// When the feed was last updated.
  pub updated_at: Option<DateTime<Utc>>,

  /// When the feed was first published.
  pub published_at: Option<DateTime<Utc>>,

  /// The entries associated with the feed.
  pub entries: HashMap<String, Entry>,
}

/// An entry associated with the feed.
#[derive(Serialize, Clone, Debug)]
pub struct Entry {
  /// The self-identified guid of the feed entry.
  pub guid: String,

  /// The url where the entry content may be found.
  pub url: Url,

  /// The title of the feed entry.
  pub title: Option<String>,

  /// A summary of the entry content.
  pub summary: Option<String>,

  /// When the entry was last updated.
  pub updated_at: Option<DateTime<Utc>>,

  /// When the entry was first published.
  pub published_at: Option<DateTime<Utc>>,
}

impl From<Entry> for immutable_feed::Entry {
  fn from(entry: Entry) -> immutable_feed::Entry {
    immutable_feed::Entry{
      guid: entry.guid,
      url: entry.url,
      title: entry.title,
      summary: entry.summary,
      updated_at: entry.updated_at,
      published_at: entry.published_at,
    }
  }
}

impl FeedAggregate {
  /// Make a new feed aggregate.
  pub fn new() -> FeedAggregate {
    FeedAggregate::with_id(Uuid::new_v4())
  }
}

impl Aggregate for FeedAggregate {
  fn with_id(uuid: Uuid) -> FeedAggregate {
    FeedAggregate {
      id: uuid,
      version: 0,
      feeds_by_feed_id: HashMap::new(),
    }
  }

  fn version(&self) -> i64 {
    self.version
  }

  fn id(&self) -> Uuid {
    self.id
  }
}

impl EventApplicable for FeedAggregate {
  type Event = FeedAggregateEvent;

  fn apply(&self, event: &FeedAggregateEvent) -> FeedAggregate {
    let mut applied = self.clone();
    match event {
      &FeedAggregateEvent::Added {
        id: _,
        ref feed_id,
        ref title,
        ref url,
        ref description,
        ref updated_at,
        ref published_at,
      } => {
        match applied.feeds_by_feed_id.get(feed_id) {
          Some(_) => unreachable!(),
          None => {
            let new_feed = Feed {
              feed_id: feed_id.to_owned(),
              title: title.to_owned(),
              url: url.to_owned(),
              description: description.to_owned(),
              updated_at: updated_at.to_owned(),
              published_at: published_at.to_owned(),
              entries: Default::default(),
            };
            applied.feeds_by_feed_id.insert(feed_id.to_owned(), new_feed);
          }
        };
      }
      &FeedAggregateEvent::EntryObserved {
        id: _,
        ref feed_id,
        ref guid,
        ref title,
        ref url,
        ref summary,
        ref updated_at,
        ref published_at,
      } => {
        let feed = applied.feeds_by_feed_id.get_mut(feed_id).unwrap();
        match feed.entries.get(guid) {
          Some(_) => unreachable!(),
          None => {
            let entry = Entry {
              guid: guid.to_owned(),
              title: title.to_owned(),
              url: url.to_owned(),
              summary: summary.to_owned(),
              updated_at: updated_at.to_owned(),
              published_at: published_at.to_owned(),
            };
            feed.entries.insert(guid.to_owned(), entry);
          }
        }
      }
      &FeedAggregateEvent::EntryTitleUpdated {
        id: _,
        ref feed_id,
        ref guid,
        ref title,
      } => {
        let feed = applied.feeds_by_feed_id.get_mut(feed_id).unwrap();
        let mut entry = feed.entries.get_mut(guid).unwrap();
        entry.title = title.to_owned();
      }
      &FeedAggregateEvent::EntrySummaryUpdated {
        id: _,
        ref feed_id,
        ref guid,
        ref summary,
      } => {
        let feed = applied.feeds_by_feed_id.get_mut(feed_id).unwrap();
        let mut entry = feed.entries.get_mut(guid).unwrap();
        entry.summary = summary.to_owned();
      }
      &FeedAggregateEvent::EntryUrlUpdated {
        id: _,
        ref feed_id,
        ref guid,
        ref url,
      } => {
        let feed = applied.feeds_by_feed_id.get_mut(feed_id).unwrap();
        let mut entry = feed.entries.get_mut(guid).unwrap();
        entry.url = url.to_owned();
      }
      &FeedAggregateEvent::EntryDisappeared {
        id: _,
        ref feed_id,
        ref guid,
      } => {
        applied.feeds_by_feed_id.get_mut(feed_id).unwrap().entries.remove(guid);
      }
    }
    applied.version += 1;
    applied
  }
}

impl From<Feed> for immutable_feed::Feed {
  fn from(feed: Feed) -> immutable_feed::Feed {
    immutable_feed::Feed{
      url: feed.url,
      title: feed.title,
      description: feed.description,
      updated_at: feed.updated_at,
      published_at: feed.published_at,
      entries: feed.entries.values().map(|v| immutable_feed::Entry::from(v.clone())).collect(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn creates_new_aggregate() {
    let uuid = Uuid::new_v4();
    let aggregate = FeedAggregate::with_id(uuid.clone());
    assert_eq!(uuid, aggregate.id);
    assert_eq!(0, aggregate.version);
  }

  #[test]
  fn works_with_sentinel_uuid() {
    let uuid = Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap();
    let aggregate = FeedAggregate::with_id(uuid.clone());
    assert_eq!(uuid, aggregate.id);
    assert_eq!(0, aggregate.version);
  }

  #[test]
  fn adds_item() {
    let aggregate = FeedAggregate::new();
    let feed_id = Uuid::new_v4().to_string();
    let event = FeedAggregateEvent::Added {
      id: aggregate.id,
      feed_id: feed_id.clone(),
      title: String::from("Examplecast"),
      url: Url::parse("http://example.com/examplecast.xml"),
      description: None,
      updated_at: None,
      published_at: None,
    };
    let aggregate_after_application = aggregate.apply(&event);
    assert_eq!(aggregate_after_application.feeds_by_feed_id.len(), 1);
    assert_eq!(aggregate_after_application.version(), 1);
    assert_eq!(
      aggregate_after_application.feeds_by_feed_id[feed_id.as_str()].title,
      String::from("Examplecast")
    );
  }
}
