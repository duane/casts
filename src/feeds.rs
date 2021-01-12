use feed_rs::model::{Link as RustLink};
use url::Url;

/// Convert an RSS/Atom link to a URL.
pub fn convert_link_to_url(link: &RustLink) -> Url {
  let escaped_href = &link.href;
  let unescaped_href = htmlescape::decode_html(escaped_href).unwrap();
  Url::parse(&unescaped_href).unwrap()
}