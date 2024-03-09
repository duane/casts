// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

use std::env;

use casts::model::postgres::PostgresReadModel;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let commit_store_addr =
        env::var("COMMIT_STORE_ADDR").unwrap_or_else(|_| "http://127.0.0.1:5059".to_string());
    log::info!("Connecting to commit store at {}", commit_store_addr);

    println!("provisioning...");
    PostgresReadModel::provision(
        std::env::args()
            .nth(1)
            .expect("USAGE: postgres-provision CASTS_DATABASE_URL")
            .to_string()
            .as_str(),
    )
    .await
    .unwrap();
    println!("Provisioned model.")
}
