//! String encodings, property keys, bounded output buffers and external-string finalization.
use super::*;

pub(in super::super::super) unsafe extern "C" fn api_create_string_utf8(
    env: Environment,
    text: *const c_char,
    length: usize,
    result: *mut Handle,
) -> i32 {
    run(env, |env| {
        let bytes = unsafe { read_bytes(text, length)? };
        let text = String::from_utf8_lossy(bytes);
        put(result, env.push(new_string(env, &text)))
    })
}

pub(in super::super::super) unsafe extern "C" fn api_create_string_latin1(
    env: Environment,
    text: *const c_char,
    length: usize,
    result: *mut Handle,
) -> i32 {
    run(env, |env| {
        let bytes = unsafe { read_bytes(text, length)? };
        let text: String = bytes.iter().map(|byte| char::from(*byte)).collect();
        put(result, env.push(new_string(env, &text)))
    })
}

pub(in super::super::super) unsafe extern "C" fn api_create_string_utf16(
    env: Environment,
    text: *const u16,
    length: usize,
    result: *mut Handle,
) -> i32 {
    run(env, |env| {
        let units = unsafe { read_units(text, length)? };
        put(
            result,
            env.push(unsafe {
                qjs::JS_NewStringUTF16(env.ctx, units.as_ptr(), units.len() as u64)
            }),
        )
    })
}

unsafe fn read_units<'a>(text: *const u16, length: usize) -> Result<&'a [u16], i32> {
    if text.is_null() {
        return if length == 0 {
            Ok(&[])
        } else {
            Err(INVALID_ARG)
        };
    }
    let length = if length == AUTO_LENGTH {
        let mut n = 0;
        while unsafe { text.add(n).read() } != 0 {
            n += 1;
        }
        n
    } else {
        length
    };
    if length > i32::MAX as usize {
        return Err(INVALID_ARG);
    }
    Ok(unsafe { std::slice::from_raw_parts(text, length) })
}

pub(in super::super::super) unsafe extern "C" fn api_create_property_key_utf8(
    env: Environment,
    text: *const c_char,
    length: usize,
    result: *mut Handle,
) -> i32 {
    unsafe { api_create_string_utf8(env, text, length, result) }
}

pub(in super::super::super) unsafe extern "C" fn api_create_property_key_latin1(
    env: Environment,
    text: *const c_char,
    length: usize,
    result: *mut Handle,
) -> i32 {
    unsafe { api_create_string_latin1(env, text, length, result) }
}

pub(in super::super::super) unsafe extern "C" fn api_create_property_key_utf16(
    env: Environment,
    text: *const u16,
    length: usize,
    result: *mut Handle,
) -> i32 {
    unsafe { api_create_string_utf16(env, text, length, result) }
}

/// UTF-8 bytes of a string value (lossy for lone surrogates).
pub(super) fn string_bytes(env: &Env, value: JSValue) -> Result<Vec<u8>, i32> {
    if !unsafe { qjs::JS_IsString(value) } {
        return Err(STRING_EXPECTED);
    }
    let mut length = 0_u64;
    let ptr = unsafe { qjs::JS_ToCStringLen2(env.ctx, &mut length, value, false) };
    if ptr.is_null() {
        return Err(env.capture_exception());
    }
    let bytes = unsafe { std::slice::from_raw_parts(ptr.cast::<u8>(), length as usize) }.to_vec();
    unsafe { qjs::JS_FreeCString(env.ctx, ptr) };
    Ok(String::from_utf8_lossy(&bytes).into_owned().into_bytes())
}

