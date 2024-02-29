use casts::grpc_interface::casts_client::CastsClient;
use casts::grpc_interface::{FeedRequest, Feeds};
use casts::{aggregate::FeedAggregate, applicable::EventApplicable};
use commit_store::grpc_interface::{Commits, Cursor};

use commit_store::grpc_interface::commit_store_client::CommitStoreClient;
use serde_json::Value;
use uuid::{uuid, Uuid};

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    let stream_id = String::from("34c3c08d-0d55-4af4-a126-170c1380f07e");

    let mut grpc_client = CastsClient::connect("http://127.0.0.1:8243")
        .await
        .expect("Failed to connect to backend commit store");

    let request = FeedRequest {
        request_id: Uuid::new_v4().to_string(),
        stream_id,
    };
    let feeds: Feeds = grpc_client
        .list_feeds(request)
        .await
        .unwrap()
        .get_mut()
        .to_owned();

    println!("{:#?}", feeds);
}
