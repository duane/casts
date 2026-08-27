//! Measure where a feed aggregate's bytes actually are.
//!
//! Not part of the product: a one-off instrument for deciding what belongs in
//! the aggregate and what belongs in a projection. Run with
//! `cargo run --example aggregate_size -- <feed-url>...`, or with no
//! arguments to use a few feeds from `feed_urls.txt`.

use immutable_feed::{fetch_feed, Entry};
use serde::Serialize;

/// The fields a command actually needs to make a decision: does this guid
/// exist (dedup), when was it published (ordering), and enough to name it.
/// Deliberately excludes `summary`, `content`, `links` and `media`.
#[derive(Serialize)]
struct SlimEntry<'a> {
    guid: &'a str,
    published_at: Option<chrono::DateTime<chrono::Utc>>,
    // Kept because the archival worker needs a URL to fetch.
    url: String,
    title: Option<&'a str>,
}

fn slim(entry: &Entry) -> SlimEntry<'_> {
    SlimEntry {
        guid: &entry.guid,
        published_at: entry.published_at,
        url: entry.url.to_string(),
        title: entry.title.as_deref(),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let urls: Vec<String> = if args.is_empty() {
        std::fs::read_to_string("../feed_urls.txt")
            .or_else(|_| std::fs::read_to_string("feed_urls.txt"))?
            .lines()
            .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
            .filter_map(|l| l.split_once('\t').map(|(_, u)| u.trim().to_string()))
            .take(8)
            .collect()
    } else {
        args
    };

    let mut total_full = 0usize;
    let mut total_slim = 0usize;
    let mut total_entries = 0usize;

    println!(
        "{:<44} {:>7} {:>12} {:>12} {:>7}",
        "feed", "entries", "full", "slim", "ratio"
    );
    for url in &urls {
        let parsed = match url::Url::parse(url) {
            Ok(u) => u,
            Err(_) => continue,
        };
        let feed = match fetch_feed(&parsed).await {
            Ok(feed) => feed,
            Err(err) => {
                eprintln!("skipping {url}: {err}");
                continue;
            }
        };
        let full = serde_json::to_vec(&feed.entries)?.len();
        let slim_entries: Vec<SlimEntry> = feed.entries.iter().map(slim).collect();
        let slim_len = serde_json::to_vec(&slim_entries)?.len();
        let title: String = feed.metadata.title.chars().take(42).collect();
        println!(
            "{:<44} {:>7} {:>12} {:>12} {:>6.1}x",
            title,
            feed.entries.len(),
            full,
            slim_len,
            full as f64 / slim_len.max(1) as f64
        );
        total_full += full;
        total_slim += slim_len;
        total_entries += feed.entries.len();
    }

    println!(
        "\n{total_entries} entries: full={:.2} MB  slim={:.2} MB  ({:.1}x smaller)",
        total_full as f64 / 1_048_576.0,
        total_slim as f64 / 1_048_576.0,
        total_full as f64 / total_slim.max(1) as f64
    );
    println!(
        "extrapolated to 106 feeds at this average: full={:.1} MB  slim={:.1} MB",
        total_full as f64 / urls.len().max(1) as f64 * 106.0 / 1_048_576.0,
        total_slim as f64 / urls.len().max(1) as f64 * 106.0 / 1_048_576.0,
    );
    Ok(())
}
