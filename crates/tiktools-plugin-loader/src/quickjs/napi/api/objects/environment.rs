//! Environment information, script execution and fatal-error entry points.
use super::super::super::{
    env::{put, run, run_js},
    types::*,
};
use super::super::{check, read_bytes};
use rquickjs::qjs;
use std::ffi::{c_char, c_void};

pub(in super::super::super) unsafe extern "C" fn api_get_last_error_info(
    env: Environment,
    result: *mut *const ExtendedErrorInfo,
) -> i32 {
    let Some(env) = (unsafe { env.as_ref() }) else {
        return INVALID_ARG;
    };
    if result.is_null() {
        return INVALID_ARG;
    }
    unsafe { result.write(env.error_info()) };
    OK
}
pub(in super::super::super) unsafe extern "C" fn api_get_version(
    env: Environment,
    result: *mut u32,
) -> i32 {
    run(env, |_| put(result, 9))
}
static NODE_VERSION: NodeVersionHolder = NodeVersionHolder(NodeVersion {
    major: 22,
    minor: 0,
    patch: 0,
    release: c"node".as_ptr(),
});
struct NodeVersionHolder(NodeVersion);
unsafe impl Sync for NodeVersionHolder {}
pub(in super::super::super) unsafe extern "C" fn api_get_node_version(
    env: Environment,
    result: *mut *const NodeVersion,
) -> i32 {
    run(env, |_| put(result, &NODE_VERSION.0 as *const NodeVersion))
}
pub(in super::super::super) unsafe extern "C" fn api_get_module_file_name(
    env: Environment,
    result: *mut *const c_char,
) -> i32 {
    run(env, |env| put(result, env.module_url.as_ptr()))
}
pub(in super::super::super) unsafe extern "C" fn api_get_uv_event_loop(
    env: Environment,
    result: *mut *mut c_void,
) -> i32 {
    run(env, |_| {
        put(result, std::ptr::null_mut())?;
        Err(GENERIC_FAILURE)
    })
}
pub(in super::super::super) unsafe extern "C" fn api_adjust_external_memory(
    env: Environment,
    change: i64,
    result: *mut i64,
) -> i32 {
    run(env, |env| {
        env.external_memory
            .set(env.external_memory.get().saturating_add(change));
        put(result, env.external_memory.get())
    })
}
pub(in super::super::super) unsafe extern "C" fn api_run_script(
    env: Environment,
    script: Handle,
    result: *mut Handle,
) -> i32 {
    run_js(env, |env| {
        let script = env.get(script)?;
        if !unsafe { qjs::JS_IsString(script) } {
            return Err(STRING_EXPECTED);
        }
        let mut length = 0_u64;
        let ptr = unsafe { qjs::JS_ToCStringLen2(env.ctx, &mut length, script, false) };
        if ptr.is_null() {
            return Err(env.capture_exception());
        }
        let value = unsafe {
            qjs::JS_Eval(
                env.ctx,
                ptr,
                length,
                c"<napi_run_script>".as_ptr(),
                qjs::JS_EVAL_TYPE_GLOBAL as i32,
            )
        };
        unsafe { qjs::JS_FreeCString(env.ctx, ptr) };
        put(result, env.push(check(env, value)?))
    })
}
pub(in super::super::super) unsafe extern "C" fn api_module_register(module: *mut c_void) {
    if module.is_null() {
        return;
    }
    super::super::super::REGISTRATIONS.with(|registrations| {
        if let Ok(mut registrations) = registrations.try_borrow_mut() {
            if registrations.len() < 1024 {
                registrations.push(module as usize);
            }
        }
    });
}
pub(in super::super::super) unsafe extern "C" fn api_fatal_error(
    location: *const c_char,
    location_length: usize,
    message: *const c_char,
    message_length: usize,
) {
    let text = |ptr: *const c_char, len: usize| {
        unsafe { read_bytes(ptr, len.min(64 * 1024)) }
            .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
            .unwrap_or_default()
    };
    eprintln!(
        "FATAL ERROR: {} {}",
        text(location, location_length),
        text(message, message_length)
    );
    std::process::abort();
}
pub(in super::super::super) unsafe extern "C" fn api_fatal_exception(
    env: Environment,
    error: Handle,
) -> i32 {
    run(env, |env| {
        let value = env.get(error)?;
        let dup = unsafe { qjs::JS_DupValue(env.ctx, value) };
        env.set_pending(dup);
        super::super::super::functions::report_uncaught(env);
        Ok(())
    })
}
