use chrono::{DateTime, Utc};
use rusqlite::NO_PARAMS;
use rusqlite::{params, Connection};
use url::Url;

pub fn open() -> Connection {
  let xdg_dirs = xdg::BaseDirectories::with_prefix("casts").unwrap();
  let db_path = xdg_dirs
    .place_data_file("casts.sqlite")
    .expect("could not create data directory");
  Connection::open(db_path).expect("could not open casts.sqlite")
}

pub fn initialize(conn: &Connection) {
  conn
    .execute_batch(
      "CREATE TABLE IF NOT EXISTS feeds (
      id INTEGER NOT NULL PRIMARY KEY,
      url TEXT NOT NULL UNIQUE,
      title TEXT,
      title_content_type TEXT,
      description TEXT,
      description_content_type TEXT
    );",
    )
    .expect("could not intialize sqlite feeds table");
  conn
    .execute_batch(
      "CREATE TABLE IF NOT EXISTS entries (
      id INTEGER NOT NULL PRIMARY KEY,
      feed_id INTEGER NOT NULL,
      title TEXT,
      title_content_type TEXT,
      summary TEXT,
      summary_content_type TEXT,
      content_url TEXT,
      content_content_type TEXT,
      published DATETIME,
      updated DATETIME,
      FOREIGN KEY (feed_id) REFERENCES feeds(id)
    );",
    )
    .expect("could not intialize sqlite feeds table");
  conn
    .execute_batch("CREATE INDEX IF NOT EXISTS entries_published ON entries (published);")
    .expect("could not intialize sqlite feeds table");
}

pub struct AddFeed {
  pub url: Url,
  pub title: Option<String>,
  pub title_content_type: Option<String>,
  pub description: Option<String>,
  pub description_content_type: Option<String>,
}

pub struct Feed {
  pub id: u32,
  pub url: Url,
  pub title: Option<String>,
  pub title_content_type: Option<String>,
  pub description: Option<String>,
  pub description_content_type: Option<String>,
}

pub fn add_feed(conn: &Connection, add_feed: &AddFeed) -> usize {
  let mut statement = conn
    .prepare(
      "INSERT INTO feeds (
      url,
      title,
      title_content_type,
      description,
      description_content_type
    ) VALUES (?, ?, ?, ?, ?)",
    )
    .unwrap();
  statement
    .execute(params![
      add_feed.url,
      add_feed.title,
      add_feed.title_content_type,
      add_feed.description,
      add_feed.description_content_type,
    ])
    .unwrap()
}

pub fn get_feed(conn: &Connection, feed_id: u32) -> Option<Feed> {
  let mut statement = conn
    .prepare(
      "SELECT id,
      url,
    title,
    title_content_type,
    description,
    description_content_type
    FROM feeds WHERE id = ?;",
    )
    .unwrap();
  let mut rows = statement.query(params![feed_id]).unwrap();
  rows.next().unwrap().map(|row| Feed {
    id: row.get(0).unwrap(),
    url: row.get(1).unwrap(),
    title: row.get(2).unwrap(),
    title_content_type: row.get(3).unwrap(),
    description: row.get(4).unwrap(),
    description_content_type: row.get(5).unwrap(),
  })
}

pub struct AddEntry {
  pub feed_id: u32,
  pub title: Option<String>,
  pub title_content_type: Option<String>,
  pub summary: Option<String>,
  pub summary_content_type: Option<String>,
  pub content_url: Option<Url>,
  pub content_content_type: Option<String>,
  pub published: Option<DateTime<Utc>>,
  pub updated: Option<DateTime<Utc>>,
}

pub fn add_entry(conn: &Connection, add_entry: &AddEntry) -> usize {
  let mut statement = conn
    .prepare(
      "INSERT INTO entries (
    feed_id,
    title,
    title_content_type,
    summary,
    summary_content_type,
    content_url,
    content_content_type,
    published,
    updated
  ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?);",
    )
    .unwrap();
  statement
    .execute(params![
      add_entry.feed_id,
      add_entry.title,
      add_entry.title_content_type,
      add_entry.summary,
      add_entry.summary_content_type,
      add_entry.content_url,
      add_entry.content_content_type,
      add_entry.published,
      add_entry.updated
    ])
    .unwrap()
}

#[derive(Debug)]
pub struct Entry {
  pub id: u32,
  pub feed_id: u32,
  pub title: Option<String>,
  pub title_content_type: Option<String>,
  pub summary: Option<String>,
  pub summary_content_type: Option<String>,
  pub content_url: Option<Url>,
  pub content_content_type: Option<String>,
  pub published: Option<DateTime<Utc>>,
  pub updated: Option<DateTime<Utc>>,
}

pub fn fetch_latest_entry(conn: &Connection) -> Option<Entry> {
  let mut statement = conn
    .prepare(
      "SELECT id, feed_id,
    title,
    title_content_type,
    summary,
    summary_content_type,
    content_url,
    content_content_type,
    published,
    updated
    FROM entries ORDER BY published DESC LIMIT 1;",
    )
    .unwrap();
  let mut rows = statement.query(NO_PARAMS).unwrap();
  rows.next().unwrap().map(|row| Entry {
    id: row.get(0).unwrap(),
    feed_id: row.get(1).unwrap(),
    title: row.get(2).unwrap(),
    title_content_type: row.get(3).unwrap(),
    summary: row.get(4).unwrap(),
    summary_content_type: row.get(5).unwrap(),
    content_url: row.get(6).unwrap(),
    content_content_type: row.get(7).unwrap(),
    published: row.get(8).unwrap(),
    updated: row.get(9).unwrap(),
  })
}
