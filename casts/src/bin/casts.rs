use anyhow::Result;
use casts::{
    aggregate::{self, FeedAggregate},
    applicable::EventApplicable,
    commands::{Command, FeedAggregateCommand},
};
use commit_store::{
    commit::{CommitMetadata, StreamMetadata},
    domain_to_grpc_commit_attempt,
    grpc_interface::{commit_store_client::CommitStoreClient, Commits, StreamCursor},
    grpc_to_domain_commit, CommitAttempt,
};
use tonic::transport::Channel;
use url::Url;
use uuid::{uuid, Uuid};

fn usage() {
    println!("USAGE: casts add FEED_URL");
    println!("USAGE: casts show FEED_ID");
    std::process::exit(1);
}

const STREAM_ID: Uuid = uuid!["34c3c08d-0d55-4af4-a126-170c1380f07e"];

async fn add(feed_url: Url) -> Result<()> {
    let (aggregate, mut grpc_client) = hydrated_client().await?;
    let add_feed_cmd = FeedAggregateCommand::AddFeed {
        stream_id: STREAM_ID,
        feed_id: Uuid::new_v4(),
        url: feed_url,
    };

    let events = add_feed_cmd.async_apply(&aggregate).await?;
    let events_count = events.len() as i64;
    let commit_attempt = CommitAttempt {
        stream_metadata: StreamMetadata {
            stream_id: STREAM_ID,
            stream_version: aggregate.version + 1,
        },
        commit_metadata: CommitMetadata {
            commit_id: Uuid::new_v4(),
            commit_sequence: aggregate.version + 1,
            commit_timestamp: chrono::Utc::now(),
        },
        metadata: serde_json::to_value(&add_feed_cmd)?,
        events: serde_json::to_value(events)?,
        events_count,
    };

    let commit_result = grpc_client
        .append_commit(domain_to_grpc_commit_attempt(&commit_attempt))
        .await?;
    let commit = grpc_to_domain_commit(commit_result.get_ref());
    println!("commit: {:?}", commit);
    Ok(())
}

async fn show(feed_id: Uuid) -> Result<()> {
    let (aggregate, _) = hydrated_client().await?;
    if let Some(feed) = aggregate.feeds_by_feed_id.get(&feed_id) {
        println!("{:?}", feed);
    }
    Ok(())
}

async fn hydrated_client() -> Result<(FeedAggregate, CommitStoreClient<Channel>)> {
    let min_version = 0;
    let max_version = i64::MAX;

    let mut grpc_client = CommitStoreClient::connect("http://127.0.0.1:7392")
        .await
        .expect("Failed to connect to backend commit store");

    let commits: Commits = grpc_client
        .get_stream_range(StreamCursor {
            stream_id: STREAM_ID.to_string(),
            min_version,
            max_version,
        })
        .await
        .unwrap()
        .get_mut()
        .to_owned();

    let mut aggregate = FeedAggregate::default();
    for commit in commits.commits.iter() {
        aggregate = aggregate.apply_many(serde_json::from_slice(commit.events.as_slice()).unwrap());
    }
    Ok((aggregate, grpc_client))
}

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    let args = std::env::args().collect::<Vec<String>>();
    println!("{:?}", args);
    if args.len() < 3 {
        usage();
    }

    match args[1].as_str() {
        "add" => add(Url::parse(&args[2])?).await?,
        "show" => show(Uuid::parse_str(&args[2])?).await?,
        _ => usage(),
    }

    Ok(())
}
