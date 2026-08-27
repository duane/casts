//! Report the size distribution of one stream's commits.
//!
//! A one-off instrument, not part of the product. `Client::get_stream_range`
//! streams one message per commit, so what matters for the 4 MB default
//! decode limit is the size of the LARGEST single commit, not the total.

use commit_store::grpc_interface::StreamCursor;
use futures::StreamExt;
use prost::Message;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let stream_id = std::env::args()
        .nth(1)
        .unwrap_or_else(|| String::from("34c3c08d-0d55-4af4-a126-170c1380f07e"));

    // Deliberately generous, so this can measure commits that the default
    // limit would refuse to decode at all.
    let mut client = casts::commit_store_client::connect()
        .await?
        .max_decoding_message_size(256 * 1024 * 1024);

    let mut stream = client
        .stream_stream_range(StreamCursor {
            stream_id,
            min_version: 0,
            max_version: i64::MAX,
            limit: i64::MAX,
            reverse: false,
        })
        .await?
        .into_inner();

    let mut count = 0usize;
    let mut total = 0usize;
    let mut over_4mb = 0usize;
    let mut biggest: Vec<(i64, usize)> = Vec::new();
    while let Some(item) = stream.next().await {
        let commit = match item {
            Ok(commit) => commit,
            Err(status) => {
                eprintln!("stream ended early after {count} commits: {status}");
                break;
            }
        };
        let size = (&commit as &dyn Message).encoded_len();
        count += 1;
        total += size;
        if size > 4 * 1024 * 1024 {
            over_4mb += 1;
        }
        biggest.push((commit.stream_version, size));
        biggest.sort_by_key(|(_, s)| std::cmp::Reverse(*s));
        biggest.truncate(5);
    }

    println!("commits: {count}");
    println!("total:   {:.2} MB", total as f64 / 1_048_576.0);
    println!("over 4 MB (tonic's default decode limit): {over_4mb}");
    println!("largest commits (version, bytes):");
    for (version, size) in &biggest {
        println!(
            "  v{version:<6} {size:>12}  ({:.2} MB)",
            *size as f64 / 1_048_576.0
        );
    }
    Ok(())
}
