// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

use casts::model::postgres::PostgresReadModel;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

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
