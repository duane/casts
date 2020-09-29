use chrono::{DateTime, Utc};
use rusqlite::NO_PARAMS;
use rusqlite::{params, Connection};
use url::Url;
use uuid::Uuid;

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
      id INTEGER PRIMARY KEY,
      url TEXT NOT NULL UNIQUE,
      title TEXT NOT NULL,
      description TEXT
    );",
    )
    .expect("could not intialize sqlite feeds table");
  conn
    .execute_batch(
      "CREATE TABLE IF NOT EXISTS entries (
      id INTEGER PRIMARY KEY,
      feed_id INTEGER NOT NULL,
      guid CHAR(128) NOT NULL,
      title TEXT NOT NULL,
      content_url TEXT NOT NULL,
      content_type TEXT NOT NULL,

      summary TEXT,
      published DATETIME,
      updated DATETIME,

      FOREIGN KEY (feed_id) REFERENCES feeds(id)
    );",
    )
    .expect("could not intialize sqlite feeds table");
  conn
    .execute_batch(
      "CREATE INDEX IF NOT EXISTS entries_published
       ON entries (published);",
    )
    .expect("could not add entries_published index");
  conn
    .execute_batch(
      "CREATE INDEX IF NOT EXISTS feed_titles
      ON feeds (title);",
    )
    .expect("could not add feed_titles index");
  conn
    .execute_batch(
      "CREATE UNIQUE INDEX IF NOT EXISTS feed_titles
      ON entries (feed_id, title);",
    )
    .expect("could not intialize sqlite feeds table");
}

#[derive(Debug)]
pub struct AddFeed {
  pub url: Url,
  pub title: String,
  pub description: Option<String>,
}

pub fn add_feed(conn: &Connection, add_feed: &AddFeed) -> i64 {
  let mut statement = conn
    .prepare(
      "INSERT INTO feeds (
      url,
      title,
      description
    ) VALUES (?, ?, ?)",
    )
    .unwrap();
  statement
    .execute(params![add_feed.url, add_feed.title, add_feed.description])
    .unwrap();
  conn.last_insert_rowid()
}

#[derive(Debug)]
pub struct UpdateFeedMetadata {
  pub id: i64,
  pub title: String,
  pub description: Option<String>,
}

pub fn update_feed(conn: &Connection, update_feed_metadata: &UpdateFeedMetadata) -> bool {
  let mut statement = conn
    .prepare("UPDATE feeds SET title = ?, description = ? WHERE id = ?;")
    .unwrap();
  statement
    .execute(params![
      update_feed_metadata.title,
      update_feed_metadata.description,
      update_feed_metadata.id
    ])
    .unwrap()
    == 1
}

#[derive(Debug)]
pub struct Feed {
  pub id: i64,
  pub url: Url,
  pub title: String,
  pub description: Option<String>,
}

pub fn get_feed_by_id(conn: &Connection, feed_id: i64) -> Option<Feed> {
  let mut statement = conn
    .prepare(
      "SELECT id,
              url,
              title,
              description
       FROM feeds WHERE id = ?;",
    )
    .unwrap();
  let mut rows = statement.query(params![feed_id]).unwrap();
  rows.next().unwrap().map(|row| Feed {
    id: row.get(0).unwrap(),
    url: row.get(1).unwrap(),
    title: row.get(2).unwrap(),
    description: row.get(3).unwrap(),
  })
}

pub fn get_feed_by_name(conn: &Connection, title: &str) -> Option<Feed> {
  let mut statement = conn
    .prepare(
      "SELECT id,
              url,
              title,
              description
       FROM feeds WHERE title = ?;",
    )
    .unwrap();
  let mut rows = statement.query(params![title]).unwrap();
  rows.next().unwrap().map(|row| Feed {
    id: row.get(0).unwrap(),
    url: row.get(1).unwrap(),
    title: row.get(2).unwrap(),
    description: row.get(3).unwrap(),
  })
}

pub fn slurp_feeds(conn: &Connection) -> Vec<Feed> {
  let mut statement = conn
    .prepare(
      "SELECT id,
              url,
              title,
              description
       FROM feeds",
    )
    .unwrap();
  statement
    .query_map(NO_PARAMS, |row| {
      Ok(Feed {
        id: row.get(0).unwrap(),
        url: row.get(1).unwrap(),
        title: row.get(2).unwrap(),
        description: row.get(3).unwrap(),
      })
    })
    .unwrap()
    .map(|r| r.unwrap())
    .collect()
}

