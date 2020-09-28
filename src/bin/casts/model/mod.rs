use rusqlite::Connection;
use url::Url;


pub fn open() -> Connection {
  Connection::open("casts.sqlite").expect("could not open casts.sqlite")
}

pub fn initialize(conn: &Connection) {
  conn.execute_batch(
    "CREATE TABLE IF NOT EXISTS feeds (
      id INTEGER NOT NULL PRIMARY KEY,
      url TEXT NOT NULL UNIQUE,
      title TEXT,
      title_content_type TEXT,
      description TEXT,
      description_content_type TEXT
    );"
  ).expect("could not intialize sqlite feeds table");
}

pub fn add_feed(conn: &Connection, feed: Url) -> usize {
  let mut statement = conn.prepare("INSERT INTO feeds (url) VALUES (?)").unwrap();
  statement.execute(&[&feed.into_string()]).unwrap()
}

pub struct SetFeedMetadata {
  pub feed_id: usize,
  pub title: Option<String>,
  pub title_content_type: Option<String>,
  pub description: Option<String>,
  pub description_content_type: Option<String>,
}

pub fn set_feed_metadata(conn: &Connection, update: &SetFeedMetadata) {
  let mut statement = conn.prepare("UPDATE feeds SET title = ?, title_content_type = ?, description = ?, description_content_type = ? WHERE id = ?").unwrap();
  statement.execute(&[&update.title, &update.title_content_type, &update.description, &update.description_content_type, &Some(update.feed_id.to_string())]).unwrap();
}
