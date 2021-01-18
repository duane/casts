use crate::aggregate::Feed;

use super::aggregate::FeedAggregate;
use super::events::FeedAggregateEvent;
use event_source::command::Command;
use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};
use uuid::Uuid;
use async_trait::async_trait;
use log::info;

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
    url: String,
  }
}

/// The error associated with running commands against a feed aggregate.
#[derive(Debug, Clone)]
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

  fn apply(
    &self,
    aggregate: &FeedAggregate,
  ) -> Result<Vec<FeedAggregateEvent>, FeedAggregateError> {
    match self {
      &FeedAggregateCommand::AddFeed { ref id, ref feed_id, ref url} => {
        match aggregate.feeds.get(feed_id) {
          Some(feed_id) => Err(FeedAggregateError::DuplicateFeedError),
          _ => Ok(vec![]),
        }
      }
    }
  }

  async fn async_apply(
    &self,
    aggregate: &FeedAggregate,
  ) -> Result<Vec<FeedAggregateEvent>, FeedAggregateError> {
    match self {
      &FeedAggregateCommand::AddFeed { ref id, ref feed_id, ref url} => {
        if let Some(_) = aggregate.feeds.get(feed_id) {
          return Err(FeedAggregateError::DuplicateFeedError);
        }
        println!("url: {}", url);
        let download_result = reqwest::get(url).await.unwrap().text().await.unwrap();
        let parsed_feed = crate::parse_feed_rs(&download_result).unwrap();
        if parsed_feed.title.is_none() {
          return Err(FeedAggregateError::BadFeedError);
        }
        let mut events = vec![
          FeedAggregateEvent::Added{
            id: id.to_owned(),
            feed_id: feed_id.to_owned(),
            url: url.to_owned(),
            title: parsed_feed.title.unwrap().content,
            description: parsed_feed.description.map(|t|t.content),
            updated_at: parsed_feed.updated,
            published_at: parsed_feed.published
          }
        ];
        for entry in parsed_feed.entries.iter() {
          let url = match entry.content {
            Some(ref content) => {
              match content.src {
                Some(ref href) => {
                  href.href.clone()
                },
                None => continue,
              }
            },
            None => continue, // any entry without a url doesn't make sense in a podcast context.
          };
          let title = entry.title.clone().map(|t|t.content);
          let summary = entry.summary.clone().map(|t|t.content);
          events.push(FeedAggregateEvent::EntryObserved {
            id: id.to_owned(),
            feed_id: feed_id.to_owned(),
            guid: entry.id.clone(),
            title,
            url,
            summary,
            updated_at: entry.updated,
            published_at: entry.published,
          });
        }

        Ok(events)
      }
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