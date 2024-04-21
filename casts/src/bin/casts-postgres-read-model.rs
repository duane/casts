use anyhow::Result;
use casts::graphql::{create_schema, Mutation, Query, Subscription};
use casts::model::postgres::PostgresReadModel;
use casts::retry_future_n_times;
use commit_store::grpc_interface::commit_store_client::CommitStoreClient;
use commit_store::Client as GrpcClient;
use juniper::{DefaultScalarValue, RootNode};
use std::{env, sync::Arc};

lazy_static::lazy_static! {
    static ref ROOT_NODE: Arc<RootNode<'static, Query, Mutation, Subscription, DefaultScalarValue>> = Arc::new(create_schema());
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let COMMIT_STORE_SERVER =
        env::var("COMMIT_STORE_SERVER").unwrap_or_else(|_| "http://127.0.0.1:5059".to_string());
    log::info!("Connecting to commit store at {}", COMMIT_STORE_SERVER);

    let grpc_client = retry_future_n_times(
        || {
            Ok(GrpcClient::new(
                CommitStoreClient::connect(COMMIT_STORE_SERVER.clone())
                    .await
                    .expect("ok this was sposed to be unreachable"),
            ))
        },
        3,
    )
    .await
    .unwrap();

    let db_url =
        env::var("DATABASE_URL").unwrap_or("postgresql://casts:casts@localhost/casts".to_string());

    let mut read_model = PostgresReadModel::connect(grpc_client, db_url).await;
    read_model.serve().await;
}
