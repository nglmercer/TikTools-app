//! Symbol creation and global symbol registry access.
use super::*;

pub(in super::super::super) unsafe extern "C" fn api_create_symbol(
    env: Environment,
    description: Handle,
    result: *mut Handle,
) -> i32 {
    run(env, |env| {
        let text = if description.is_null() {
            CString::default()
        } else {
            let bytes = string_bytes(env, env.get(description)?)?;
            CString::new(bytes).unwrap_or_default()
        };
        put(
            result,
            env.push(unsafe { qjs::JS_NewSymbol(env.ctx, text.as_ptr(), false) }),
        )
    })
}

pub(in super::super::super) unsafe extern "C" fn api_node_api_symbol_for(
    env: Environment,
    text: *const c_char,
    length: usize,
    result: *mut Handle,
) -> i32 {
    run(env, |env| {
        let bytes = unsafe { read_bytes(text, length)? };
        let name = CString::new(bytes).map_err(|_| INVALID_ARG)?;
        put(
            result,
            env.push(unsafe { qjs::JS_NewSymbol(env.ctx, name.as_ptr(), true) }),
        )
    })
}
