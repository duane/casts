//! Break down the size of one commit: where do the bytes actually go?
//!
//! Reads a narrow version range so it costs the server almost nothing,
//! unlike a whole-stream scan.

use commit_store::grpc_interface::StreamCursor;
use futures::StreamExt;
use serde_json::Value;

/// Serialized size of a JSON value.
fn len(value: &Value) -> usize {
    serde_json::to_string(value).map(|s| s.len()).unwrap_or(0)
}

/// The largest contributors inside one entry-like object.
fn field_sizes(value: &Value) -> Vec<(String, usize)> {
    let mut out = vec![];
    if let Some(map) = value.as_object() {
        for (k, v) in map {
            out.push((k.clone(), len(v)));
        }
    }
    out.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    out
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let stream_id = std::env::var("STREAM_ID")
        .unwrap_or_else(|_| String::from("34c3c08d-0d55-4af4-a126-170c1380f07e"));
    let version: i64 = std::env::args()
        .nth(1)
        .unwrap_or_else(|| String::from("68"))
        .parse()?;

    let mut client = casts::commit_store_client::connect()
        .await?
        .max_decoding_message_size(64 * 1024 * 1024);

    let mut stream = client
        .stream_stream_range(StreamCursor {
            stream_id,
            min_version: version,
            max_version: version,
            limit: 1,
            reverse: false,
        })
        .await?
        .into_inner();

    let Some(commit) = stream.next().await else {
        println!("no commit at version {version}");
        return Ok(());
    };
    let commit = commit?;

    let events: Value = serde_json::from_slice(&commit.events)?;
    let metadata: Value = serde_json::from_slice(&commit.metadata)?;
    let events_len = len(&events);
    let metadata_len = len(&metadata);

    println!("commit v{version}");
    println!(
        "  events:   {events_len:>10} bytes ({:.2} MB)",
        events_len as f64 / 1_048_576.0
    );
    println!(
        "  metadata: {metadata_len:>10} bytes ({:.2} MB)",
        metadata_len as f64 / 1_048_576.0
    );
    println!(
        "  TOTAL:    {:>10} bytes ({:.2} MB)",
        events_len + metadata_len,
        (events_len + metadata_len) as f64 / 1_048_576.0
    );

    println!("\n-- metadata top-level --");
    for (k, n) in field_sizes(&metadata).iter().take(4) {
        println!("  {k:<24} {n:>10}");
    }
    // The command wrapper: {"AddFeed": {...}} etc.
    if let Some((_, inner)) = metadata.as_object().and_then(|m| m.iter().next()) {
        println!("-- inside the command --");
        for (k, n) in field_sizes(inner).iter().take(6) {
            println!("  {k:<24} {n:>10}");
        }
    }

    // Events are a list; look inside the first event's payload.
    println!("\n-- first event --");
    if let Some(first) = events.as_array().and_then(|a| a.first()) {
        if let Some((name, inner)) = first.as_object().and_then(|m| m.iter().next()) {
            println!("  event type: {name}");
            for (k, n) in field_sizes(inner).iter().take(6) {
                println!("  {k:<24} {n:>10}");
            }
            // Per-entry breakdown: what makes ONE entry big?
            let entries = inner.get("entries").and_then(|e| e.as_array());
            if let Some(entries) = entries {
                println!("\n-- entries: {} in this commit --", entries.len());
                let total: usize = entries.iter().map(len).sum();
                println!(
                    "  total {total} bytes, mean {} bytes/entry",
                    total.checked_div(entries.len()).unwrap_or(0)
                );
                let mut sized: Vec<(usize, &Value)> = entries.iter().map(|e| (len(e), e)).collect();
                sized.sort_by_key(|(n, _)| std::cmp::Reverse(*n));
                if let Some((biggest, entry)) = sized.first() {
                    println!("  largest single entry: {biggest} bytes");
                    for (k, n) in field_sizes(entry).iter().take(6) {
                        println!("    {k:<22} {n:>10}");
                    }
                }
            }
        }
    }
    Ok(())
}
