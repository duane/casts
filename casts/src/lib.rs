// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

extern crate log;
extern crate pretty_env_logger;

pub mod aggregate;
pub mod applicable;
pub mod commands;
pub mod events;
pub mod util;

#[cfg(feature = "grpc")]
pub mod grpc;
#[cfg(feature = "grpc")]
pub mod grpc_interface {
    tonic::include_proto!("grpc_interface");
}

pub mod config;

pub mod model;

#[cfg(feature = "graphql")]
pub mod graphql;

use futures::Future;

pub async fn retry_future_n_times<T, E, Fut, F: FnMut() -> Fut>(
    mut f: F,
    retries: i32,
) -> Result<T, E>
where
    Fut: Future<Output = Result<T, E>>,
{
    let mut count = 0;
    loop {
        let result = f().await;

        if result.is_ok() {
            break result;
        } else {
            if count > retries {
                break result;
            }
            count += 1;
        }
    }
}
