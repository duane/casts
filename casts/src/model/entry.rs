use url::Url;
use uuid::Uuid;

pub struct Entry {
    pub id: Uuid,

    pub feed_id: Uuid,

    /// The self-identified guid of the feed entry.
    pub guid: String,

    /// The url where the entry content may be found.
    pub url: Url,

    /// The title of the feed entry.
    pub title: String,

    /// A summary (typically html) of the entry content.
    pub summary: String,
    // /// Content associated with media.
    // pub content: Option<Content>,

    // /// Other links associated with content
    // pub links: Vec<Link>,

    // /// Other media
    // pub media: Vec<MediaObject>,
}
