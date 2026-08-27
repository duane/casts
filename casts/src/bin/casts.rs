use anyhow::Result;
use casts::{
    aggregate::FeedAggregate,
    applicable::EventApplicable,
    commands::{Command, FeedAggregateCommand},
    config::{ClientConfig, ClientConfigBuilder, ServerConfig, ServerConfigBuilder},
};
use chrono::Utc;
use commit_store::{
    commit::{CommitMetadata, StreamMetadata},
    domain_to_grpc_commit_attempt,
    grpc_interface::{commit_store_client::CommitStoreClient, Commits, StreamCursor},
    grpc_to_domain_commit, CommitAttempt,
};
use immutable_feed::fetch_feed;
use prost::Message;
use tonic::transport::Channel;
use url::Url;
use uuid::Uuid;

use futures::StreamExt;

fn usage() {
    println!("USAGE: casts add FEED_URL");
    println!("USAGE: casts show FEED_ID");
    println!("USAGE: casts mark-listened FEED_ID ENTRY_GUID ELAPSED_SECONDS");
    println!("USAGE: casts import-feeds FILE [LIMIT]");
    println!("    FILE: lines of \"<title>\\t<feed url>\"; '#'-prefixed lines are comments.");
    println!("    Each feed is added with do_not_archive set. LIMIT caps how many");
    println!("    lines are imported (for trying it against one entry first).");
    std::process::exit(1);
}

async fn mark_listened(
    mut client: CommitStoreClient<tonic::transport::Channel>,
    stream_id: Uuid,
    feed_id: Uuid,
    entry_guid: String,
    elapsed_seconds: i64,
) -> Result<()> {
    let aggregate = hydrated_client(&mut client, &stream_id).await?;
    let command = FeedAggregateCommand::MarkEntryListened {
        stream_id,
        feed_id,
        entry_guid,
        elapsed_seconds,
        elapsed_nanos: 0,
    };
    let events = command.async_apply(&aggregate).await?;
    if events.is_empty() {
        println!("position unchanged; nothing committed");
        return Ok(());
    }
    let events_count = events.len() as i64;
    let commit_attempt = CommitAttempt {
        stream_metadata: StreamMetadata {
            stream_id,
            stream_version: aggregate.version + 1,
        },
        commit_metadata: CommitMetadata {
            commit_id: Uuid::new_v4(),
            commit_sequence: aggregate.version + 1,
            commit_timestamp: Utc::now(),
        },
        metadata: serde_json::to_value(&command)?,
        events: serde_json::to_value(events)?,
        events_count,
        snapshot: false,
    };
    let commit_result = client
        .append_commit(domain_to_grpc_commit_attempt(&commit_attempt))
        .await?;
    let commit = grpc_to_domain_commit(commit_result.get_ref());
    println!("commit: {:?}", commit.commit_metadata.commit_id);
    Ok(())
}

/// Connect to the commit store named by `COMMIT_STORE_ADDR` (default
/// http://localhost:7392), like every other client binary.
pub async fn get_client() -> CommitStoreClient<Channel> {
    casts::commit_store_client::connect()
        .await
        .expect("Failed to connect to backend commit store")
}

