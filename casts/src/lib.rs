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

pub mod model;

#[cfg(feature = "graphql")]
pub mod graphql;
