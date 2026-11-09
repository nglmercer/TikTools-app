//! Functions, calls, errors, exceptions and promises.

use std::ffi::{c_char, c_void, CStr};

use rquickjs::qjs::{self, JSValue};

use super::{
    super::{
        env::{put, run, run_js, Env},
        functions::{new_function, CallbackInfo},
        types::*,
    },
    check, check_int, global_get, new_string, read_bytes, Owned,
};

pub(in super::super) unsafe extern "C" fn api_create_function(
    env: Environment,
    name: *const c_char,
    length: usize,
    cb: Callback,
    data: *mut c_void,
    result: *mut Handle,
) -> i32 {
    run(env, |env| {
        if cb.is_none() {
            return Err(INVALID_ARG);
        }
        let name = String::from_utf8_lossy(unsafe { read_bytes(name, length)? }).into_owned();
        put(result, env.push(new_function(env, &name, cb, data)))
    })
}

pub(in super::super) unsafe extern "C" fn api_get_cb_info(
    env: Environment,
    info: CbInfo,
    argc: *mut usize,
    argv: *mut Handle,
    this_arg: *mut Handle,
    data: *mut *mut c_void,
) -> i32 {
    run(env, |env| {
        if info.is_null() {
            return Err(INVALID_ARG);
        }
        let info = unsafe { &*(info as *const CallbackInfo) };
        if !argc.is_null() {
            let capacity = unsafe { argc.read() };
            if !argv.is_null() {
                for index in 0..capacity {
                    let value = if index < info.argc {
                        unsafe { info.argv.add(index).read() }
                    } else {
                        qjs::JS_UNDEFINED
                    };
                    unsafe { argv.add(index).write(env.push_dup(value)) };
                }
            }
            unsafe { argc.write(info.argc) };
        }
        if !this_arg.is_null() {
            unsafe { this_arg.write(env.push_dup(info.this)) };
        }
        if !data.is_null() {
            unsafe { data.write(info.data) };
        }
        Ok(())
    })
}

pub(in super::super) unsafe extern "C" fn api_get_new_target(
    env: Environment,
    info: CbInfo,
    result: *mut Handle,
) -> i32 {
    run(env, |env| {
        if info.is_null() {
            return Err(INVALID_ARG);
        }
        let info = unsafe { &*(info as *const CallbackInfo) };
        let handle = if unsafe { qjs::JS_IsUndefined(info.new_target) } {
            std::ptr::null_mut()
        } else {
            env.push_dup(info.new_target)
        };
        put(result, handle)
    })
}

fn collect_args(env: &Env, argc: usize, argv: *const Handle) -> Result<Vec<JSValue>, i32> {
    if argc > 0 && argv.is_null() {
        return Err(INVALID_ARG);
    }
    (0..argc)
        .map(|index| env.get(unsafe { argv.add(index).read() }))
        .collect()
}

unsafe fn call_with(
    env: &Env,
    recv: Handle,
    func: Handle,
    argc: usize,
    argv: *const Handle,
    result: *mut Handle,
) -> Result<(), i32> {
    let this = if recv.is_null() {
        qjs::JS_UNDEFINED
    } else {
        env.get(recv)?
    };
    let func = env.get(func)?;
    if !unsafe { qjs::JS_IsFunction(env.ctx, func) } {
        return Err(FUNCTION_EXPECTED);
    }
    let mut args = collect_args(env, argc, argv)?;
    let value = unsafe { qjs::JS_Call(env.ctx, func, this, args.len() as i32, args.as_mut_ptr()) };
    let value = check(env, value)?;
    if result.is_null() {
        unsafe { qjs::JS_FreeValue(env.ctx, value) };
        Ok(())
    } else {
        put(result, env.push(value))
    }
}

pub(in super::super) unsafe extern "C" fn api_call_function(
    env: Environment,
    recv: Handle,
    func: Handle,
    argc: usize,
    argv: *const Handle,
    result: *mut Handle,
) -> i32 {
    run_js(env, |env| unsafe {
        call_with(env, recv, func, argc, argv, result)
    })
}

pub(in super::super) unsafe extern "C" fn api_make_callback(
    env: Environment,
    _context: Opaque,
    recv: Handle,
    func: Handle,
    argc: usize,
    argv: *const Handle,
    result: *mut Handle,
) -> i32 {
    run_js(env, |env| unsafe {
        call_with(env, recv, func, argc, argv, result)
    })
}

