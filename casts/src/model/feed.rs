use url::Url;
use uuid::Uuid;

pub struct Feed {
    pub id: Uuid,
    pub stream_id: Uuid,
    pub title: String,
    pub description: String,
    pub url: Url,
}
