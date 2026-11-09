//! ArrayBuffers, typed arrays, DataViews and Node-style Buffers (which are
//! plain `Uint8Array`s here).

use std::{
    alloc::{alloc_zeroed, dealloc, Layout},
    ffi::{c_int, c_void},
};

use rquickjs::qjs::{self, JSValue};

use super::{
    super::{
        env::{put, run, run_js, Env, PendingFinalizer},
        types::*,
    },
    check, check_int, global_get, is_object, make_error, Owned,
};

fn layout(len: usize) -> Layout {
    Layout::from_size_align(len.max(1), 1).expect("byte layouts are always valid")
}

unsafe extern "C" fn free_owned(_rt: *mut qjs::JSRuntime, opaque: *mut c_void, ptr: *mut c_void) {
    unsafe { dealloc(ptr.cast(), layout(opaque as usize)) };
}

struct ExternalBuffer {
    env: *mut Env,
    finalize: Finalize,
    noenv: NoEnvFinalize,
    hint: *mut c_void,
}

unsafe extern "C" fn free_external(
    _rt: *mut qjs::JSRuntime,
    opaque: *mut c_void,
    ptr: *mut c_void,
) {
    let rec = unsafe { Box::from_raw(opaque as *mut ExternalBuffer) };
    if let Some(noenv) = rec.noenv {
        unsafe { noenv(ptr, rec.hint) };
    } else if let Some(finalize) = rec.finalize {
        let env = unsafe { &*rec.env };
        if env.dead.get() {
            unsafe { finalize(rec.env, ptr, rec.hint) };
        } else {
            env.finalizers.borrow_mut().push_back(PendingFinalizer {
                cb: Some(finalize),
                data: ptr,
                hint: rec.hint,
            });
        }
    }
}

fn new_owned_arraybuffer(env: &Env, len: usize, shared: bool) -> Result<(JSValue, *mut u8), i32> {
    let data = unsafe { alloc_zeroed(layout(len)) };
    if data.is_null() {
        return Err(GENERIC_FAILURE);
    }
    let value = unsafe {
        qjs::JS_NewArrayBuffer(
            env.ctx,
            data,
            len as u64,
            Some(free_owned),
            len as *mut c_void,
            shared,
        )
    };
    if unsafe { qjs::JS_IsException(value) } {
        unsafe { dealloc(data, layout(len)) };
        return Err(env.capture_exception());
    }
    Ok((value, data))
}

fn new_external_arraybuffer(
    env: &Env,
    data: *mut c_void,
    len: usize,
    finalize: Finalize,
    noenv: NoEnvFinalize,
    hint: *mut c_void,
    shared: bool,
) -> Result<JSValue, i32> {
    let rec = Box::into_raw(Box::new(ExternalBuffer {
        env: env as *const Env as *mut Env,
        finalize,
        noenv,
        hint,
    }));
    let value = unsafe {
        qjs::JS_NewArrayBuffer(
            env.ctx,
            data.cast(),
            len as u64,
            Some(free_external),
            rec.cast(),
            shared,
        )
    };
    if unsafe { qjs::JS_IsException(value) } {
        drop(unsafe { Box::from_raw(rec) });
        return Err(env.capture_exception());
    }
    Ok(value)
}

pub(in super::super) unsafe extern "C" fn api_create_arraybuffer(
    env: Environment,
    len: usize,
    data: *mut *mut c_void,
    result: *mut Handle,
) -> i32 {
    run_js(env, |env| {
        let (value, ptr) = new_owned_arraybuffer(env, len, false)?;
        if !data.is_null() {
            unsafe { data.write(ptr.cast()) };
        }
        put(result, env.push(value))
    })
}

pub(in super::super) unsafe extern "C" fn api_create_sharedarraybuffer(
    env: Environment,
    len: usize,
    data: *mut *mut c_void,
    result: *mut Handle,
) -> i32 {
    run_js(env, |env| {
        let (value, ptr) = new_owned_arraybuffer(env, len, true)?;
        if !data.is_null() {
            unsafe { data.write(ptr.cast()) };
        }
        put(result, env.push(value))
    })
}

