// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

use commit_store::dispatch::DispatchingAsyncReadWriteStore;
use commit_store::Client as GrpcClient;
use commit_store::{dispatch::DispatchedStore, sqlite::SqliteStore};

// Store

#[derive(Clone, Debug)]
pub enum Grpc2Store {
    Sqlite(DispatchedStore<SqliteStore>),
    // Postgres(DispatchedStore<PostgresStoreAsync>),
    Grpc(GrpcClient),
}

impl Grpc2Store {
    pub fn store(&mut self) -> &mut dyn DispatchingAsyncReadWriteStore {
        match self {
            Grpc2Store::Sqlite(ref mut sqlite) => sqlite,
            // GraphQLStore::Postgres(ref mut postgres) => postgres,
            Grpc2Store::Grpc(ref mut client) => client,
        }
    }
}
