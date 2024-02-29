// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

extern crate log;
extern crate pretty_env_logger;

pub mod aggregate;
pub mod applicable;
pub mod commands;
pub mod events;
pub mod graphql;
pub mod util;

pub mod grpc;
pub mod grpc_interface {
    tonic::include_proto!("grpc_interface");
}
