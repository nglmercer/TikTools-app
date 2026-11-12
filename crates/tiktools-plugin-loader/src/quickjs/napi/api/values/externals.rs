//! Opaque native external values and finalizer records.
use super::*;

pub(in super::super::super) unsafe extern "C" fn api_create_external(
    env: Environment,
    data: *mut c_void,
    finalize: Finalize,
    hint: *mut c_void,
    result: *mut Handle,
) -> i32 {
    run(env, |env| {
        let value = check(env, new_external(env, data, finalize, hint))?;
        put(result, env.push(value))
    })
}

pub(in super::super::super) unsafe extern "C" fn api_get_value_external(
    env: Environment,
    value: Handle,
    result: *mut *mut c_void,
) -> i32 {
    run(env, |env| {
        let rec = external_rec(env, env.get(value)?).ok_or(INVALID_ARG)?;
        put(result, rec.data)
    })
}
