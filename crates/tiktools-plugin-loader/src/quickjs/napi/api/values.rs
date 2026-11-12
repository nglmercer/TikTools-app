//! Primitive values, strings, numbers, symbols, bigints, dates, externals.

mod numbers;
pub(in super::super) use numbers::{
    api_create_double, api_create_int32, api_create_int64, api_create_uint32, api_get_value_double,
    api_get_value_int32, api_get_value_int64, api_get_value_uint32,
};

mod strings;
pub(in super::super) use strings::{
    api_create_external_string_latin1, api_create_external_string_utf16,
    api_create_property_key_latin1, api_create_property_key_utf16, api_create_property_key_utf8,
    api_create_string_latin1, api_create_string_utf16, api_create_string_utf8,
    api_get_value_string_latin1, api_get_value_string_utf16, api_get_value_string_utf8,
};

mod symbols;
pub(in super::super) use symbols::{api_create_symbol, api_node_api_symbol_for};

mod externals;
pub(in super::super) use externals::{api_create_external, api_get_value_external};

mod coercion;
pub(in super::super) use coercion::{
    api_coerce_to_bool, api_coerce_to_number, api_coerce_to_object, api_coerce_to_string,
    api_get_boolean, api_get_value_bool, api_strict_equals, api_typeof_value,
};

mod bigints;
pub(in super::super) use bigints::{
    api_create_bigint_int64, api_create_bigint_uint64, api_create_bigint_words,
    api_get_value_bigint_int64, api_get_value_bigint_uint64, api_get_value_bigint_words,
};

mod dates;
pub(in super::super) use dates::{api_create_date, api_get_date_value, api_is_date};

use numbers::number;
use strings::string_bytes;

use std::ffi::{c_char, c_void, CString};

use rquickjs::qjs::{self, JSValue};

use super::{
    super::{
        env::{put, run, run_js, Env},
        functions::{external_rec, new_external},
        types::*,
    },
    bool_value, check, check_int, eval, global_get, is_object, new_string, read_bytes, Owned,
};

macro_rules! simple_out {
    ($name:ident, $body:expr) => {
        pub(in super::super) unsafe extern "C" fn $name(
            env: Environment,
            result: *mut Handle,
        ) -> i32 {
            run(env, |env| {
                let value: JSValue = $body(env);
                put(result, env.push(value))
            })
        }
    };
}

simple_out!(api_get_undefined, |_: &Env| qjs::JS_UNDEFINED);
simple_out!(api_get_null, |_: &Env| qjs::JS_NULL);
simple_out!(api_get_global, super::global);
