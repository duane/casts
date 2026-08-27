// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

//! Cursor paging over a stream's entries.
//!
//! `ListEntries` used to return every entry of every feed in one message.
//! That is unbounded in the size of the library: after a 1000-feed import the
//! equivalent `ListFeeds` response reached 78 MB, which is past what any
//! client will accept and past commit-store's own limits. This module gives
//! the read path a total order and an opaque cursor over it, so a caller
//! walks the library in bounded pages instead.
//!
//! Kept free of the generated gRPC types on purpose: the ordering and the
//! cursor round-trip are the parts worth testing, and they are pure.

use anyhow::{anyhow, Context, Result};
use immutable_feed::Entry;
use uuid::Uuid;

/// Page size used when the caller names none.
pub const DEFAULT_PAGE_SIZE: usize = 100;

/// Largest page the server will build, whatever the caller asks for. A page
/// is clamped to this, never truncated: the response still carries a
/// `next_page_token`, so a caller that asks for more than this gets all of
/// it, just across more round trips.
pub const MAX_PAGE_SIZE: usize = 500;

/// The position of one entry in the total order, and so also the cursor that
/// resumes after it.
///
/// Field order is the sort order, which is what `derive(Ord)` gives us:
///
/// * `dated` first, so entries with no publication date form one block at
///   the front rather than scattering through the results. They have no
///   position in a date ordering, and a page boundary that fell inside a set
///   of mutually incomparable entries could show one twice or skip it.
/// * `neg_published_*`, negated so that ascending order on the tuple is
///   newest-first -- the order a podcast client wants.
/// * `feed_id` then `guid` to break ties. Without these the order is only
///   partial: two entries published in the same second would sort
///   arbitrarily, and a cursor pointing at one of them could not say which.
///   A guid is unique only within its feed, hence both.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct EntryCursor {
    dated: u8,
    neg_published_seconds: i64,
    neg_published_nanos: i64,
    feed_id: Uuid,
    guid: String,
}

impl EntryCursor {
    /// The cursor identifying `entry` within `feed_id`.
    pub fn of(feed_id: Uuid, entry: &Entry) -> Self {
        let (dated, neg_published_seconds, neg_published_nanos) = match entry.published_at {
            Some(published) => (
                1,
                -published.timestamp(),
                -i64::from(published.timestamp_subsec_nanos()),
            ),
            None => (0, 0, 0),
        };
        EntryCursor {
            dated,
            neg_published_seconds,
            neg_published_nanos,
            feed_id,
            guid: entry.guid.clone(),
        }
    }

    /// Encode as the opaque `page_token` handed back to clients.
    ///
    /// Hex-encoded so that it is opaque in fact and not merely by
    /// convention: a token that looked like a readable tuple would invite
    /// clients to build their own, and the ordering it encodes is this
    /// server's to change.
    pub fn encode(&self) -> String {
        // `guid` is last and takes the remainder of the string, so it may
        // contain the separator; nothing before it can.
        let plain = format!(
            "{}|{}|{}|{}|{}",
            self.dated,
            self.neg_published_seconds,
            self.neg_published_nanos,
            self.feed_id,
            self.guid
        );
        plain.bytes().map(|b| format!("{b:02x}")).collect()
    }

    /// Decode a `page_token` produced by `encode`.
    pub fn decode(token: &str) -> Result<Self> {
        if !token.len().is_multiple_of(2) {
            return Err(anyhow!("page token is not valid hex"));
        }
        let bytes: Result<Vec<u8>> = (0..token.len())
            .step_by(2)
            .map(|i| {
                u8::from_str_radix(&token[i..i + 2], 16).context("page token is not valid hex")
            })
            .collect();
        let plain = String::from_utf8(bytes?).context("page token is not valid UTF-8")?;

        let mut parts = plain.splitn(5, '|');
        let mut next = |what: &str| -> Result<String> {
            parts
                .next()
                .map(str::to_owned)
                .ok_or_else(|| anyhow!("page token is missing its {what}"))
        };
        let dated: u8 = next("dated flag")?
            .parse()
            .context("page token dated flag")?;
        let neg_published_seconds: i64 = next("seconds")?.parse().context("page token seconds")?;
        let neg_published_nanos: i64 = next("nanos")?.parse().context("page token nanos")?;
        let feed_id: Uuid = next("feed id")?.parse().context("page token feed id")?;
        let guid = next("guid")?;

        Ok(EntryCursor {
            dated,
            neg_published_seconds,
            neg_published_nanos,
            feed_id,
            guid,
        })
    }
}