/// Bulk-import feeds from a `<title>\t<feed url>` file (see `feed_urls.txt`
/// and `scripts/`), each with `do_not_archive: Some(true)` -- for loading a
/// large batch of feeds to browse without also queuing the archival worker
/// to start downloading all their audio.
///
/// Sequential, not concurrent: each iteration folds the previous commit's
/// events into `aggregate` before the next, the same way `poll`/`refresh_feeds`
/// thread state forward. That is enough for a standalone import run;
/// unlike `poller::commit_with_rebase`, there is no retry-on-conflict, since
/// this is not meant to run alongside another concurrent writer to the same
/// stream. One bad line (malformed, unreachable, already added) is skipped
/// with a message rather than aborting the whole import.
async fn import_feeds(
    mut client: CommitStoreClient<tonic::transport::Channel>,
    stream_id: &Uuid,
    path: &str,
    limit: Option<usize>,
) -> Result<()> {
    let mut aggregate = hydrated_client(&mut client, stream_id).await?;
    let contents = std::fs::read_to_string(path)?;

    let mut imported = 0usize;
    let mut skipped = 0usize;
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if limit.is_some_and(|limit| imported >= limit) {
            println!("reached limit of {}; stopping", limit.unwrap());
            break;
        }

        let Some((title, url_str)) = line.split_once('\t') else {
            eprintln!("skipping malformed line (expected <title>\\t<url>): {line}");
            skipped += 1;
            continue;
        };

        let feed_url = match Url::parse(url_str.trim()) {
            Ok(url) => url,
            Err(err) => {
                eprintln!("skipping {title:?}: invalid url {url_str:?}: {err}");
                skipped += 1;
                continue;
            }
        };

        // Fetch and parse before issuing any command; command application
        // is pure. A dead host or a feed that no longer parses must not
        // abort the rest of a 1000-entry import.
        let feed = match fetch_feed(&feed_url).await {
            Ok(feed) => feed,
            Err(err) => {
                eprintln!("skipping {title:?}: fetch failed: {err}");
                skipped += 1;
                continue;
            }
        };

        let entry_count = feed.entries.len();
        // `add_feed_paginated` splits a large feed's entries across many
        // commits (see poller.rs) -- a single-commit AddFeed blew
        // commit-store's 2 MB message limit for any long-running show
        // (Crime Junkie: 7.16 MB), which is exactly what this import first
        // surfaced.
        match casts::poller::add_feed_paginated(
            &mut client,
            stream_id,
            aggregate.clone(),
            Uuid::new_v4(),
            feed_url,
            feed,
            // The entire point of this tool: importing a large batch to
            // browse must not also queue all of it for the archival worker.
            Some(true),
        )
        .await
        {
            Ok(next) => {
                aggregate = next;
                imported += 1;
                println!("imported ({imported}): {title} ({entry_count} entries)");
            }
            Err(err) => {
                // Most commonly a rejected AddFeed (duplicate) -- expected
                // and fine on a second run over the same file, or after
                // `seed`.
                eprintln!("skipping {title:?}: {err:?}");
                skipped += 1;
            }
        }
    }

    println!("done: {imported} imported, {skipped} skipped");
    Ok(())
}

async fn add(
    mut client: CommitStoreClient<tonic::transport::Channel>,
    stream_id: &Uuid,
    feed_url: Url,
) -> Result<()> {
    let aggregate = hydrated_client(&mut client, stream_id).await?;
    // Fetch and parse before issuing any command; command application is pure.
    let feed = fetch_feed(&feed_url).await?;
    // Paginated (see poller.rs): a single-commit AddFeed blows commit-store's
    // 2 MB message limit for any feed with a large back-catalog.
    let aggregate = casts::poller::add_feed_paginated(
        &mut client,
        stream_id,
        aggregate,
        Uuid::new_v4(),
        feed_url,
        feed,
        // No CLI flag sets this yet -- always start archiving normally.
        None,
    )
    .await?;
    println!("added; stream now at version {}", aggregate.version);
    Ok(())
}

/// Re-fetch every feed in the stream and commit the observed changes.
///
/// Thin wrapper over `casts::poller::refresh_feeds`, which the background
/// archival worker runs on a timer. Exits non-zero if any feed failed, so a
/// scheduled run still reports trouble without losing the feeds that worked.
async fn poll(stream_id: Uuid) -> Result<()> {
    let mut client = get_client()
        .await
        .max_decoding_message_size(8 * 1024 * 1024);
    let report = casts::poller::refresh_feeds(&mut client, &stream_id).await?;
    println!(
        "{} changed, {} unchanged, {} failed",
        report.changed,
        report.unchanged,
        report.failures.len()
    );
    for (feed_id, reason) in &report.failures {
        eprintln!("  {feed_id}: {reason}");
    }
    if report.ok() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("{} feed(s) failed", report.failures.len()))
    }
}

