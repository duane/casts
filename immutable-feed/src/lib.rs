// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

use anyhow::Result;
use chrono::{DateTime, Utc};
use feed_rs::{
    model::{
        Content as RustContent, Entry as RustEntry, Feed as RustFeed, Image as RustImage,
        Link as RustLink, MediaContent as RustMediaContent, MediaObject as RustMediaObject,
    },
    parser,
};
use std::collections::{HashMap, HashSet};

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
    /// The well-formed url where the feed may be found.
    pub url: Option<Url>,

    /// If the url fails to parse, the raw url is stored here.
    pub raw_url: String,

    /// Metadata associated with the feed.
    pub metadata: FeedMetadata,

    /// The entries associated with the feed in serialized order
    pub entries: Vec<Entry>,
}

/// Data associated with but not identifying the feed.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FeedMetadata {
    // pub title_txt: Option<String>, // converts html -> txt if necessary
    // pub title_html: Option<String>, // access the title as html
    // pub title_content: Option<Content>, // access the title as Content
    /// The title of the feed. This is typically text.
    pub title: String,

    /// Any image associated with the feed
    pub logo: Option<Image>,

    /// The author(s) of the feed.
    pub authors: Vec<String>,

    /// The description of the feed.
    pub description: Option<String>,
    // pub description_txt: Option<String>,
    // pub description_html: Option<String>,
    // pub description_content: Option<Content>,
    // When the feed was last updated.
    // pub updated_at: Option<DateTime<Utc>>,

    // When the feed was originally published.
    // pub published_at: Option<DateTime<Utc>>,
}

impl FeedMetadata {
    /// Produce a delta between two feeds.
    pub fn diff(&self, other: &FeedMetadata) -> Vec<FeedMetadataMutation> {
        let mut mutations: Vec<FeedMetadataMutation> = vec![];
        if self.title != other.title {
            mutations.push(FeedMetadataMutation::ReplacedTitle(self.title.clone()))
        }
        if self.logo != other.logo {
            mutations.push(FeedMetadataMutation::ReplacedLogo(self.logo.clone()))
        }
        if self.authors != other.authors {
            mutations.push(FeedMetadataMutation::ReplacedAuthors(self.authors.clone()))
        }
        if self.description != other.description {
            mutations.push(FeedMetadataMutation::ReplacedDescription(
                self.description.clone(),
            ));
        }
        mutations
    }

    /// Apply a delta to a feed to produce a new feed.
    pub fn apply_diff(&self, mutations: Vec<FeedMetadataMutation>) -> FeedMetadata {
        let mut applied = self.to_owned();
        for mutation in mutations {
            match mutation {
                FeedMetadataMutation::ReplacedAuthors(authors) => applied.authors = authors,
                FeedMetadataMutation::ReplacedDescription(description) => {
                    applied.description = description
                }
                FeedMetadataMutation::ReplacedLogo(logo) => applied.logo = logo,
                FeedMetadataMutation::ReplacedTitle(title) => applied.title = title,
            }
        }
        applied
    }
}

