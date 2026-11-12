//! Array creation, lengths and indexed property operations.
use super::super::super::{
    env::{put, run, run_js},
    types::*,
};
use super::super::{check, check_int, Owned};
use super::object_arg;
use rquickjs::qjs;

pub(in super::super::super) unsafe extern "C" fn api_create_array(
    env: Environment,
    result: *mut Handle,
) -> i32 {
    run(env, |env| {
        put(result, env.push(unsafe { qjs::JS_NewArray(env.ctx) }))
    })
}

pub(in super::super::super) unsafe extern "C" fn api_create_array_with_length(
    env: Environment,
    length: usize,
    result: *mut Handle,
) -> i32 {
    run(env, |env| {
        let array = Owned::new(env, unsafe { qjs::JS_NewArray(env.ctx) });
        check_int(env, unsafe {
            qjs::JS_SetLength(env.ctx, array.value, length as i64)
        })?;
        put(result, env.push(array.release()))
    })
}

pub(in super::super::super) unsafe extern "C" fn api_is_array(
    env: Environment,
    value: Handle,
    result: *mut bool,
) -> i32 {
    run(env, |env| {
        put(result, unsafe { qjs::JS_IsArray(env.get(value)?) })
    })
}

pub(in super::super::super) unsafe extern "C" fn api_get_array_length(
    env: Environment,
    value: Handle,
    result: *mut u32,
) -> i32 {
    run(env, |env| {
        let value = env.get(value)?;
        if !unsafe { qjs::JS_IsArray(value) } {
            return Err(ARRAY_EXPECTED);
        }
        let mut length = 0_i64;
        check_int(env, unsafe {
            qjs::JS_GetLength(env.ctx, value, &mut length)
        })?;
        put(result, length as u32)
    })
}

pub(in super::super::super) unsafe extern "C" fn api_get_element(
    env: Environment,
    object: Handle,
    index: u32,
    result: *mut Handle,
) -> i32 {
    run_js(env, |env| {
        let object = object_arg(env, object)?;
        let value = unsafe { qjs::JS_GetPropertyUint32(env.ctx, object, index) };
        put(result, env.push(check(env, value)?))
    })
}

pub(in super::super::super) unsafe extern "C" fn api_set_element(
    env: Environment,
    object: Handle,
    index: u32,
    value: Handle,
) -> i32 {
    run_js(env, |env| {
        let (object, value) = (object_arg(env, object)?, env.get(value)?);
        let dup = unsafe { qjs::JS_DupValue(env.ctx, value) };
        check_int(env, unsafe {
            qjs::JS_SetPropertyUint32(env.ctx, object, index, dup)
        })?;
        Ok(())
    })
}

pub(in super::super::super) unsafe extern "C" fn api_has_element(
    env: Environment,
    object: Handle,
    index: u32,
    result: *mut bool,
) -> i32 {
    run_js(env, |env| {
        let object = object_arg(env, object)?;
        let atom = unsafe { qjs::JS_NewAtomUInt32(env.ctx, index) };
        let found = unsafe { qjs::JS_HasProperty(env.ctx, object, atom) };
        unsafe { qjs::JS_FreeAtom(env.ctx, atom) };
        put(result, check_int(env, found)? > 0)
    })
}

pub(in super::super::super) unsafe extern "C" fn api_delete_element(
    env: Environment,
    object: Handle,
    index: u32,
    result: *mut bool,
) -> i32 {
    run_js(env, |env| {
        let object = object_arg(env, object)?;
        let atom = unsafe { qjs::JS_NewAtomUInt32(env.ctx, index) };
        let removed = unsafe { qjs::JS_DeleteProperty(env.ctx, object, atom, 0) };
        unsafe { qjs::JS_FreeAtom(env.ctx, atom) };
        let removed = check_int(env, removed)? > 0;
        if result.is_null() {
            return Ok(());
        }
        put(result, removed)
    })
}
