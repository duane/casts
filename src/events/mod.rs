use event_source::events::Event;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Aggregate events!
#[derive(Serialize, Deserialize, Debug)]
pub enum FeedAggregateEvent {
  /// A feed has been added to the database.
  Added {
    /// The aggregate ID.
    id: Uuid,

    /// The feed ID.
    feed_id: String,
    
    /// The title of the feed.
    title: String,
    
    /// The url where the feed may be found.
    url: String,
    
    /// A summary of the feed.
    summary: Option<String>,
    
    /// When the feed was updated.
    updated_at: Option<DateTime<Utc>>,
    
    /// When the feed was published.
    published_at: Option<DateTime<Utc>>
  },
  
  /// A new entry was observed.
  EntryObserved {
    /// The aggregate id to which this event applies.
    id: Uuid,

    /// The feed in which the entry was observed.
    feed_id: String,

    /// The unique identifier of the entry.
    guid: String,

    /// The title of the entry.
    title: String,
    
    /// The url where the entry content may be found.
    url: String,
    
    /// A description of the entry.
    description: Option<String>,

    /// When the feed was updated (according to the feed).
    updated_at: Option<DateTime<Utc>>,
    
    /// When the feed was published (according to the feed).
    published_at: Option<DateTime<Utc>>
  },

  /// The entry updated its title.
  EntryTitleUpdated {
    /// aggregate id to which this event applies.
    id: Uuid,
  
    /// The feed in which the entry was updated.
    feed_id: String,
    
    /// The guid of the entry whose url was updated.
    guid: String,
    
    /// The new title of the entry.
    title: String
  },
  
  /// An entry's url has been updated.
  EntryUrlUpdated {
    /// aggregate id to which this event applies.
    id: Uuid,
  
    /// The feed in which the entry was updated.
    feed_id: String,
    
    /// The guid of the entry whose url was updated.
    guid: String,
    
    /// The new url of the entry.
    url: String,
  },

  /// An entry's description has been updated.
  EntryDescriptionUpdated {
    /// The id of the aggregate to which this entry applies.
    id: Uuid,

    /// The feed id of the entry whose description has been updated.
    feed_id: String,

    /// The guid of the entry whose description has been updated
    guid: String,

    /// The new description of the entry.
    description: Option<String>,
  },
  
  /// A previously observed entry has disappeared.
  EntryDisappeared {
    /// The id of the aggregate to which this event applies.
    id: Uuid,
    
    /// The id of the feed from which the entry has disappeared.
    feed_id: String,

    /// The guid of the entry that has disappeared.
    guid: String,
  },
}

impl Event for FeedAggregateEvent {}
