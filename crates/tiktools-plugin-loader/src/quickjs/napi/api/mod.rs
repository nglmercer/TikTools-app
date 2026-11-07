//! Node-API function implementations, grouped by area. Every `api_*`
//! function is referenced from the generated `TABLE`.

mod asyncs;
mod buffers;
mod calls;
mod objects;
mod values;

pub(super) use asyncs::*;
pub(super) use buffers::*;
pub(super) use calls::*;
pub(super) use objects::*;
pub(super) use values::*;

use std::ffi::{c_char, CStr};

use rquickjs::qjs::{self, JSValue};

use super::{env::Env, types::*};

/// Reads a possibly length-prefixed C string (`NAPI_AUTO_LENGTH`).
pub(super) unsafe fn read_bytes<'a>(ptr: *const c_char, len: usize) -> Result<&'a [u8], i32> {
    if ptr.is_null() {
        return if len == 0 { Ok(&[]) } else { Err(INVALID_ARG) };
    }
    if len == AUTO_LENGTH {
        Ok(unsafe { CStr::from_ptr(ptr) }.to_bytes())
    } else if len > i32::MAX as usize {
        Err(INVALID_ARG)
    } else {
        Ok(unsafe { std::slice::from_raw_parts(ptr.cast::<u8>(), len) })
    }
}

pub(super) fn new_string(env: &Env, text: &str) -> JSValue {
    unsafe { qjs::JS_NewStringLen(env.ctx, text.as_ptr().cast(), text.len() as u64) }
}

pub(super) fn bool_value(value: bool) -> JSValue {
    qjs::JS_MKVAL(qjs::JS_TAG_BOOL as i32, i32::from(value))
}

pub(super) fn global(env: &Env) -> JSValue {
    unsafe { qjs::JS_GetGlobalObject(env.ctx) }
}

/// `globalThis[name]` (owned).
pub(super) fn global_get(env: &Env, name: &std::ffi::CStr) -> JSValue {
    unsafe {
        let global = global(env);
        let value = qjs::JS_GetPropertyStr(env.ctx, global, name.as_ptr());
        qjs::JS_FreeValue(env.ctx, global);
        value
    }
}

/// Turns an engine exception result into a pending napi exception.
pub(super) fn check(env: &Env, value: JSValue) -> Result<JSValue, i32> {
    if unsafe { qjs::JS_IsException(value) } {
        Err(env.capture_exception())
    } else {
        Ok(value)
    }
}

/// Like [`check`] for plain `int` returns (-1 means exception).
pub(super) fn check_int(env: &Env, value: i32) -> Result<i32, i32> {
    if value < 0 {
        Err(env.capture_exception())
    } else {
        Ok(value)
    }
}

pub(super) fn is_object(value: JSValue) -> bool {
    unsafe { qjs::JS_IsObject(value) }
}

/// RAII owner of an engine value.
pub(super) struct Owned<'a> {
    env: &'a Env,
    pub value: JSValue,
}

impl<'a> Owned<'a> {
    pub fn new(env: &'a Env, value: JSValue) -> Self {
        Self { env, value }
    }

    /// Hands the value to the handle arena.
    pub fn into_handle(self) -> Handle {
        let handle = self.env.push(self.value);
        std::mem::forget(self);
        handle
    }

    pub fn release(self) -> JSValue {
        let value = self.value;
        std::mem::forget(self);
        value
    }
}

impl Drop for Owned<'_> {
    fn drop(&mut self) {
        unsafe { qjs::JS_FreeValue(self.env.ctx, self.value) };
    }
}

/// An atom from a value, freed on drop.
pub(super) struct Atom<'a> {
    env: &'a Env,
    pub atom: qjs::JSAtom,
}

impl<'a> Atom<'a> {
    pub fn from_value(env: &'a Env, value: JSValue) -> Result<Self, i32> {
        let atom = unsafe { qjs::JS_ValueToAtom(env.ctx, value) };
        if atom == 0 {
            return Err(env.capture_exception());
        }
        Ok(Self { env, atom })
    }

    pub fn from_name(env: &'a Env, name: &CStr) -> Result<Self, i32> {
        let atom = unsafe { qjs::JS_NewAtom(env.ctx, name.as_ptr()) };
        if atom == 0 {
            return Err(env.capture_exception());
        }
        Ok(Self { env, atom })
    }
}

impl Drop for Atom<'_> {
    fn drop(&mut self) {
        unsafe { qjs::JS_FreeAtom(self.env.ctx, self.atom) };
    }
}

/// Evaluates a script in the global scope (owned result).
pub(super) fn eval(env: &Env, source: &str) -> Result<JSValue, i32> {
    let code = std::ffi::CString::new(source).map_err(|_| INVALID_ARG)?;
    let value = unsafe {
        qjs::JS_Eval(
            env.ctx,
            code.as_ptr(),
            source.len() as u64,
            c"<napi>".as_ptr(),
            qjs::JS_EVAL_TYPE_GLOBAL as i32,
        )
    };
    check(env, value)
}
