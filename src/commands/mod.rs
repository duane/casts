// use crate::aggregate::Feed;

use super::aggregate::FeedAggregate;
use super::events::FeedAggregateEvent;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use event_source::command::Command;
use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};
use url::Url;
use uuid::Uuid;
use std::convert::Into;
// use log::info;

/// The commands available for the aggregate.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum FeedAggregateCommand {
  /// Subscribe to a new feed.
  AddFeed {
    /// The id of the aggregate.
    id: Uuid,

    /// The feed id to represent this new feed.
    feed_id: String,

    /// The url where the feed is located.
    url: Url,
  },

  /// Observe an entry for a feed.
  FeedEntryObserved {
    /// aggregate id
    id: Uuid,

    /// Entry title
    title: String,

    /// The feed whose entry is observed.
    feed_id: String,

    /// (theoretically) unique guid of the feed entry
    guid: String,

    /// Url of the primary media content attached to the entry.
    url: Url,

    /// Text summary of the entry.
    summary: Option<String>,

    /// When the entry was last updated.
    updated_at: Option<DateTime<Utc>>,

    /// When the entry was first published.
    published_at: Option<DateTime<Utc>>,
  },
}

/// The error associated with running commands against a feed aggregate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeedAggregateError {
  /// A feed with this id already exists associated with the aggregate.
  DuplicateFeedError,

  /// The feed is not well-formed.
  BadFeedError,
}

impl Error for FeedAggregateError {
  fn cause(&self) -> Option<&dyn Error> {
    None
  }

  fn description(&self) -> &str {
    match self {
      &FeedAggregateError::DuplicateFeedError => "This feed has already been added.",
      &FeedAggregateError::BadFeedError => "The url does not point to a feed.",
    }
  }
}

impl Display for FeedAggregateError {
  fn fmt(&self, f: &mut Formatter) -> FmtResult {
    write!(f, "{:?}", self)
  }
}

#[async_trait]
impl Command for FeedAggregateCommand {
  type Aggregate = FeedAggregate;
  type Error = FeedAggregateError;

  fn id(&self) -> &Uuid {
    match self {
      &FeedAggregateCommand::AddFeed {
        id: ref inner_id,
        feed_id: _,
        url: _,
      } => inner_id,
      &FeedAggregateCommand::FeedEntryObserved {
        id: ref inner_id,
        title: _,
        feed_id: _,
        guid: _,
        url: _,
        summary: _,
        updated_at: _,
        published_at: _,
      } => inner_id,
    }
  }

  fn apply(
    &self,
    aggregate: &FeedAggregate,
  ) -> Result<Vec<FeedAggregateEvent>, FeedAggregateError> {
    match self {
      &FeedAggregateCommand::AddFeed {
        id: _,
        ref feed_id,
        url: _,
      } => match aggregate.feeds.get(feed_id) {
        Some(_feed_id) => Err(FeedAggregateError::DuplicateFeedError),
        _ => Ok(vec![]),
      },
      &FeedAggregateCommand::FeedEntryObserved {
        id: ref id,
        title: ref title,
        feed_id: ref feed_id,
        guid: ref guid,
        url: ref url,
        summary: ref summary,
        updated_at: ref updated_at,
        published_at: ref published_at,
      } => match aggregate.feeds.get(feed_id) {
        Some(aggregate_feed) => {
          let immutable_feed_aggregate = immutable_feed::Feed::from(aggregate_feed.clone());
          match immutable_feed_aggregate.entries.iter().find(|e| e.guid.as_str() == guid) {
            Some(existing_entry) => {
              let observed_entry = immutable_feed::Entry{
                guid: guid.to_owned(),
                title: Some(title.to_owned()),
                url: url.to_owned(),
                summary: summary.to_owned(),
                updated_at: updated_at.to_owned(),
                published_at: published_at.to_owned(),
              };
              let diff = observed_entry.diff(&existing_entry);
              for mutation in diff.iter() {
                println!("{:?}", mutation);
              }
            },
            None => {
              return Ok(
                vec![
                  FeedAggregateEvent::EntryObserved{
                    id: Uuid::new_v4(),
                    title: Some(title.to_owned()),
                    feed_id: feed_id.to_owned(),
                    guid: guid.to_owned(),
                    url: url.to_owned(),
                    summary: summary.to_owned(),
                    updated_at: updated_at.to_owned(),
                    published_at: published_at.to_owned(),
                  },
                ]
              );
            },
          }
          Ok(vec![])
        },
        None => panic!(format!("unknown feed: {:?}", feed_id)),
      },
    }
  }

