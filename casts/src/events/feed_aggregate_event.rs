use super::Event;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

/// Aggregate events!
#[derive(Serialize, Deserialize, Debug)]
pub enum FeedAggregateEvent {
    /// A feed has been added to the database.
    Added {
        /// The aggregate ID.
        stream_id: Uuid,

        /// The feed ID.
        feed_id: Uuid,

        /// The title of the feed.
        title: String,

        /// The url where the feed may be found.
        url: Url,

        /// A description of the feed.
        description: Option<String>,

        /// When the feed was updated.
        updated_at: Option<DateTime<Utc>>,

        /// When the feed was published.
        published_at: Option<DateTime<Utc>>,
    },

    /// A new entry was observed.
    EntryObserved {
        /// The aggregate id to which this event applies.
        stream_id: Uuid,

        /// The feed in which the entry was observed.
        feed_id: Uuid,

        /// The unique identifier of the entry.
        guid: String,

        /// The url where the entry content may be found.
        url: Url,

        /// The title of the entry.
        title: Option<String>,

        /// A summary of the entry.
        summary: Option<String>,

        /// When the feed was updated (according to the feed).
        updated_at: Option<DateTime<Utc>>,

        /// When the feed was published (according to the feed).
        published_at: Option<DateTime<Utc>>,
    },

    /// The entry updated its title.
    EntryTitleUpdated {
        /// aggregate id to which this event applies.
        stream_id: Uuid,

        /// The feed in which the entry was updated.
        feed_id: Uuid,

        /// The guid of the entry whose url was updated.
        guid: String,

        /// The new title of the entry.
        title: Option<String>,
    },

    /// An entry's url has been updated.
    EntryUrlUpdated {
        /// aggregate id to which this event applies.
        stream_id: Uuid,

        /// The feed in which the entry was updated.
        feed_id: Uuid,

        /// The guid of the entry whose url was updated.
        guid: String,

        /// The new url of the entry.
        url: Url,
    },

    /// An entry's summary has been updated.
    EntrySummaryUpdated {
        /// The id of the aggregate to which this entry applies.
        stream_id: Uuid,

        /// The feed id of the entry whose summary has been updated.
        feed_id: Uuid,

        /// The guid of the entry whose summary has been updated
        guid: String,

        /// The new summary of the entry.
        summary: Option<String>,
    },

    /// A previously observed entry has disappeared.
    EntryDisappeared {
        /// The id of the aggregate to which this event applies.
        stream_id: Uuid,

        /// The id of the feed from which the entry has disappeared.
        feed_id: Uuid,

        /// The guid of the entry that has disappeared.
        guid: String,
    },
}

impl Event for FeedAggregateEvent {}
