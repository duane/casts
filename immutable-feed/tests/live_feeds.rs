//! Live-network checks against real feeds, `#[ignore]` by default (run with
//! `cargo test -p immutable-feed --test live_feeds -- --ignored`).
//!
//! Offline regression coverage lives in `atom.rs`, against captured copies of
//! these same documents. These exist to catch the case that fixtures cannot:
//! a publisher changing what it emits. Every real bug found in this parser so
//! far -- durations 60x short, Atom enclosures unplayable, a panic on
//! link-less entries -- came from running real feeds through it.
#![cfg(test)]

use immutable_feed::fetch_feed;
use url::Url;

async fn probe(raw: &str) {
    let url = Url::parse(raw).unwrap();
    let feed = fetch_feed(&url)
        .await
        .unwrap_or_else(|e| panic!("{raw}: {e}"));
    println!(
        "OK {raw}\n   title={:?} entries={} authors={:?}",
        feed.metadata.title,
        feed.entries.len(),
        feed.metadata.authors
    );
    assert!(!feed.metadata.title.is_empty(), "{raw}: no feed title");
    assert!(!feed.entries.is_empty(), "{raw}: no entries");
    for e in feed.entries.iter().take(2) {
        println!(
            "   entry title={:?} guid={:?} links={} media={}",
            e.title.as_deref().unwrap_or("-"),
            &e.guid[..e.guid.len().min(24)],
            e.links.len(),
            e.media.len()
        );
    }
}

#[tokio::test]
#[ignore = "hits the network"]
async fn atom_xkcd() {
    probe("https://xkcd.com/atom.xml").await;
}

#[tokio::test]
#[ignore = "hits the network"]
async fn atom_rust_blog() {
    probe("https://blog.rust-lang.org/feed.xml").await;
}

#[tokio::test]
#[ignore = "hits the network"]
async fn atom_verge() {
    probe("https://www.theverge.com/rss/index.xml").await;
}

#[tokio::test]
#[ignore = "hits the network"]
async fn rss_podcast_history_of_rome() {
    probe("https://historyofrome.libsyn.com/rss").await;
}

#[tokio::test]
#[ignore = "hits the network"]
async fn rss_podcast_majority_report() {
    probe("https://feeds.feedburner.com/MajorityReport").await;
}
