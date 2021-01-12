#![deny(missing_docs)]

use chrono::{DateTime, Utc};
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;
use rusqlite::NO_PARAMS;
use rusqlite::{params, Connection};
use url::Url;

/// Open a new connection to the backing databse.
pub fn open() -> Connection {
  let xdg_dirs = xdg::BaseDirectories::with_prefix("casts").unwrap();
  let db_path = xdg_dirs
    .place_data_file("casts.sqlite")
    .expect("could not create data directory");
  Connection::open(db_path).expect("could not open casts.sqlite")
}

/// Create the schema.
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

/// AddFeed command
#[derive(Debug)]
pub struct AddFeed {
  /// URL of the feed
  pub url: Url,

  /// Title of the feed.
  pub title: String,

  /// Description of the feed (likely text/html)
  pub description: Option<String>,
}

/// Execute the AddFeed command.
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

/// UpdateFeedMetadata command.
#[derive(Debug)]
pub struct UpdateFeedMetadata {
  /// ID of the entry to update.
  pub id: i64,

  /// Title of the entry.
  pub title: String,

  /// Optional, likely text/html description of the entry.
  pub description: Option<String>,
}

/// Execute the UpdateFeedMetadata command.
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

/// Feed model
#[derive(Debug)]
pub struct Feed {
  /// ID assigned by the database.
  pub id: i64,

  /// URL of the feed
  pub url: Url,

  /// Title fo the feed.
  pub title: String,

  /// Description of the feed.
  pub description: Option<String>,
}

impl Feed {
  /// Write an atom feed to the given writer representing the feed.
  pub fn render_to_atom<W: std::io::Write>(
    &self,
    writer: W,
    iter: &mut dyn Iterator<Item = &Entry>,
  ) -> anyhow::Result<()> {
    let mut writer = Writer::new(writer);

    let rss_elem_name = b"rss";
    let rss_elem_name_len = rss_elem_name.len();
    let mut rss_elem = BytesStart::owned(rss_elem_name.to_vec(), rss_elem_name_len);
    rss_elem.push_attribute(("xmlns:dc", "http://purl.org/dc/elements/1.1/"));
    rss_elem.push_attribute(("xmlns:itunes", "http://www.itunes.com/dtds/podcast-1.0.dtd"));
    rss_elem.push_attribute(("version", "2.0"));

    writer.write_event(Event::Decl(BytesDecl::new(b"1.0", Some(b"UTF-8"), None)))?;
    writer.write_event(Event::Start(rss_elem))?;
    writer.write_event(Event::Start(BytesStart::owned_name("channel")))?;
    writer.write_event(Event::Start(BytesStart::owned_name("title")))?;
    writer.write_event(Event::Text(BytesText::from_plain(
      self.title.clone().into_bytes().as_slice(),
    )))?;
    writer.write_event(Event::End(BytesEnd::borrowed(b"title")))?;
    /*
        writer.write_event(Event::Start(BytesStart::owned_name("link")))?;
        writer.write_event(Event::Text(BytesText::from_plain(self.url.clone().into_string().into_bytes().as_slice())))?;
        writer.write_event(Event::End(BytesEnd::borrowed(b"link")))?;
    */
    match self.description {
      Some(ref desc) => {
        writer.write_event(Event::Start(BytesStart::owned_name("description")))?;
        writer.write_event(Event::Text(BytesText::from_plain(
          desc.clone().into_bytes().as_slice(),
        )))?;
        writer.write_event(Event::End(BytesEnd::borrowed(b"description")))?;
      }
      _ => (),
    };

    for entry in iter {
      entry.render_to_atom(&mut writer)?;
    }

    writer.write_event(Event::End(BytesEnd::borrowed(b"channel")))?;
    writer.write_event(Event::End(BytesEnd::borrowed(b"rss")))?;
    Ok(())
  }
}

/// Retrieve the feed by primary key.
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

/// Read the feed with the given, exact name into memory.
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

/// Read all feeds into memory.
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

/// AddEntry command.
pub struct AddEntry {
  /// The primary key of the feed.
  pub feed_id: i64,

  /// The guid of the entry.
  pub guid: String,

  /// The title of the entry.
  pub title: String,

  /// The url of the content (likely audio).
  pub content_url: Url,

  /// The content type of the content (unreliable).
  pub content_type: String,

  /// The summary of the content.
  pub summary: Option<String>,

  /// The published date.
  pub published: Option<DateTime<Utc>>,

  /// The last-modified date.
  pub updated: Option<DateTime<Utc>>,
}

/// Execute the AddEntry command.
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

/// The Feed Entry model.
#[derive(Debug)]
pub struct Entry {
  /// The id of the entry row.
  pub id: i64,

  /// The id of the parent feed row.
  pub feed_id: i64,

  /// The GUID of the entry.
  pub guid: String,

  /// Title of the entry.
  pub title: String,

  /// URL of the content (audio) of the feed entry.
  pub content_url: Url,

  /// Content type of the feed entry (unreliable).
  pub content_type: String,

  /// Summary of the entrie's contents.
  pub summary: Option<String>,

  /// Published date (mostly reliable)
  pub published: Option<DateTime<Utc>>,

  /// Last updated date (must)
  pub updated: Option<DateTime<Utc>>,
}
impl Entry {
  fn render_to_atom<W: std::io::Write>(&self, writer: &mut Writer<W>) -> anyhow::Result<()> {
    writer.write_event(Event::Start(BytesStart::owned_name("item")))?;

    writer.write_event(Event::Start(BytesStart::owned_name("guid")))?;
    writer.write_event(Event::Text(BytesText::from_plain(
      self.guid.clone().into_bytes().as_slice(),
    )))?;
    writer.write_event(Event::End(BytesEnd::borrowed(b"guid")))?;

    writer.write_event(Event::Start(BytesStart::owned_name("title")))?;
    writer.write_event(Event::Text(BytesText::from_plain(
      self.title.clone().into_bytes().as_slice(),
    )))?;
    writer.write_event(Event::End(BytesEnd::borrowed(b"title")))?;

    match self.summary {
      None => (),
      Some(ref summary) => {
        writer.write_event(Event::Start(BytesStart::owned_name("description")))?;
        writer.write_event(Event::Text(BytesText::from_plain(
          summary.clone().into_bytes().as_slice(),
        )))?;
        writer.write_event(Event::End(BytesEnd::borrowed(b"description")))?;
      }
    }

    match self.published {
      None => (),
      Some(ref published) => {
        writer.write_event(Event::Start(BytesStart::owned_name("pubDate")))?;
        writer.write_event(Event::Text(BytesText::from_plain(
          published.to_rfc3339().clone().into_bytes().as_slice(),
        )))?;
        writer.write_event(Event::End(BytesEnd::borrowed(b"pubDate")))?;
      }
    }

    let mut enclosure_elem = BytesStart::owned_name("enclosure");
    enclosure_elem.push_attribute(("url", self.content_url.clone().to_string().as_str()));
    writer.write_event(Event::Start(enclosure_elem))?;
    writer.write_event(Event::End(BytesEnd::borrowed(b"enclosure")))?;

    writer.write_event(Event::End(BytesEnd::borrowed(b"item")))?;
    Ok(())
  }
}

/// Read all entires into memory that belong to feed indicated by `feed_id`.
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
       FROM entries
       WHERE feed_id = ?;",
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
    .map(|x| x.unwrap())
    .collect()
}

/// Read an entry into memory by guid.
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

/// Read all entries published since given date time into memory.
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
