// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

use casts::graphql::{create_schema, create_sqlite_store, GraphQLStore, Schema};
use core::ffi::c_void;
use juniper::futures::StreamExt;
use juniper::{resolve_into_stream, DefaultScalarValue, ExecutionError, InputValue, ValuesStream};
use juniper_subscriptions::Connection;
use juniper_swift::{GraphQLExecuteContext, GraphQLUnownedBuffer};
use serde_json::{from_value, Value};
use std::collections::HashMap;
use tokio::runtime::{Builder, Runtime};

lazy_static::lazy_static! {
    static ref SCHEMA: Schema = create_schema();
    static ref STORE: GraphQLStore = create_sqlite_store();
    static ref RUNTIME: Runtime = create_runtime();
}
fn create_runtime() -> Runtime {
    let runtime = Builder::new_multi_thread()
        .worker_threads(4)
        .thread_name("gql-threadpool")
        .thread_stack_size(3 * 1024 * 1024)
        .enable_io()
        .build()
        .unwrap();
    runtime
}

#[no_mangle]
pub unsafe extern "C" fn gql_casts(
    document: GraphQLUnownedBuffer,
    op_name: GraphQLUnownedBuffer,
    variables: GraphQLUnownedBuffer,
    context: *const c_void,
    callback: extern "C" fn(GraphQLUnownedBuffer, *const c_void),
) {
    let context = GraphQLExecuteContext { context };
    let document_opt: Option<String> = document.into();
    let document_str = document_opt.expect("document is a required parameter");
    let variables_opt: Option<String> = variables.into();
    let variables_json_opt: Option<serde_json::Value> = variables_opt
        .map(|s| serde_json::from_str(&s))
        .map(|r| r.unwrap());
    let op_name_str: Option<String> = op_name.into();
    let variables: Option<InputValue<DefaultScalarValue>> =
        variables_json_opt.map(|value| serde_json::from_value(value).unwrap());

    println!("variables: {:?}", variables);

    let _guard = RUNTIME.enter();
    let _join_handle = RUNTIME.spawn(async move {
        println!("inside runtime.");
        let inner_context = context;
        let request = juniper::http::GraphQLRequest::new(document_str, op_name_str, variables);
        let response = request.execute(&SCHEMA, &STORE).await;
        println!("response: {:?}", response);
        let response_body = serde_json::to_string(&response).unwrap();
        println!("Response body: {:?}", response_body);
        let response_buffer = GraphQLUnownedBuffer::from(response_body);
        println!("got a response back! doing the callback");
        callback(response_buffer, inner_context.context);
    });
    println!("returning");
}

#[no_mangle]
pub unsafe extern "C" fn gql_casts_subscribe(
    document: GraphQLUnownedBuffer,
    _op_name: GraphQLUnownedBuffer,
    variables: GraphQLUnownedBuffer,
    context: *const c_void,
    callback: extern "C" fn(GraphQLUnownedBuffer, *const c_void),
) {
    let context = GraphQLExecuteContext { context };
    let document_opt: Option<String> = document.into();
    let document_str = document_opt.expect("document is a required parameter");
    let variables_opt: Option<String> = variables.into();
    let variables_json_opt: Option<serde_json::Value> = variables_opt
        .map(|s| serde_json::from_str(&s))
        .map(|r| r.unwrap());
    // let op_name_str: Option<String> = op_name.into();
    let variables: HashMap<String, InputValue<DefaultScalarValue>> = variables_json_opt
        .map(|value| {
            if let Value::Object(obj) = value {
                let mut hash_obj = HashMap::new();
                for (key, value) in obj.into_iter() {
                    hash_obj.insert(key, from_value(value).unwrap());
                }
                hash_obj
            } else {
                HashMap::new()
            }
        })
        .unwrap_or_default();

    let _guard = RUNTIME.enter();
    let _join_handle = RUNTIME.spawn(async move {
        println!("inside runtime.");
        let inner_context = context;
        // let str_ref = op_name_str.map(|s| s.as_ref());
        let (value_stream_value, errors): (
            juniper::Value<ValuesStream>,
            Vec<ExecutionError<DefaultScalarValue>>,
        ) = resolve_into_stream(&document_str, None, &SCHEMA, &variables, &STORE)
            .await
            .unwrap();

        let value_stream = value_stream_value.into();
        let mut subscription_connection: Connection<'_, DefaultScalarValue> =
            Connection::from_stream(value_stream, errors);
        while let Some(update) = subscription_connection.next().await {
            let serialized_response: String =
                serde_json::to_string(&serde_json::to_value(update).unwrap()).unwrap();
            println!("response emitted!: {:?}", serialized_response);
            let serialized_response_buffer = GraphQLUnownedBuffer::from(serialized_response);
            callback(serialized_response_buffer, inner_context.context);
            println!("callback called");
        }
    });
    println!("returning");
}
