//! Strong/weak reference ownership and reference counts.
use super::super::super::{
    env::{put, run, run_js, Env, Reference},
    types::*,
};
use super::super::{check, global_get, is_object, Owned};
use rquickjs::qjs::{self, JSValue};
use std::ffi::c_void;

fn new_weakref(env: &Env, target: JSValue) -> Result<JSValue, i32> {
    let ctor = Owned::new(env, global_get(env, c"WeakRef"));
    let mut args = [target];
    check(env, unsafe {
        qjs::JS_CallConstructor(env.ctx, ctor.value, 1, args.as_mut_ptr())
    })
}

/// `weakref.deref()` (owned; `undefined` once collected).
fn deref_weakref(env: &Env, weak: JSValue) -> Result<JSValue, i32> {
    let method = Owned::new(env, unsafe {
        qjs::JS_GetPropertyStr(env.ctx, weak, c"deref".as_ptr())
    });
    check(env, unsafe {
        qjs::JS_Call(env.ctx, method.value, weak, 0, std::ptr::null_mut())
    })
}

pub(super) fn create_reference(env: &Env, target: JSValue, count: u32) -> Result<*mut c_void, i32> {
    let (value, weak) = if count > 0 || !is_object(target) {
        (unsafe { qjs::JS_DupValue(env.ctx, target) }, false)
    } else {
        (new_weakref(env, target)?, true)
    };
    let boxed = Box::new(Reference { value, weak, count });
    let key = &*boxed as *const Reference as usize;
    env.refs.borrow_mut().insert(key, boxed);
    Ok(key as *mut c_void)
}

pub(in super::super::super) unsafe extern "C" fn api_create_reference(
    env: Environment,
    value: Handle,
    count: u32,
    result: *mut *mut c_void,
) -> i32 {
    run_js(env, |env| {
        let target = env.get(value)?;
        let reference = create_reference(env, target, count)?;
        put(result, reference)
    })
}

pub(in super::super::super) unsafe extern "C" fn api_delete_reference(
    env: Environment,
    reference: *mut c_void,
) -> i32 {
    run(env, |env| {
        let removed = env
            .refs
            .borrow_mut()
            .remove(&(reference as usize))
            .ok_or(INVALID_ARG)?;
        unsafe { qjs::JS_FreeValue(env.ctx, removed.value) };
        Ok(())
    })
}

pub(in super::super::super) unsafe extern "C" fn api_reference_ref(
    env: Environment,
    reference: *mut c_void,
    result: *mut u32,
) -> i32 {
    run_js(env, |env| {
        let key = reference as usize;
        let (weak, value) = {
            let refs = env.refs.borrow();
            let entry = refs.get(&key).ok_or(INVALID_ARG)?;
            (entry.weak, entry.value)
        };
        if weak {
            let target = deref_weakref(env, value)?;
            if unsafe { qjs::JS_IsUndefined(target) } {
                return Err(GENERIC_FAILURE);
            }
            let mut refs = env.refs.borrow_mut();
            let entry = refs.get_mut(&key).ok_or(INVALID_ARG)?;
            unsafe { qjs::JS_FreeValue(env.ctx, entry.value) };
            entry.value = target;
            entry.weak = false;
        }
        let count = {
            let mut refs = env.refs.borrow_mut();
            let entry = refs.get_mut(&key).ok_or(INVALID_ARG)?;
            entry.count += 1;
            entry.count
        };
        if result.is_null() {
            return Ok(());
        }
        put(result, count)
    })
}

pub(in super::super::super) unsafe extern "C" fn api_reference_unref(
    env: Environment,
    reference: *mut c_void,
    result: *mut u32,
) -> i32 {
    run_js(env, |env| {
        let key = reference as usize;
        let (count, value) = {
            let mut refs = env.refs.borrow_mut();
            let entry = refs.get_mut(&key).ok_or(INVALID_ARG)?;
            if entry.count == 0 {
                return Err(GENERIC_FAILURE);
            }
            entry.count -= 1;
            (entry.count, entry.value)
        };
        if count == 0 && is_object(value) {
            let weak = new_weakref(env, value)?;
            let mut refs = env.refs.borrow_mut();
            let entry = refs.get_mut(&key).ok_or(INVALID_ARG)?;
            unsafe { qjs::JS_FreeValue(env.ctx, entry.value) };
            entry.value = weak;
            entry.weak = true;
        }
        if result.is_null() {
            return Ok(());
        }
        put(result, count)
    })
}

pub(in super::super::super) unsafe extern "C" fn api_get_reference_value(
    env: Environment,
    reference: *mut c_void,
    result: *mut Handle,
) -> i32 {
    run_js(env, |env| {
        let (weak, value) = {
            let refs = env.refs.borrow();
            let entry = refs.get(&(reference as usize)).ok_or(INVALID_ARG)?;
            (entry.weak, entry.value)
        };
        let handle = if weak {
            let target = deref_weakref(env, value)?;
            if unsafe { qjs::JS_IsUndefined(target) } {
                std::ptr::null_mut()
            } else {
                env.push(target)
            }
        } else {
            env.push_dup(value)
        };
        put(result, handle)
    })
}
