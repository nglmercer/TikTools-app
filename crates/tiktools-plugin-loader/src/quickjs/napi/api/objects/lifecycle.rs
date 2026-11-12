//! Per-environment instance data and synchronous/asynchronous cleanup hooks.
use super::super::super::{
    env::{put, run},
    types::*,
};
use std::ffi::c_void;

pub(in super::super::super) unsafe extern "C" fn api_set_instance_data(
    env: Environment,
    data: *mut c_void,
    finalize: Finalize,
    hint: *mut c_void,
) -> i32 {
    run(env, |env| {
        env.instance_data.set((data, finalize, hint));
        Ok(())
    })
}
pub(in super::super::super) unsafe extern "C" fn api_get_instance_data(
    env: Environment,
    result: *mut *mut c_void,
) -> i32 {
    run(env, |env| put(result, env.instance_data.get().0))
}

pub(in super::super::super) unsafe extern "C" fn api_add_env_cleanup_hook(
    env: Environment,
    hook: CleanupHook,
    arg: *mut c_void,
) -> i32 {
    run(env, |env| {
        if hook.is_none() {
            return Err(INVALID_ARG);
        }
        env.cleanup_hooks.borrow_mut().push((hook, arg));
        Ok(())
    })
}
pub(in super::super::super) unsafe extern "C" fn api_remove_env_cleanup_hook(
    env: Environment,
    hook: CleanupHook,
    arg: *mut c_void,
) -> i32 {
    run(env, |env| {
        let mut hooks = env.cleanup_hooks.borrow_mut();
        if let Some(position) = hooks
            .iter()
            .position(|(h, a)| h.map(|f| f as usize) == hook.map(|f| f as usize) && *a == arg)
        {
            hooks.remove(position);
        }
        Ok(())
    })
}
pub(in super::super::super) unsafe extern "C" fn api_add_async_cleanup_hook(
    env: Environment,
    hook: AsyncCleanupHook,
    arg: *mut c_void,
    handle: *mut Opaque,
) -> i32 {
    run(env, |env| {
        if hook.is_none() {
            return Err(INVALID_ARG);
        }
        let id = Box::into_raw(Box::new(0_u8)) as usize;
        env.async_cleanup_hooks.borrow_mut().push((hook, arg, id));
        if handle.is_null() {
            return Ok(());
        }
        put(handle, id as Opaque)
    })
}
pub(in super::super::super) unsafe extern "C" fn api_remove_async_cleanup_hook(handle: Opaque) {
    super::super::super::env::mark_async_hook_removed(handle as usize);
}