pub(in super::super) unsafe extern "C" fn api_create_external_arraybuffer(
    env: Environment,
    data: *mut c_void,
    len: usize,
    finalize: Finalize,
    hint: *mut c_void,
    result: *mut Handle,
) -> i32 {
    run_js(env, |env| {
        let value = new_external_arraybuffer(env, data, len, finalize, None, hint, false)?;
        put(result, env.push(value))
    })
}

pub(in super::super) unsafe extern "C" fn api_create_external_sharedarraybuffer(
    env: Environment,
    data: *mut c_void,
    len: usize,
    finalize: NoEnvFinalize,
    hint: *mut c_void,
    result: *mut Handle,
) -> i32 {
    run_js(env, |env| {
        let value = new_external_arraybuffer(env, data, len, None, finalize, hint, true)?;
        put(result, env.push(value))
    })
}

fn is_instance_of(env: &Env, value: JSValue, global_name: &std::ffi::CStr) -> bool {
    let ctor = Owned::new(env, global_get(env, global_name));
    if !unsafe { qjs::JS_IsFunction(env.ctx, ctor.value) } {
        return false;
    }
    unsafe { qjs::JS_IsInstanceOf(env.ctx, value, ctor.value) > 0 }
}

pub(in super::super) unsafe extern "C" fn api_is_arraybuffer(
    env: Environment,
    value: Handle,
    result: *mut bool,
) -> i32 {
    run(env, |env| {
        let value = env.get(value)?;
        put(
            result,
            unsafe { qjs::JS_IsArrayBuffer(value) }
                && !is_instance_of(env, value, c"SharedArrayBuffer"),
        )
    })
}

pub(in super::super) unsafe extern "C" fn api_is_sharedarraybuffer(
    env: Environment,
    value: Handle,
    result: *mut bool,
) -> i32 {
    run(env, |env| {
        let value = env.get(value)?;
        put(result, is_instance_of(env, value, c"SharedArrayBuffer"))
    })
}

fn buffer_bytes(env: &Env, buffer: JSValue) -> Result<(*mut u8, usize), i32> {
    let mut size = 0_u64;
    let ptr = unsafe { qjs::JS_GetArrayBuffer(env.ctx, &mut size, buffer) };
    if ptr.is_null() && size != 0 {
        return Err(env.capture_exception());
    }
    Ok((ptr, size as usize))
}

pub(in super::super) unsafe extern "C" fn api_get_arraybuffer_info(
    env: Environment,
    value: Handle,
    data: *mut *mut c_void,
    len: *mut usize,
) -> i32 {
    run(env, |env| {
        let value = env.get(value)?;
        if !unsafe { qjs::JS_IsArrayBuffer(value) } {
            return Err(INVALID_ARG);
        }
        let (ptr, size) = buffer_bytes(env, value)?;
        if !data.is_null() {
            unsafe { data.write(ptr.cast()) };
        }
        if !len.is_null() {
            unsafe { len.write(size) };
        }
        Ok(())
    })
}

pub(in super::super) unsafe extern "C" fn api_detach_arraybuffer(
    env: Environment,
    value: Handle,
) -> i32 {
    run(env, |env| {
        let value = env.get(value)?;
        if !unsafe { qjs::JS_IsArrayBuffer(value) } {
            return Err(DETACHABLE_ARRAYBUFFER_EXPECTED);
        }
        unsafe { qjs::JS_DetachArrayBuffer(env.ctx, value) };
        Ok(())
    })
}

pub(in super::super) unsafe extern "C" fn api_is_detached_arraybuffer(
    env: Environment,
    value: Handle,
    result: *mut bool,
) -> i32 {
    run(env, |env| {
        let value = env.get(value)?;
        if !unsafe { qjs::JS_IsArrayBuffer(value) } {
            return put(result, false);
        }
        let detached = Owned::new(env, unsafe {
            qjs::JS_GetPropertyStr(env.ctx, value, c"detached".as_ptr())
        });
        put(
            result,
            unsafe { qjs::JS_ToBool(env.ctx, detached.value) } > 0,
        )
    })
}

// ----- typed arrays ---------------------------------------------------

