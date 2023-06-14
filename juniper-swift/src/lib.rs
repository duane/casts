// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

#![feature(vec_into_raw_parts)]
#![feature(concat_idents)]

use std::collections::HashMap;
use std::ffi::c_void;
use std::mem;

use juniper::{DefaultScalarValue, InputValue};

#[cfg(test)]
#[macro_use]
extern crate lazy_static;

#[repr(C)]
#[derive(Clone, Debug)]
pub struct SourcePosition {
    pub index: usize,
    pub line: usize,
    pub col: usize,
}

impl Into<juniper::parser::SourcePosition> for SourcePosition {
    fn into(self) -> juniper::parser::SourcePosition {
        juniper::parser::SourcePosition::new(self.index, self.line, self.col)
    }
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct StringSpan {
    pub item: GraphQLUnownedBuffer,
    pub start: SourcePosition,
    pub end: SourcePosition,
}

impl StringSpan {
    fn clone_into(self) -> juniper::parser::Spanning<String> {
        juniper::parser::Spanning {
            item: self.item.clone().into(),
            start: self.start.clone().into(),
            end: self.end.clone().into(),
        }
    }
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct ValueSpan {
    pub item: GraphQLInputValue,
    pub start: SourcePosition,
    pub end: SourcePosition,
}

impl ValueSpan {
    fn clone_into(&self) -> juniper::parser::Spanning<InputValue<DefaultScalarValue>> {
        juniper::parser::Spanning {
            item: self.item.clone().into_native(),
            start: self.start.clone().into(),
            end: self.end.clone().into(),
        }
    }
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct GraphQLInputVariables {
    pub count: usize,
    pub names: *const GraphQLUnownedBuffer,
    pub values: *const GraphQLInputValue,
}

impl GraphQLInputVariables {
    pub fn get_map(&self) -> HashMap<String, InputValue<DefaultScalarValue>> {
        let mut map = HashMap::new();
        let names_slice = std::ptr::slice_from_raw_parts(self.names, self.count);
        let values_slice = std::ptr::slice_from_raw_parts(self.values, self.count);
        for i in 0..self.count {
            let name: String = unsafe {
                <GraphQLUnownedBuffer as Into<Option<String>>>::into((&*names_slice)[i].clone())
            }
            .unwrap_or_default();
            let value: InputValue<DefaultScalarValue> =
                unsafe { &*values_slice }[i].clone().into_native();
            map.insert(name, value);
        }
        map
    }
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct GraphQLInputValue {
    pub typ: u32,
    pub count: usize,          // length for LIST and OBJECT types
    pub slot_b: *const c_void, // GraphQLUnownedBuffer (aka string), StringSpan, ValueSpan, or NULL array, of length slot_a
    pub slot_c: *const c_void, // NULL or StringSpan, of length slot_a.
}

impl GraphQLInputValue {
    pub fn into_native(self) -> InputValue<DefaultScalarValue> {
        match self.typ {
            0 => InputValue::Null,
            // Scalar
            1 => {
                let buf_ptr = self.slot_b as *const GraphQLUnownedBuffer;
                let string_value: String = unsafe {
                    <GraphQLUnownedBuffer as Into<Option<String>>>::into((&*buf_ptr).clone())
                        .unwrap_or_default()
                };
                InputValue::Scalar(juniper::DefaultScalarValue::String(string_value))
            }
            // Enum
            2 => {
                let buf_ptr = self.slot_b as *const GraphQLUnownedBuffer;
                let string_value: String = unsafe {
                    <GraphQLUnownedBuffer as Into<Option<String>>>::into((&*buf_ptr).clone())
                        .unwrap_or_default()
                };
                InputValue::Enum(string_value)
            }
            // Variable
            3 => {
                let buf_ptr = self.slot_b as *const GraphQLUnownedBuffer;
                let string_value: String = unsafe {
                    <GraphQLUnownedBuffer as Into<Option<String>>>::into((&*buf_ptr).clone())
                        .unwrap_or_default()
                };
                InputValue::Variable(string_value)
            }
            // List
            4 => {
                let valuespan_arr_ptr = self.slot_b as *const ValueSpan;
                let slice = std::ptr::slice_from_raw_parts(valuespan_arr_ptr, self.count);
                let mut buf = vec![];
                for i in 0..self.count {
                    let input_val: juniper::parser::Spanning<InputValue> =
                        unsafe { &*slice }[i].clone_into();
                    buf.push(input_val)
                }
                InputValue::List(buf)
            }
            // Object
            5 => {
                let stringspan_arr_ptr = self.slot_c as *const StringSpan;
                let valuespan_arr_ptr = self.slot_c as *const ValueSpan;
                let stringslice = std::ptr::slice_from_raw_parts(stringspan_arr_ptr, self.count);
                let valueslice = std::ptr::slice_from_raw_parts(valuespan_arr_ptr, self.count);
                let mut buf = vec![];
                for i in 0..self.count {
                    let string_val: juniper::parser::Spanning<String> =
                        unsafe { &*stringslice }[i].clone().clone_into();
                    let input_val: juniper::parser::Spanning<InputValue> =
                        unsafe { &*valueslice }[i].clone_into();
                    buf.push((string_val, input_val))
                }
                InputValue::Object(buf)
            }
            _ => todo!("unimplemented"),
        }
    }
}

#[repr(C)]
#[derive(Clone, Debug)]
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
        // unimplemented!("memory leak");
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

impl Into<String> for GraphQLUnownedBuffer {
    fn into(self) -> String {
        let o_string: Option<String> = self.into();
        o_string.unwrap_or_default()
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

#[repr(C)]
#[derive(Clone, Debug)]
pub struct GraphQLOwnedHandle {
    pub pointee: *const c_void,
}
// unsafe impl Send for GraphQLOwnedHandle {}

#[repr(C)]
#[derive(Clone, Debug)]
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
