//! Handle, escapable, callback and async-context scopes.
use super::super::super::{
    env::{put, run},
    types::*,
};

pub(in super::super::super) unsafe extern "C" fn api_open_handle_scope(
    env: Environment,
    result: *mut Opaque,
) -> i32 {
    run(env, |env| put(result, env.open_scope(false) as Opaque))
}
pub(in super::super::super) unsafe extern "C" fn api_close_handle_scope(
    env: Environment,
    scope: Opaque,
) -> i32 {
    run(env, |env| env.close_scope(scope as usize))
}
pub(in super::super::super) unsafe extern "C" fn api_open_escapable_handle_scope(
    env: Environment,
    result: *mut Opaque,
) -> i32 {
    run(env, |env| put(result, env.open_scope(true) as Opaque))
}
pub(in super::super::super) unsafe extern "C" fn api_close_escapable_handle_scope(
    env: Environment,
    scope: Opaque,
) -> i32 {
    run(env, |env| env.close_scope(scope as usize))
}
pub(in super::super::super) unsafe extern "C" fn api_escape_handle(
    env: Environment,
    scope: Opaque,
    escapee: Handle,
    result: *mut Handle,
) -> i32 {
    run(env, |env| {
        let value = env.get(escapee)?;
        put(result, env.escape(scope as usize, value)?)
    })
}

pub(in super::super::super) unsafe extern "C" fn api_open_callback_scope(
    _env: Environment,
    _resource: Handle,
    _context: Opaque,
    result: *mut Opaque,
) -> i32 {
    if result.is_null() {
        return INVALID_ARG;
    }
    unsafe { result.write(1 as Opaque) };
    OK
}
pub(in super::super::super) unsafe extern "C" fn api_close_callback_scope(
    _env: Environment,
    _scope: Opaque,
) -> i32 {
    OK
}
pub(in super::super::super) unsafe extern "C" fn api_async_init(
    _env: Environment,
    _resource: Handle,
    _name: Handle,
    result: *mut Opaque,
) -> i32 {
    if result.is_null() {
        return INVALID_ARG;
    }
    unsafe { result.write(1 as Opaque) };
    OK
}
pub(in super::super::super) unsafe extern "C" fn api_async_destroy(
    _env: Environment,
    _context: Opaque,
) -> i32 {
    OK
}
