//! Objects, properties, classes, wrapping, references, scopes and the
//! miscellaneous environment APIs.

use std::ffi::{c_char, c_void, CStr, CString};

use rquickjs::qjs::{self, JSValue};

use super::{
    super::{
        env::{put, run, run_js, Env, PendingFinalizer, Reference},
        functions::{external_rec, new_constructor, new_external, new_function},
        types::*,
    },
    bool_value, check, check_int, global_get, is_object, new_string, read_bytes, Atom, Owned,
};

fn object_arg(env: &Env, handle: Handle) -> Result<JSValue, i32> {
    let value = env.get(handle)?;
    if is_object(value) {
        Ok(value)
    } else if unsafe { qjs::JS_IsUndefined(value) || qjs::JS_IsNull(value) } {
        Err(OBJECT_EXPECTED)
    } else {
        Ok(value)
    }
}

// ----- arrays ---------------------------------------------------------

pub(in super::super) unsafe extern "C" fn api_create_array(env: Environment, result: *mut Handle) -> i32 {
    run(env, |env| put(result, env.push(unsafe { qjs::JS_NewArray(env.ctx) })))
}

pub(in super::super) unsafe extern "C" fn api_create_array_with_length(env: Environment, length: usize, result: *mut Handle) -> i32 {
    run(env, |env| {
        let array = Owned::new(env, unsafe { qjs::JS_NewArray(env.ctx) });
        check_int(env, unsafe { qjs::JS_SetLength(env.ctx, array.value, length as i64) })?;
        put(result, env.push(array.release()))
    })
}

pub(in super::super) unsafe extern "C" fn api_is_array(env: Environment, value: Handle, result: *mut bool) -> i32 {
    run(env, |env| put(result, unsafe { qjs::JS_IsArray(env.get(value)?) }))
}

pub(in super::super) unsafe extern "C" fn api_get_array_length(env: Environment, value: Handle, result: *mut u32) -> i32 {
    run(env, |env| {
        let value = env.get(value)?;
        if !unsafe { qjs::JS_IsArray(value) } {
            return Err(ARRAY_EXPECTED);
        }
        let mut length = 0_i64;
        check_int(env, unsafe { qjs::JS_GetLength(env.ctx, value, &mut length) })?;
        put(result, length as u32)
    })
}

pub(in super::super) unsafe extern "C" fn api_get_element(env: Environment, object: Handle, index: u32, result: *mut Handle) -> i32 {
    run_js(env, |env| {
        let object = object_arg(env, object)?;
        let value = unsafe { qjs::JS_GetPropertyUint32(env.ctx, object, index) };
        put(result, env.push(check(env, value)?))
    })
}

pub(in super::super) unsafe extern "C" fn api_set_element(env: Environment, object: Handle, index: u32, value: Handle) -> i32 {
    run_js(env, |env| {
        let (object, value) = (object_arg(env, object)?, env.get(value)?);
        let dup = unsafe { qjs::JS_DupValue(env.ctx, value) };
        check_int(env, unsafe { qjs::JS_SetPropertyUint32(env.ctx, object, index, dup) })?;
        Ok(())
    })
}

pub(in super::super) unsafe extern "C" fn api_has_element(env: Environment, object: Handle, index: u32, result: *mut bool) -> i32 {
    run_js(env, |env| {
        let object = object_arg(env, object)?;
        let atom = unsafe { qjs::JS_NewAtomUInt32(env.ctx, index) };
        let found = unsafe { qjs::JS_HasProperty(env.ctx, object, atom) };
        unsafe { qjs::JS_FreeAtom(env.ctx, atom) };
        put(result, check_int(env, found)? > 0)
    })
}

pub(in super::super) unsafe extern "C" fn api_delete_element(env: Environment, object: Handle, index: u32, result: *mut bool) -> i32 {
    run_js(env, |env| {
        let object = object_arg(env, object)?;
        let atom = unsafe { qjs::JS_NewAtomUInt32(env.ctx, index) };
        let removed = unsafe { qjs::JS_DeleteProperty(env.ctx, object, atom, 0) };
        unsafe { qjs::JS_FreeAtom(env.ctx, atom) };
        let removed = check_int(env, removed)? > 0;
        if result.is_null() {
            return Ok(());
        }
        put(result, removed)
    })
}

// ----- objects & properties -------------------------------------------

