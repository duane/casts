use futures::StreamExt;
use uuid::uuid;

use commit_store::{dispatch::DispatchingAsyncReadWriteStore, Client};

#[tokio::main]
async fn main() {
    // tracing_subscriber::fmt().init();

    // Was: a hardcoded `include_str!("../../data/tls/ca.pem")` used as the
    // only trust root. That file is ISRG Root X1 -- Let's Encrypt's public
    // root -- so this binary could never verify the private-CA commit store,
    // and would have trusted any publicly-issued certificate instead. Goes
    // through the shared connector now, like every other casts binary.
    let client = casts::commit_store_client::connect()
        .await
        .expect("failed to connect to commit store");
    let mut grpc_client = Client::new(client);

    log::info!("Connected to commit store");

    let mut commit_stream = grpc_client
        .subscribe_stream(&uuid!("34c3c08d-0d55-4af4-a126-170c1380f07e"))
        .await;
    while let Some(commit) = commit_stream.next().await {
        log::info!("Received commit: {:?}", commit.unwrap());
    }
    panic!("exiting.");
}
