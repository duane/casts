// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

//! Entry point: wire the `Casts` service to a real commit store and serve it.

use anyhow::Result;
use casts::grpc::Grpc2Store;
use casts::grpc_interface::casts_server::CastsServer;
use casts::service::CastsImpl;
use commit_store::{grpc_interface::commit_store_client::CommitStoreClient, Client};
use std::str::FromStr;
use tonic::transport::Server;

pub async fn serve() -> Result<()> {
    let server_config = casts::config::ServerConfigBuilder::default().build()?;

    let commit_store_cfg = commit_store::config::ClientConfigBuilder::default().build();

    log::info!("Connecting to commit store at {}", commit_store_cfg.addr);

    let endpoint = tonic::transport::Uri::from_str(&commit_store_cfg.addr)?;

    let client: CommitStoreClient<tonic::transport::Channel> =
        CommitStoreClient::connect(endpoint).await?;

    let grpc_client = Client::new(client);

    let commit_store_impl = CastsImpl::new(Grpc2Store::Grpc(grpc_client));

    let service = CastsServer::new(commit_store_impl);

    log::info!("Started server at {:?}", server_config.listen);
    let mut builder = Server::builder();

    builder
        .add_service(service)
        .serve(server_config.listen)
        .await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    serve().await?;

    Ok(())
}
