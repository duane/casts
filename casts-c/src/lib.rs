// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

use casts::graphql::{create_schema, create_store, Schema};
use core::ffi::c_void;
use juniper_swift::{GraphQLExecuteContext, GraphQLUnownedBuffer, GraphQLInputValue};
use tokio::runtime::{Builder, Runtime};
use juniper::{InputValue, DefaultScalarValue};
use juniper::parser::{Spanning, SourcePosition};

lazy_static::lazy_static! {
    static ref SCHEMA: Schema = create_schema();
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
    variables: *const GraphQLInputValue,
    context: *const c_void,
    callback: extern "C" fn(GraphQLUnownedBuffer, *const c_void),
) {
    let context = GraphQLExecuteContext { context };
    let document_opt: Option<String> = document.into();
    let document_str = document_opt.expect("document is a required parameter");
    let op_name_str: Option<String> = op_name.into();
    let variables = InputValue::Object(vec![(Spanning { item: "streamId".to_string(), start: SourcePosition::new(0, 0, 0), end: SourcePosition::new(0, 0, 0) }, Spanning { item: InputValue::Scalar(DefaultScalarValue::String("34c3c08d-0d55-4af4-a126-170c1380f07e".to_string())), start: SourcePosition::new(0, 0, 0), end: SourcePosition::new(0, 0, 0) })]);
    let _guard = RUNTIME.enter();
    let _joinHandle = RUNTIME.spawn(async move {
        let inner_context = context;
        let request = juniper::http::GraphQLRequest::new(document_str, op_name_str, Some(variables));
        let response = request.execute(&SCHEMA, &create_store().await).await;
        let response_body = serde_json::to_string(&response).unwrap();
        let response_buffer = GraphQLUnownedBuffer::from(response_body);
        callback(response_buffer, inner_context.context);
    });
}
