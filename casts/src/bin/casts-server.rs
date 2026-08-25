// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

//! The GraphQL HTTP server: graphiql on `/`, GraphQL on `/graphql`.
//!
//! `juniper_hyper` at the pinned juniper rev is written against hyper 0.14,
//! so this binary uses the renamed `hyper_014` dependency; the rest of the
//! crate is on hyper 1.x.

use anyhow::Result;
use casts::graphql::{create_schema, GraphQLStore, Mutation, Query, Subscription};
use casts::retry_future_n_times;
use commit_store::grpc_interface::commit_store_client::CommitStoreClient;
use commit_store::Client;
use hyper_014::{
    server::Server,
    service::{make_service_fn, service_fn},
    Method, Response, StatusCode,
};
use juniper::{DefaultScalarValue, RootNode};
use std::{convert::Infallible, str::FromStr, sync::Arc};

lazy_static::lazy_static! {
    static ref ROOT_NODE: Arc<RootNode<'static, Query, Mutation, Subscription, DefaultScalarValue>> = Arc::new(create_schema());
}

async fn commit_store_connect(addr: String) -> Result<Client> {
    let endpoint = tonic::transport::Uri::from_str(&addr)?;
    let client = CommitStoreClient::connect(endpoint).await?;
    Ok(Client::new(client))
}

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    let server_config = casts::config::ServerConfigBuilder::default().build()?;
    let commit_store_cfg = commit_store::config::ClientConfigBuilder::default().build();

    log::info!("Connecting to commit store at {}", commit_store_cfg.addr);
    let grpc_client =
        retry_future_n_times(|| commit_store_connect(commit_store_cfg.addr.clone()), 3).await?;

    let store = Arc::new(GraphQLStore::Grpc(grpc_client));
    let new_service = make_service_fn(move |_| {
        let store = store.clone();

        async {
            let result = service_fn(move |req| {
                let store = store.clone();

                async move {
                    let result = match (req.method(), req.uri().path()) {
                        (&Method::GET, "/") => juniper_hyper::graphiql("/graphql", None).await,
                        (&Method::GET, "/graphql") | (&Method::POST, "/graphql") => {
                            juniper_hyper::graphql((*ROOT_NODE).clone(), store.clone(), req).await
                        }
                        _ => {
                            let mut response = Response::new(String::new());
                            *response.status_mut() = StatusCode::NOT_FOUND;
                            response
                        }
                    };
                    Ok::<_, Infallible>(result)
                }
            });
            Ok::<_, hyper_014::Error>(result)
        }
    });

    let server = Server::bind(&server_config.listen).serve(new_service);
    log::info!("Listening on http://{}", server_config.listen);

    if let Err(e) = server.await {
        eprintln!("server error: {e}")
    }
    Ok(())
}
