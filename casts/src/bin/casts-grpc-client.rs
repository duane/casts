use casts::grpc_interface::casts_client::CastsClient;
use casts::grpc_interface::{Feeds, ListFeedsRequest};

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    let stream_id = String::from("34c3c08d-0d55-4af4-a126-170c1380f07e");

    let mut grpc_client = CastsClient::connect("http://127.0.0.1:7392")
        .await
        .expect("Failed to connect to backend commit store");

    let request = ListFeedsRequest { stream_id };
    let feeds: Feeds = grpc_client
        .list_feeds(request)
        .await
        .unwrap()
        .get_mut()
        .to_owned();

    println!("{:#?}", feeds);
}
