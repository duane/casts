use crate::model;
use crate::view;
use notify_rust::Notification;
use rusqlite::Connection;
use url::Url;

use casts::{blocking_download_feed_rs, parse_feed_rs};

use feed_rs::model::{Entry as RustEntry, Feed as RustFeed, Link as RustLink};

fn convert_link_to_url(link: &RustLink) -> Url {
  let escaped_href = &link.href;
  let unescaped_href = htmlescape::decode_html(escaped_href).unwrap();
  Url::parse(&unescaped_href).unwrap()
}

pub fn subscribe(url: &Url) {
  let conn = model::open();
  model::initialize(&conn);

  let download_result = blocking_download_feed_rs(url).unwrap();
  let parsed_feed = parse_feed_rs(&download_result).unwrap();
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
        let url = Url::parse(&unescaped_href).unwrap();
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

  conn.close().expect("could rm not close db");
}

pub fn notify_change(typ: &str, id: &String, field: &str, old: &String, new: &String) {
  println!("Found change in feed:");
  println!("Item changed: {} {}", typ, id);
  println!("Field changed: {}", field);
  println!("Old value: {}", old);
  println!("New value: {}", new);
}

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

pub fn update_feed_item(_conn: &Connection, persisted: &model::Entry, recent: &RustEntry) {
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

pub fn poll(conn: &Connection, feed: &model::Feed) {
  println!("Polling {}....", feed.url.as_ref());
  let download_result = blocking_download_feed_rs(&feed.url).unwrap();
  let parsed_feed = parse_feed_rs(&download_result).unwrap();
  update_feed(&conn, feed, &parsed_feed);

  for entry in parsed_feed.entries.into_iter().rev() {
    match model::find_entry_by_guid(&conn, feed.id, &entry.id) {
      Some(persisted) => update_feed_item(conn, &persisted, &entry), /*println!("Already know entry {}, skipping....", persisted.guid)*/
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
        let persisted_entry = model::find_entry_by_guid(&conn, feed.id, &entry.id).unwrap();
        Notification::new()
          .summary("New episode")
          .body(&view::entry_summary(&persisted_entry))
          .show()
          .unwrap();
      }
    }
  }
}
