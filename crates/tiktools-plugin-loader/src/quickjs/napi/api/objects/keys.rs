//! Own/prototype key enumeration and array projection.
use super::super::super::{
    env::{put, run_js, Env},
    types::*,
};
use super::super::{check_int, Owned};
use super::object_arg;
use rquickjs::qjs::{self, JSValue};
use std::ffi::CStr;

/// Collects own property keys (string and/or symbol) of one object.
fn own_keys(
    env: &Env,
    object: JSValue,
    mask: u32,
    enum_only: bool,
    convert_numbers: bool,
) -> Result<Vec<JSValue>, i32> {
    let mut table: *mut qjs::JSPropertyEnum = std::ptr::null_mut();
    let mut length = 0_u32;
    let flags = mask | if enum_only { qjs::JS_GPN_ENUM_ONLY } else { 0 };
    check_int(env, unsafe {
        qjs::JS_GetOwnPropertyNames(env.ctx, &mut table, &mut length, object, flags as i32)
    })?;
    let mut keys = Vec::with_capacity(length as usize);
    for index in 0..length as usize {
        let atom = unsafe { (*table.add(index)).atom };
        let key = unsafe {
            if convert_numbers {
                qjs::JS_AtomToString(env.ctx, atom)
            } else {
                qjs::JS_AtomToValue(env.ctx, atom)
            }
        };
        keys.push(key);
    }
    unsafe { qjs::JS_FreePropertyEnum(env.ctx, table, length) };
    Ok(keys)
}

fn keys_array(env: &Env, keys: Vec<JSValue>) -> Result<JSValue, i32> {
    let array = unsafe { qjs::JS_NewArray(env.ctx) };
    for (index, key) in keys.into_iter().enumerate() {
        check_int(env, unsafe {
            qjs::JS_SetPropertyUint32(env.ctx, array, index as u32, key)
        })?;
    }
    Ok(array)
}

pub(in super::super::super) unsafe extern "C" fn api_get_property_names(
    env: Environment,
    object: Handle,
    result: *mut Handle,
) -> i32 {
    unsafe { api_get_all_property_names(env, object, 0, 2 | 16, 1, result) }
}

pub(in super::super::super) unsafe extern "C" fn api_get_all_property_names(
    env: Environment,
    object: Handle,
    key_mode: i32,
    key_filter: i32,
    key_conversion: i32,
    result: *mut Handle,
) -> i32 {
    run_js(env, |env| {
        let object = object_arg(env, object)?;
        let mut mask = 0;
        if key_filter & 8 == 0 {
            mask |= qjs::JS_GPN_STRING_MASK;
        }
        if key_filter & 16 == 0 {
            mask |= qjs::JS_GPN_SYMBOL_MASK;
        }
        let enum_only = key_filter & 2 != 0;
        let mut collected = Vec::new();
        let mut seen: Vec<String> = Vec::new();
        let mut current = Owned::new(env, unsafe { qjs::JS_DupValue(env.ctx, object) });
        loop {
            for key in own_keys(env, current.value, mask, enum_only, key_conversion == 1)? {
                let text = unsafe {
                    let cstr = qjs::JS_ToCString(env.ctx, key);
                    if cstr.is_null() {
                        String::new()
                    } else {
                        let owned = CStr::from_ptr(cstr).to_string_lossy().into_owned();
                        qjs::JS_FreeCString(env.ctx, cstr);
                        owned
                    }
                };
                if seen.contains(&text) {
                    unsafe { qjs::JS_FreeValue(env.ctx, key) };
                } else {
                    seen.push(text);
                    collected.push(key);
                }
            }
            if key_mode == 1 {
                break;
            }
            let proto = unsafe { qjs::JS_GetPrototype(env.ctx, current.value) };
            if unsafe { !qjs::JS_IsObject(proto) } {
                unsafe { qjs::JS_FreeValue(env.ctx, proto) };
                break;
            }
            current = Owned::new(env, proto);
        }
        put(result, env.push(keys_array(env, collected)?))
    })
}
