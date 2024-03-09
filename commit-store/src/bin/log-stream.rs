use std::env;
use tokio_stream::StreamExt;
use uuid::uuid;

use anyhow::Result;
use commit_store::{
    grpc_interface::commit_store_client::CommitStoreClient, DispatchingAsyncClientStore, GrpcClient,
};

async fn commit_store_connect(commit_store_addr: String) -> Result<GrpcClient> {
    Ok(GrpcClient::new(
        CommitStoreClient::connect(commit_store_addr)
            .await
            .expect("Failed to connect to backend commit store"),
    ))
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let commit_store_addr =
        env::var("COMMIT_STORE_ADDR").unwrap_or_else(|_| "http://127.0.0.1:5059".to_string());
    log::info!("Connecting to commit store at {}", commit_store_addr);

    let mut grpc_client = commit_store_connect(commit_store_addr.clone())
        .await
        .unwrap();

    println!("Connected to commit store at {}", commit_store_addr);

    let mut commit_stream = grpc_client
        .subscribe_stream(&uuid!("34c3c08d-0d55-4af4-a126-170c1380f07e"))
        .await;
    while let Some(commit) = commit_stream.next().await {
        log::info!("Received commit: {:?}", commit.unwrap());
    }
    panic!("exiting.");
}