pub(in super::super) unsafe extern "C" fn api_new_instance(
    env: Environment,
    constructor: Handle,
    argc: usize,
    argv: *const Handle,
    result: *mut Handle,
) -> i32 {
    run_js(env, |env| {
        let constructor = env.get(constructor)?;
        if !unsafe { qjs::JS_IsFunction(env.ctx, constructor) } {
            return Err(FUNCTION_EXPECTED);
        }
        let mut args = collect_args(env, argc, argv)?;
        let value = unsafe {
            qjs::JS_CallConstructor(env.ctx, constructor, args.len() as i32, args.as_mut_ptr())
        };
        put(result, env.push(check(env, value)?))
    })
}

// ----- errors ----------------------------------------------------------

/// `new <kind>(message)` with an optional `code` property (owned result).
pub(in super::super) fn make_error(
    env: &Env,
    kind: &CStr,
    code: Option<JSValue>,
    message: &str,
) -> Result<JSValue, i32> {
    let text = Owned::new(env, new_string(env, message));
    make_error_from(env, kind, code, text.value)
}

fn make_error_from(
    env: &Env,
    kind: &CStr,
    code: Option<JSValue>,
    message: JSValue,
) -> Result<JSValue, i32> {
    let ctor = Owned::new(env, global_get(env, kind));
    let mut args = [message];
    let error = Owned::new(
        env,
        check(env, unsafe {
            qjs::JS_CallConstructor(env.ctx, ctor.value, 1, args.as_mut_ptr())
        })?,
    );
    if let Some(code) = code {
        let dup = unsafe { qjs::JS_DupValue(env.ctx, code) };
        check_int(env, unsafe {
            qjs::JS_SetPropertyStr(env.ctx, error.value, c"code".as_ptr(), dup)
        })?;
    }
    Ok(error.release())
}

fn create_error_kind(
    env: Environment,
    kind: &CStr,
    code: Handle,
    message: Handle,
    result: *mut Handle,
) -> i32 {
    run(env, |env| {
        let message = env.get(message)?;
        if !unsafe { qjs::JS_IsString(message) } {
            return Err(STRING_EXPECTED);
        }
        let code = if code.is_null() {
            None
        } else {
            let code = env.get(code)?;
            if !unsafe { qjs::JS_IsString(code) } {
                return Err(STRING_EXPECTED);
            }
            Some(code)
        };
        put(result, env.push(make_error_from(env, kind, code, message)?))
    })
}

pub(in super::super) unsafe extern "C" fn api_create_error(
    env: Environment,
    code: Handle,
    message: Handle,
    result: *mut Handle,
) -> i32 {
    create_error_kind(env, c"Error", code, message, result)
}
pub(in super::super) unsafe extern "C" fn api_create_type_error(
    env: Environment,
    code: Handle,
    message: Handle,
    result: *mut Handle,
) -> i32 {
    create_error_kind(env, c"TypeError", code, message, result)
}
pub(in super::super) unsafe extern "C" fn api_create_range_error(
    env: Environment,
    code: Handle,
    message: Handle,
    result: *mut Handle,
) -> i32 {
    create_error_kind(env, c"RangeError", code, message, result)
}
pub(in super::super) unsafe extern "C" fn api_create_syntax_error(
    env: Environment,
    code: Handle,
    message: Handle,
    result: *mut Handle,
) -> i32 {
    create_error_kind(env, c"SyntaxError", code, message, result)
}

fn throw_kind(env: Environment, kind: &CStr, code: *const c_char, message: *const c_char) -> i32 {
    run_js(env, |env| {
        let message = if message.is_null() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(message) }
                .to_string_lossy()
                .into_owned()
        };
        let code_value = if code.is_null() {
            None
        } else {
            let text = unsafe { CStr::from_ptr(code) }
                .to_string_lossy()
                .into_owned();
            Some(Owned::new(env, new_string(env, &text)))
        };
        let error = make_error(env, kind, code_value.as_ref().map(|c| c.value), &message)?;
        env.set_pending(error);
        Ok(())
    })
}

