//! Primitive values, strings, numbers, symbols, bigints, dates, externals.

use std::ffi::{c_char, c_void, CString};

use rquickjs::qjs::{self, JSValue};

use super::{
    super::{
        env::{put, run, run_js, Env},
        functions::{external_rec, new_external},
        types::*,
    },
    bool_value, check, global_get, is_object, new_string, read_bytes, Owned,
};

fn number(value: f64) -> JSValue {
    qjs::JS_NewFloat64(value)
}

macro_rules! simple_out {
    ($name:ident, $body:expr) => {
        pub(in super::super) unsafe extern "C" fn $name(
            env: Environment,
            result: *mut Handle,
        ) -> i32 {
            run(env, |env| {
                let value: JSValue = $body(env);
                put(result, env.push(value))
            })
        }
    };
}

simple_out!(api_get_undefined, |_: &Env| qjs::JS_UNDEFINED);
simple_out!(api_get_null, |_: &Env| qjs::JS_NULL);
simple_out!(api_get_global, super::global);

pub(in super::super) unsafe extern "C" fn api_get_boolean(
    env: Environment,
    value: bool,
    result: *mut Handle,
) -> i32 {
    run(env, |env| put(result, env.push(bool_value(value))))
}

pub(in super::super) unsafe extern "C" fn api_create_double(
    env: Environment,
    value: f64,
    result: *mut Handle,
) -> i32 {
    run(env, |env| put(result, env.push(number(value))))
}
pub(in super::super) unsafe extern "C" fn api_create_int32(
    env: Environment,
    value: i32,
    result: *mut Handle,
) -> i32 {
    run(env, |env| put(result, env.push(number(f64::from(value)))))
}
pub(in super::super) unsafe extern "C" fn api_create_uint32(
    env: Environment,
    value: u32,
    result: *mut Handle,
) -> i32 {
    run(env, |env| put(result, env.push(number(f64::from(value)))))
}
pub(in super::super) unsafe extern "C" fn api_create_int64(
    env: Environment,
    value: i64,
    result: *mut Handle,
) -> i32 {
    run(env, |env| put(result, env.push(number(value as f64))))
}

// ----- strings --------------------------------------------------------

