use anyhow::Result;
use std::collections::{HashMap, HashSet};
use url::Url;
use std::time::Duration;

use feed_rs::model::{
  Content as RustContent, Entry as RustEntry, Feed as RustFeed, Image as RustImage,
  Link as RustLink, MediaContent,
};
use feed_rs::parser;

use std::convert::From;

/// The feed model.

#[derive(Clone, Debug)]
/// A feed according to feeds-rs.
pub struct Feed {
  /// The definitive url of the feed
  pub url: Url,

  /// A title for the feed (optional).
  pub title: String,

  /// A description for the feed (optional).
  pub description: Option<String>,

  /// An image to display with the feed (optional).
  pub logo: Option<Image>,

  /// Entries for the feed.
  pub entries: Vec<Entry>,
}

// Methods

impl Feed {
  /// Diff two feeds.
  pub fn diff(&self, other: &Feed) -> Vec<FeedMutation> {
    let mut mutations = vec![];
    if self.title != other.title {
      mutations.push(FeedMutation::ReplacedTitle(other.title.clone()));
    }

    if self.description != other.description {
      mutations.push(FeedMutation::ReplacedDescription(other.description.clone()))
    }

    if self.logo != other.logo {
      mutations.push(FeedMutation::ReplacedLogo(other.logo.clone()));
    }

    let existing_entry_by_guid: HashMap<&str, &Entry> = self
      .entries
      .iter()
      .map(|entry| (entry.id.as_ref(), entry))
      .collect();
    let remote_entry_by_guid: HashMap<&str, &Entry> = other
      .entries
      .iter()
      .map(|entry| (entry.id.as_ref(), entry))
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

// Conversions
impl From<immutable_feed::Feed> for Feed {
  fn from(feed: immutable_feed::Feed) -> Feed {
    Feed { url: feed.url, title: feed.title, description: feed.description, logo: None, entries: vec![] }
  }
}

#[derive(Clone, PartialEq, Eq, Debug)]
/// A single entry in a feed.
pub struct Entry {
  /// a GUID assigned by feeds-rs if not specified by the feed.
  pub id: String,
  /// The title of the entry.
  pub title: Option<String>,
  /// The link of the entry.
  pub link: Option<Link>,
  /// The summary of the entry (as text/html).
  pub summary: Option<String>,

  /// The content of the entry.
  pub media: Option<Media>,
}

impl Entry {
  /// Enumerate the differences between the two entries.
  pub fn diff(&self, other: &Entry) -> Vec<EntryMutation> {
    assert_eq!(self.id, other.id);

    let mut mutations = vec![];

    if self.title != other.title {
      mutations.push(EntryMutation::ReplacedTitle(other.title.to_owned()));
    }

    if self.link != other.link {
      mutations.push(EntryMutation::ReplacedLink(other.link.to_owned()));
    }

    if self.summary != other.summary {
      mutations.push(EntryMutation::ReplacedSummary(other.summary.to_owned()));
    }

    if self.media != other.media {
      mutations.push(EntryMutation::ReplacedContent(other.media.to_owned()));
    }

    mutations
  }
}

#[derive(Clone, PartialEq, Eq, Debug)]
/// Representing an Image as seen from a feed.
pub struct Image {
  /// The URI where the image may be found.
  pub uri: String,

  /// The Title of the image.
  pub title: String,

  /// The Link representing the image.
  pub link: Option<Link>,

  /// The width of the image in pixels.
  pub width: Option<u32>,

  /// The height of the image in pixels.
  pub height: Option<u32>,

  /// A description of the image.
  pub description: Option<String>,
}

/// An audio blob
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Media {
  /// the url of the blob
  pub url: Option<Url>,

  /// the content type of the blob
  pub content_type: Option<String>,
  
  /// the duration of the blob
  pub duration: Option<Duration>,

  /// the size of the blob
  pub size: Option<u64>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
/// A link to some media on the internet.
pub struct Link {
  /// The URL of the link.
  pub href: String,

  /// The rel of the link.
  pub rel: Option<String>,

  /// The media type of the the linked-to content.
  pub media_type: Option<String>,

  /// The language linked to by href.
  pub href_lang: Option<String>,

  /// The title of the link.
  pub title: Option<String>,

  /// The length of the content in bytes.
  pub length: Option<u64>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
/// A description of how to interpret the content of the entry.
pub struct Content {
  /// The body of the content.
  pub body: Option<String>,

  /// The content type of the content.
  pub content_type: String,

  /// The length of the content in bytes.
  pub length: Option<u64>,

  /// A link where the content may be found.
  pub src: Option<Link>,
}

#[derive(Debug)]
/// Feed delta: a struct representing changes to a feed.
pub enum FeedMutation {
  /// Indicates a differing feed title.
  ReplacedTitle(String),

  /// Indicates a differing description.
  ReplacedDescription(Option<String>),

  /// Indicates a differing logo.
  ReplacedLogo(Option<Image>),

  /// Indicates a new entry.
  AddedEntry(Entry),

  /// Indicates a removed entry.
  RemovedEntry(Entry),

  /// Indicates a modified entry.
  ModifiedEntry(Entry, Vec<EntryMutation>),
}

#[derive(Debug)]
/// A single change to an entry.
pub enum EntryMutation {
  /// Indicates a differing entry title.
  ReplacedTitle(Option<String>),

  /// Indicates a differing entry link.
  ReplacedLink(Option<Link>),

  /// Indicates a differing summary.
  ReplacedSummary(Option<String>),

  /// Indicates differing content.
  ReplacedContent(Option<Media>),
}

// conversion

impl Feed {
  fn from_url_and_feed(url: Url, feed: RustFeed) -> Feed {
    Feed {
      url: url,
      title: feed.title.map(|text| text.content).unwrap_or_default(),
      description: feed.description.map(|d| d.content),
      logo: feed.logo.map(Image::from),
      entries: feed.entries.into_iter().map(Entry::from).collect(),
    }
  }
}

impl From<RustEntry> for Entry {
  fn from(entry: RustEntry) -> Entry {
    Entry{
      id: entry.id,
      title: entry.title.map(|text|text.content),
      link: entry.links.first().map(|link| Link::from(link.to_owned())),
      summary: entry.summary.map(|text| text.content),
      media: entry.media.first().map(|m|m.content.first().unwrap().to_owned()).map(Media::from)
    }
  }
}

impl From<RustImage> for Image {
  fn from(image: RustImage) -> Image {
    Image {
      uri: image.uri,
      title: image.title.unwrap_or_default(),
      link: image.link.map(Link::from),
      width: image.width,
      height: image.height,
      description: image.description,
    }
  }
}

impl From<RustLink> for Link {
  fn from(link: RustLink) -> Link {
    Link {
      href: link.href,
      rel: link.rel,
      media_type: link.media_type,
      href_lang: link.href_lang,
      title: link.title,
      length: link.length,
    }
  }
}

impl From<RustContent> for Content {
  fn from(content: RustContent) -> Content {
    Content {
      body: content.body,
      content_type: content.content_type.to_string(),
      length: content.length,
      src: content.src.map(Link::from)
    }
  }
}

impl From<MediaContent> for Media {
  fn from(content: MediaContent) -> Media {
    Media {
      url: content.url,
      content_type: content.content_type.map(|ct|ct.to_string()),
      duration: content.duration,
      size: content.size,
    }
  }
}