pub(in super::super) unsafe extern "C" fn api_create_object(env: Environment, result: *mut Handle) -> i32 {
    run(env, |env| put(result, env.push(unsafe { qjs::JS_NewObject(env.ctx) })))
}

pub(in super::super) unsafe extern "C" fn api_get_prototype(env: Environment, object: Handle, result: *mut Handle) -> i32 {
    run_js(env, |env| {
        let object = object_arg(env, object)?;
        let proto = unsafe { qjs::JS_GetPrototype(env.ctx, object) };
        put(result, env.push(check(env, proto)?))
    })
}

pub(in super::super) unsafe extern "C" fn api_set_prototype(env: Environment, object: Handle, proto: Handle) -> i32 {
    run_js(env, |env| {
        let (object, proto) = (object_arg(env, object)?, env.get(proto)?);
        check_int(env, unsafe { qjs::JS_SetPrototype(env.ctx, object, proto) })?;
        Ok(())
    })
}

pub(in super::super) unsafe extern "C" fn api_set_named_property(env: Environment, object: Handle, name: *const c_char, value: Handle) -> i32 {
    run_js(env, |env| {
        let object = object_arg(env, object)?;
        let value = env.get(value)?;
        if name.is_null() {
            return Err(INVALID_ARG);
        }
        let dup = unsafe { qjs::JS_DupValue(env.ctx, value) };
        check_int(env, unsafe { qjs::JS_SetPropertyStr(env.ctx, object, name, dup) })?;
        Ok(())
    })
}

pub(in super::super) unsafe extern "C" fn api_get_named_property(env: Environment, object: Handle, name: *const c_char, result: *mut Handle) -> i32 {
    run_js(env, |env| {
        let object = object_arg(env, object)?;
        if name.is_null() {
            return Err(INVALID_ARG);
        }
        let value = unsafe { qjs::JS_GetPropertyStr(env.ctx, object, name) };
        put(result, env.push(check(env, value)?))
    })
}

