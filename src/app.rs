use crate::model;
use rusqlite::Connection;
use std::io::stdout;
use url::Url;

use crate::{blocking_download_feed_rs, parse_feed_rs};

use feed_rs::model::{Entry as RustEntry, Feed as RustFeed};
use crate::feeds::convert_link_to_url;
use log::info;

/// Render a feed to atom.
pub fn render(feed: &model::Feed, entries: &mut Vec<model::Entry>) {
  feed.render_to_atom(stdout(), &mut entries.iter()).unwrap();
}

/// Add a feed to the database.
pub fn subscribe(url: &Url) -> anyhow::Result<()> {
  let conn = model::open();
  model::initialize(&conn);

  let download_result = blocking_download_feed_rs(url)?;
  let parsed_feed = parse_feed_rs(&download_result)?;
  let title = parsed_feed.title.unwrap().content.clone();
  let description = parsed_feed.description.map(|d| d.content.clone());
  let add_feed = model::AddFeed {
    url: url.clone(),
    title,
    description,
  };

  let feed_id = model::add_feed(&conn, &add_feed) as i64;
  for entry in parsed_feed.entries.into_iter().rev() {
    let guid = entry.id.clone();
    let title = entry.title.unwrap().content.clone();
    let summary = entry.summary.map(|s| s.content.clone());
    let (content_url, content_type) = match entry.content {
      Some(c) => {
        let escaped_href = &c.src.unwrap().href;
        let unescaped_href = htmlescape::decode_html(escaped_href).unwrap();
        let url = Url::parse(&unescaped_href)?;
        let content_type = c.content_type.to_string();
        (url, content_type)
      }
      None => continue,
    };

    let add_entry = model::AddEntry {
      feed_id,
      guid,
      title,
      content_url,
      content_type,
      summary,
      published: entry.published,
      updated: entry.updated,
    };
    let _entry_id = model::add_entry(&conn, &add_entry);
  }

  Ok(conn.close().expect("could rm not close db"))
}

/// Log a detected change in the feed processing.
pub fn notify_change(typ: &str, id: &String, field: &str, old: &String, new: &String) {
  info!("Found change in feed:");
  info!("Item changed: {} {}", typ, id);
  info!("Field changed: {}", field);
  info!("Old value: {}", old);
  info!("New value: {}", new);
}

/// Update the persisted, cached feed with the more recent one.
pub fn update_feed(conn: &Connection, persisted: &model::Feed, recent: &RustFeed) {
  let title = recent.title.as_ref().unwrap().content.clone();
  let description = recent.description.as_ref().map(|d| d.content.clone());
  let update_feed = model::UpdateFeedMetadata {
    id: persisted.id,
    title,
    description,
  };

  let _feed_did_update = model::update_feed(conn, &update_feed);
  match recent.title.as_ref() {
    Some(t) => {
      if t.content != persisted.title {
        notify_change(
          "feed",
          &persisted.title.to_string(),
          "title",
          &persisted.title,
          &t.content,
        );
      }
    }
    None => (),
  }
  if recent.description.as_ref().map(|t| t.content.clone()) != persisted.description {
    notify_change(
      "feed",
      &persisted.title.to_string(),
      "description",
      &persisted
        .description
        .clone()
        .unwrap_or("No description.".to_string()),
      &recent
        .description
        .as_ref()
        .map(|d| d.content.clone())
        .unwrap_or("No description.".to_string()),
    );
  }
}

/// Update the entry to a feed.
pub fn update_feed_item_if_changed(_conn: &Connection, persisted: &model::Entry, recent: &RustEntry) {
  let title = recent.title.as_ref().unwrap().content.clone();

  let (content_url, content_type) = match recent.content.as_ref() {
    Some(content) => (
      convert_link_to_url(content.src.as_ref().unwrap()),
      content.content_type.to_string(),
    ),
    None => panic!("All feed items should have content."),
  };
  let summary = &recent.summary.as_ref().map(|t| &t.content);
  let published = recent.published.as_ref();
  let _updated = recent.updated.as_ref();

  let old_title = &persisted.title;
  let old_summary = &persisted.summary.as_ref();
  let old_content_url = &persisted.content_url;
  let old_content_type = &persisted.content_type;
  let old_published = persisted.published.as_ref();
  let _old_updated = persisted.updated.as_ref();

  if &title != old_title {
    notify_change("entry", &persisted.guid, "title", old_title, &title);
  }
  if &content_url != old_content_url {
    notify_change(
      "entry",
      &persisted.guid,
      "content_url",
      &old_content_url.to_string(),
      &content_url.to_string(),
    );
  }
  if &content_type != old_content_type {
    notify_change(
      "entry",
      &persisted.guid,
      "content_type",
      old_content_type,
      &content_type,
    );
  }
  if summary != old_summary {
    notify_change(
      "entry",
      &persisted.guid,
      "summary",
      old_summary.unwrap_or(&"No summary.".to_string()),
      summary.unwrap_or(&"No summary.".to_string()),
    );
  }
  if published != old_published {
    notify_change(
      "entry",
      &persisted.guid,
      "published",
      &old_published
        .as_ref()
        .map(|d| d.to_string())
        .unwrap_or("No published.".to_string()),
      &published
        .as_ref()
        .map(|d| d.to_string())
        .unwrap_or("No published.".to_string()),
    );
  }
}

/// Look for updates to the existing feed.
pub fn poll(conn: &Connection, feed: &model::Feed) {
  println!("Polling {}....", feed.url.as_ref());
  let download_result = blocking_download_feed_rs(&feed.url).unwrap();
  let parsed_feed = parse_feed_rs(&download_result).unwrap();
  update_feed(&conn, feed, &parsed_feed);

  for entry in parsed_feed.entries.into_iter().rev() {
    match model::find_entry_by_guid(&conn, feed.id, &entry.id) {
      Some(persisted) => update_feed_item_if_changed(conn, &persisted, &entry),
      None => {
        println!("Unknown episode {}, adding...", entry.id);
        let title = entry.title.as_ref().unwrap().content.clone();

        let (content_url, content_type) = match entry.content.as_ref() {
          Some(content) => (
            convert_link_to_url(content.src.as_ref().unwrap()),
            content.content_type.to_string(),
          ),
          None => panic!("All feed items should have content."),
        };
        let summary = &entry.summary.as_ref().map(|t| &t.content);
        let published = entry.published.as_ref();
        let updated = entry.updated.as_ref();

        let add_entry = model::AddEntry {
          guid: entry.id.clone(),
          feed_id: feed.id,
          title,
          summary: summary.map(|s| s.clone()),
          content_url,
          content_type,
          published: published.map(|d| d.clone()),
          updated: updated.map(|d| d.clone()),
        };
        model::add_entry(&conn, &add_entry);
        let _persisted_entry = model::find_entry_by_guid(&conn, feed.id, &entry.id).unwrap();
      }
    }
  }
}
