//! BigInt creation, decimal/word conversion and lossless integer extraction.
use super::*;

pub(in super::super::super) unsafe extern "C" fn api_create_bigint_int64(
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

pub(in super::super::super) unsafe extern "C" fn api_create_bigint_uint64(
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
pub(in super::super::super) unsafe extern "C" fn api_create_bigint_words(
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

pub(in super::super::super) unsafe extern "C" fn api_get_value_bigint_int64(
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

pub(in super::super::super) unsafe extern "C" fn api_get_value_bigint_uint64(
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

pub(in super::super::super) unsafe extern "C" fn api_get_value_bigint_words(
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
