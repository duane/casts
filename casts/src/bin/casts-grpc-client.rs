use casts::config::ClientConfigBuilder;
use futures::StreamExt;
use std::str::FromStr;
use tonic::transport::{Certificate, ClientTlsConfig};
use uuid::uuid;

use commit_store::{
    dispatch::DispatchingAsyncReadWriteStore,
    grpc_interface::commit_store_client::CommitStoreClient, Client,
};

#[tokio::main]
async fn main() {
    // tracing_subscriber::fmt().init();

    let client_cfg = ClientConfigBuilder::default().build().unwrap();

    log::info!("Connecting to commit store at {}", client_cfg.url);

    let addr = hyper::Uri::from_str(client_cfg.url.as_str()).unwrap();
    let host = addr.host().unwrap().to_string();
    let ca_pem = include_str!("../../data/tls/ca.pem");
    let tls_cfg = ClientTlsConfig::new()
        .ca_certificate(Certificate::from_pem(ca_pem))
        .domain_name(host);
    let url_str: String = client_cfg.url.to_string();
    let channel = tonic::transport::Channel::from_shared(url_str)
        .unwrap()
        .tls_config(tls_cfg)
        .unwrap()
        .connect()
        .await
        .unwrap();
    let mut grpc_client = Client::new(CommitStoreClient::with_origin(
        channel,
        tonic::transport::Uri::from_str(&addr.to_string()).unwrap(),
    ));

    log::info!("Connected to commit store at {}", addr);

    let mut commit_stream = grpc_client
        .subscribe_stream(&uuid!("34c3c08d-0d55-4af4-a126-170c1380f07e"))
        .await;
    while let Some(commit) = commit_stream.next().await {
        log::info!("Received commit: {:?}", commit.unwrap());
    }
    panic!("exiting.");
}