pub(in super::super) unsafe extern "C" fn api_create_string_utf8(
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

pub(in super::super) unsafe extern "C" fn api_create_string_latin1(
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

pub(in super::super) unsafe extern "C" fn api_create_string_utf16(
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

pub(in super::super) unsafe extern "C" fn api_create_property_key_utf8(
    env: Environment,
    text: *const c_char,
    length: usize,
    result: *mut Handle,
) -> i32 {
    unsafe { api_create_string_utf8(env, text, length, result) }
}
pub(in super::super) unsafe extern "C" fn api_create_property_key_latin1(
    env: Environment,
    text: *const c_char,
    length: usize,
    result: *mut Handle,
) -> i32 {
    unsafe { api_create_string_latin1(env, text, length, result) }
}
pub(in super::super) unsafe extern "C" fn api_create_property_key_utf16(
    env: Environment,
    text: *const u16,
    length: usize,
    result: *mut Handle,
) -> i32 {
    unsafe { api_create_string_utf16(env, text, length, result) }
}

/// UTF-8 bytes of a string value (lossy for lone surrogates).
fn string_bytes(env: &Env, value: JSValue) -> Result<Vec<u8>, i32> {
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

pub(in super::super) unsafe extern "C" fn api_get_value_string_utf8(
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

pub(in super::super) unsafe extern "C" fn api_get_value_string_latin1(
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

pub(in super::super) unsafe extern "C" fn api_get_value_string_utf16(
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

pub(in super::super) unsafe extern "C" fn api_create_external_string_latin1(
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

pub(in super::super) unsafe extern "C" fn api_create_external_string_utf16(
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

// ----- symbols --------------------------------------------------------

pub(in super::super) unsafe extern "C" fn api_create_symbol(
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

pub(in super::super) unsafe extern "C" fn api_node_api_symbol_for(
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

// ----- typeof / getters -----------------------------------------------

pub(in super::super) unsafe extern "C" fn api_typeof_value(
    env: Environment,
    value: Handle,
    result: *mut i32,
) -> i32 {
    run(env, |env| {
        let value = env.get(value)?;
        let kind = unsafe {
            if qjs::JS_IsUndefined(value) {
                TYPE_UNDEFINED
            } else if qjs::JS_IsNull(value) {
                TYPE_NULL
            } else if qjs::JS_IsBool(value) {
                TYPE_BOOLEAN
            } else if qjs::JS_IsNumber(value) {
                TYPE_NUMBER
            } else if qjs::JS_IsString(value) {
                TYPE_STRING
            } else if qjs::JS_IsSymbol(value) {
                TYPE_SYMBOL
            } else if qjs::JS_IsBigInt(value) {
                TYPE_BIGINT
            } else if qjs::JS_IsFunction(env.ctx, value) {
                TYPE_FUNCTION
            } else if external_rec(env, value).is_some() {
                TYPE_EXTERNAL
            } else if qjs::JS_IsObject(value) {
                TYPE_OBJECT
            } else {
                return Err(INVALID_ARG);
            }
        };
        put(result, kind)
    })
}

pub(in super::super) unsafe extern "C" fn api_create_external(
    env: Environment,
    data: *mut c_void,
    finalize: Finalize,
    hint: *mut c_void,
    result: *mut Handle,
) -> i32 {
    run(env, |env| {
        let value = check(env, new_external(env, data, finalize, hint))?;
        put(result, env.push(value))
    })
}

pub(in super::super) unsafe extern "C" fn api_get_value_external(
    env: Environment,
    value: Handle,
    result: *mut *mut c_void,
) -> i32 {
    run(env, |env| {
        let rec = external_rec(env, env.get(value)?).ok_or(INVALID_ARG)?;
        put(result, rec.data)
    })
}

fn to_f64(env: &Env, value: JSValue) -> Result<f64, i32> {
    if !unsafe { qjs::JS_IsNumber(value) } {
        return Err(NUMBER_EXPECTED);
    }
    let mut out = 0.0;
    if unsafe { qjs::JS_ToFloat64(env.ctx, &mut out, value) } < 0 {
        return Err(env.capture_exception());
    }
    Ok(out)
}

pub(in super::super) unsafe extern "C" fn api_get_value_double(
    env: Environment,
    value: Handle,
    result: *mut f64,
) -> i32 {
    run(env, |env| put(result, to_f64(env, env.get(value)?)?))
}

pub(in super::super) unsafe extern "C" fn api_get_value_int32(
    env: Environment,
    value: Handle,
    result: *mut i32,
) -> i32 {
    run(env, |env| {
        let number = to_f64(env, env.get(value)?)?;
        let out = if number.is_finite() {
            (number as i64) as i32
        } else {
            0
        };
        put(result, out)
    })
}

pub(in super::super) unsafe extern "C" fn api_get_value_uint32(
    env: Environment,
    value: Handle,
    result: *mut u32,
) -> i32 {
    run(env, |env| {
        let number = to_f64(env, env.get(value)?)?;
        let out = if number.is_finite() {
            (number as i64) as u32
        } else {
            0
        };
        put(result, out)
    })
}

pub(in super::super) unsafe extern "C" fn api_get_value_int64(
    env: Environment,
    value: Handle,
    result: *mut i64,
) -> i32 {
    run(env, |env| {
        let number = to_f64(env, env.get(value)?)?;
        let out = if number.is_finite() { number as i64 } else { 0 };
        put(result, out)
    })
}

pub(in super::super) unsafe extern "C" fn api_get_value_bool(
    env: Environment,
    value: Handle,
    result: *mut bool,
) -> i32 {
    run(env, |env| {
        let value = env.get(value)?;
        if !unsafe { qjs::JS_IsBool(value) } {
            return Err(BOOLEAN_EXPECTED);
        }
        put(result, unsafe { qjs::JS_ToBool(env.ctx, value) } > 0)
    })
}

// ----- coercion -------------------------------------------------------

pub(in super::super) unsafe extern "C" fn api_coerce_to_bool(
    env: Environment,
    value: Handle,
    result: *mut Handle,
) -> i32 {
    run_js(env, |env| {
        let flag = unsafe { qjs::JS_ToBool(env.ctx, env.get(value)?) };
        put(
            result,
            env.push(bool_value(super::check_int(env, flag)? > 0)),
        )
    })
}

pub(in super::super) unsafe extern "C" fn api_coerce_to_number(
    env: Environment,
    value: Handle,
    result: *mut Handle,
) -> i32 {
    run_js(env, |env| {
        let mut out = 0.0;
        super::check_int(env, unsafe {
            qjs::JS_ToFloat64(env.ctx, &mut out, env.get(value)?)
        })?;
        put(result, env.push(number(out)))
    })
}

pub(in super::super) unsafe extern "C" fn api_coerce_to_string(
    env: Environment,
    value: Handle,
    result: *mut Handle,
) -> i32 {
    run_js(env, |env| {
        let string = unsafe { qjs::JS_ToString(env.ctx, env.get(value)?) };
        put(result, env.push(check(env, string)?))
    })
}

pub(in super::super) unsafe extern "C" fn api_coerce_to_object(
    env: Environment,
    value: Handle,
    result: *mut Handle,
) -> i32 {
    run_js(env, |env| {
        let value = env.get(value)?;
        if unsafe { qjs::JS_IsUndefined(value) || qjs::JS_IsNull(value) } {
            return Err(INVALID_ARG);
        }
        if is_object(value) {
            return put(result, env.push_dup(value));
        }
        let ctor = Owned::new(env, global_get(env, c"Object"));
        let mut args = [value];
        let object =
            unsafe { qjs::JS_Call(env.ctx, ctor.value, qjs::JS_UNDEFINED, 1, args.as_mut_ptr()) };
        put(result, env.push(check(env, object)?))
    })
}

pub(in super::super) unsafe extern "C" fn api_strict_equals(
    env: Environment,
    a: Handle,
    b: Handle,
    result: *mut bool,
) -> i32 {
    run(env, |env| {
        let (a, b) = (env.get(a)?, env.get(b)?);
        let same = unsafe { qjs::JS_IsStrictEqual(env.ctx, a, b) };
        put(result, same)
    })
}

// ----- bigint ---------------------------------------------------------

pub(in super::super) unsafe extern "C" fn api_create_bigint_int64(
    env: Environment,
    value: i64,
    result: *mut Handle,
) -> i32 {
    run(env, |env| {
        put(
            result,
            env.push(unsafe { qjs::JS_NewBigInt64(env.ctx, value) }),
        )
    })
}
pub(in super::super) unsafe extern "C" fn api_create_bigint_uint64(
    env: Environment,
    value: u64,
    result: *mut Handle,
) -> i32 {
    run(env, |env| {
        put(
            result,
            env.push(unsafe { qjs::JS_NewBigUint64(env.ctx, value) }),
        )
    })
}

/// Builds a BigInt from sign and little-endian 64-bit words by evaluating
/// `BigInt("0x...")` (negated for the sign bit).
pub(in super::super) unsafe extern "C" fn api_create_bigint_words(
    env: Environment,
    sign: i32,
    count: usize,
    words: *const u64,
    result: *mut Handle,
) -> i32 {
    run_js(env, |env| {
        if count > i32::MAX as usize / 64 || (words.is_null() && count > 0) {
            return Err(INVALID_ARG);
        }
        let words = if count == 0 {
            &[][..]
        } else {
            unsafe { std::slice::from_raw_parts(words, count) }
        };
        let mut hex = String::from("0x0");
        for word in words.iter().rev() {
            hex.push_str(&format!("{word:016x}"));
        }
        let ctor = Owned::new(env, global_get(env, c"BigInt"));
        let text = Owned::new(env, new_string(env, &hex));
        let mut args = [text.value];
        let big =
            unsafe { qjs::JS_Call(env.ctx, ctor.value, qjs::JS_UNDEFINED, 1, args.as_mut_ptr()) };
        let mut big = Owned::new(env, check(env, big)?);
        if sign != 0 && !words.iter().all(|w| *w == 0) {
            let zero = unsafe { qjs::JS_NewBigInt64(env.ctx, 0) };
            let negated = {
                let zero = Owned::new(env, zero);
                let mut args = [zero.value, big.value];
                let minus = Owned::new(env, super::eval(env, "(a, b) => a - b")?);
                let value = unsafe {
                    qjs::JS_Call(
                        env.ctx,
                        minus.value,
                        qjs::JS_UNDEFINED,
                        2,
                        args.as_mut_ptr(),
                    )
                };
                check(env, value)?
            };
            big = Owned::new(env, negated);
        }
        put(result, env.push(big.release()))
    })
}

/// Decimal string of a bigint value.
fn bigint_text(env: &Env, value: JSValue) -> Result<String, i32> {
    if !unsafe { qjs::JS_IsBigInt(value) } {
        return Err(BIGINT_EXPECTED);
    }
    let string = Owned::new(
        env,
        check(env, unsafe { qjs::JS_ToString(env.ctx, value) })?,
    );
    Ok(String::from_utf8_lossy(&string_bytes(env, string.value)?).into_owned())
}

/// (negative, little-endian words) of a decimal integer string.
fn decimal_to_words(text: &str) -> (bool, Vec<u64>) {
    let (negative, digits) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let mut words: Vec<u64> = vec![0];
    for digit in digits.bytes().filter(u8::is_ascii_digit) {
        let mut carry = u128::from(digit - b'0');
        for word in &mut words {
            let value = u128::from(*word) * 10 + carry;
            *word = value as u64;
            carry = value >> 64;
        }
        if carry > 0 {
            words.push(carry as u64);
        }
    }
    while words.len() > 1 && words.last() == Some(&0) {
        words.pop();
    }
    (negative, words)
}

pub(in super::super) unsafe extern "C" fn api_get_value_bigint_int64(
    env: Environment,
    value: Handle,
    out: *mut i64,
    lossless: *mut bool,
) -> i32 {
    run(env, |env| {
        let (negative, words) = decimal_to_words(&bigint_text(env, env.get(value)?)?);
        let low = words[0];
        let signed = if negative {
            (low as i64).wrapping_neg()
        } else {
            low as i64
        };
        let exact = words.len() == 1
            && if negative {
                low <= 1 << 63
            } else {
                low < 1 << 63
            }
            || words == [0];
        if out.is_null() || lossless.is_null() {
            return Err(INVALID_ARG);
        }
        unsafe {
            out.write(signed);
            lossless.write(exact);
        }
        Ok(())
    })
}

pub(in super::super) unsafe extern "C" fn api_get_value_bigint_uint64(
    env: Environment,
    value: Handle,
    out: *mut u64,
    lossless: *mut bool,
) -> i32 {
    run(env, |env| {
        let (negative, words) = decimal_to_words(&bigint_text(env, env.get(value)?)?);
        let low = words[0];
        let unsigned = if negative { low.wrapping_neg() } else { low };
        let exact = words.len() == 1 && (!negative || low == 0);
        if out.is_null() || lossless.is_null() {
            return Err(INVALID_ARG);
        }
        unsafe {
            out.write(unsigned);
            lossless.write(exact);
        }
        Ok(())
    })
}

pub(in super::super) unsafe extern "C" fn api_get_value_bigint_words(
    env: Environment,
    value: Handle,
    sign: *mut i32,
    count: *mut usize,
    words_out: *mut u64,
) -> i32 {
    run(env, |env| {
        if count.is_null() {
            return Err(INVALID_ARG);
        }
        let (negative, words) = decimal_to_words(&bigint_text(env, env.get(value)?)?);
        if words_out.is_null() {
            unsafe { count.write(words.len()) };
            return Ok(());
        }
        if sign.is_null() {
            return Err(INVALID_ARG);
        }
        let capacity = unsafe { count.read() };
        let take = words.len().min(capacity);
        unsafe {
            std::ptr::copy_nonoverlapping(words.as_ptr(), words_out, take);
            sign.write(i32::from(negative));
            count.write(take);
        }
        Ok(())
    })
}

// ----- dates ----------------------------------------------------------

pub(in super::super) unsafe extern "C" fn api_create_date(
    env: Environment,
    time: f64,
    result: *mut Handle,
) -> i32 {
    run(env, |env| {
        put(result, env.push(unsafe { qjs::JS_NewDate(env.ctx, time) }))
    })
}
pub(in super::super) unsafe extern "C" fn api_is_date(
    env: Environment,
    value: Handle,
    result: *mut bool,
) -> i32 {
    run(env, |env| {
        put(result, unsafe { qjs::JS_IsDate(env.get(value)?) })
    })
}
pub(in super::super) unsafe extern "C" fn api_get_date_value(
    env: Environment,
    value: Handle,
    result: *mut f64,
) -> i32 {
    run_js(env, |env| {
        let value = env.get(value)?;
        if !unsafe { qjs::JS_IsDate(value) } {
            return Err(DATE_EXPECTED);
        }
        let mut out = 0.0;
        super::check_int(env, unsafe { qjs::JS_ToFloat64(env.ctx, &mut out, value) })?;
        put(result, out)
    })
}