/// (napi type, quickjs enum, element size)
const KINDS: [(i32, qjs::JSTypedArrayEnum, usize); 11] = [
    (0, qjs::JSTypedArrayEnum_JS_TYPED_ARRAY_INT8, 1),
    (1, qjs::JSTypedArrayEnum_JS_TYPED_ARRAY_UINT8, 1),
    (2, qjs::JSTypedArrayEnum_JS_TYPED_ARRAY_UINT8C, 1),
    (3, qjs::JSTypedArrayEnum_JS_TYPED_ARRAY_INT16, 2),
    (4, qjs::JSTypedArrayEnum_JS_TYPED_ARRAY_UINT16, 2),
    (5, qjs::JSTypedArrayEnum_JS_TYPED_ARRAY_INT32, 4),
    (6, qjs::JSTypedArrayEnum_JS_TYPED_ARRAY_UINT32, 4),
    (7, qjs::JSTypedArrayEnum_JS_TYPED_ARRAY_FLOAT32, 4),
    (8, qjs::JSTypedArrayEnum_JS_TYPED_ARRAY_FLOAT64, 8),
    (9, qjs::JSTypedArrayEnum_JS_TYPED_ARRAY_BIG_INT64, 8),
    (10, qjs::JSTypedArrayEnum_JS_TYPED_ARRAY_BIG_UINT64, 8),
];

fn number(value: f64) -> JSValue {
    qjs::JS_NewFloat64(value)
}

fn throw_range(env: &Env, message: &str) -> i32 {
    match make_error(env, c"RangeError", None, message) {
        Ok(error) => {
            env.set_pending(error);
            INVALID_ARG
        }
        Err(status) => status,
    }
}

fn make_typed_array(
    env: &Env,
    qjs_kind: qjs::JSTypedArrayEnum,
    buffer: JSValue,
    offset: usize,
    length: usize,
) -> Result<JSValue, i32> {
    let mut args = [buffer, number(offset as f64), number(length as f64)];
    check(env, unsafe {
        qjs::JS_NewTypedArray(env.ctx, 3, args.as_mut_ptr(), qjs_kind)
    })
}

pub(in super::super) unsafe extern "C" fn api_create_typedarray(
    env: Environment,
    kind: i32,
    length: usize,
    buffer: Handle,
    offset: usize,
    result: *mut Handle,
) -> i32 {
    run_js(env, |env| {
        let (_, qjs_kind, size) = *KINDS
            .iter()
            .find(|entry| entry.0 == kind)
            .ok_or(INVALID_ARG)?;
        let buffer = env.get(buffer)?;
        if !unsafe { qjs::JS_IsArrayBuffer(buffer) } {
            return Err(INVALID_ARG);
        }
        if size > 1 && !offset.is_multiple_of(size) {
            return Err(throw_range(
                env,
                &format!("start offset of typed array should be a multiple of {size}"),
            ));
        }
        let (_, total) = buffer_bytes(env, buffer)?;
        if length
            .checked_mul(size)
            .and_then(|bytes| bytes.checked_add(offset))
            .is_none_or(|end| end > total)
        {
            return Err(throw_range(env, "Invalid typed array length"));
        }
        put(
            result,
            env.push(make_typed_array(env, qjs_kind, buffer, offset, length)?),
        )
    })
}

pub(in super::super) unsafe extern "C" fn api_is_typedarray(
    env: Environment,
    value: Handle,
    result: *mut bool,
) -> i32 {
    run(env, |env| {
        put(
            result,
            unsafe { qjs::JS_GetTypedArrayType(env.get(value)?) } >= 0,
        )
    })
}

/// (buffer handle value, byte offset, byte length, element size, napi kind)
fn typed_array_parts(
    env: &Env,
    value: JSValue,
) -> Result<(JSValue, usize, usize, usize, i32), i32> {
    let qjs_kind = unsafe { qjs::JS_GetTypedArrayType(value) };
    if qjs_kind < 0 {
        return Err(INVALID_ARG);
    }
    let kind = KINDS
        .iter()
        .find(|entry| entry.1 as i64 == i64::from(qjs_kind))
        .map_or(1, |entry| entry.0);
    let (mut offset, mut byte_length, mut bpe) = (0_u64, 0_u64, 0_u64);
    let buffer = check(env, unsafe {
        qjs::JS_GetTypedArrayBuffer(env.ctx, value, &mut offset, &mut byte_length, &mut bpe)
    })?;
    Ok((
        buffer,
        offset as usize,
        byte_length as usize,
        (bpe as usize).max(1),
        kind,
    ))
}

