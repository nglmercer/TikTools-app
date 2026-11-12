//! Property descriptors, class construction and initial object properties.
use super::super::super::{
    env::{put, run_js, Env},
    functions::{new_constructor, new_function},
    types::*,
};
use super::super::{check_int, is_object, read_bytes, Atom, Owned};
use rquickjs::qjs::{self, JSValue};
use std::ffi::{c_char, c_void, CStr};

fn property_flags(attributes: i32) -> i32 {
    let mut flags = 0;
    if attributes & PROP_WRITABLE != 0 {
        flags |= qjs::JS_PROP_WRITABLE as i32;
    }
    if attributes & PROP_ENUMERABLE != 0 {
        flags |= qjs::JS_PROP_ENUMERABLE as i32;
    }
    if attributes & PROP_CONFIGURABLE != 0 {
        flags |= qjs::JS_PROP_CONFIGURABLE as i32;
    }
    flags
}

unsafe fn define_property(
    env: &Env,
    target: JSValue,
    descriptor: &PropertyDescriptor,
) -> Result<(), i32> {
    let (atom, name): (Atom<'_>, String) = if descriptor.utf8name.is_null() {
        let key = env.get(descriptor.name)?;
        if !unsafe { qjs::JS_IsString(key) || qjs::JS_IsSymbol(key) } {
            return Err(NAME_EXPECTED);
        }
        (Atom::from_value(env, key)?, String::new())
    } else {
        let text = unsafe { CStr::from_ptr(descriptor.utf8name) };
        (
            Atom::from_name(env, text)?,
            text.to_string_lossy().into_owned(),
        )
    };
    let flags = property_flags(descriptor.attributes) | qjs::JS_PROP_THROW as i32;
    let data = descriptor.data;
    let result = if descriptor.getter.is_some() || descriptor.setter.is_some() {
        let getter = descriptor.getter.map_or(qjs::JS_UNDEFINED, |cb| {
            new_function(env, &name, Some(cb), data)
        });
        let setter = descriptor.setter.map_or(qjs::JS_UNDEFINED, |cb| {
            new_function(env, &name, Some(cb), data)
        });
        let accessor_flags = flags & !(qjs::JS_PROP_WRITABLE as i32);
        unsafe {
            qjs::JS_DefinePropertyGetSet(env.ctx, target, atom.atom, getter, setter, accessor_flags)
        }
    } else if let Some(method) = descriptor.method {
        let function = new_function(env, &name, Some(method), data);
        unsafe { qjs::JS_DefinePropertyValue(env.ctx, target, atom.atom, function, flags) }
    } else {
        let value = env.get(descriptor.value)?;
        let dup = unsafe { qjs::JS_DupValue(env.ctx, value) };
        unsafe { qjs::JS_DefinePropertyValue(env.ctx, target, atom.atom, dup, flags) }
    };
    check_int(env, result)?;
    Ok(())
}

pub(in super::super::super) unsafe extern "C" fn api_define_properties(
    env: Environment,
    object: Handle,
    count: usize,
    properties: *const PropertyDescriptor,
) -> i32 {
    run_js(env, |env| {
        let object = env.get(object)?;
        if !is_object(object) {
            return Err(OBJECT_EXPECTED);
        }
        if properties.is_null() && count > 0 {
            return Err(INVALID_ARG);
        }
        for index in 0..count {
            unsafe { define_property(env, object, &*properties.add(index))? };
        }
        Ok(())
    })
}

pub(in super::super::super) unsafe extern "C" fn api_define_class(
    env: Environment,
    name: *const c_char,
    length: usize,
    constructor: Callback,
    data: *mut c_void,
    count: usize,
    properties: *const PropertyDescriptor,
    result: *mut Handle,
) -> i32 {
    run_js(env, |env| {
        if constructor.is_none() || (properties.is_null() && count > 0) {
            return Err(INVALID_ARG);
        }
        let name = String::from_utf8_lossy(unsafe { read_bytes(name, length)? }).into_owned();
        let function = Owned::new(env, new_constructor(env, &name, constructor, data));
        let proto = Owned::new(env, unsafe { qjs::JS_NewObject(env.ctx) });
        check_int(env, unsafe {
            qjs::JS_SetConstructor(env.ctx, function.value, proto.value)
        })?;
        for index in 0..count {
            let descriptor = unsafe { &*properties.add(index) };
            let target = if descriptor.attributes & PROP_STATIC != 0 {
                function.value
            } else {
                proto.value
            };
            unsafe { define_property(env, target, descriptor)? };
        }
        put(result, env.push(function.release()))
    })
}

pub(in super::super::super) unsafe extern "C" fn api_create_object_with_properties(
    env: Environment,
    prototype: Handle,
    names: *mut Handle,
    values: *mut Handle,
    count: usize,
    result: *mut Handle,
) -> i32 {
    run_js(env, |env| {
        let object = if prototype.is_null() {
            Owned::new(env, unsafe { qjs::JS_NewObject(env.ctx) })
        } else {
            let proto = env.get(prototype)?;
            Owned::new(env, unsafe { qjs::JS_NewObjectProto(env.ctx, proto) })
        };
        if (names.is_null() || values.is_null()) && count > 0 {
            return Err(INVALID_ARG);
        }
        for index in 0..count {
            let key = env.get(unsafe { names.add(index).read() })?;
            let value = env.get(unsafe { values.add(index).read() })?;
            let atom = Atom::from_value(env, key)?;
            let dup = unsafe { qjs::JS_DupValue(env.ctx, value) };
            check_int(env, unsafe {
                qjs::JS_DefinePropertyValue(env.ctx, object.value, atom.atom, dup, 7)
            })?;
        }
        put(result, env.push(object.release()))
    })
}
