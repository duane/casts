// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

use commit_store::{postgres::PostgresStoreAsync, AsyncStore};

#[tokio::main]
async fn main() {
    PostgresStoreAsync::with_connection(
        std::env::args()
            .nth(1)
            .expect("USAGE: postgres-provision DATABASE_URL")
            .to_string(),
    )
    .await
    .initialize()
    .await
    .expect("failed to initialize database")
}