pub(in super::super) unsafe extern "C" fn api_get_typedarray_info(
    env: Environment,
    value: Handle,
    kind: *mut i32,
    length: *mut usize,
    data: *mut *mut c_void,
    arraybuffer: *mut Handle,
    byte_offset: *mut usize,
) -> i32 {
    run(env, |env| {
        let (buffer, offset, byte_length, bpe, napi_kind) =
            typed_array_parts(env, env.get(value)?)?;
        let buffer = Owned::new(env, buffer);
        if !kind.is_null() {
            unsafe { kind.write(napi_kind) };
        }
        if !length.is_null() {
            unsafe { length.write(byte_length / bpe) };
        }
        if !data.is_null() {
            let (ptr, _) = buffer_bytes(env, buffer.value)?;
            unsafe {
                data.write(if ptr.is_null() {
                    ptr.cast()
                } else {
                    ptr.add(offset).cast()
                })
            };
        }
        if !byte_offset.is_null() {
            unsafe { byte_offset.write(offset) };
        }
        if !arraybuffer.is_null() {
            unsafe { arraybuffer.write(env.push(buffer.release())) };
        }
        Ok(())
    })
}

// ----- DataView ---------------------------------------------------------

pub(in super::super) unsafe extern "C" fn api_create_dataview(
    env: Environment,
    length: usize,
    buffer: Handle,
    offset: usize,
    result: *mut Handle,
) -> i32 {
    run_js(env, |env| {
        let buffer = env.get(buffer)?;
        if !unsafe { qjs::JS_IsArrayBuffer(buffer) } {
            return Err(INVALID_ARG);
        }
        let (_, total) = buffer_bytes(env, buffer)?;
        if length.checked_add(offset).is_none_or(|end| end > total) {
            return Err(throw_range(
                env,
                "byte_offset + byte_length should be less than or equal to the size in bytes of the array passed in",
            ));
        }
        let ctor = Owned::new(env, global_get(env, c"DataView"));
        let mut args = [buffer, number(offset as f64), number(length as f64)];
        let view = unsafe { qjs::JS_CallConstructor(env.ctx, ctor.value, 3, args.as_mut_ptr()) };
        put(result, env.push(check(env, view)?))
    })
}

pub(in super::super) unsafe extern "C" fn api_is_dataview(
    env: Environment,
    value: Handle,
    result: *mut bool,
) -> i32 {
    run(env, |env| {
        let value = env.get(value)?;
        put(
            result,
            is_object(value) && is_instance_of(env, value, c"DataView"),
        )
    })
}

fn prop_usize(env: &Env, object: JSValue, name: &std::ffi::CStr) -> Result<usize, i32> {
    let value = Owned::new(env, unsafe {
        qjs::JS_GetPropertyStr(env.ctx, object, name.as_ptr())
    });
    let mut out = 0.0;
    check_int(env, unsafe {
        qjs::JS_ToFloat64(env.ctx, &mut out, value.value)
    })?;
    Ok(out as usize)
}

pub(in super::super) unsafe extern "C" fn api_get_dataview_info(
    env: Environment,
    value: Handle,
    byte_length: *mut usize,
    data: *mut *mut c_void,
    arraybuffer: *mut Handle,
    byte_offset: *mut usize,
) -> i32 {
    run(env, |env| {
        let view = env.get(value)?;
        if !is_instance_of(env, view, c"DataView") {
            return Err(INVALID_ARG);
        }
        let length = prop_usize(env, view, c"byteLength")?;
        let offset = prop_usize(env, view, c"byteOffset")?;
        let buffer = Owned::new(env, unsafe {
            qjs::JS_GetPropertyStr(env.ctx, view, c"buffer".as_ptr())
        });
        if !byte_length.is_null() {
            unsafe { byte_length.write(length) };
        }
        if !data.is_null() {
            let (ptr, _) = buffer_bytes(env, buffer.value)?;
            unsafe {
                data.write(if ptr.is_null() {
                    ptr.cast()
                } else {
                    ptr.add(offset).cast()
                })
            };
        }
        if !byte_offset.is_null() {
            unsafe { byte_offset.write(offset) };
        }
        if !arraybuffer.is_null() {
            unsafe { arraybuffer.write(env.push(buffer.release())) };
        }
        Ok(())
    })
}

// ----- Buffers (Uint8Array) -------------------------------------------

