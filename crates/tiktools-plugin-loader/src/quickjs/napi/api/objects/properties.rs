//! Object construction, prototypes and named/keyed property access.
use super::super::super::{
    env::{put, run, run_js},
    types::*,
};
use super::super::{check, check_int, Atom};
use super::object_arg;
use rquickjs::qjs;
use std::ffi::{c_char, CStr};

pub(in super::super::super) unsafe extern "C" fn api_create_object(
    env: Environment,
    result: *mut Handle,
) -> i32 {
    run(env, |env| {
        put(result, env.push(unsafe { qjs::JS_NewObject(env.ctx) }))
    })
}

pub(in super::super::super) unsafe extern "C" fn api_get_prototype(
    env: Environment,
    object: Handle,
    result: *mut Handle,
) -> i32 {
    run_js(env, |env| {
        let object = object_arg(env, object)?;
        let proto = unsafe { qjs::JS_GetPrototype(env.ctx, object) };
        put(result, env.push(check(env, proto)?))
    })
}

pub(in super::super::super) unsafe extern "C" fn api_set_prototype(
    env: Environment,
    object: Handle,
    proto: Handle,
) -> i32 {
    run_js(env, |env| {
        let (object, proto) = (object_arg(env, object)?, env.get(proto)?);
        check_int(env, unsafe { qjs::JS_SetPrototype(env.ctx, object, proto) })?;
        Ok(())
    })
}

pub(in super::super::super) unsafe extern "C" fn api_set_named_property(
    env: Environment,
    object: Handle,
    name: *const c_char,
    value: Handle,
) -> i32 {
    run_js(env, |env| {
        let object = object_arg(env, object)?;
        let value = env.get(value)?;
        if name.is_null() {
            return Err(INVALID_ARG);
        }
        let dup = unsafe { qjs::JS_DupValue(env.ctx, value) };
        check_int(env, unsafe {
            qjs::JS_SetPropertyStr(env.ctx, object, name, dup)
        })?;
        Ok(())
    })
}

pub(in super::super::super) unsafe extern "C" fn api_get_named_property(
    env: Environment,
    object: Handle,
    name: *const c_char,
    result: *mut Handle,
) -> i32 {
    run_js(env, |env| {
        let object = object_arg(env, object)?;
        if name.is_null() {
            return Err(INVALID_ARG);
        }
        let value = unsafe { qjs::JS_GetPropertyStr(env.ctx, object, name) };
        put(result, env.push(check(env, value)?))
    })
}

pub(in super::super::super) unsafe extern "C" fn api_has_named_property(
    env: Environment,
    object: Handle,
    name: *const c_char,
    result: *mut bool,
) -> i32 {
    run_js(env, |env| {
        let object = object_arg(env, object)?;
        if name.is_null() {
            return Err(INVALID_ARG);
        }
        let atom = Atom::from_name(env, unsafe { CStr::from_ptr(name) })?;
        let found = unsafe { qjs::JS_HasProperty(env.ctx, object, atom.atom) };
        put(result, check_int(env, found)? > 0)
    })
}

pub(in super::super::super) unsafe extern "C" fn api_get_property(
    env: Environment,
    object: Handle,
    key: Handle,
    result: *mut Handle,
) -> i32 {
    run_js(env, |env| {
        let (object, key) = (object_arg(env, object)?, env.get(key)?);
        let atom = Atom::from_value(env, key)?;
        let value = unsafe { qjs::JS_GetProperty(env.ctx, object, atom.atom) };
        put(result, env.push(check(env, value)?))
    })
}

pub(in super::super::super) unsafe extern "C" fn api_set_property(
    env: Environment,
    object: Handle,
    key: Handle,
    value: Handle,
) -> i32 {
    run_js(env, |env| {
        let (object, key, value) = (object_arg(env, object)?, env.get(key)?, env.get(value)?);
        let atom = Atom::from_value(env, key)?;
        let dup = unsafe { qjs::JS_DupValue(env.ctx, value) };
        check_int(env, unsafe {
            qjs::JS_SetProperty(env.ctx, object, atom.atom, dup)
        })?;
        Ok(())
    })
}

pub(in super::super::super) unsafe extern "C" fn api_has_property(
    env: Environment,
    object: Handle,
    key: Handle,
    result: *mut bool,
) -> i32 {
    run_js(env, |env| {
        let (object, key) = (object_arg(env, object)?, env.get(key)?);
        let atom = Atom::from_value(env, key)?;
        let found = unsafe { qjs::JS_HasProperty(env.ctx, object, atom.atom) };
        put(result, check_int(env, found)? > 0)
    })
}

pub(in super::super::super) unsafe extern "C" fn api_delete_property(
    env: Environment,
    object: Handle,
    key: Handle,
    result: *mut bool,
) -> i32 {
    run_js(env, |env| {
        let (object, key) = (object_arg(env, object)?, env.get(key)?);
        let atom = Atom::from_value(env, key)?;
        let removed = unsafe { qjs::JS_DeleteProperty(env.ctx, object, atom.atom, 0) };
        let removed = check_int(env, removed)? > 0;
        if result.is_null() {
            return Ok(());
        }
        put(result, removed)
    })
}

pub(in super::super::super) unsafe extern "C" fn api_has_own_property(
    env: Environment,
    object: Handle,
    key: Handle,
    result: *mut bool,
) -> i32 {
    run_js(env, |env| {
        let (object, key) = (object_arg(env, object)?, env.get(key)?);
        if !unsafe { qjs::JS_IsString(key) || qjs::JS_IsSymbol(key) } {
            return Err(NAME_EXPECTED);
        }
        let atom = Atom::from_value(env, key)?;
        let mut descriptor: qjs::JSPropertyDescriptor = unsafe { std::mem::zeroed() };
        let found = check_int(env, unsafe {
            qjs::JS_GetOwnProperty(env.ctx, &mut descriptor, object, atom.atom)
        })?;
        if found > 0 {
            unsafe {
                qjs::JS_FreeValue(env.ctx, descriptor.value);
                qjs::JS_FreeValue(env.ctx, descriptor.getter);
                qjs::JS_FreeValue(env.ctx, descriptor.setter);
            }
        }
        put(result, found > 0)
    })
}

pub(in super::super::super) unsafe extern "C" fn api_object_freeze(
    env: Environment,
    object: Handle,
) -> i32 {
    run_js(env, |env| {
        let object = object_arg(env, object)?;
        check_int(env, unsafe { qjs::JS_FreezeObject(env.ctx, object) })?;
        Ok(())
    })
}

pub(in super::super::super) unsafe extern "C" fn api_object_seal(
    env: Environment,
    object: Handle,
) -> i32 {
    run_js(env, |env| {
        let object = object_arg(env, object)?;
        check_int(env, unsafe { qjs::JS_SealObject(env.ctx, object) })?;
        Ok(())
    })
}

pub(in super::super::super) unsafe extern "C" fn api_instanceof(
    env: Environment,
    object: Handle,
    constructor: Handle,
    result: *mut bool,
) -> i32 {
    run_js(env, |env| {
        let (object, constructor) = (env.get(object)?, env.get(constructor)?);
        if !unsafe { qjs::JS_IsFunction(env.ctx, constructor) } {
            let error = super::super::make_error(
                env,
                c"TypeError",
                None,
                "Constructor must be a function",
            )?;
            env.set_pending(error);
            return Err(FUNCTION_EXPECTED);
        }
        let found = unsafe { qjs::JS_IsInstanceOf(env.ctx, object, constructor) };
        put(result, check_int(env, found)? > 0)
    })
}