async fn hydrated_client(
    grpc_client: &mut CommitStoreClient<tonic::transport::Channel>,
    stream_id: &Uuid,
) -> Result<FeedAggregate> {
    let mut commit_stream = grpc_client
        .stream_stream_range(StreamCursor {
            stream_id: stream_id.to_string(),
            min_version: 0,
            max_version: i64::MAX,
            limit: i64::MAX,
            reverse: false,
        })
        .await?
        .into_inner();

    let mut aggregate = FeedAggregate::default();
    while let Some(commit) = commit_stream.next().await {
        let commit = commit?;
        aggregate = aggregate.apply_many(serde_json::from_slice(commit.events.as_slice())?);
    }
    Ok(aggregate)
}

fn client_config() -> ClientConfig {
    ClientConfigBuilder::default().build().unwrap()
}

fn server_config() -> ServerConfig {
    ServerConfigBuilder::default().build().unwrap()
}

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    let stream_id = Uuid::parse_str("34c3c08d-0d55-4af4-a126-170c1380f07e")?;

    let args = std::env::args().collect::<Vec<String>>();
    let grpc_client = get_client().await;

    match args[1].as_str() {
        "server-config" => {
            println!("Server config: {:?}", server_config());
        }
        "client-config" => {
            let server_config = server_config();
            println!("Server config: {:?}", server_config);
            println!("Client config: {:?}", client_config());
            panic!("don't know what to do")
        }
        "add" => {
            if std::env::args().len() != 3 {
                usage();
            }
            add(grpc_client, &stream_id, Url::parse(&args[2])?).await?
        }
        "mark-listened" => {
            if args.len() != 5 {
                usage();
            }
            mark_listened(
                grpc_client,
                stream_id,
                Uuid::parse_str(&args[2])?,
                args[3].clone(),
                args[4].parse()?,
            )
            .await?
        }
        "list-commits-events-json" => {
            let mut grpc_client = get_client()
                .await
                .max_encoding_message_size(8 * 1024 * 1024);

            let response_result = grpc_client
                .get_stream_range(StreamCursor {
                    stream_id: stream_id.to_string(),
                    min_version: 0,
                    max_version: i64::MAX,
                    limit: i64::MAX,
                    reverse: false,
                })
                .await;

            let mut response: tonic::Response<Commits> = response_result.unwrap();
            let commits = response.get_mut().to_owned();

            println!("[");
            for commit in commits.commits.iter() {
                let domain_commit = grpc_to_domain_commit(commit);
                println!("{},", serde_json::to_string(&domain_commit.events).unwrap());
            }
            println!("]");
        }
        "list-commits" => {
            let mut grpc_client = get_client().await;

            println!("Fetching commits");
            let response_result = grpc_client
                .stream_stream_range(StreamCursor {
                    stream_id: stream_id.to_string(),
                    min_version: 0,
                    max_version: i64::MAX,
                    limit: i64::MAX,
                    reverse: false,
                })
                .await;

            let mut response = response_result.unwrap();
            while let Some(rpc_commit_result) = response.get_mut().next().await {
                let rpc_commit = rpc_commit_result.unwrap();
                let rpc_commit_as_msg: &dyn Message = &rpc_commit;
                let domain_commit = grpc_to_domain_commit(&rpc_commit);
                println!(
                    "{}\t{}\t{}\t{:-#}",
                    domain_commit.stream_metadata.stream_version,
                    domain_commit.commit_metadata.commit_id,
                    domain_commit.commit_metadata.commit_timestamp,
                    byte_unit::Byte::from_i64_with_unit(
                        rpc_commit_as_msg.encoded_len() as i64,
                        byte_unit::Unit::B
                    )
                    .unwrap()
                );
            }
        }
        "import-feeds" => {
            if args.len() < 3 || args.len() > 4 {
                usage();
            }
            let limit = match args.get(3) {
                Some(s) => Some(s.parse::<usize>()?),
                None => None,
            };
            import_feeds(grpc_client, &stream_id, &args[2], limit).await?
        }
        "poll" => {
            // `poll [STREAM_ID]` -- defaults to the built-in stream.
            let poll_stream_id = if args.len() >= 3 {
                Uuid::parse_str(&args[2])?
            } else {
                stream_id
            };
            poll(poll_stream_id).await?
        }
        _ => usage(),
    }

    Ok(())
}
