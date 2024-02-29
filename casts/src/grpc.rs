// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

use crate::grpc_interface::{casts_client::CastsClient, Feed as GrpcFeed};
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use commit_store::{
    dispatch::DispatchedStore, sqlite::SqliteStoreAsync, DispatchingAsyncClientStore, GrpcClient,
};
use prost_types::Timestamp as GrpcTimestamp;
use uuid::Uuid;

// Store

#[derive(Clone, Debug)]
pub enum Grpc2Store {
    Sqlite(DispatchedStore<SqliteStoreAsync>),
    // Postgres(DispatchedStore<PostgresStoreAsync>),
    Grpc(GrpcClient),
}

impl Grpc2Store {
    pub fn store(&mut self) -> &mut dyn DispatchingAsyncClientStore {
        match self {
            Grpc2Store::Sqlite(ref mut sqlite) => sqlite,
            // GraphQLStore::Postgres(ref mut postgres) => postgres,
            Grpc2Store::Grpc(ref mut client) => client,
        }
    }
}
