use anyhow::Result;
use casts::graphql::{create_schema, GraphQLStore, Mutation, Query, Subscription};
use casts::retry_future_n_times;
use commit_store::grpc_interface::commit_store_client::CommitStoreClient;
use commit_store::Client;
use hyper::{service::service_fn, Method, Response, StatusCode};
use juniper::{DefaultScalarValue, RootNode};
use std::{convert::Infallible, env, sync::Arc};

lazy_static::lazy_static! {
    static ref ROOT_NODE: Arc<RootNode<'static, Query, Mutation, Subscription, DefaultScalarValue>> = Arc::new(create_schema());
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let addr = ([127, 0, 0, 1], 8243).into();

    let COMMIT_STORE_SERVER =
        env::var("COMMIT_STORE_SERVER").unwrap_or_else(|_| "http://[127.0.0.1]:5059".to_string());
    log::info!("Connecting to commit store at {}", COMMIT_STORE_SERVER);

    let grpc_client = retry_future_n_times(|| commit_store_connect(COMMIT_STORE_SERVER.clone()), 3)
        .await
        .unwrap();

    let async_wrapper_client = GraphQLStore::Grpc(grpc_client);
    let grpc_store1 = Arc::new(async_wrapper_client);
    let new_service = make_service_fn(move |_| {
        let grpc_store2 = grpc_store1.clone();

        async {
            let result = service_fn(move |req| {
                let grpc_store3 = grpc_store2.clone();

                async move {
                    let result = match (req.method(), req.uri().path()) {
                        (&Method::GET, "/") => juniper_hyper::graphiql("/graphql", None).await,
                        (&Method::GET, "/graphql") | (&Method::POST, "/graphql") => {
                            juniper_hyper::graphql(
                                (*ROOT_NODE).to_owned(),
                                grpc_store3.clone(),
                                req,
                            )
                            .await
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
            Ok::<_, hyper::Error>(result)
        }
    });

    let server = Server::bind(&addr).serve(new_service);
    println!("Listening on http://{addr}");

    if let Err(e) = server.await {
        eprintln!("server error: {e}")
    }
}
