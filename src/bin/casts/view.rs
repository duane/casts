use crate::model::Entry;
use crate::model::Feed;
use html2text::from_read;

pub fn show_feed(feed: &Feed, verbose: bool) {
  if verbose {
    println!("{}", feed.title);
    println!("{}", feed.url.to_string());
    println!(
      "{}",
      feed
        .description
        .clone()
        .unwrap_or("No description.".to_string())
    );
  } else {
    println!("{}", feed.title)
  }
}

pub fn show_feed_and_entry(feed: &Feed, entry: &Entry, verbose: bool) {
  print!("{}\t", feed.title);
  show_entry(feed, entry, verbose);
}

pub fn show_entry(feed: &Feed, entry: &Entry, verbose: bool) {
  print!("{}\t", &entry.title);
  if super::cache::cache_location(&feed, &entry).exists() {
    println!("downloaded");
  } else {
    println!("not downloaded");
  }
  if verbose {
    println!("{}", entry_summary(&entry));
  }
}

pub fn entry_summary(entry: &Entry) -> String {
  let bytes = match entry.summary.as_ref() {
    Some(summary) => summary.clone(),
    None => "No summary".to_string(),
  }
  .into_bytes();
  from_read(bytes.as_slice(), 72)
}
