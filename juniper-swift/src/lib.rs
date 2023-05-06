// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

#![feature(vec_into_raw_parts)]
#![feature(concat_idents)]

use std::ffi::c_void;
use std::mem;

#[cfg(test)]
#[macro_use]
extern crate lazy_static;

#[repr(C)]
#[derive(Debug)]
pub struct GraphQLUnownedBuffer {
    pub buffer: *const c_void,
    pub length: usize,
}
unsafe impl Send for GraphQLUnownedBuffer {}
impl Into<Option<Vec<u8>>> for GraphQLUnownedBuffer {
    fn into(self) -> Option<Vec<u8>> {
        if self.buffer.is_null() {
            return None;
        }
        let buffer_slice: &[u8] =
            unsafe { std::slice::from_raw_parts(self.buffer as *const u8, self.length) };
        mem::forget(buffer_slice);
        let mut buffer_vec: Vec<u8> = Vec::with_capacity(buffer_slice.len());
        buffer_vec.extend_from_slice(buffer_slice);
        Some(buffer_vec)
    }
}
impl Into<Option<String>> for GraphQLUnownedBuffer {
    fn into(self) -> Option<String> {
        let owned_vec: Option<Vec<u8>> = self.into();
        owned_vec.map(|vec| unsafe { String::from_utf8_unchecked(vec) })
    }
}

impl From<String> for GraphQLUnownedBuffer {
    fn from(s: String) -> Self {
        let (buffer, length, _) = s.into_bytes().into_raw_parts();
        GraphQLUnownedBuffer {
            buffer: buffer as *const c_void,
            length,
        }
    }
}

pub struct GraphQLExecuteContext {
    pub context: *const c_void,
}
unsafe impl Send for GraphQLExecuteContext {}

#[no_mangle]
pub extern "C" fn gqlbridge_free_buffer(result: GraphQLUnownedBuffer) {
    let buffer = result.buffer;
    let length = result.length;
    unsafe { Vec::from_raw_parts(buffer as *mut u8, length, length) };
}

#[macro_export]
macro_rules! graphql_binding {
    ($binding_name:ident, $pool_name:ident, $schema:expr) => {
        use futures::executor::ThreadPool;

        lazy_static! {
            static ref $pool_name: ThreadPool = ThreadPool::new().unwrap();
        }

        #[no_mangle]
        pub extern "C" fn $binding_name(
            document: GraphQLUnownedBuffer,
            op_name: GraphQLUnownedBuffer,
            context: *const c_void,
            callback: extern "C" fn(GraphQLUnownedBuffer, *const c_void),
        ) {
            let context = GraphQLExecuteContext { context };
            let document_opt: Option<String> = document.into();
            let document_str = document_opt.expect("document is a required parameter");
            let op_name_str: Option<String> = op_name.into();
            (*$pool_name).spawn_ok(async move {
                let request = juniper::http::GraphQLRequest::new(document_str, op_name_str, None);
                let response = request.execute(&$schema, &());
                let response_body = serde_json::to_string(&response).unwrap();
                let response_buffer = GraphQLUnownedBuffer::from(response_body);
                callback(response_buffer, context.context);
            })
        }
    };
}

#[cfg(test)]
mod tests {
    use juniper::{EmptyMutation, RootNode};

    pub struct Query;

    #[juniper::object]
    impl Query {
        fn hello_world() -> &str {
            "Hello World!"
        }
    }

    pub type Schema = RootNode<'static, Query, EmptyMutation<()>>;

    lazy_static! {
        static ref TEST_SCHEMA: Schema = Schema::new(Query {}, EmptyMutation::new());
    }

    graphql_binding!(test_binding_execute, TEST_BINDING_THREADPOOL, TEST_SCHEMA);
}
