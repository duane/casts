// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

use casts::graphql::{create_schema, create_store, Schema};
use core::ffi::c_void;
use juniper_swift::{GraphQLExecuteContext, GraphQLUnownedBuffer};

lazy_static::lazy_static! {
    static ref SCHEMA: Schema = create_schema();
    static ref GQL_CASTS_POOL: ThreadPool = ThreadPool::new().unwrap();
}
use futures::executor::ThreadPool;

#[no_mangle]
pub unsafe extern "C" fn gql_casts(
    document: GraphQLUnownedBuffer,
    op_name: GraphQLUnownedBuffer,
    context: *const c_void,
    callback: extern "C" fn(GraphQLUnownedBuffer, *const c_void),
) {
    let context = GraphQLExecuteContext { context };
    let document_opt: Option<String> = document.into();
    let document_str = document_opt.expect("document is a required parameter");
    let op_name_str: Option<String> = op_name.into();
    (*GQL_CASTS_POOL).spawn_ok(async move {
        let inner_context = context;
        let request = juniper::http::GraphQLRequest::new(document_str, op_name_str, None);
        let response = request.execute(&SCHEMA, &create_store().await).await;
        let response_body = serde_json::to_string(&response).unwrap();
        let response_buffer = GraphQLUnownedBuffer::from(response_body);
        callback(response_buffer, inner_context.context);
    })
}