pub(in super::super) unsafe extern "C" fn api_has_named_property(env: Environment, object: Handle, name: *const c_char, result: *mut bool) -> i32 {
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

pub(in super::super) unsafe extern "C" fn api_get_property(env: Environment, object: Handle, key: Handle, result: *mut Handle) -> i32 {
    run_js(env, |env| {
        let (object, key) = (object_arg(env, object)?, env.get(key)?);
        let atom = Atom::from_value(env, key)?;
        let value = unsafe { qjs::JS_GetProperty(env.ctx, object, atom.atom) };
        put(result, env.push(check(env, value)?))
    })
}

pub(in super::super) unsafe extern "C" fn api_set_property(env: Environment, object: Handle, key: Handle, value: Handle) -> i32 {
    run_js(env, |env| {
        let (object, key, value) = (object_arg(env, object)?, env.get(key)?, env.get(value)?);
        let atom = Atom::from_value(env, key)?;
        let dup = unsafe { qjs::JS_DupValue(env.ctx, value) };
        check_int(env, unsafe { qjs::JS_SetProperty(env.ctx, object, atom.atom, dup) })?;
        Ok(())
    })
}

pub(in super::super) unsafe extern "C" fn api_has_property(env: Environment, object: Handle, key: Handle, result: *mut bool) -> i32 {
    run_js(env, |env| {
        let (object, key) = (object_arg(env, object)?, env.get(key)?);
        let atom = Atom::from_value(env, key)?;
        let found = unsafe { qjs::JS_HasProperty(env.ctx, object, atom.atom) };
        put(result, check_int(env, found)? > 0)
    })
}

pub(in super::super) unsafe extern "C" fn api_delete_property(env: Environment, object: Handle, key: Handle, result: *mut bool) -> i32 {
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

pub(in super::super) unsafe extern "C" fn api_has_own_property(env: Environment, object: Handle, key: Handle, result: *mut bool) -> i32 {
    run_js(env, |env| {
        let (object, key) = (object_arg(env, object)?, env.get(key)?);
        if !unsafe { qjs::JS_IsString(key) || qjs::JS_IsSymbol(key) } {
            return Err(NAME_EXPECTED);
        }
        let atom = Atom::from_value(env, key)?;
        let mut descriptor: qjs::JSPropertyDescriptor = unsafe { std::mem::zeroed() };
        let found = check_int(env, unsafe { qjs::JS_GetOwnProperty(env.ctx, &mut descriptor, object, atom.atom) })?;
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

/// Collects own property keys (string and/or symbol) of one object.
fn own_keys(env: &Env, object: JSValue, mask: u32, enum_only: bool, convert_numbers: bool) -> Result<Vec<JSValue>, i32> {
    let mut table: *mut qjs::JSPropertyEnum = std::ptr::null_mut();
    let mut length = 0_u32;
    let flags = mask | if enum_only { qjs::JS_GPN_ENUM_ONLY } else { 0 };
    check_int(env, unsafe { qjs::JS_GetOwnPropertyNames(env.ctx, &mut table, &mut length, object, flags as i32) })?;
    let mut keys = Vec::with_capacity(length as usize);
    for index in 0..length as usize {
        let atom = unsafe { (*table.add(index)).atom };
        let key = unsafe {
            if convert_numbers {
                qjs::JS_AtomToString(env.ctx, atom)
            } else {
                qjs::JS_AtomToValue(env.ctx, atom)
            }
        };
        keys.push(key);
    }
    unsafe { qjs::JS_FreePropertyEnum(env.ctx, table, length) };
    Ok(keys)
}

fn keys_array(env: &Env, keys: Vec<JSValue>) -> Result<JSValue, i32> {
    let array = unsafe { qjs::JS_NewArray(env.ctx) };
    for (index, key) in keys.into_iter().enumerate() {
        check_int(env, unsafe { qjs::JS_SetPropertyUint32(env.ctx, array, index as u32, key) })?;
    }
    Ok(array)
}

pub(in super::super) unsafe extern "C" fn api_get_property_names(env: Environment, object: Handle, result: *mut Handle) -> i32 {
    unsafe { api_get_all_property_names(env, object, 0, 2 | 16, 1, result) }
}

pub(in super::super) unsafe extern "C" fn api_get_all_property_names(
    env: Environment,
    object: Handle,
    key_mode: i32,
    key_filter: i32,
    key_conversion: i32,
    result: *mut Handle,
) -> i32 {
    run_js(env, |env| {
        let object = object_arg(env, object)?;
        let mut mask = 0;
        if key_filter & 8 == 0 {
            mask |= qjs::JS_GPN_STRING_MASK;
        }
        if key_filter & 16 == 0 {
            mask |= qjs::JS_GPN_SYMBOL_MASK;
        }
        let enum_only = key_filter & 2 != 0;
        let mut collected = Vec::new();
        let mut seen: Vec<String> = Vec::new();
        let mut current = Owned::new(env, unsafe { qjs::JS_DupValue(env.ctx, object) });
        loop {
            for key in own_keys(env, current.value, mask, enum_only, key_conversion == 1)? {
                let text = unsafe {
                    let cstr = qjs::JS_ToCString(env.ctx, key);
                    if cstr.is_null() {
                        String::new()
                    } else {
                        let owned = CStr::from_ptr(cstr).to_string_lossy().into_owned();
                        qjs::JS_FreeCString(env.ctx, cstr);
                        owned
                    }
                };
                if seen.contains(&text) {
                    unsafe { qjs::JS_FreeValue(env.ctx, key) };
                } else {
                    seen.push(text);
                    collected.push(key);
                }
            }
            if key_mode == 1 {
                break;
            }
            let proto = unsafe { qjs::JS_GetPrototype(env.ctx, current.value) };
            if unsafe { !qjs::JS_IsObject(proto) } {
                unsafe { qjs::JS_FreeValue(env.ctx, proto) };
                break;
            }
            current = Owned::new(env, proto);
        }
        put(result, env.push(keys_array(env, collected)?))
    })
}

pub(in super::super) unsafe extern "C" fn api_object_freeze(env: Environment, object: Handle) -> i32 {
    run_js(env, |env| {
        let object = object_arg(env, object)?;
        check_int(env, unsafe { qjs::JS_FreezeObject(env.ctx, object) })?;
        Ok(())
    })
}

pub(in super::super) unsafe extern "C" fn api_object_seal(env: Environment, object: Handle) -> i32 {
    run_js(env, |env| {
        let object = object_arg(env, object)?;
        check_int(env, unsafe { qjs::JS_SealObject(env.ctx, object) })?;
        Ok(())
    })
}

pub(in super::super) unsafe extern "C" fn api_instanceof(env: Environment, object: Handle, constructor: Handle, result: *mut bool) -> i32 {
    run_js(env, |env| {
        let (object, constructor) = (env.get(object)?, env.get(constructor)?);
        if !unsafe { qjs::JS_IsFunction(env.ctx, constructor) } {
            let error = super::make_error(env, c"TypeError", None, "Constructor must be a function")?;
            env.set_pending(error);
            return Err(FUNCTION_EXPECTED);
        }
        let found = unsafe { qjs::JS_IsInstanceOf(env.ctx, object, constructor) };
        put(result, check_int(env, found)? > 0)
    })
}

// ----- property definition / classes ----------------------------------

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

unsafe fn define_property(env: &Env, target: JSValue, descriptor: &PropertyDescriptor) -> Result<(), i32> {
    let (atom, name): (Atom<'_>, String) = if descriptor.utf8name.is_null() {
        let key = env.get(descriptor.name)?;
        if !unsafe { qjs::JS_IsString(key) || qjs::JS_IsSymbol(key) } {
            return Err(NAME_EXPECTED);
        }
        (Atom::from_value(env, key)?, String::new())
    } else {
        let text = unsafe { CStr::from_ptr(descriptor.utf8name) };
        (Atom::from_name(env, text)?, text.to_string_lossy().into_owned())
    };
    let flags = property_flags(descriptor.attributes) | qjs::JS_PROP_THROW as i32;
    let data = descriptor.data;
    let result = if descriptor.getter.is_some() || descriptor.setter.is_some() {
        let getter = descriptor
            .getter
            .map_or(qjs::JS_UNDEFINED, |cb| new_function(env, &name, Some(cb), data));
        let setter = descriptor
            .setter
            .map_or(qjs::JS_UNDEFINED, |cb| new_function(env, &name, Some(cb), data));
        let accessor_flags = flags & !(qjs::JS_PROP_WRITABLE as i32);
        unsafe { qjs::JS_DefinePropertyGetSet(env.ctx, target, atom.atom, getter, setter, accessor_flags) }
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

pub(in super::super) unsafe extern "C" fn api_define_properties(
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

pub(in super::super) unsafe extern "C" fn api_define_class(
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
        check_int(env, unsafe { qjs::JS_SetConstructor(env.ctx, function.value, proto.value) })?;
        for index in 0..count {
            let descriptor = unsafe { &*properties.add(index) };
            let target = if descriptor.attributes & PROP_STATIC != 0 { function.value } else { proto.value };
            unsafe { define_property(env, target, descriptor)? };
        }
        put(result, env.push(function.release()))
    })
}

pub(in super::super) unsafe extern "C" fn api_create_object_with_properties(
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
            check_int(
                env,
                unsafe { qjs::JS_DefinePropertyValue(env.ctx, object.value, atom.atom, dup, 7) },
            )?;
        }
        put(result, env.push(object.release()))
    })
}

// ----- wrap / finalizers / type tags ----------------------------------

pub(in super::super) unsafe extern "C" fn api_wrap(
    env: Environment,
    object: Handle,
    native: *mut c_void,
    finalize: Finalize,
    hint: *mut c_void,
    reference: *mut *mut c_void,
) -> i32 {
    run_js(env, |env| {
        let object = env.get(object)?;
        if !is_object(object) {
            return Err(OBJECT_EXPECTED);
        }
        if own_hidden(env, object, env.wrap_atom).is_some() {
            return Err(INVALID_ARG);
        }
        let holder = check(env, new_external(env, native, finalize, hint))?;
        check_int(env, unsafe {
            qjs::JS_DefinePropertyValue(env.ctx, object, env.wrap_atom, holder, qjs::JS_PROP_CONFIGURABLE as i32)
        })?;
        if !reference.is_null() {
            let reference_ptr = create_reference(env, object, 0)?;
            unsafe { reference.write(reference_ptr) };
        }
        Ok(())
    })
}

/// The external holder stored under a hidden atom, if any.
fn own_hidden(env: &Env, object: JSValue, atom: qjs::JSAtom) -> Option<&'static super::super::functions::ExternalRec> {
    let holder = Owned::new(env, unsafe { qjs::JS_GetProperty(env.ctx, object, atom) });
    external_rec(env, holder.value)
}

pub(in super::super) unsafe extern "C" fn api_unwrap(env: Environment, object: Handle, result: *mut *mut c_void) -> i32 {
    run_js(env, |env| {
        let object = env.get(object)?;
        if !is_object(object) {
            return Err(OBJECT_EXPECTED);
        }
        let rec = own_hidden(env, object, env.wrap_atom).ok_or(INVALID_ARG)?;
        put(result, rec.data)
    })
}

pub(in super::super) unsafe extern "C" fn api_remove_wrap(env: Environment, object: Handle, result: *mut *mut c_void) -> i32 {
    run_js(env, |env| {
        let object = env.get(object)?;
        if !is_object(object) {
            return Err(OBJECT_EXPECTED);
        }
        let rec = own_hidden(env, object, env.wrap_atom).ok_or(INVALID_ARG)?;
        let data = rec.data;
        // The addon takes the pointer back: the finalizer must not run.
        unsafe { (*(rec as *const _ as *mut super::super::functions::ExternalRec)).finalize = None };
        unsafe { qjs::JS_DeleteProperty(env.ctx, object, env.wrap_atom, 0) };
        if !result.is_null() {
            unsafe { result.write(data) };
        }
        Ok(())
    })
}

pub(in super::super) unsafe extern "C" fn api_add_finalizer(
    env: Environment,
    object: Handle,
    data: *mut c_void,
    finalize: Finalize,
    hint: *mut c_void,
    reference: *mut *mut c_void,
) -> i32 {
    run_js(env, |env| {
        let object = env.get(object)?;
        if !is_object(object) || finalize.is_none() {
            return Err(if finalize.is_none() { INVALID_ARG } else { OBJECT_EXPECTED });
        }
        let holder = check(env, new_external(env, data, finalize, hint))?;
        let symbol = Owned::new(env, unsafe { qjs::JS_NewSymbol(env.ctx, c"tiktools.napi.finalizer".as_ptr(), false) });
        let atom = Atom::from_value(env, symbol.value)?;
        check_int(env, unsafe {
            qjs::JS_DefinePropertyValue(env.ctx, object, atom.atom, holder, qjs::JS_PROP_CONFIGURABLE as i32)
        })?;
        if !reference.is_null() {
            let reference_ptr = create_reference(env, object, 0)?;
            unsafe { reference.write(reference_ptr) };
        }
        Ok(())
    })
}

pub(in super::super) unsafe extern "C" fn api_post_finalizer(env: Environment, finalize: Finalize, data: *mut c_void, hint: *mut c_void) -> i32 {
    run(env, |env| {
        env.finalizers.borrow_mut().push_back(PendingFinalizer { cb: finalize, data, hint });
        Ok(())
    })
}

unsafe extern "C" fn free_tag(_env: Environment, data: *mut c_void, _hint: *mut c_void) {
    drop(unsafe { Box::from_raw(data as *mut TypeTag) });
}

pub(in super::super) unsafe extern "C" fn api_type_tag_object(env: Environment, object: Handle, tag: *const TypeTag) -> i32 {
    run_js(env, |env| {
        let object = env.get(object)?;
        if !is_object(object) {
            return Err(OBJECT_EXPECTED);
        }
        if tag.is_null() {
            return Err(INVALID_ARG);
        }
        if own_hidden(env, object, env.tag_atom).is_some() {
            return Err(INVALID_ARG);
        }
        let boxed = Box::into_raw(Box::new(unsafe { *tag }));
        let holder = check(env, new_external(env, boxed.cast(), Some(free_tag), std::ptr::null_mut()))?;
        check_int(env, unsafe {
            qjs::JS_DefinePropertyValue(env.ctx, object, env.tag_atom, holder, 0)
        })?;
        Ok(())
    })
}

pub(in super::super) unsafe extern "C" fn api_check_object_type_tag(env: Environment, object: Handle, tag: *const TypeTag, result: *mut bool) -> i32 {
    run_js(env, |env| {
        let object = env.get(object)?;
        if !is_object(object) {
            return Err(OBJECT_EXPECTED);
        }
        if tag.is_null() {
            return Err(INVALID_ARG);
        }
        let matches = own_hidden(env, object, env.tag_atom)
            .is_some_and(|rec| unsafe { *(rec.data as *const TypeTag) == *tag });
        put(result, matches)
    })
}

// ----- references ------------------------------------------------------

fn new_weakref(env: &Env, target: JSValue) -> Result<JSValue, i32> {
    let ctor = Owned::new(env, global_get(env, c"WeakRef"));
    let mut args = [target];
    check(env, unsafe { qjs::JS_CallConstructor(env.ctx, ctor.value, 1, args.as_mut_ptr()) })
}

/// `weakref.deref()` (owned; `undefined` once collected).
fn deref_weakref(env: &Env, weak: JSValue) -> Result<JSValue, i32> {
    let method = Owned::new(env, unsafe { qjs::JS_GetPropertyStr(env.ctx, weak, c"deref".as_ptr()) });
    check(env, unsafe { qjs::JS_Call(env.ctx, method.value, weak, 0, std::ptr::null_mut()) })
}

fn create_reference(env: &Env, target: JSValue, count: u32) -> Result<*mut c_void, i32> {
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

pub(in super::super) unsafe extern "C" fn api_create_reference(env: Environment, value: Handle, count: u32, result: *mut *mut c_void) -> i32 {
    run_js(env, |env| {
        let target = env.get(value)?;
        let reference = create_reference(env, target, count)?;
        put(result, reference)
    })
}

pub(in super::super) unsafe extern "C" fn api_delete_reference(env: Environment, reference: *mut c_void) -> i32 {
    run(env, |env| {
        let removed = env.refs.borrow_mut().remove(&(reference as usize)).ok_or(INVALID_ARG)?;
        unsafe { qjs::JS_FreeValue(env.ctx, removed.value) };
        Ok(())
    })
}

pub(in super::super) unsafe extern "C" fn api_reference_ref(env: Environment, reference: *mut c_void, result: *mut u32) -> i32 {
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

pub(in super::super) unsafe extern "C" fn api_reference_unref(env: Environment, reference: *mut c_void, result: *mut u32) -> i32 {
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

pub(in super::super) unsafe extern "C" fn api_get_reference_value(env: Environment, reference: *mut c_void, result: *mut Handle) -> i32 {
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

// ----- scopes ----------------------------------------------------------

pub(in super::super) unsafe extern "C" fn api_open_handle_scope(env: Environment, result: *mut Opaque) -> i32 {
    run(env, |env| put(result, env.open_scope(false) as Opaque))
}
pub(in super::super) unsafe extern "C" fn api_close_handle_scope(env: Environment, scope: Opaque) -> i32 {
    run(env, |env| env.close_scope(scope as usize))
}
pub(in super::super) unsafe extern "C" fn api_open_escapable_handle_scope(env: Environment, result: *mut Opaque) -> i32 {
    run(env, |env| put(result, env.open_scope(true) as Opaque))
}
pub(in super::super) unsafe extern "C" fn api_close_escapable_handle_scope(env: Environment, scope: Opaque) -> i32 {
    run(env, |env| env.close_scope(scope as usize))
}
pub(in super::super) unsafe extern "C" fn api_escape_handle(env: Environment, scope: Opaque, escapee: Handle, result: *mut Handle) -> i32 {
    run(env, |env| {
        let value = env.get(escapee)?;
        put(result, env.escape(scope as usize, value)?)
    })
}

// ----- instance data, cleanup hooks, misc ------------------------------

pub(in super::super) unsafe extern "C" fn api_set_instance_data(env: Environment, data: *mut c_void, finalize: Finalize, hint: *mut c_void) -> i32 {
    run(env, |env| {
        env.instance_data.set((data, finalize, hint));
        Ok(())
    })
}
pub(in super::super) unsafe extern "C" fn api_get_instance_data(env: Environment, result: *mut *mut c_void) -> i32 {
    run(env, |env| put(result, env.instance_data.get().0))
}

pub(in super::super) unsafe extern "C" fn api_add_env_cleanup_hook(env: Environment, hook: CleanupHook, arg: *mut c_void) -> i32 {
    run(env, |env| {
        if hook.is_none() {
            return Err(INVALID_ARG);
        }
        env.cleanup_hooks.borrow_mut().push((hook, arg));
        Ok(())
    })
}
pub(in super::super) unsafe extern "C" fn api_remove_env_cleanup_hook(env: Environment, hook: CleanupHook, arg: *mut c_void) -> i32 {
    run(env, |env| {
        let mut hooks = env.cleanup_hooks.borrow_mut();
        if let Some(position) = hooks.iter().position(|(h, a)| h.map(|f| f as usize) == hook.map(|f| f as usize) && *a == arg) {
            hooks.remove(position);
        }
        Ok(())
    })
}
pub(in super::super) unsafe extern "C" fn api_add_async_cleanup_hook(
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
pub(in super::super) unsafe extern "C" fn api_remove_async_cleanup_hook(handle: Opaque) {
    super::super::env::mark_async_hook_removed(handle as usize);
}

pub(in super::super) unsafe extern "C" fn api_get_last_error_info(env: Environment, result: *mut *const ExtendedErrorInfo) -> i32 {
    let Some(env) = (unsafe { env.as_ref() }) else { return INVALID_ARG };
    if result.is_null() {
        return INVALID_ARG;
    }
    unsafe { result.write(env.error_info()) };
    OK
}
pub(in super::super) unsafe extern "C" fn api_get_version(env: Environment, result: *mut u32) -> i32 {
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
pub(in super::super) unsafe extern "C" fn api_get_node_version(env: Environment, result: *mut *const NodeVersion) -> i32 {
    run(env, |_| put(result, &NODE_VERSION.0 as *const NodeVersion))
}
pub(in super::super) unsafe extern "C" fn api_get_module_file_name(env: Environment, result: *mut *const c_char) -> i32 {
    run(env, |env| put(result, env.module_url.as_ptr()))
}
pub(in super::super) unsafe extern "C" fn api_get_uv_event_loop(env: Environment, result: *mut *mut c_void) -> i32 {
    run(env, |_| {
        put(result, std::ptr::null_mut())?;
        Err(GENERIC_FAILURE)
    })
}
pub(in super::super) unsafe extern "C" fn api_adjust_external_memory(env: Environment, change: i64, result: *mut i64) -> i32 {
    run(env, |env| {
        env.external_memory.set(env.external_memory.get().saturating_add(change));
        put(result, env.external_memory.get())
    })
}
pub(in super::super) unsafe extern "C" fn api_run_script(env: Environment, script: Handle, result: *mut Handle) -> i32 {
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
            qjs::JS_Eval(env.ctx, ptr, length, c"<napi_run_script>".as_ptr(), qjs::JS_EVAL_TYPE_GLOBAL as i32)
        };
        unsafe { qjs::JS_FreeCString(env.ctx, ptr) };
        put(result, env.push(check(env, value)?))
    })
}
pub(in super::super) unsafe extern "C" fn api_module_register(module: *mut c_void) {
    if module.is_null() {
        return;
    }
    super::super::REGISTRATIONS.with(|registrations| {
        if let Ok(mut registrations) = registrations.try_borrow_mut() {
            if registrations.len() < 1024 {
                registrations.push(module as usize);
            }
        }
    });
}
pub(in super::super) unsafe extern "C" fn api_fatal_error(
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
    eprintln!("FATAL ERROR: {} {}", text(location, location_length), text(message, message_length));
    std::process::abort();
}
pub(in super::super) unsafe extern "C" fn api_fatal_exception(env: Environment, error: Handle) -> i32 {
    run(env, |env| {
        let value = env.get(error)?;
        let dup = unsafe { qjs::JS_DupValue(env.ctx, value) };
        env.set_pending(dup);
        super::super::functions::report_uncaught(env);
        Ok(())
    })
}
pub(in super::super) unsafe extern "C" fn api_open_callback_scope(_env: Environment, _resource: Handle, _context: Opaque, result: *mut Opaque) -> i32 {
    if result.is_null() {
        return INVALID_ARG;
    }
    unsafe { result.write(1 as Opaque) };
    OK
}
pub(in super::super) unsafe extern "C" fn api_close_callback_scope(_env: Environment, _scope: Opaque) -> i32 {
    OK
}
pub(in super::super) unsafe extern "C" fn api_async_init(_env: Environment, _resource: Handle, _name: Handle, result: *mut Opaque) -> i32 {
    if result.is_null() {
        return INVALID_ARG;
    }
    unsafe { result.write(1 as Opaque) };
    OK
}
pub(in super::super) unsafe extern "C" fn api_async_destroy(_env: Environment, _context: Opaque) -> i32 {
    OK
}

#[allow(dead_code)]
fn _unused(_: CString) {
    let _ = (bool_value, new_string);
}