  async fn async_apply(
    &self,
    aggregate: &FeedAggregate,
  ) -> Result<Vec<FeedAggregateEvent>, FeedAggregateError> {
    match self {
      &FeedAggregateCommand::AddFeed {
        ref id,
        ref feed_id,
        ref url,
      } => {
        if let Some(_) = aggregate.feeds.get(feed_id) {
          return Err(FeedAggregateError::DuplicateFeedError);
        }
        println!("url: {}", url);
        let download_result = reqwest::get(url.to_string()).await.unwrap().text().await.unwrap();
        let parsed_feed = crate::parse_feed_rs(&download_result).unwrap();
        if parsed_feed.title.is_none() {
          return Err(FeedAggregateError::BadFeedError);
        }
        let mut events = vec![FeedAggregateEvent::Added {
          id: id.to_owned(),
          url: url.to_owned(),
          feed_id: feed_id.to_owned(),
          title: parsed_feed.title.unwrap().content,
          description: parsed_feed.description.map(|t| t.content),
          updated_at: parsed_feed.updated,
          published_at: parsed_feed.published,
        }];
        for entry in parsed_feed.entries.iter() {
          let entry_url = match entry.content {
            Some(ref content) => match content.src {
              Some(ref href) => href.href.clone(),
              None => continue,
            },
            None => continue, // any entry without a url doesn't make sense in a podcast context.
          };
          let title = entry.title.clone().map(|t| t.content);
          let summary = entry.summary.clone().map(|t| t.content);
          events.push(FeedAggregateEvent::EntryObserved {
            id: id.to_owned(),
            feed_id: feed_id.to_owned(),
            guid: entry.id.clone(),
            title,
            url: Url::parse(entry_url.as_str()).expect("could not parse url"),
            summary,
            updated_at: entry.updated,
            published_at: entry.published,
          });
        }

        Ok(events)
      },
      &FeedAggregateCommand::FeedEntryObserved {
        id: ref id,
        title: ref title,
        feed_id: ref feed_id,
        guid: ref guid,
        url: ref url,
        summary: ref summary,
        updated_at: ref updated_at,
        published_at: ref published_at,
      } => match aggregate.feeds.get(feed_id) {
        Some(aggregate_feed) => {
          let immutable_feed_aggregate = immutable_feed::Feed::from(aggregate_feed.clone());
          match immutable_feed_aggregate.entries.iter().find(|e| e.guid.as_str() == guid) {
            Some(existing_entry) => {
              let observed_entry = immutable_feed::Entry{
                guid: guid.to_owned(),
                title: Some(title.to_owned()),
                url: url.to_owned(),
                summary: summary.to_owned(),
                updated_at: updated_at.to_owned(),
                published_at: published_at.to_owned(),
              };
              let diff = observed_entry.diff(&existing_entry);
              for mutation in diff.iter() {
                println!("{:?}", mutation);
              }
            },
            None => {
              return Ok(
                vec![
                  FeedAggregateEvent::EntryObserved{
                    id: Uuid::new_v4(),
                    title: Some(title.to_owned()),
                    feed_id: feed_id.to_owned(),
                    guid: guid.to_owned(),
                    url: url.to_owned(),
                    summary: summary.to_owned(),
                    updated_at: updated_at.to_owned(),
                    published_at: published_at.to_owned(),
                  },
                ]
              );
            },
          }
          Ok(vec![])
        },
        None => panic!(format!("unknown feed: {:?}", feed_id)),
      },
    }
  }
}

/*
#[cfg(test)]
mod tests {
  use super::*;
  use event_source::aggregate::Aggregate;

  #[test]
  fn success() {
    let aggregate = ListAggregate::new();
    let command = ListAggregateCommand::AddListItem {
      id: Uuid::new_v4(),
      text: "Some text".to_string(),
    };
    match command.apply(&aggregate) {
      Ok(_) => assert!(true),
      Err(_) => assert!(false, "Adding an item should have succeeded"),
    }
  }

  #[test]
  fn duplicate_item_error() {
    let aggregate = ListAggregate::new();
    let uuid = Uuid::new_v4();
    let text = "First".to_string();
    let event = ListAggregateEvent::AddedListItem{id: uuid.clone(), text: text.clone()};
    let aggregate2 = aggregate.apply(&event);
    let command = ListAggregateCommand::AddListItem {
      id: uuid.clone(),
      text: "Some other text".to_string(),
    };
    match command.apply(&aggregate2) {
      Err(ListAggregateError::DuplicateUuidError) => assert!(true),
      _ => assert!(
        false,
        "The command should fail as an item with that uuid already exists"
      ),
    }
  }
}
*/
