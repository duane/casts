use anyhow::Result;
use casts::{
    aggregate::FeedAggregate,
    applicable::EventApplicable,
    commands::{Command, FeedAggregateCommand},
};
use commit_store::{
    commit::{CommitMetadata, StreamMetadata},
    domain_to_grpc_commit_attempt,
    grpc_interface::{commit_store_client::CommitStoreClient, Commits, StreamCursor},
    grpc_to_domain_commit, CommitAttempt,
};
use uuid::{uuid, Uuid};

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    let args = std::env::args().collect::<Vec<String>>();
    let stream_id = uuid!["34c3c08d-0d55-4af4-a126-170c1380f07e"];
    if args.len() != 3 || args[1] != "add" {
        println!("USAGE: casts add FEED_URL");
        std::process::exit(1);
    }

    let min_version = 0;
    let max_version = i64::MAX;

    let mut grpc_client = CommitStoreClient::connect("http://127.0.0.1:7392")
        .await
        .expect("Failed to connect to backend commit store");

    let commits: Commits = grpc_client
        .get_stream_range(StreamCursor {
            stream_id: stream_id.to_string(),
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

    let add_feed_cmd = FeedAggregateCommand::AddFeed {
        stream_id,
        feed_id: Uuid::new_v4(),
        url: url::Url::parse(&args[2]).expect(&format!("Not a valid url: {:?}", args[1])),
    };

    let events = add_feed_cmd
        .async_apply(&aggregate)
        .await
        .expect("Failed to apply feed observation");
    let events_count = events.len() as i64;
    let commit_attempt = CommitAttempt {
        stream_metadata: StreamMetadata {
            stream_id,
            stream_version: aggregate.version + 1,
        },
        commit_metadata: CommitMetadata {
            commit_id: Uuid::new_v4(),
            commit_sequence: aggregate.version + 1,
            commit_timestamp: chrono::Utc::now(),
        },
        metadata: serde_json::to_value(&add_feed_cmd).unwrap(),
        events: serde_json::to_value(events).unwrap(),
        events_count,
    };

    let commit_result = grpc_client
        .append_commit(domain_to_grpc_commit_attempt(&commit_attempt))
        .await
        .expect("Failed to append commit");
    let commit = grpc_to_domain_commit(commit_result.get_ref());
    println!("commit: {:?}", commit);
    Ok(())
}
