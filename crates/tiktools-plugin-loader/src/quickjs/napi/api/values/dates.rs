//! Date creation, classification and timestamp extraction.
use super::*;

pub(in super::super::super) unsafe extern "C" fn api_create_date(
    env: Environment,
    time: f64,
    result: *mut Handle,
) -> i32 {
    run(env, |env| {
        put(result, env.push(unsafe { qjs::JS_NewDate(env.ctx, time) }))
    })
}

pub(in super::super::super) unsafe extern "C" fn api_is_date(
    env: Environment,
    value: Handle,
    result: *mut bool,
) -> i32 {
    run(env, |env| {
        put(result, unsafe { qjs::JS_IsDate(env.get(value)?) })
    })
}

pub(in super::super::super) unsafe extern "C" fn api_get_date_value(
    env: Environment,
    value: Handle,
    result: *mut f64,
) -> i32 {
    run_js(env, |env| {
        let value = env.get(value)?;
        if !unsafe { qjs::JS_IsDate(value) } {
            return Err(DATE_EXPECTED);
        }
        let mut out = 0.0;
        super::check_int(env, unsafe { qjs::JS_ToFloat64(env.ctx, &mut out, value) })?;
        put(result, out)
    })
}
