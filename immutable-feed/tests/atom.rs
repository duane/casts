//! Atom parsing, against real captured feeds and the edge cases they do not
//! happen to cover.
//!
//! The fixtures under `fixtures/` are real documents (trimmed to two entries)
//! from xkcd, the Rust blog, and The Verge, plus two hand-built files for
//! shapes those three do not exercise: a podcast published as Atom, and
//! entries missing a usable link. Live equivalents live in `live_feeds.rs`
//! behind `#[ignore]`; these run offline so CI has no network dependency.

use immutable_feed::parse_doc;
use url::Url;

fn parse(fixture: &str) -> immutable_feed::Feed {
    let doc = std::fs::read(format!("fixtures/{fixture}.xml"))
        .unwrap_or_else(|e| panic!("read fixtures/{fixture}.xml: {e}"));
    let url = Url::parse("http://example.com/feed.xml").unwrap();
    parse_doc(&url, &doc).unwrap_or_else(|e| panic!("parse {fixture}: {e}"))
}

#[test]
fn real_atom_feeds_parse_with_usable_entries() {
    for fixture in ["atom_xkcd", "atom_rust_blog", "atom_verge"] {
        let feed = parse(fixture);
        assert!(!feed.metadata.title.is_empty(), "{fixture}: no feed title");
        assert_eq!(feed.entries.len(), 2, "{fixture}: entry count");
        for entry in &feed.entries {
            assert!(!entry.guid.is_empty(), "{fixture}: entry without a guid");
            assert!(
                entry.title.is_some(),
                "{fixture}: entry without a title: {:?}",
                entry.guid
            );
        }
    }
}

/// feed-rs only builds media from an RSS `<enclosure>`; an Atom enclosure is
/// a plain link, so a podcast published as Atom used to arrive unplayable.
#[test]
fn atom_enclosures_become_playable_media_with_durations() {
    let feed = parse("atom_podcast");
    assert_eq!(feed.metadata.title, "Atomcast");
    assert_eq!(feed.entries.len(), 2);

    let media: Vec<_> = feed
        .entries
        .iter()
        .map(|e| e.media.first().expect("an Atom enclosure must yield media"))
        .collect();

    let urls: Vec<String> = media
        .iter()
        .map(|m| m.content[0].url.as_ref().unwrap().to_string())
        .collect();
    assert_eq!(
        urls,
        vec![
            "http://example.com/atomcast/1.mp3",
            "http://example.com/atomcast/2.mp3"
        ]
    );
    assert!(media
        .iter()
        .all(|m| m.content[0].content_type.as_deref() == Some("audio/mpeg")));

    // 55:40 and 1:02:03, not 55 and 1 seconds.
    let durations: Vec<Option<(u32, u32)>> = media.iter().map(|m| m.duration).collect();
    assert_eq!(durations, vec![Some((3340, 0)), Some((3723, 0))]);
}

/// Atom permits an entry with only an `<id>`. Parsing one used to panic and
/// take down whatever was fetching the feed.
#[test]
fn entries_without_links_fall_back_to_the_id_or_are_dropped() {
    let feed = parse("atom_entry_without_link");

    let titles: Vec<&str> = feed
        .entries
        .iter()
        .map(|e| e.title.as_deref().unwrap_or_default())
        .collect();
    assert_eq!(
        titles,
        vec!["Has a link", "Id is a URL, no link element"],
        "the entry whose id is not a URL has nowhere to point and is dropped"
    );

    // The surviving link-less entry points at its id.
    assert_eq!(feed.entries[1].url.as_str(), "http://example.com/sparse/2");
}
