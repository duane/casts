// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

// rss/atom
extern crate anyhow;
extern crate chrono;
extern crate feed_rs;
extern crate htmlescape;
extern crate url;

use std::collections::{HashMap, HashSet};

use anyhow::Result;
use chrono::{DateTime, Utc};
use feed_rs::{
    model::{Entry as RustEntry, Feed as RustFeed, Link as RustLink},
    parser,
};
use serde::{Deserialize, Serialize};
use url::Url;
// --------------
// util functions
// --------------

/// Convert an RSS/Atom link to a URL.
pub fn convert_link_to_url(link: &RustLink) -> Url {
    let escaped_href = &link.href;
    let unescaped_href = htmlescape::decode_html(escaped_href).unwrap();
    Url::parse(&unescaped_href).unwrap()
}

/// The feed itself.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Feed {
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
    pub entries: Vec<Entry>,
}

impl Feed {
    /// Create a feed from the feed-rs type and the url from which it originated.
    pub fn from_url_and_feed(url: Url, rust_feed: RustFeed) -> Feed {
        Feed {
            url,
            title: rust_feed.title.map(|text| text.content).unwrap_or_default(),
            description: rust_feed.description.map(|d| d.content),
            entries: rust_feed.entries.into_iter().map(Entry::from).collect(),
            updated_at: None,
            published_at: None,
        }
    }

    /// Produce an arbitrarily ordered list of changes between two feeds.
    pub fn diff(&self, other: &Feed) -> Vec<FeedMutation> {
        let mut mutations = vec![];
        if self.title != other.title {
            mutations.push(FeedMutation::ReplacedTitle(other.title.clone()));
        }

        if self.description != other.description {
            mutations.push(FeedMutation::ReplacedDescription(other.description.clone()))
        }

        // if self.logo != other.logo {
        //   mutations.push(FeedMutation::ReplacedLogo(other.logo.clone()));
        // }

        let existing_entry_by_guid: HashMap<&str, &Entry> = self
            .entries
            .iter()
            .map(|entry| (entry.guid.as_ref(), entry))
            .collect();
        let remote_entry_by_guid: HashMap<&str, &Entry> = other
            .entries
            .iter()
            .map(|entry| (entry.guid.as_ref(), entry))
            .collect();
        let all_observed_guids: HashSet<&str> = existing_entry_by_guid
            .keys()
            .chain(remote_entry_by_guid.keys())
            .map(|s| *s)
            .collect();
        for guid in all_observed_guids.iter() {
            let existing_entry = existing_entry_by_guid.get(*guid);
            let remote_entry = remote_entry_by_guid.get(*guid);
            match (existing_entry, remote_entry) {
                (Some(existing_entry), Some(remote_entry)) => {
                    let entry_mutations = existing_entry.diff(remote_entry);
                    if !entry_mutations.is_empty() {
                        mutations.push(FeedMutation::ModifiedEntry(
                            (*existing_entry).to_owned(),
                            entry_mutations,
                        ))
                    }
                }
                (Some(existing_entry), None) => {
                    mutations.push(FeedMutation::RemovedEntry((*existing_entry).to_owned()));
                }
                (None, Some(remote_entry)) => {
                    mutations.push(FeedMutation::AddedEntry((*remote_entry).to_owned()));
                }
                _ => unreachable!("iterating over a guid not in either set should not be possible"),
            };
        }
        mutations
    }
}

impl From<RustFeed> for Feed {
    fn from(feed: RustFeed) -> Feed {
        let href = Url::parse(
            &feed
                .links
                .first()
                .map(|l| &l.href)
                .expect("feed has no link"),
        )
        .expect("not a url");
        Feed {
            title: feed
                .title
                .map(|c| c.content)
                .expect("All feeds must have titles"),
            url: href,
            description: feed.description.map(|t| t.content),
            updated_at: feed.updated,
            published_at: feed.published,
            entries: feed.entries.into_iter().map(Entry::from).collect(),
        }
    }
}

/// An entry associated with the feed.
#[derive(Clone, Debug, Serialize, Deserialize)]
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

impl Entry {
    /// Enumerate the differences between the two entries.
    pub fn diff(&self, other: &Entry) -> Vec<EntryMutation> {
        assert_eq!(self.guid, other.guid);

        let mut mutations = vec![];

        if self.title != other.title {
            mutations.push(EntryMutation::ReplacedTitle(other.title.to_owned()));
        }

        if self.url != other.url {
            mutations.push(EntryMutation::ReplacedUrl(Some(other.url.to_owned())));
        }

        if self.summary != other.summary {
            mutations.push(EntryMutation::ReplacedSummary(other.summary.to_owned()));
        }

        // if self.media != other.media {
        //   mutations.push(EntryMutation::ReplacedContent(other.media.to_owned()));
        // }

        mutations
    }
}

impl From<RustEntry> for Entry {
    fn from(entry: RustEntry) -> Entry {
        let content_url = convert_link_to_url(entry.links.first().unwrap());
        Entry {
            guid: entry.id.clone(),
            url: content_url,
            title: entry.title.map(|t| t.content),
            summary: entry.summary.map(|t| t.content),
            updated_at: entry.updated,
            published_at: entry.published,
        }
    }
}

pub async fn fetch_body(url: &Url) -> Result<Vec<u8>> {
    let response = reqwest::get(url.as_str()).await?;
    let body = response.text().await?;
    Ok(body.as_bytes().into())
}

/// Parse the document from a vector of bytes.
pub fn parse_doc(url: &Url, doc: &[u8]) -> Result<Feed> {
    Ok(Feed::from_url_and_feed(
        url.to_owned(),
        parser::parse(doc)?.into(),
    ))
}

pub async fn fetch_feed(url: &Url) -> Result<Feed> {
    let body = fetch_body(url).await?;
    Ok(parse_doc(url, body.as_slice())?)
}

// Delta bits

#[derive(Debug, Serialize, Deserialize)]
/// Feed delta: a struct representing changes to a feed.
pub enum FeedMutation {
    /// Indicates a differing feed title.
    ReplacedTitle(String),

    /// Indicates a differing description.
    ReplacedDescription(Option<String>),

    /// Indicates a differing logo.
    // ReplacedLogo(Option<Image>),

    /// Indicates a new entry.
    AddedEntry(Entry),

    /// Indicates a removed entry.
    RemovedEntry(Entry),

    /// Indicates a modified entry.
    ModifiedEntry(Entry, Vec<EntryMutation>),
}

#[derive(Debug, Serialize, Deserialize)]
/// A single change to an entry.
pub enum EntryMutation {
    /// Indicates a differing entry title.
    ReplacedTitle(Option<String>),

    /// Indicates a differing entry link.
    ReplacedUrl(Option<Url>),

    /// Indicates a differing summary.
    ReplacedSummary(Option<String>),
    ///// Indicates differing content.
    // ReplacedContent(Option<Media>),
}
