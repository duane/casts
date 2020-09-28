use rusqlite::Connection;
use url::Url;

pub fn open() -> Connection {
    let xdg_dirs = xdg::BaseDirectories::with_prefix("casts").unwrap();
    let db_path = xdg_dirs
        .place_data_file("casts.sqlite")
        .expect("could not create data directory");
    Connection::open(db_path).expect("could not open casts.sqlite")
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
    );",
    )
    .expect("could not intialize sqlite feeds table");
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS entries (
      id INTEGER NOT NULL PRIMARY KEY,
      feed_id INTEGER NOT NULL,
      title TEXT,
      title_content_type TEXT,
      summary TEXT,
      summary_content_type TEXT,
      content_url TEXT,
      content_content_type TEXT,
      FOREIGN KEY (feed_id) REFERENCES feeds(id)
    );",
    )
    .expect("could not intialize sqlite feeds table");
}

pub struct AddFeed {
    pub feed: Url,
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
        .execute(&[
            &Some(add_feed.feed.clone().into_string()),
            &add_feed.title,
            &add_feed.title_content_type,
            &add_feed.description,
            &add_feed.description_content_type,
        ])
        .unwrap()
}

pub struct AddEntry {
    pub feed_id: usize,
    pub title: Option<String>,
    pub title_content_type: Option<String>,
    pub summary: Option<String>,
    pub summary_content_type: Option<String>,
    pub content_url: Option<Url>,
    pub content_content_type: Option<String>,
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
    content_content_type
  ) VALUES (?, ?, ?, ?, ?, ?, ?);",
        )
        .unwrap();
    statement
        .execute(&[
            &Some(add_entry.feed_id.to_string()),
            &add_entry.title,
            &add_entry.title_content_type,
            &add_entry.summary,
            &add_entry.summary_content_type,
            &add_entry
                .content_url
                .as_ref()
                .map(|u| u.clone().into_string()),
            &add_entry.content_content_type,
        ])
        .unwrap()
}