fn new_buffer(env: &Env, len: usize) -> Result<(JSValue, *mut u8), i32> {
    let (buffer, ptr) = new_owned_arraybuffer(env, len, false)?;
    let buffer = Owned::new(env, buffer);
    let view = make_typed_array(
        env,
        qjs::JSTypedArrayEnum_JS_TYPED_ARRAY_UINT8,
        buffer.value,
        0,
        len,
    )?;
    Ok((view, ptr))
}

pub(in super::super) unsafe extern "C" fn api_create_buffer(
    env: Environment,
    len: usize,
    data: *mut *mut c_void,
    result: *mut Handle,
) -> i32 {
    run_js(env, |env| {
        let (view, ptr) = new_buffer(env, len)?;
        if !data.is_null() {
            unsafe { data.write(ptr.cast()) };
        }
        put(result, env.push(view))
    })
}

pub(in super::super) unsafe extern "C" fn api_create_buffer_copy(
    env: Environment,
    len: usize,
    source: *const c_void,
    data: *mut *mut c_void,
    result: *mut Handle,
) -> i32 {
    run_js(env, |env| {
        if source.is_null() && len > 0 {
            return Err(INVALID_ARG);
        }
        let (view, ptr) = new_buffer(env, len)?;
        if len > 0 {
            unsafe { std::ptr::copy_nonoverlapping(source.cast::<u8>(), ptr, len) };
        }
        if !data.is_null() {
            unsafe { data.write(ptr.cast()) };
        }
        put(result, env.push(view))
    })
}

pub(in super::super) unsafe extern "C" fn api_create_external_buffer(
    env: Environment,
    len: usize,
    data: *mut c_void,
    finalize: Finalize,
    hint: *mut c_void,
    result: *mut Handle,
) -> i32 {
    run_js(env, |env| {
        let buffer = Owned::new(
            env,
            new_external_arraybuffer(env, data, len, finalize, None, hint, false)?,
        );
        let view = make_typed_array(
            env,
            qjs::JSTypedArrayEnum_JS_TYPED_ARRAY_UINT8,
            buffer.value,
            0,
            len,
        )?;
        put(result, env.push(view))
    })
}

pub(in super::super) unsafe extern "C" fn api_create_buffer_from_arraybuffer(
    env: Environment,
    buffer: Handle,
    offset: usize,
    length: usize,
    result: *mut Handle,
) -> i32 {
    run_js(env, |env| {
        let buffer = env.get(buffer)?;
        if !unsafe { qjs::JS_IsArrayBuffer(buffer) } {
            return Err(ARRAYBUFFER_EXPECTED);
        }
        let (_, total) = buffer_bytes(env, buffer)?;
        if length.checked_add(offset).is_none_or(|end| end > total) {
            return Err(throw_range(env, "The byte offset + length is out of range"));
        }
        let view = make_typed_array(
            env,
            qjs::JSTypedArrayEnum_JS_TYPED_ARRAY_UINT8,
            buffer,
            offset,
            length,
        )?;
        put(result, env.push(view))
    })
}

pub(in super::super) unsafe extern "C" fn api_is_buffer(
    env: Environment,
    value: Handle,
    result: *mut bool,
) -> i32 {
    run(env, |env| {
        let value = env.get(value)?;
        let is_view = unsafe { qjs::JS_GetTypedArrayType(value) } >= 0
            || (is_object(value) && is_instance_of(env, value, c"DataView"));
        put(result, is_view)
    })
}

pub(in super::super) unsafe extern "C" fn api_get_buffer_info(
    env: Environment,
    value: Handle,
    data: *mut *mut c_void,
    len: *mut usize,
) -> i32 {
    run(env, |env| {
        let (buffer, offset, byte_length, _, _) = typed_array_parts(env, env.get(value)?)?;
        let buffer = Owned::new(env, buffer);
        let (ptr, _) = buffer_bytes(env, buffer.value)?;
        if !data.is_null() {
            unsafe {
                data.write(if ptr.is_null() {
                    ptr.cast()
                } else {
                    ptr.add(offset).cast()
                })
            };
        }
        if !len.is_null() {
            unsafe { len.write(byte_length) };
        }
        Ok(())
    })
}

#[allow(dead_code)]
fn _unused(_: c_int) {}
