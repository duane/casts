use anyhow::Result;
use casts::{
    aggregate::{Aggregate, FeedAggregate},
    applicable::EventApplicable,
    commands::{Command, FeedAggregateCommand},
    events::FeedAggregateEvent,
};
use chrono::Utc;
use commit_store::grpc_to_domain_commit;
use commit_store::{
    commit::{CommitMetadata, StreamMetadata},
    domain_to_grpc_commit_attempt,
    grpc_interface::{commit_store_client::CommitStoreClient, Commits, StreamCursor},
    CommitAttempt,
};
use immutable_feed::fetch_feed;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();
    let stream_id =
        Uuid::parse_str(&std::env::args().nth(1).expect("USAGE: read STREAM_ID")).unwrap();
    let min_version = 0;
    let max_version = i64::MAX;

    let mut grpc_client: CommitStoreClient<tonic::transport::Channel> =
        CommitStoreClient::connect("http://127.0.0.1:7392")
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
    for (feed_id, existing_feed) in aggregate.feeds_by_feed_id.clone().into_iter() {
        let observed_feed = fetch_feed(&existing_feed.url.clone().expect("requires url")).await?;
        let diff = observed_feed.diff(&existing_feed);
        println!("found {} changes", diff.len());
        let observed_feed_cmd = FeedAggregateCommand::ObserveFeed {
            stream_id,
            feed_id,
            feed: observed_feed,
        };
        let events = observed_feed_cmd
            .async_apply(&aggregate)
            .await
            .expect("Failed to apply feed observation");
        let events_count = events.len() as i64;
        if events_count == 0 {
            continue;
        }
        let commit_attempt = CommitAttempt {
            stream_metadata: StreamMetadata {
                stream_id,
                stream_version: aggregate.version() + 1,
            },
            commit_metadata: CommitMetadata {
                commit_id: Uuid::new_v4(),
                commit_sequence: aggregate.version() + 1,
                commit_timestamp: Utc::now(),
            },
            metadata: serde_json::to_value(observed_feed_cmd).unwrap(),
            events: serde_json::to_value(events.clone()).unwrap(),
            events_count,
        };
        let commit_result = grpc_to_domain_commit(
            grpc_client
                .append_commit(domain_to_grpc_commit_attempt(&commit_attempt))
                .await
                .unwrap()
                .get_ref(),
        );
        let committed_events: Vec<FeedAggregateEvent> =
            serde_json::from_value(commit_result.events).unwrap();
        aggregate = aggregate.apply_many(committed_events);
    }
    Ok(())
}
