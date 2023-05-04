use casts::graphql::{GrpcStore, Mutation, Query, Schema};
use commit_store::GrpcClient;
use std::{convert::Infallible, sync::Arc};

use commit_store::grpc_interface::commit_store_client::CommitStoreClient;
use hyper::{
    server::Server,
    service::{make_service_fn, service_fn},
    Method, Response, StatusCode,
};
use juniper::EmptySubscription;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let addr = ([127, 0, 0, 1], 3000).into();

    let root_node = Arc::new(Schema::new(Query, Mutation, EmptySubscription::new()));

    let grpc_client = CommitStoreClient::connect("http://127.0.0.1:5059")
        .await
        .expect("Failed to connect to backend commit store");
    let async_wrapper_client = GrpcClient::new(grpc_client);
    let grpc_store1 = Arc::new(GrpcStore::new(async_wrapper_client));
    let new_service = make_service_fn(move |_| {
        let root_node = root_node.clone();
        let grpc_store2 = grpc_store1.clone();

        async {
            let result = service_fn(move |req| {
                let root_node = root_node.clone();
                let grpc_store3 = grpc_store2.clone();

                async move {
                    let result = match (req.method(), req.uri().path()) {
                        (&Method::GET, "/") => juniper_hyper::graphiql("/graphql", None).await,
                        (&Method::GET, "/graphql") | (&Method::POST, "/graphql") => {
                            juniper_hyper::graphql(root_node, grpc_store3.clone(), req).await
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