pub(in super::super) unsafe extern "C" fn api_throw_error(
    env: Environment,
    code: *const c_char,
    message: *const c_char,
) -> i32 {
    throw_kind(env, c"Error", code, message)
}
pub(in super::super) unsafe extern "C" fn api_throw_type_error(
    env: Environment,
    code: *const c_char,
    message: *const c_char,
) -> i32 {
    throw_kind(env, c"TypeError", code, message)
}
pub(in super::super) unsafe extern "C" fn api_throw_range_error(
    env: Environment,
    code: *const c_char,
    message: *const c_char,
) -> i32 {
    throw_kind(env, c"RangeError", code, message)
}
pub(in super::super) unsafe extern "C" fn api_throw_syntax_error(
    env: Environment,
    code: *const c_char,
    message: *const c_char,
) -> i32 {
    throw_kind(env, c"SyntaxError", code, message)
}

pub(in super::super) unsafe extern "C" fn api_throw_value(env: Environment, error: Handle) -> i32 {
    run_js(env, |env| {
        let value = env.get(error)?;
        let dup = unsafe { qjs::JS_DupValue(env.ctx, value) };
        env.set_pending(dup);
        Ok(())
    })
}

pub(in super::super) unsafe extern "C" fn api_is_error(
    env: Environment,
    value: Handle,
    result: *mut bool,
) -> i32 {
    run(env, |env| {
        put(result, unsafe { qjs::JS_IsError(env.get(value)?) })
    })
}

pub(in super::super) unsafe extern "C" fn api_is_exception_pending(
    env: Environment,
    result: *mut bool,
) -> i32 {
    run(env, |env| put(result, env.has_pending()))
}

pub(in super::super) unsafe extern "C" fn api_get_and_clear_last_exception(
    env: Environment,
    result: *mut Handle,
) -> i32 {
    run(env, |env| {
        let value = env.take_pending().unwrap_or(qjs::JS_UNDEFINED);
        put(result, env.push(value))
    })
}

// ----- promises ---------------------------------------------------------

pub(in super::super) unsafe extern "C" fn api_create_promise(
    env: Environment,
    deferred: *mut Opaque,
    promise: *mut Handle,
) -> i32 {
    run_js(env, |env| {
        if deferred.is_null() || promise.is_null() {
            return Err(INVALID_ARG);
        }
        let mut functions = [qjs::JS_UNDEFINED; 2];
        let value = check(env, unsafe {
            qjs::JS_NewPromiseCapability(env.ctx, functions.as_mut_ptr())
        })?;
        let boxed = Box::into_raw(Box::new(functions));
        unsafe {
            deferred.write(boxed.cast());
            promise.write(env.push(value));
        }
        Ok(())
    })
}

unsafe fn settle_deferred(
    env: &Env,
    deferred: Opaque,
    value: Handle,
    index: usize,
) -> Result<(), i32> {
    if deferred.is_null() {
        return Err(INVALID_ARG);
    }
    let value = env.get(value)?;
    let functions = unsafe { Box::from_raw(deferred as *mut [JSValue; 2]) };
    let mut args = [value];
    let outcome = unsafe {
        qjs::JS_Call(
            env.ctx,
            functions[index],
            qjs::JS_UNDEFINED,
            1,
            args.as_mut_ptr(),
        )
    };
    unsafe {
        qjs::JS_FreeValue(env.ctx, functions[0]);
        qjs::JS_FreeValue(env.ctx, functions[1]);
    }
    let outcome = check(env, outcome)?;
    unsafe { qjs::JS_FreeValue(env.ctx, outcome) };
    Ok(())
}

pub(in super::super) unsafe extern "C" fn api_resolve_deferred(
    env: Environment,
    deferred: Opaque,
    value: Handle,
) -> i32 {
    run_js(env, |env| unsafe {
        settle_deferred(env, deferred, value, 0)
    })
}
pub(in super::super) unsafe extern "C" fn api_reject_deferred(
    env: Environment,
    deferred: Opaque,
    value: Handle,
) -> i32 {
    run_js(env, |env| unsafe {
        settle_deferred(env, deferred, value, 1)
    })
}
pub(in super::super) unsafe extern "C" fn api_is_promise(
    env: Environment,
    value: Handle,
    result: *mut bool,
) -> i32 {
    run(env, |env| {
        put(result, unsafe { qjs::JS_IsPromise(env.get(value)?) })
    })
}
