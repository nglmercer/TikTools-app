//! Value classification, boolean access, coercions and strict equality.
use super::*;

pub(in super::super::super) unsafe extern "C" fn api_get_boolean(
    env: Environment,
    value: bool,
    result: *mut Handle,
) -> i32 {
    run(env, |env| put(result, env.push(bool_value(value))))
}

pub(in super::super::super) unsafe extern "C" fn api_typeof_value(
    env: Environment,
    value: Handle,
    result: *mut i32,
) -> i32 {
    run(env, |env| {
        let value = env.get(value)?;
        let kind = unsafe {
            if qjs::JS_IsUndefined(value) {
                TYPE_UNDEFINED
            } else if qjs::JS_IsNull(value) {
                TYPE_NULL
            } else if qjs::JS_IsBool(value) {
                TYPE_BOOLEAN
            } else if qjs::JS_IsNumber(value) {
                TYPE_NUMBER
            } else if qjs::JS_IsString(value) {
                TYPE_STRING
            } else if qjs::JS_IsSymbol(value) {
                TYPE_SYMBOL
            } else if qjs::JS_IsBigInt(value) {
                TYPE_BIGINT
            } else if qjs::JS_IsFunction(env.ctx, value) {
                TYPE_FUNCTION
            } else if external_rec(env, value).is_some() {
                TYPE_EXTERNAL
            } else if qjs::JS_IsObject(value) {
                TYPE_OBJECT
            } else {
                return Err(INVALID_ARG);
            }
        };
        put(result, kind)
    })
}

pub(in super::super::super) unsafe extern "C" fn api_get_value_bool(
    env: Environment,
    value: Handle,
    result: *mut bool,
) -> i32 {
    run(env, |env| {
        let value = env.get(value)?;
        if !unsafe { qjs::JS_IsBool(value) } {
            return Err(BOOLEAN_EXPECTED);
        }
        put(result, unsafe { qjs::JS_ToBool(env.ctx, value) } > 0)
    })
}

pub(in super::super::super) unsafe extern "C" fn api_coerce_to_bool(
    env: Environment,
    value: Handle,
    result: *mut Handle,
) -> i32 {
    run_js(env, |env| {
        let flag = unsafe { qjs::JS_ToBool(env.ctx, env.get(value)?) };
        put(
            result,
            env.push(bool_value(super::check_int(env, flag)? > 0)),
        )
    })
}

pub(in super::super::super) unsafe extern "C" fn api_coerce_to_number(
    env: Environment,
    value: Handle,
    result: *mut Handle,
) -> i32 {
    run_js(env, |env| {
        let mut out = 0.0;
        super::check_int(env, unsafe {
            qjs::JS_ToFloat64(env.ctx, &mut out, env.get(value)?)
        })?;
        put(result, env.push(number(out)))
    })
}

pub(in super::super::super) unsafe extern "C" fn api_coerce_to_string(
    env: Environment,
    value: Handle,
    result: *mut Handle,
) -> i32 {
    run_js(env, |env| {
        let string = unsafe { qjs::JS_ToString(env.ctx, env.get(value)?) };
        put(result, env.push(check(env, string)?))
    })
}

pub(in super::super::super) unsafe extern "C" fn api_coerce_to_object(
    env: Environment,
    value: Handle,
    result: *mut Handle,
) -> i32 {
    run_js(env, |env| {
        let value = env.get(value)?;
        if unsafe { qjs::JS_IsUndefined(value) || qjs::JS_IsNull(value) } {
            return Err(INVALID_ARG);
        }
        if is_object(value) {
            return put(result, env.push_dup(value));
        }
        let ctor = Owned::new(env, global_get(env, c"Object"));
        let mut args = [value];
        let object =
            unsafe { qjs::JS_Call(env.ctx, ctor.value, qjs::JS_UNDEFINED, 1, args.as_mut_ptr()) };
        put(result, env.push(check(env, object)?))
    })
}

pub(in super::super::super) unsafe extern "C" fn api_strict_equals(
    env: Environment,
    a: Handle,
    b: Handle,
    result: *mut bool,
) -> i32 {
    run(env, |env| {
        let (a, b) = (env.get(a)?, env.get(b)?);
        let same = unsafe { qjs::JS_IsStrictEqual(env.ctx, a, b) };
        put(result, same)
    })
}
