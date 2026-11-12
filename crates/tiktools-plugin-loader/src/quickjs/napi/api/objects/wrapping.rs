//! Native pointer holders, finalizers and object type tags.
use super::super::super::{
    env::{put, run, run_js, Env, PendingFinalizer},
    functions::{external_rec, new_external},
    types::*,
};
use super::super::{check, check_int, is_object, Atom, Owned};
use super::references::create_reference;
use rquickjs::qjs::{self, JSValue};
use std::ffi::c_void;

pub(in super::super::super) unsafe extern "C" fn api_wrap(
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
            qjs::JS_DefinePropertyValue(
                env.ctx,
                object,
                env.wrap_atom,
                holder,
                qjs::JS_PROP_CONFIGURABLE as i32,
            )
        })?;
        if !reference.is_null() {
            let reference_ptr = create_reference(env, object, 0)?;
            unsafe { reference.write(reference_ptr) };
        }
        Ok(())
    })
}

/// The external holder stored under a hidden atom, if any.
fn own_hidden(
    env: &Env,
    object: JSValue,
    atom: qjs::JSAtom,
) -> Option<&'static super::super::super::functions::ExternalRec> {
    let holder = Owned::new(env, unsafe { qjs::JS_GetProperty(env.ctx, object, atom) });
    external_rec(env, holder.value)
}

pub(in super::super::super) unsafe extern "C" fn api_unwrap(
    env: Environment,
    object: Handle,
    result: *mut *mut c_void,
) -> i32 {
    run_js(env, |env| {
        let object = env.get(object)?;
        if !is_object(object) {
            return Err(OBJECT_EXPECTED);
        }
        let rec = own_hidden(env, object, env.wrap_atom).ok_or(INVALID_ARG)?;
        put(result, rec.data)
    })
}

pub(in super::super::super) unsafe extern "C" fn api_remove_wrap(
    env: Environment,
    object: Handle,
    result: *mut *mut c_void,
) -> i32 {
    run_js(env, |env| {
        let object = env.get(object)?;
        if !is_object(object) {
            return Err(OBJECT_EXPECTED);
        }
        let rec = own_hidden(env, object, env.wrap_atom).ok_or(INVALID_ARG)?;
        let data = rec.data;
        // The addon takes the pointer back: the finalizer must not run.
        unsafe {
            (*(rec as *const _ as *mut super::super::super::functions::ExternalRec)).finalize = None
        };
        unsafe { qjs::JS_DeleteProperty(env.ctx, object, env.wrap_atom, 0) };
        if !result.is_null() {
            unsafe { result.write(data) };
        }
        Ok(())
    })
}

pub(in super::super::super) unsafe extern "C" fn api_add_finalizer(
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
            return Err(if finalize.is_none() {
                INVALID_ARG
            } else {
                OBJECT_EXPECTED
            });
        }
        let holder = check(env, new_external(env, data, finalize, hint))?;
        let symbol = Owned::new(env, unsafe {
            qjs::JS_NewSymbol(env.ctx, c"tiktools.napi.finalizer".as_ptr(), false)
        });
        let atom = Atom::from_value(env, symbol.value)?;
        check_int(env, unsafe {
            qjs::JS_DefinePropertyValue(
                env.ctx,
                object,
                atom.atom,
                holder,
                qjs::JS_PROP_CONFIGURABLE as i32,
            )
        })?;
        if !reference.is_null() {
            let reference_ptr = create_reference(env, object, 0)?;
            unsafe { reference.write(reference_ptr) };
        }
        Ok(())
    })
}

pub(in super::super::super) unsafe extern "C" fn api_post_finalizer(
    env: Environment,
    finalize: Finalize,
    data: *mut c_void,
    hint: *mut c_void,
) -> i32 {
    run(env, |env| {
        env.finalizers.borrow_mut().push_back(PendingFinalizer {
            cb: finalize,
            data,
            hint,
        });
        Ok(())
    })
}

unsafe extern "C" fn free_tag(_env: Environment, data: *mut c_void, _hint: *mut c_void) {
    drop(unsafe { Box::from_raw(data as *mut TypeTag) });
}

pub(in super::super::super) unsafe extern "C" fn api_type_tag_object(
    env: Environment,
    object: Handle,
    tag: *const TypeTag,
) -> i32 {
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
        let holder = check(
            env,
            new_external(env, boxed.cast(), Some(free_tag), std::ptr::null_mut()),
        )?;
        check_int(env, unsafe {
            qjs::JS_DefinePropertyValue(env.ctx, object, env.tag_atom, holder, 0)
        })?;
        Ok(())
    })
}

pub(in super::super::super) unsafe extern "C" fn api_check_object_type_tag(
    env: Environment,
    object: Handle,
    tag: *const TypeTag,
    result: *mut bool,
) -> i32 {
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
