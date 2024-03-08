use anyhow::Result;
use casts::graphql::{create_schema, Mutation, Query, Subscription};
use casts::util::aretry3;
use commit_store::grpc_interface::commit_store_client::CommitStoreClient;
use commit_store::{DispatchingAsyncClientStore, GrpcClient};
use futures::StreamExt;
use juniper::{DefaultScalarValue, RootNode};
use std::{env, sync::Arc};

lazy_static::lazy_static! {
    static ref ROOT_NODE: Arc<RootNode<'static, Query, Mutation, Subscription, DefaultScalarValue>> = Arc::new(create_schema());
}

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

    let mut grpc_client = aretry3(|| commit_store_connect(commit_store_addr.clone()), 3)
        .await
        .unwrap();

    let mut commit_stream = grpc_client.subscribe(None).await;
    while let Some(commit) = commit_stream.next().await {
        log::info!("Received commit: {:?}", commit);
    }
}