fn copy_out(bytes: &[u8], buf: *mut c_char, size: usize, result: *mut usize) -> Result<(), i32> {
    if buf.is_null() {
        return put(result, bytes.len());
    }
    if size == 0 {
        if !result.is_null() {
            unsafe { result.write(0) };
        }
        return Ok(());
    }
    let mut take = bytes.len().min(size - 1);
    // Do not split a multi-byte sequence.
    while take > 0 && take < bytes.len() && (bytes[take] & 0xC0) == 0x80 {
        take -= 1;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf.cast::<u8>(), take);
        buf.add(take).write(0);
    }
    if !result.is_null() {
        unsafe { result.write(take) };
    }
    Ok(())
}

pub(in super::super::super) unsafe extern "C" fn api_get_value_string_utf8(
    env: Environment,
    value: Handle,
    buf: *mut c_char,
    size: usize,
    result: *mut usize,
) -> i32 {
    run(env, |env| {
        let bytes = string_bytes(env, env.get(value)?)?;
        copy_out(&bytes, buf, size, result)
    })
}

pub(in super::super::super) unsafe extern "C" fn api_get_value_string_latin1(
    env: Environment,
    value: Handle,
    buf: *mut c_char,
    size: usize,
    result: *mut usize,
) -> i32 {
    run(env, |env| {
        let text = String::from_utf8_lossy(&string_bytes(env, env.get(value)?)?).into_owned();
        let bytes: Vec<u8> = text.encode_utf16().map(|unit| unit as u8).collect();
        if buf.is_null() {
            return put(result, bytes.len());
        }
        if size == 0 {
            return Ok(());
        }
        let take = bytes.len().min(size - 1);
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf.cast::<u8>(), take);
            buf.add(take).write(0);
        }
        if !result.is_null() {
            unsafe { result.write(take) };
        }
        Ok(())
    })
}

pub(in super::super::super) unsafe extern "C" fn api_get_value_string_utf16(
    env: Environment,
    value: Handle,
    buf: *mut u16,
    size: usize,
    result: *mut usize,
) -> i32 {
    run(env, |env| {
        let value = env.get(value)?;
        if !unsafe { qjs::JS_IsString(value) } {
            return Err(STRING_EXPECTED);
        }
        let mut length = 0_u64;
        let ptr = unsafe { qjs::JS_ToCStringLenUTF16(env.ctx, &mut length, value) };
        if ptr.is_null() {
            return Err(env.capture_exception());
        }
        let units = unsafe { std::slice::from_raw_parts(ptr, length as usize) }.to_vec();
        unsafe { qjs::JS_FreeCStringUTF16(env.ctx, ptr) };
        if buf.is_null() {
            return put(result, units.len());
        }
        if size == 0 {
            return Ok(());
        }
        let take = units.len().min(size - 1);
        unsafe {
            std::ptr::copy_nonoverlapping(units.as_ptr(), buf, take);
            buf.add(take).write(0);
        }
        if !result.is_null() {
            unsafe { result.write(take) };
        }
        Ok(())
    })
}

pub(in super::super::super) unsafe extern "C" fn api_create_external_string_latin1(
    env: Environment,
    text: *mut c_char,
    length: usize,
    finalize: Finalize,
    hint: *mut c_void,
    result: *mut Handle,
    copied: *mut bool,
) -> i32 {
    let status = unsafe { api_create_string_latin1(env, text, length, result) };
    if status == OK {
        if let Some(finalize) = finalize {
            unsafe { finalize(env, text.cast(), hint) };
        }
        if !copied.is_null() {
            unsafe { copied.write(true) };
        }
    }
    status
}

pub(in super::super::super) unsafe extern "C" fn api_create_external_string_utf16(
    env: Environment,
    text: *mut u16,
    length: usize,
    finalize: Finalize,
    hint: *mut c_void,
    result: *mut Handle,
    copied: *mut bool,
) -> i32 {
    let status = unsafe { api_create_string_utf16(env, text, length, result) };
    if status == OK {
        if let Some(finalize) = finalize {
            unsafe { finalize(env, text.cast(), hint) };
        }
        if !copied.is_null() {
            unsafe { copied.write(true) };
        }
    }
    status
}