/// One page of entries, and where to resume.
pub struct EntryPage {
    /// The entries themselves, in the total order described on `EntryCursor`.
    pub entries: Vec<(Uuid, Entry)>,
    /// The token that resumes after the last entry here, or `None` when this
    /// page is the last one.
    pub next_page_token: Option<String>,
}

/// Clamp a caller's requested page size.
///
/// Zero means "no preference". This clamps rather than rejects an oversized
/// request because the result is still complete: the caller gets every entry,
/// across more pages than it asked for.
pub fn page_size(requested: i32) -> Result<usize> {
    if requested < 0 {
        return Err(anyhow!("limit must not be negative"));
    }
    if requested == 0 {
        return Ok(DEFAULT_PAGE_SIZE);
    }
    Ok((requested as usize).min(MAX_PAGE_SIZE))
}

/// Select one page from every `(feed_id, entry)` pair in the stream.
///
/// `after` is the previous page's cursor; `None` starts at the beginning.
/// The returned `next_page_token` is `Some` exactly when entries remain, so a
/// caller can tell "this is the whole tail" from "there is more" without
/// guessing from the page's length.
pub fn select_page(
    mut all: Vec<(Uuid, Entry)>,
    after: Option<&EntryCursor>,
    limit: usize,
) -> EntryPage {
    all.sort_by_cached_key(|(feed_id, entry)| EntryCursor::of(*feed_id, entry));

    let start = match after {
        // Everything at or before the cursor has already been sent. The
        // entries are sorted, so this is the first one strictly after it.
        Some(cursor) => {
            all.partition_point(|(feed_id, entry)| &EntryCursor::of(*feed_id, entry) <= cursor)
        }
        None => 0,
    };

    // Take one more than the page to learn whether a next page exists,
    // rather than inferring it from a full page and handing back a token
    // that yields nothing.
    let mut window: Vec<(Uuid, Entry)> = all.into_iter().skip(start).take(limit + 1).collect();
    let has_more = window.len() > limit;
    if has_more {
        window.pop();
    }

    let next_page_token = if has_more {
        window
            .last()
            .map(|(feed_id, entry)| EntryCursor::of(*feed_id, entry).encode())
    } else {
        None
    };

    EntryPage {
        entries: window,
        next_page_token,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, TimeZone, Utc};
    use url::Url;

    fn entry(guid: &str, published: Option<DateTime<Utc>>) -> Entry {
        Entry {
            guid: String::from(guid),
            url: Url::parse("http://example.com/episode.mp3").unwrap(),
            title: Some(String::from(guid)),
            summary: None,
            updated_at: None,
            published_at: published,
            content: None,
            links: vec![],
            media: vec![],
        }
    }

    fn at(seconds: i64) -> Option<DateTime<Utc>> {
        Some(Utc.timestamp_opt(seconds, 0).unwrap())
    }

    /// Walk every page and return the guids in the order a client would see
    /// them, plus how many round trips it took.
    fn walk(all: Vec<(Uuid, Entry)>, limit: usize) -> (Vec<String>, usize) {
        let mut seen = vec![];
        let mut token: Option<String> = None;
        let mut pages = 0;
        loop {
            let cursor = token.as_deref().map(|t| EntryCursor::decode(t).unwrap());
            let page = select_page(all.clone(), cursor.as_ref(), limit);
            pages += 1;
            seen.extend(page.entries.iter().map(|(_, e)| e.guid.clone()));
            match page.next_page_token {
                Some(next) => token = Some(next),
                None => break,
            }
            assert!(pages < 100, "paging failed to terminate");
        }
        (seen, pages)
    }

    #[test]
    fn cursor_round_trips_through_its_token() {
        let feed_id = Uuid::new_v4();
        // A guid holding the separator, non-ASCII text, and characters that
        // a naive URL encoding would mangle: podcast guids are arbitrary.
        let awkward = entry("a|b c/d?e=f&g#h\u{e9}\u{1f600}", at(1_700_000_000));
        let cursor = EntryCursor::of(feed_id, &awkward);
        assert_eq!(EntryCursor::decode(&cursor.encode()).unwrap(), cursor);

        let undated = EntryCursor::of(feed_id, &entry("plain", None));
        assert_eq!(EntryCursor::decode(&undated.encode()).unwrap(), undated);
    }

    #[test]
    fn malformed_tokens_are_rejected_not_misread() {
        // Odd length, non-hex, valid hex that is not UTF-8, and valid UTF-8
        // with too few fields. None of these may decode to a usable cursor.
        for bad in ["abc", "zz", "ff", &"41".repeat(3)] {
            assert!(
                EntryCursor::decode(bad).is_err(),
                "token {bad:?} should not decode"
            );
        }
    }

    #[test]
    fn newest_first_with_undated_entries_leading() {
        let feed_id = Uuid::new_v4();
        let all = vec![
            (feed_id, entry("old", at(1_000))),
            (feed_id, entry("newest", at(3_000))),
            (feed_id, entry("undated", None)),
            (feed_id, entry("middle", at(2_000))),
        ];
        let page = select_page(all, None, 10);
        let guids: Vec<String> = page.entries.iter().map(|(_, e)| e.guid.clone()).collect();
        assert_eq!(guids, vec!["undated", "newest", "middle", "old"]);
        assert!(page.next_page_token.is_none());
    }

    #[test]
    fn paging_visits_every_entry_exactly_once() {
        let feed_a = Uuid::new_v4();
        let feed_b = Uuid::new_v4();
        let mut all = vec![];
        for i in 0..25 {
            all.push((feed_a, entry(&format!("a-{i}"), at(i))));
            all.push((feed_b, entry(&format!("b-{i}"), at(i))));
        }
        let expected: Vec<String> = select_page(all.clone(), None, 1000)
            .entries
            .iter()
            .map(|(_, e)| e.guid.clone())
            .collect();
        assert_eq!(expected.len(), 50);

        for limit in [1, 3, 7, 49, 50, 51] {
            let (seen, _) = walk(all.clone(), limit);
            assert_eq!(seen, expected, "paging by {limit} lost or repeated entries");
        }
    }

    /// Entries sharing a publication timestamp are the case a
    /// published-date-only cursor cannot express: it can say "resume after
    /// this second" but not which of that second's entries were already
    /// sent, so it must either repeat them or skip them.
    #[test]
    fn entries_with_identical_timestamps_still_page_cleanly() {
        let feed_a = Uuid::new_v4();
        let feed_b = Uuid::new_v4();
        let mut all = vec![];
        for i in 0..5 {
            all.push((feed_a, entry(&format!("a-{i}"), at(500))));
            all.push((feed_b, entry(&format!("b-{i}"), at(500))));
        }
        let (seen, pages) = walk(all, 3);
        let mut sorted = seen.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 10, "an entry was dropped or duplicated");
        assert_eq!(seen.len(), 10);
        assert_eq!(pages, 4);
    }

    /// A page that exactly empties the list must not hand back a token: a
    /// client following it would make a round trip for nothing, and could
    /// not tell that from a page that happens to be empty right now.
    #[test]
    fn an_exactly_full_last_page_reports_no_next_page() {
        let feed_id = Uuid::new_v4();
        let all: Vec<(Uuid, Entry)> = (0..4)
            .map(|i| (feed_id, entry(&format!("e-{i}"), at(i))))
            .collect();
        let page = select_page(all, None, 4);
        assert_eq!(page.entries.len(), 4);
        assert!(page.next_page_token.is_none());
    }

    #[test]
    fn empty_stream_pages_to_nothing() {
        let page = select_page(vec![], None, 10);
        assert!(page.entries.is_empty());
        assert!(page.next_page_token.is_none());
    }

    #[test]
    fn page_size_defaults_clamps_and_rejects() {
        assert_eq!(page_size(0).unwrap(), DEFAULT_PAGE_SIZE);
        assert_eq!(page_size(10).unwrap(), 10);
        // Clamped, not refused: the caller still receives everything, in
        // more pages than it asked for.
        assert_eq!(page_size(10_000).unwrap(), MAX_PAGE_SIZE);
        assert!(page_size(-1).is_err());
    }
}
