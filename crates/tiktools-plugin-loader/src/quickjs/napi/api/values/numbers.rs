//! Numeric value creation and checked primitive extraction.
use super::*;

pub(super) fn number(value: f64) -> JSValue {
    qjs::JS_NewFloat64(value)
}

pub(in super::super::super) unsafe extern "C" fn api_create_double(
    env: Environment,
    value: f64,
    result: *mut Handle,
) -> i32 {
    run(env, |env| put(result, env.push(number(value))))
}

pub(in super::super::super) unsafe extern "C" fn api_create_int32(
    env: Environment,
    value: i32,
    result: *mut Handle,
) -> i32 {
    run(env, |env| put(result, env.push(number(f64::from(value)))))
}

pub(in super::super::super) unsafe extern "C" fn api_create_uint32(
    env: Environment,
    value: u32,
    result: *mut Handle,
) -> i32 {
    run(env, |env| put(result, env.push(number(f64::from(value)))))
}

pub(in super::super::super) unsafe extern "C" fn api_create_int64(
    env: Environment,
    value: i64,
    result: *mut Handle,
) -> i32 {
    run(env, |env| put(result, env.push(number(value as f64))))
}

fn to_f64(env: &Env, value: JSValue) -> Result<f64, i32> {
    if !unsafe { qjs::JS_IsNumber(value) } {
        return Err(NUMBER_EXPECTED);
    }
    let mut out = 0.0;
    if unsafe { qjs::JS_ToFloat64(env.ctx, &mut out, value) } < 0 {
        return Err(env.capture_exception());
    }
    Ok(out)
}

pub(in super::super::super) unsafe extern "C" fn api_get_value_double(
    env: Environment,
    value: Handle,
    result: *mut f64,
) -> i32 {
    run(env, |env| put(result, to_f64(env, env.get(value)?)?))
}

pub(in super::super::super) unsafe extern "C" fn api_get_value_int32(
    env: Environment,
    value: Handle,
    result: *mut i32,
) -> i32 {
    run(env, |env| {
        let number = to_f64(env, env.get(value)?)?;
        let out = if number.is_finite() {
            (number as i64) as i32
        } else {
            0
        };
        put(result, out)
    })
}

pub(in super::super::super) unsafe extern "C" fn api_get_value_uint32(
    env: Environment,
    value: Handle,
    result: *mut u32,
) -> i32 {
    run(env, |env| {
        let number = to_f64(env, env.get(value)?)?;
        let out = if number.is_finite() {
            (number as i64) as u32
        } else {
            0
        };
        put(result, out)
    })
}

pub(in super::super::super) unsafe extern "C" fn api_get_value_int64(
    env: Environment,
    value: Handle,
    result: *mut i64,
) -> i32 {
    run(env, |env| {
        let number = to_f64(env, env.get(value)?)?;
        let out = if number.is_finite() { number as i64 } else { 0 };
        put(result, out)
    })
}