impl From<RustFeed> for FeedMetadata {
    fn from(feed: RustFeed) -> FeedMetadata {
        FeedMetadata {
            title: feed
                .title
                .map(|c| c.content)
                .expect("All feeds must have titles"),
            authors: feed.authors.into_iter().map(|p| p.name).collect(),
            description: feed.description.map(|t| t.content),
            // updated_at: feed.updated,
            // published_at: feed.published,
            logo: feed.logo.map(Image::from),
        }
    }
}

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct Image {
    pub url: Option<Url>,
    pub raw_url: String,
    pub link: Option<Link>,
    pub description: Option<String>,
    pub title: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

impl From<RustImage> for Image {
    fn from(rust_image: RustImage) -> Self {
        Image {
            url: Url::parse(&rust_image.uri).ok(),
            raw_url: rust_image.uri,
            title: rust_image.title,
            link: rust_image.link.map(Link::from),
            width: rust_image.width,
            height: rust_image.height,
            description: rust_image.description,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MediaContent {
    pub url: Option<Url>,
    pub content_type: Option<String>,
    pub height: Option<u32>,
    pub width: Option<u32>,
    // pub duration: Option<(u32, u32)>,
    pub size: Option<u64>,
    // pub rating: Option<MediaRating>,
}

impl From<RustMediaContent> for MediaContent {
    fn from(value: RustMediaContent) -> Self {
        MediaContent {
            url: value.url,
            content_type: value.content_type.map(|m| m.to_string()),
            height: value.height,
            width: value.width,
            // duration: value.duration,
            size: value.size,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MediaObject {
    /// Title of the object (from the media:title element)
    pub title: Option<String>,
    /// Collection of the media content elements
    pub content: Vec<MediaContent>,
    /// Duration of the object
    pub duration: Option<(u32, u32)>,
    // Representative images for the object (from media:thumbnail elements)
    // pub thumbnails: Vec<MediaThumbnail>,
    // A text transcript, closed captioning or lyrics of the media content.
    // pub texts: Vec<MediaText>,
    /// Short description of the media object (from the media:description element)
    pub description: Option<String>,
    // Community info (from the media:community element)
    // pub community: Option<MediaCommunity>,
    // Credits
    // pub credits: Vec<MediaCredit>,
}

impl From<RustMediaObject> for MediaObject {
    fn from(value: RustMediaObject) -> Self {
        MediaObject {
            title: value.title.and_then(|t| t.src),
            content: value.content.into_iter().map(MediaContent::from).collect(),
            duration: value
                .duration
                .map(|d| (d.as_secs() as u32, d.subsec_nanos())),
            description: value.description.and_then(|t| t.src),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Content {
    pub body: Option<String>,
    pub content_type: String,
    pub length: Option<u64>,
    pub src: Option<Link>,
}

impl From<RustContent> for Content {
    fn from(rust_content: RustContent) -> Content {
        Content {
            body: rust_content.body,
            content_type: rust_content.content_type.essence_str().to_owned(),
            length: rust_content.length,
            src: rust_content.src.map(Link::from),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Link {
    /// The url where the link points.
    pub href: String,

    /// Sometimes useful.
    pub media_type: Option<String>,

    /// language of referred to content.
    pub href_lang: Option<String>,

    /// Title of link.
    pub title: Option<String>,

    /// Unused.
    pub rel: Option<String>,

    /// Unused - but length of content in seconds is a likely guess.
    pub length: Option<u64>,
}

impl From<RustLink> for Link {
    fn from(rust_link: RustLink) -> Link {
        Link {
            href: rust_link.href,
            rel: rust_link.rel,
            media_type: rust_link.media_type,
            href_lang: rust_link.href_lang,
            title: rust_link.title,
            length: rust_link.length,
        }
    }
}

impl Feed {
    /// Create a feed from the feed-rs type and the url from which it originated.
    pub fn from_url_and_feed(url: Url, rust_feed: RustFeed) -> Feed {
        let raw_url = url.to_string();
        let metadata = FeedMetadata::from(rust_feed.clone());
        let entries = rust_feed.entries.into_iter().map(Entry::from).collect();
        Feed {
            url: Some(url),
            raw_url,
            metadata,
            entries,
        }
    }

    /// Produce an arbitrarily ordered list of changes between two feeds.
    pub fn diff(&self, other: &Feed) -> Vec<FeedMutation> {
        let mut mutations: Vec<FeedMutation> = vec![];

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
            .copied()
            .collect();
        for guid in all_observed_guids.iter() {
            let existing_entry = existing_entry_by_guid.get(*guid);
            let remote_entry = remote_entry_by_guid.get(*guid);
            let (feed_mutations, entry_mutations): (Vec<FeedMutation>, Vec<EntryMutation>) =
                match (existing_entry, remote_entry) {
                    (Some(existing_entry), Some(remote_entry)) => {
                        (vec![], existing_entry.diff(remote_entry))
                    }
                    (Some(entry), None) => (
                        vec![FeedMutation::EntryRemoved(entry.guid.to_owned())],
                        vec![],
                    ),
                    (None, Some(remote_entry)) => (
                        vec![FeedMutation::EntryAdded(remote_entry.to_owned().clone())],
                        vec![],
                    ),
                    _ => unreachable!(
                        "iterating over a guid not in either set should not be possible"
                    ),
                };
            mutations.extend(feed_mutations);
            if !entry_mutations.is_empty() {
                mutations.push(FeedMutation::EntryChanged(
                    guid.to_string(),
                    entry_mutations,
                ));
            }
        }
        mutations
    }

    pub fn apply_diff(&self, mutations: Vec<FeedMutation>) -> Feed {
        let mut applied = self.to_owned();
        let mut entry_mutations_by_guid = HashMap::new();
        let mut new_entries: HashMap<String, Entry> = HashMap::new();
        let mut removed_entries: HashSet<String> = HashSet::new();
        for mutation in mutations {
            match mutation {
                FeedMutation::MetadataChange(metadata_mutation) => {
                    applied.metadata = applied.metadata.apply_diff(vec![metadata_mutation]);
                }
                FeedMutation::EntryChanged(guid, entry_mutations) => {
                    entry_mutations_by_guid.insert(guid, entry_mutations);
                }
                FeedMutation::EntryAdded(entry) => {
                    new_entries.insert(entry.guid.clone(), entry);
                }
                FeedMutation::EntryRemoved(guid) => {
                    removed_entries.insert(guid);
                }
            }
        }
        let entries: Vec<Entry> = applied
            .entries
            .into_iter()
            .flat_map(|entry| match entry_mutations_by_guid.remove(&entry.guid) {
                _ if removed_entries.contains(&entry.guid)
                    || new_entries.contains_key(&entry.guid) =>
                {
                    None
                }
                Some(mutations) => entry.apply(mutations),
                None => Some(entry),
            })
            .chain(new_entries.values().cloned())
            .collect();
        applied.entries = entries;
        applied
    }
}

impl From<RustFeed> for Feed {
    fn from(feed: RustFeed) -> Feed {
        let raw_url = feed
            .links
            .first()
            .map(|l| l.href.clone())
            .unwrap_or_default();
        let url = Url::parse(&raw_url).ok();
        Feed {
            url,
            raw_url,
            metadata: FeedMetadata::from(feed.clone()),
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

    /// Content (typically html) associated with media.
    pub content: Option<Content>,

    /// Other links associated with content
    pub links: Vec<Link>,

    /// Other media
    pub media: Vec<MediaObject>,
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

        if self.content != other.content {
            mutations.push(EntryMutation::ReplacedContent(other.content.to_owned()));
        }

        if self.media != other.media {
            mutations.push(EntryMutation::ReplacedMedia(other.media.to_owned()));
        }

        if self.published_at != other.published_at {
            mutations.push(EntryMutation::ReplacedPublished(
                other.published_at.to_owned(),
            ));
        }

        mutations
    }

    fn apply(&self, mutations: Vec<EntryMutation>) -> Option<Entry> {
        let mut applied = self.clone();
        for mutation in mutations {
            match mutation {
                EntryMutation::ReplacedContent(new_content) => applied.content = new_content,
                EntryMutation::ReplacedMedia(new_media) => applied.media = new_media,
                EntryMutation::ReplacedLinks(new_links) => applied.links = new_links,
                EntryMutation::ReplacedSummary(new_summary) => applied.summary = new_summary,
                EntryMutation::ReplacedTitle(new_title) => applied.title = new_title,
                EntryMutation::ReplacedPublished(new_published) => {
                    applied.published_at = new_published
                }
                EntryMutation::ReplacedUrl(new_url) => {
                    applied.url = new_url.expect("failed to parse URL")
                }
            }
        }
        Some(applied)
    }
}

impl From<RustEntry> for Entry {
    fn from(entry: RustEntry) -> Entry {
        let content_url = convert_link_to_url(
            entry
                .links
                .first()
                .unwrap_or_else(|| panic!("Entry {:?} has no links", entry)),
        );
        Entry {
            guid: entry.id.clone(),
            url: content_url,
            title: entry.title.map(|t| t.content),
            summary: entry.summary.map(|t| t.content),
            updated_at: entry.updated,
            published_at: entry.published,
            content: entry.content.map(Content::from),
            links: entry.links.into_iter().map(Link::from).collect(),
            media: entry.media.into_iter().map(MediaObject::from).collect(),
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
    Ok(Feed::from_url_and_feed(url.to_owned(), parser::parse(doc)?))
}

pub async fn fetch_feed(url: &Url) -> Result<Feed> {
    let body = fetch_body(url).await?;
    parse_doc(url, body.as_slice())
}

// Delta bits

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Feed delta: a struct representing changes to a feed.
pub enum FeedMutation {
    /// Stores changes to metadata.
    MetadataChange(FeedMetadataMutation),

    /// New entry added to feed.
    EntryAdded(Entry),

    /// Entry disappeared from feed.
    EntryRemoved(String),

    /// Indicates a modified entry.
    /// The first parameter is the guid of the entry that was changed.
    /// the second parameter is the change in question.
    EntryChanged(String, Vec<EntryMutation>),
}

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FeedMetadataMutation {
    /// Stores the new title.
    ReplacedTitle(String),

    /// Stores a new description
    ReplacedDescription(Option<String>),

    /// Stores a new logo.
    ReplacedLogo(Option<Image>),

    /// Stores the new set of authors
    ReplacedAuthors(Vec<String>),
}

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, Serialize, Deserialize)]
/// A single change to an entry.
pub enum EntryMutation {
    /// Stores the new title.
    ReplacedTitle(Option<String>),

    /// Stores the new URL.
    ReplacedUrl(Option<Url>),

    /// Stores the new summary.
    ReplacedSummary(Option<String>),

    /// Stores the new content.
    ReplacedContent(Option<Content>),

    /// Stores the new media vector
    ReplacedMedia(Vec<MediaObject>),

    /// Stores the new published date
    ReplacedPublished(Option<DateTime<Utc>>),

    /// Stores the new links.
    ReplacedLinks(Vec<Link>),
}
