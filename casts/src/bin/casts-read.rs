use casts::{aggregate::FeedAggregate, applicable::EventApplicable};
use commit_store::grpc_interface::{Commits, StreamCursor};

use commit_store::grpc_interface::commit_store_client::CommitStoreClient;
use serde_json::Value;
use uuid::Uuid;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    let stream_id = Uuid::parse_str(&std::env::args().nth(1).expect("USAGE: read STREAM_ID"))
        .unwrap()
        .to_string();
    let min_version = 0;
    let max_version = i64::MAX;

    // Connect to the commit store named by COMMIT_STORE_ADDR, like every
    // other client binary.
    let addr = commit_store::config::ClientConfigBuilder::default()
        .build()
        .addr;
    let endpoint =
        tonic::transport::Uri::try_from(addr).expect("COMMIT_STORE_ADDR is not a valid URI");
    let mut grpc_client = CommitStoreClient::connect(endpoint)
        .await
        .expect("failed to connect to the commit store");

    let commits: Commits = grpc_client
        .get_stream_range(StreamCursor {
            stream_id,
            min_version,
            max_version,
            limit: i64::MAX,
            reverse: false,
        })
        .await
        .unwrap()
        .get_mut()
        .to_owned();

    let mut aggregate = FeedAggregate::default();
    for commit in commits.commits.iter() {
        aggregate = aggregate.apply_many(serde_json::from_slice(commit.events.as_slice()).unwrap());
    }
    let json_aggregate: Value = serde_json::to_value(aggregate).unwrap();

    println!("{:#?}", json_aggregate);
}
