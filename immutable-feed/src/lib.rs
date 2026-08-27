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

/// Convert an RSS/Atom link to a URL, or `None` if the href is not one.
///
/// Feeds are third-party input: an unparseable href is a fact about the
/// world, not an invariant violation, so this reports rather than panics.
pub fn convert_link_to_url(link: &RustLink) -> Option<Url> {
    let unescaped_href = htmlescape::decode_html(&link.href).ok()?;
    Url::parse(&unescaped_href).ok()
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
    /// Produce the delta that moves `self` toward `other`.
    ///
    /// Same convention as `Entry::diff` and `Feed::diff`: `self` is the
    /// existing state, `other` is the observation, and every mutation carries
    /// the observed value. (It used to carry `self`'s values -- the opposite
    /// direction from the other two diff impls -- and nothing called it.)
    pub fn diff(&self, other: &FeedMetadata) -> Vec<FeedMetadataMutation> {
        let mut mutations: Vec<FeedMetadataMutation> = vec![];
        if self.title != other.title {
            mutations.push(FeedMetadataMutation::ReplacedTitle(other.title.clone()))
        }
        if self.logo != other.logo {
            mutations.push(FeedMetadataMutation::ReplacedLogo(other.logo.clone()))
        }
        if self.authors != other.authors {
            mutations.push(FeedMetadataMutation::ReplacedAuthors(other.authors.clone()))
        }
        if self.description != other.description {
            mutations.push(FeedMetadataMutation::ReplacedDescription(
                other.description.clone(),
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
        FetchedFeed::from_url_and_feed(url, rust_feed).feed
    }

    /// Produce an arbitrarily ordered list of changes that move `self`
    /// (the existing state) toward `other` (the observation).
    pub fn diff(&self, other: &Feed) -> Vec<FeedMutation> {
        let mut mutations: Vec<FeedMutation> = vec![];

        // Metadata changes (title, logo, authors, description) sync through
        // the same delta stream as entries; they used to be silently dropped.
        mutations.extend(
            self.metadata
                .diff(&other.metadata)
                .into_iter()
                .map(FeedMutation::MetadataChange),
        );

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

impl FetchedFeed {
    /// Parse a feed known to live at `url`, keeping its show notes.
    pub fn from_url_and_feed(url: Url, rust_feed: RustFeed) -> FetchedFeed {
        let raw_url = url.to_string();
        let metadata = FeedMetadata::from(rust_feed.clone());
        let (entries, contents): (Vec<Entry>, Vec<EntryContent>) = rust_feed
            .entries
            .into_iter()
            .filter_map(Entry::from_rust_entry)
            .unzip();
        FetchedFeed {
            feed: Feed {
                url: Some(url),
                raw_url,
                metadata,
                entries,
            },
            contents,
        }
    }
}

impl From<RustFeed> for FetchedFeed {
    fn from(feed: RustFeed) -> FetchedFeed {
        let raw_url = feed
            .links
            .first()
            .map(|l| l.href.clone())
            .unwrap_or_default();
        let url = Url::parse(&raw_url).ok();
        let metadata = FeedMetadata::from(feed.clone());
        let (entries, contents): (Vec<Entry>, Vec<EntryContent>) = feed
            .entries
            .into_iter()
            .filter_map(Entry::from_rust_entry)
            .unzip();
        FetchedFeed {
            feed: Feed {
                url,
                raw_url,
                metadata,
                entries,
            },
            contents,
        }
    }
}

impl From<RustFeed> for Feed {
    /// The feed alone, discarding show notes.
    ///
    /// Only for callers that genuinely do not store content -- anything on a
    /// write path wants [`FetchedFeed`], because notes dropped here cannot
    /// be recovered from the entries that reference them by digest.
    fn from(feed: RustFeed) -> Feed {
        FetchedFeed::from(feed).feed
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

    /// When the entry was last updated.
    pub updated_at: Option<DateTime<Utc>>,

    /// When the entry was first published.
    pub published_at: Option<DateTime<Utc>>,

    /// Other links associated with content
    pub links: Vec<Link>,

    /// Other media
    pub media: Vec<MediaObject>,

    /// Identifies this entry's show-notes ([`EntryContent`]), which are held
    /// out of band rather than in the entry itself.
    ///
    /// `summary` and `content` used to live here. Measured on one commit,
    /// they were 86% of every entry (1,920 + 1,841 of 4,398 bytes) and no
    /// command ever read them -- deduplication is by guid, ordering by
    /// publication date, archival by media URL. Carrying them made the unit
    /// that every commit, aggregate, and snapshot is built from roughly
    /// seven times larger than the decisions needed.
    ///
    /// `None` means "no show notes"; it does not mean "not loaded", because
    /// an entry never holds them either way. A digest is a content address,
    /// so identical notes across entries store once and a re-upload is a
    /// no-op.
    ///
    /// Defaults when absent so entries written before the split still
    /// deserialize.
    #[serde(default)]
    pub content_digest: Option<String>,
}

/// The show notes for one entry: the part deliberately kept out of the event
/// stream. Stored by [`EntryContent::digest`] and fetched only when
/// something actually renders an episode.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EntryContent {
    /// The entry these notes belong to.
    pub guid: String,

    /// A summary of the entry content.
    pub summary: Option<String>,

    /// Content (typically html) associated with media.
    pub content: Option<Content>,
}

impl EntryContent {
    /// Whether there is anything worth storing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.summary.is_none() && self.content.is_none()
    }

    /// The content address of these notes: hex SHA-256 over the payload.
    ///
    /// Keyed on the notes alone, deliberately not on the guid or feed: two
    /// entries with identical notes share one stored object, and re-storing
    /// unchanged notes writes the same bytes to the same key.
    ///
    /// `None` when there is nothing to store, so an entry without notes
    /// carries no digest rather than a digest of emptiness.
    #[must_use]
    pub fn digest(&self) -> Option<String> {
        if self.is_empty() {
            return None;
        }
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        // Serialized through serde so the hash covers exactly the bytes that
        // get stored, rather than a separate hand-rolled encoding that could
        // drift from them.
        let payload = serde_json::json!({
            "summary": self.summary,
            "content": self.content,
        });
        hasher.update(serde_json::to_vec(&payload).unwrap_or_default());
        Some(format!("{:x}", hasher.finalize()))
    }
}

/// A parsed feed together with the show notes lifted out of its entries.
///
/// Parsing has to surface both halves: the caller stores the notes and
/// commits the entries, and neither is recoverable from the other.
#[derive(Clone, Debug)]
pub struct FetchedFeed {
    /// The feed, whose entries carry digests rather than notes.
    pub feed: Feed,

    /// The notes for those entries, in the same order.
    pub contents: Vec<EntryContent>,
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

        // Comparing 32 bytes of digest where this used to compare ~3.8 KB of
        // HTML. A publisher rewriting show notes still produces a mutation,
        // because the digest changes with them.
        if self.content_digest != other.content_digest {
            mutations.push(EntryMutation::ReplacedContentDigest(
                other.content_digest.to_owned(),
            ));
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
                EntryMutation::ReplacedContentDigest(new_digest) => {
                    applied.content_digest = new_digest;
                }
                // Two variants that nothing emits any more, kept because the
                // stream still contains them: an event once written is
                // history, and dropping the variants would make every commit
                // holding one fail to deserialize -- taking hydration, and
                // so every read, down. They apply to fields the entry no
                // longer has, so replaying one is correctly a no-op.
                EntryMutation::ReplacedContent(_) | EntryMutation::ReplacedSummary(_) => {}
                EntryMutation::ReplacedMedia(new_media) => applied.media = new_media,
                EntryMutation::ReplacedLinks(new_links) => applied.links = new_links,
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

impl Entry {
    /// Convert a parsed entry, or drop it when it carries no usable URL.
    ///
    /// An entry needs somewhere to point. Atom permits an entry with only an
    /// `<id>` and no `<link>`, and a feed can always carry a malformed href;
    /// this used to panic on the first such entry, taking down whatever was
    /// fetching the feed. An id that happens to be a URL (the Atom
    /// convention) serves as the fallback before giving up.
    fn from_rust_entry(entry: RustEntry) -> Option<(Entry, EntryContent)> {
        let content_url = entry
            .links
            .first()
            .and_then(convert_link_to_url)
            .or_else(|| Url::parse(&entry.id).ok());
        let Some(content_url) = content_url else {
            log::warn!(
                "skipping entry {:?} ({:?}): no link and its id is not a URL",
                entry.title.as_ref().map(|t| &t.content),
                entry.id
            );
            return None;
        };
        let links: Vec<Link> = entry.links.into_iter().map(Link::from).collect();
        let mut media: Vec<MediaObject> = entry.media.into_iter().map(MediaObject::from).collect();
        if media.is_empty() {
            media.extend(enclosure_media(&links));
        }

        let content = EntryContent {
            guid: entry.id.clone(),
            summary: entry.summary.map(|t| t.content),
            content: entry.content.map(Content::from),
        };
        Some((
            Entry {
                guid: entry.id.clone(),
                url: content_url,
                title: entry.title.map(|t| t.content),
                updated_at: entry.updated,
                published_at: entry.published,
                links,
                media,
                content_digest: content.digest(),
            },
            content,
        ))
    }
}

/// Build media objects from Atom `<link rel="enclosure">` elements.
///
/// feed-rs turns an RSS `<enclosure>` into a `MediaObject`, but an Atom
/// enclosure stays a plain link — so a podcast published as Atom arrived with
/// no playable media at all, and the player had nothing to open. Normalising
/// here keeps every consumer (gRPC, GraphQL, the iOS player) free of feed
/// dialects.
fn enclosure_media(links: &[Link]) -> Option<MediaObject> {
    let content: Vec<MediaContent> = links
        .iter()
        .filter(|link| link.rel.as_deref() == Some("enclosure"))
        .filter(|link| {
            link.media_type
                .as_deref()
                .is_some_and(|t| t.starts_with("audio/") || t.starts_with("video/"))
        })
        .map(|link| MediaContent {
            url: Url::parse(&link.href).ok(),
            content_type: link.media_type.clone(),
            height: None,
            width: None,
            size: link.length,
        })
        .collect();

    (!content.is_empty()).then_some(MediaObject {
        title: None,
        content,
        // Filled in by `parse_doc` from <itunes:duration> when present.
        duration: None,
        description: None,
    })
}

// A bare User-Agent-less GET is rejected outright (HTTP 400) by at least one
// widely-used podcast redirector (podtrac.com) -- confirmed live against a
// real feed, not a guess, while diagnosing why the archival worker's
// downloads were failing. `reqwest::get` sends no default User-Agent, and
// fetching a *feed* through the same kind of redirector is exactly as
// exposed as fetching an *episode* through one, so this shares the fix.
static HTTP_CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();

fn http_client() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent(concat!("casts-feed-fetcher/", env!("CARGO_PKG_VERSION")))
            // Unset by default, meaning one dead or slow host could hang a
            // fetch indefinitely -- fatal for a sequential batch import
            // (e.g. `casts import-feeds`) over many feeds, where a single
            // stuck request would stall everything behind it.
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("failed to build the feed-fetching HTTP client")
    })
}

pub async fn fetch_body(url: &Url) -> Result<Vec<u8>> {
    let response = http_client().get(url.as_str()).send().await?;
    let body = response.text().await?;
    Ok(body.as_bytes().into())
}

/// Parse an `<itunes:duration>` value into whole seconds.
///
/// The tag admits three forms in the wild: bare seconds (`1234`), `MM:SS`,
/// and `H:MM:SS`. feed-rs (checked through 2.4.0) only recognises the first
/// and last: its `parse_npt` requires three colon-separated parts, so `55:40`
/// falls through to the bare-seconds branch, whose regex matches the leading
/// run of digits and yields 55 seconds for a 55-minute episode. Podcast feeds
/// use `MM:SS` constantly, so durations came out ~60x too small.
fn parse_itunes_duration(text: &str) -> Option<u64> {
    let text = text.trim();
    let parts: Vec<&str> = text.split(':').collect();
    let nums: Option<Vec<u64>> = parts.iter().map(|p| p.trim().parse::<u64>().ok()).collect();
    match nums?[..] {
        [seconds] => Some(seconds),
        [minutes, seconds] => Some(minutes * 60 + seconds),
        [hours, minutes, seconds] => Some(hours * 3600 + minutes * 60 + seconds),
        _ => None,
    }
}

/// Read every `<itunes:duration>` out of the raw document, in item order.
///
/// feed-rs has already thrown away the original text by the time it hands
/// back a `Duration`, so recovering the true value means looking at the
/// source again. Entries without the tag yield `None`, which keeps this list
/// aligned with the parsed entries positionally.
fn itunes_durations(doc: &[u8]) -> Vec<Option<u64>> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_reader(doc);
    reader.trim_text(true);
    let mut buf = Vec::new();
    let mut durations = Vec::new();
    let mut current: Option<u64> = None;
    let mut in_item = false;
    let mut in_duration = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"item" | b"entry" => {
                    in_item = true;
                    current = None;
                }
                b"itunes:duration" if in_item => in_duration = true,
                _ => {}
            },
            Ok(Event::Text(e)) if in_duration => {
                current = e
                    .unescape()
                    .ok()
                    .and_then(|text| parse_itunes_duration(&text));
            }
            Ok(Event::End(e)) => match e.name().as_ref() {
                b"item" | b"entry" => {
                    durations.push(current.take());
                    in_item = false;
                }
                b"itunes:duration" => in_duration = false,
                _ => {}
            },
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    durations
}

/// Parse the document from a vector of bytes.
pub fn parse_doc(url: &Url, doc: &[u8]) -> Result<Feed> {
    Ok(parse_doc_with_content(url, doc)?.feed)
}

/// [`parse_doc`], keeping the show notes lifted out of the entries.
pub fn parse_doc_with_content(url: &Url, doc: &[u8]) -> Result<FetchedFeed> {
    let mut fetched = FetchedFeed::from_url_and_feed(url.to_owned(), parser::parse(doc)?);
    let feed = &mut fetched.feed;

    // Correct the durations feed-rs mis-parsed (see `parse_itunes_duration`).
    // Entries keep document order, so the two lists line up positionally;
    // a short list simply leaves the tail untouched.
    for (entry, itunes) in feed.entries.iter_mut().zip(itunes_durations(doc)) {
        let Some(seconds) = itunes else { continue };
        for media in &mut entry.media {
            media.duration = Some((u32::try_from(seconds).unwrap_or(u32::MAX), 0));
        }
    }
    Ok(fetched)
}

pub async fn fetch_feed(url: &Url) -> Result<Feed> {
    Ok(fetch_feed_with_content(url).await?.feed)
}

/// Fetch and parse a feed, keeping the show notes alongside it.
///
/// Any caller that writes to the store wants this rather than `fetch_feed`:
/// entries reference their notes by digest, so notes dropped at parse time
/// leave a dangling reference nothing can resolve.
pub async fn fetch_feed_with_content(url: &Url) -> Result<FetchedFeed> {
    let body = fetch_body(url).await?;
    parse_doc_with_content(url, body.as_slice())
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

    /// Stores the new content digest, identifying show notes held out of
    /// band. Replaces `ReplacedSummary` and `ReplacedContent`.
    ReplacedContentDigest(Option<String>),

    /// Legacy: the summary, when entries still carried it inline.
    ///
    /// Nothing emits this. It exists so commits written before the content
    /// split still deserialize -- see `Entry::apply`.
    ReplacedSummary(Option<String>),

    /// Legacy: the content, when entries still carried it inline. See
    /// `ReplacedSummary`.
    ReplacedContent(Option<Content>),

    /// Stores the new media vector
    ReplacedMedia(Vec<MediaObject>),

    /// Stores the new published date
    ReplacedPublished(Option<DateTime<Utc>>),

    /// Stores the new links.
    ReplacedLinks(Vec<Link>),
}

#[cfg(test)]
mod tests {
    use super::{itunes_durations, parse_doc, parse_itunes_duration};
    use url::Url;

    mod content_split {
        use super::super::*;

        fn entry(guid: &str, content_digest: Option<&str>) -> Entry {
            Entry {
                guid: String::from(guid),
                url: Url::parse("http://example.com/e.mp3").unwrap(),
                title: Some(String::from("An episode")),
                updated_at: None,
                published_at: None,
                links: vec![],
                media: vec![],
                content_digest: content_digest.map(String::from),
            }
        }

        fn feed(entries: Vec<Entry>) -> Feed {
            Feed {
                url: Url::parse("http://example.com/feed.xml").ok(),
                raw_url: String::from("http://example.com/feed.xml"),
                metadata: FeedMetadata {
                    title: String::from("A feed"),
                    authors: vec![],
                    logo: None,
                    description: None,
                },
                entries,
            }
        }

        #[test]
        fn identical_notes_produce_the_same_digest_regardless_of_guid() {
            let a = EntryContent {
                guid: String::from("guid-a"),
                summary: Some(String::from("shared notes")),
                content: None,
            };
            let mut b = a.clone();
            b.guid = String::from("guid-b");
            assert_eq!(a.digest(), b.digest());
        }

        #[test]
        fn different_notes_produce_different_digests() {
            let notes = |s: &str| EntryContent {
                guid: String::from("guid-1"),
                summary: Some(String::from(s)),
                content: None,
            };
            assert_ne!(notes("one").digest(), notes("two").digest());
        }

        #[test]
        fn empty_notes_have_no_digest() {
            let empty = EntryContent {
                guid: String::from("guid-1"),
                summary: None,
                content: None,
            };
            assert!(empty.is_empty());
            assert_eq!(empty.digest(), None);
        }

        /// The whole point of comparing by digest: a publisher rewriting
        /// show notes without touching anything else must still produce a
        /// mutation, or a content edit would go unnoticed by a diff that
        /// only ever sees the slim entry.
        #[test]
        fn a_changed_digest_produces_a_replaced_content_digest_mutation() {
            let before = entry("guid-1", Some("old-digest"));
            let after = entry("guid-1", Some("new-digest"));
            let mutations = before.diff(&after);
            assert!(matches!(
                mutations.as_slice(),
                [EntryMutation::ReplacedContentDigest(Some(d))] if d == "new-digest"
            ));
        }

        #[test]
        fn an_unchanged_digest_produces_no_mutation() {
            let before = entry("guid-1", Some("same"));
            let after = entry("guid-1", Some("same"));
            assert!(before.diff(&after).is_empty());
        }

        /// `Feed::diff` -> `Feed::apply_diff` must round-trip a digest
        /// change end to end, the same path `refresh_feeds` drives in casts.
        #[test]
        fn feed_diff_and_apply_round_trip_a_digest_change() {
            let existing = feed(vec![entry("guid-1", Some("old"))]);
            let observed = feed(vec![entry("guid-1", Some("new"))]);
            let changes = existing.diff(&observed);
            let applied = existing.apply_diff(changes);
            assert_eq!(applied.entries[0].content_digest, Some(String::from("new")));
        }

        /// Commits already written carry `ReplacedSummary`/`ReplacedContent`
        /// -- nothing emits them any more, but the stream still contains
        /// them, and replaying a commit must not fail just because the
        /// field it once touched no longer exists on `Entry`.
        #[test]
        fn legacy_content_mutations_replay_as_no_ops_instead_of_failing() {
            let existing = feed(vec![entry("guid-1", Some("keep-me"))]);
            let legacy_changes = vec![FeedMutation::EntryChanged(
                String::from("guid-1"),
                vec![
                    EntryMutation::ReplacedSummary(Some(String::from("old inline summary"))),
                    EntryMutation::ReplacedContent(Some(Content {
                        body: Some(String::from("<p>old inline content</p>")),
                        content_type: String::from("text/html"),
                        length: None,
                        src: None,
                    })),
                ],
            )];
            let applied = existing.apply_diff(legacy_changes);
            // Replaying the legacy mutations must not touch the digest --
            // they carry no digest information at all.
            assert_eq!(
                applied.entries[0].content_digest,
                Some(String::from("keep-me"))
            );
        }

        /// An entry serialized before the split has `summary`/`content`
        /// inline and no `content_digest` field at all. It must still
        /// deserialize -- the alternative is every historical commit
        /// failing to parse, which fails hydration for every stream that
        /// has one.
        #[test]
        fn a_pre_split_entry_deserializes_with_no_digest() {
            let legacy_json = serde_json::json!({
                "guid": "guid-1",
                "url": "http://example.com/e.mp3",
                "title": "An episode",
                "summary": "inline notes, the old way",
                "updated_at": null,
                "published_at": null,
                "content": null,
                "links": [],
                "media": []
            });
            let parsed: Entry = serde_json::from_value(legacy_json)
                .expect("a pre-split entry must still deserialize");
            assert_eq!(parsed.guid, "guid-1");
            assert_eq!(parsed.content_digest, None);
        }

        /// Parsing a real document must produce a digest for every entry
        /// that has notes, and the two halves must agree on which entry is
        /// which -- a content object addressed under the wrong guid is as
        /// bad as losing it, since nothing would ever resolve it correctly.
        #[test]
        fn parsing_pairs_each_entry_with_its_own_content_by_guid() {
            let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0"><channel>
<title>Test feed</title>
<item>
  <title>Episode One</title>
  <guid>guid-1</guid>
  <link>http://example.com/one.mp3</link>
  <description>Summary for episode one</description>
</item>
<item>
  <title>Episode Two</title>
  <guid>guid-2</guid>
  <link>http://example.com/two.mp3</link>
  <description>Summary for episode two</description>
</item>
</channel></rss>"#;
            let url = Url::parse("http://example.com/feed.xml").unwrap();
            let fetched = parse_doc_with_content(&url, xml).expect("valid rss must parse");

            assert_eq!(fetched.feed.entries.len(), 2);
            assert_eq!(fetched.contents.len(), 2);

            for entry in &fetched.feed.entries {
                let digest = entry
                    .content_digest
                    .as_ref()
                    .unwrap_or_else(|| panic!("{} should have notes", entry.guid));
                let matching = fetched
                    .contents
                    .iter()
                    .find(|c| c.guid == entry.guid)
                    .unwrap_or_else(|| panic!("no stored content for {}", entry.guid));
                assert_eq!(matching.digest().as_deref(), Some(digest.as_str()));
            }
        }

        /// `fetch_feed`'s caller-facing contract (an entry with a digest but
        /// no inline notes) must hold even for the plain `parse_doc` that
        /// discards the content half -- a caller using the wrong function
        /// still gets consistent entries, just without notes anywhere.
        #[test]
        fn parse_doc_discards_content_but_keeps_digests() {
            let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0"><channel>
<title>Test feed</title>
<item>
  <title>Episode One</title>
  <guid>guid-1</guid>
  <link>http://example.com/one.mp3</link>
  <description>Summary for episode one</description>
</item>
</channel></rss>"#;
            let url = Url::parse("http://example.com/feed.xml").unwrap();
            let feed = parse_doc(&url, xml).expect("valid rss must parse");
            assert!(feed.entries[0].content_digest.is_some());
        }
    }

    #[test]
    fn itunes_duration_accepts_all_three_forms() {
        // Bare seconds.
        assert_eq!(parse_itunes_duration("15"), Some(15));
        // MM:SS -- the form feed-rs reads as bare seconds.
        assert_eq!(parse_itunes_duration("55:40"), Some(55 * 60 + 40));
        assert_eq!(parse_itunes_duration("01:12"), Some(72));
        // H:MM:SS.
        assert_eq!(parse_itunes_duration("1:02:03"), Some(3723));
        assert_eq!(parse_itunes_duration("10:00:00"), Some(36000));
        // Junk stays junk rather than becoming a plausible-looking number.
        assert_eq!(parse_itunes_duration("about an hour"), None);
        assert_eq!(parse_itunes_duration(""), None);
        assert_eq!(parse_itunes_duration("1:2:3:4"), None);
    }

    const FEED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
  <channel>
    <title>Examplecast</title>
    <link>http://example.com/</link>
    <description>A test feed.</description>
    <item>
      <title>Long episode</title>
      <guid>guid-1</guid>
      <link>http://example.com/1</link>
      <enclosure url="http://example.com/1.mp3" length="1" type="audio/mpeg"/>
      <itunes:duration>55:40</itunes:duration>
    </item>
    <item>
      <title>Short trailer</title>
      <guid>guid-2</guid>
      <link>http://example.com/2</link>
      <enclosure url="http://example.com/2.mp3" length="1" type="audio/mpeg"/>
      <itunes:duration>15</itunes:duration>
    </item>
    <item>
      <title>Epic</title>
      <guid>guid-3</guid>
      <link>http://example.com/3</link>
      <enclosure url="http://example.com/3.mp3" length="1" type="audio/mpeg"/>
      <itunes:duration>4:38:12</itunes:duration>
    </item>
  </channel>
</rss>"#;

    #[test]
    fn durations_are_read_in_item_order() {
        assert_eq!(
            itunes_durations(FEED.as_bytes()),
            vec![Some(55 * 60 + 40), Some(15), Some(4 * 3600 + 38 * 60 + 12)]
        );
    }

    /// The regression this exists for: a 55m40s episode must not come back as
    /// 55 seconds.
    #[test]
    fn parsed_feed_carries_corrected_durations() {
        let url = Url::parse("http://example.com/feed.xml").unwrap();
        let feed = parse_doc(&url, FEED.as_bytes()).unwrap();
        assert_eq!(feed.entries.len(), 3);

        let durations: Vec<Option<(u32, u32)>> = feed
            .entries
            .iter()
            .map(|e| e.media.first().and_then(|m| m.duration))
            .collect();
        assert_eq!(
            durations,
            vec![Some((3340, 0)), Some((15, 0)), Some((16692, 0))]
        );
    }
}