pub struct AddEntry {
  pub feed_id: i64,
  pub guid: String,
  pub title: String,
  pub content_url: Url,
  pub content_type: String,
  pub summary: Option<String>,
  pub published: Option<DateTime<Utc>>,
  pub updated: Option<DateTime<Utc>>,
}

pub fn add_entry(conn: &Connection, add_entry: &AddEntry) -> usize {
  let mut statement = conn
    .prepare(
      "INSERT INTO entries (
         feed_id,
         guid,
         title,
         content_url,
         content_type,
         summary,
         published,
         updated
       ) VALUES (?, ?, ?, ?, ?, ?, ?, ?);",
    )
    .unwrap();
  statement
    .execute(params![
      add_entry.feed_id,
      add_entry.guid,
      add_entry.title,
      add_entry.content_url,
      add_entry.content_type,
      add_entry.summary,
      add_entry.published,
      add_entry.updated
    ])
    .unwrap()
}

#[derive(Debug)]
pub struct Entry {
  pub id: i64,
  pub feed_id: i64,
  pub guid: String,
  pub title: String,
  pub content_url: Url,
  pub content_type: String,

  pub summary: Option<String>,
  pub published: Option<DateTime<Utc>>,
  pub updated: Option<DateTime<Utc>>,
}

pub fn slurp_entries(conn: &Connection, feed_id: i64) -> Vec<Entry> {
  let mut statement = conn
    .prepare(
      "SELECT id,
              feed_id,
              guid,
              title,
              content_url,
              content_type,
              summary,
              published,
              updated
       FROM entries WHERE feed_id = ?;",
    )
    .unwrap();
  statement
    .query_map(params![feed_id], |row| {
      Ok(Entry {
        id: row.get(0).unwrap(),
        feed_id: row.get(1).unwrap(),
        guid: row.get(2).unwrap(),
        title: row.get(3).unwrap(),
        content_url: row.get(4).unwrap(),
        content_type: row.get(5).unwrap(),
        summary: row.get(6).unwrap(),
        published: row.get(7).unwrap(),
        updated: row.get(8).unwrap(),
      })
    })
    .unwrap()
    .map(|t| t.unwrap())
    .collect()
}

pub fn find_entry_by_guid(conn: &Connection, feed_id: i64, guid: &String) -> Option<Entry> {
  let mut statement = conn
    .prepare(
      "SELECT id,
              feed_id,
              guid,
              title,
              content_url,
              content_type,
              summary,
              published,
              updated
       FROM entries
       WHERE guid = ? and feed_id = ?;",
    )
    .unwrap();
  let row = statement
    .query_map(params![guid, feed_id], |row| {
      Ok(Entry {
        id: row.get(0).unwrap(),
        feed_id: row.get(1).unwrap(),
        guid: row.get(2).unwrap(),
        title: row.get(3).unwrap(),
        content_url: row.get(4).unwrap(),
        content_type: row.get(5).unwrap(),
        summary: row.get(6).unwrap(),
        published: row.get(7).unwrap(),
        updated: row.get(8).unwrap(),
      })
    })
    .unwrap()
    .map(|x| x.unwrap())
    .next();
  row
}

pub fn slurp_entries_since(conn: &Connection, date: &DateTime<Utc>) -> Vec<Entry> {
  let mut statement = conn
    .prepare(
      "SELECT id,
              feed_id,
              guid,
              title,
              content_url,
              content_type,
              summary,
              published,
              updated
       FROM entries
       WHERE published >= ? 
       ORDER BY published DESC;",
    )
    .unwrap();
  statement
    .query_map(params![date], |row| {
      Ok(Entry {
        id: row.get(0).unwrap(),
        feed_id: row.get(1).unwrap(),
        guid: row.get(2).unwrap(),
        title: row.get(3).unwrap(),
        content_url: row.get(4).unwrap(),
        content_type: row.get(5).unwrap(),
        summary: row.get(6).unwrap(),
        published: row.get(7).unwrap(),
        updated: row.get(8).unwrap(),
      })
    })
    .unwrap()
    .map(|x| x.unwrap())
    .collect()
}
