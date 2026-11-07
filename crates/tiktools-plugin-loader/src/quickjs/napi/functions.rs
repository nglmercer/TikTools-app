//! Native function plumbing: JS-callable wrappers around addon callbacks,
//! externals, async work and threadsafe functions.

use std::{
    collections::VecDeque,
    ffi::{c_int, c_void, CString},
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        Condvar, Mutex,
    },
};

use rquickjs::qjs::{self, JSContext, JSValue};

use super::{
    env::{Env, HostEvent, HostEvents, SendPtr},
    types::*,
};

pub(super) struct FnRecord {
    pub env: *mut Env,
    pub cb: Callback,
    pub data: *mut c_void,
}

/// What `napi_get_cb_info` reads.
pub(super) struct CallbackInfo {
    pub this: JSValue,
    pub argv: *const JSValue,
    pub argc: usize,
    pub new_target: JSValue,
    pub data: *mut c_void,
}

fn c_string(text: &str) -> CString {
    CString::new(text.replace('\0', "")).unwrap_or_default()
}

unsafe fn invoke(
    rec: &FnRecord,
    ctx: *mut JSContext,
    this: JSValue,
    argc: c_int,
    argv: *mut JSValue,
    new_target: JSValue,
) -> JSValue {
    let env = unsafe { &*rec.env };
    let Some(cb) = rec.cb else {
        return qjs::JS_UNDEFINED;
    };
    if env.dead.get() {
        return qjs::JS_UNDEFINED;
    }
    let token = env.open_scope(false);
    let info = CallbackInfo {
        this,
        argv,
        argc: argc.max(0) as usize,
        new_target,
        data: rec.data,
    };
    let result = unsafe { cb(rec.env, &info as *const CallbackInfo as *mut c_void) };
    let ret = if result.is_null() {
        qjs::JS_UNDEFINED
    } else {
        match env.get(result) {
            Ok(value) => unsafe { qjs::JS_DupValue(ctx, value) },
            Err(_) => qjs::JS_UNDEFINED,
        }
    };
    let _ = env.close_scope(token);
    if let Some(exception) = env.take_pending() {
        unsafe { qjs::JS_FreeValue(ctx, ret) };
        return unsafe { qjs::JS_Throw(ctx, exception) };
    }
    ret
}

unsafe extern "C" fn closure_call(
    ctx: *mut JSContext,
    this: JSValue,
    argc: c_int,
    argv: *mut JSValue,
    _magic: c_int,
    opaque: *mut c_void,
) -> JSValue {
    let rec = unsafe { &*(opaque as *const FnRecord) };
    unsafe { invoke(rec, ctx, this, argc, argv, qjs::JS_UNDEFINED) }
}

unsafe extern "C" fn closure_free(opaque: *mut c_void) {
    drop(unsafe { Box::from_raw(opaque as *mut FnRecord) });
}

/// Creates a JS function whose lifetime owns the callback record.
pub(super) fn new_function(env: &Env, name: &str, cb: Callback, data: *mut c_void) -> JSValue {
    let rec = Box::into_raw(Box::new(FnRecord {
        env: env as *const Env as *mut Env,
        cb,
        data,
    }));
    let name = c_string(name);
    unsafe {
        qjs::JS_NewCClosure(
            env.ctx,
            Some(closure_call),
            name.as_ptr(),
            Some(closure_free),
            0,
            0,
            rec.cast(),
        )
    }
}

/// Constructor records are addressed by the C function's `magic` and live
/// for the process (classes are few and long-lived).
static CONSTRUCTORS: Mutex<Vec<usize>> = Mutex::new(Vec::new());

unsafe extern "C" fn ctor_call(
    ctx: *mut JSContext,
    new_target: JSValue,
    argc: c_int,
    argv: *mut JSValue,
    magic: c_int,
) -> JSValue {
    let rec = {
        let table = CONSTRUCTORS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        match table.get(magic as usize) {
            Some(rec) => *rec as *const FnRecord,
            None => return qjs::JS_UNDEFINED,
        }
    };
    let rec = unsafe { &*rec };
    unsafe {
        let proto = qjs::JS_GetPropertyStr(ctx, new_target, c"prototype".as_ptr());
        let this = if qjs::JS_IsObject(proto) {
            qjs::JS_NewObjectProto(ctx, proto)
        } else {
            qjs::JS_NewObject(ctx)
        };
        qjs::JS_FreeValue(ctx, proto);
        let result = invoke(rec, ctx, this, argc, argv, new_target);
        if qjs::JS_IsException(result) {
            qjs::JS_FreeValue(ctx, this);
            return result;
        }
        if qjs::JS_IsObject(result) {
            qjs::JS_FreeValue(ctx, this);
            result
        } else {
            qjs::JS_FreeValue(ctx, result);
            this
        }
    }
}

pub(super) fn new_constructor(env: &Env, name: &str, cb: Callback, data: *mut c_void) -> JSValue {
    let rec = Box::into_raw(Box::new(FnRecord {
        env: env as *const Env as *mut Env,
        cb,
        data,
    }));
    let index = {
        let mut table = CONSTRUCTORS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        table.push(rec as usize);
        table.len() - 1
    };
    let name = c_string(name);
    unsafe {
        let function_ctor = qjs::JS_GetPropertyStr(
            env.ctx,
            qjs::JS_GetGlobalObject(env.ctx),
            c"Function".as_ptr(),
        );
        let function_proto = qjs::JS_GetPropertyStr(env.ctx, function_ctor, c"prototype".as_ptr());
        qjs::JS_FreeValue(env.ctx, function_ctor);
        let func: unsafe extern "C" fn(*mut JSContext, JSValue, c_int, *mut JSValue, c_int) -> JSValue =
            ctor_call;
        let cfunc: qjs::JSCFunction = std::mem::transmute(func);
        let value = qjs::JS_NewCFunction3(
            env.ctx,
            cfunc,
            name.as_ptr(),
            0,
            qjs::JSCFunctionEnum_JS_CFUNC_constructor_magic,
            index as c_int,
            function_proto,
            0,
        );
        qjs::JS_FreeValue(env.ctx, function_proto);
        value
    }
}

// ----- externals ------------------------------------------------------

pub(super) struct ExternalRec {
    pub env: *mut Env,
    pub data: *mut c_void,
    pub finalize: Finalize,
    pub hint: *mut c_void,
}

pub(super) fn new_external(
    env: &Env,
    data: *mut c_void,
    finalize: Finalize,
    hint: *mut c_void,
) -> JSValue {
    let object = unsafe { qjs::JS_NewObjectClass(env.ctx, env.external_class as _) };
    if unsafe { qjs::JS_IsException(object) } {
        return object;
    }
    let rec = Box::into_raw(Box::new(ExternalRec {
        env: env as *const Env as *mut Env,
        data,
        finalize,
        hint,
    }));
    unsafe { qjs::JS_SetOpaque(object, rec.cast()) };
    object
}

pub(super) fn external_rec(env: &Env, value: JSValue) -> Option<&'static ExternalRec> {
    if !unsafe { qjs::JS_IsObject(value) } {
        return None;
    }
    let ptr = unsafe { qjs::JS_GetOpaque(value, env.external_class as _) };
    (!ptr.is_null()).then(|| unsafe { &*(ptr as *const ExternalRec) })
}

/// Class finalizer: queues (or, once the env is gone, directly runs) the
/// addon's finalize callback and frees the record.
pub(super) unsafe fn finalize_external(value: JSValue) {
    let mut class_id: qjs::JSClassID = 0;
    let ptr = unsafe { qjs::JS_GetAnyOpaque(value, &mut class_id) };
    if ptr.is_null() {
        return;
    }
    let rec = unsafe { Box::from_raw(ptr as *mut ExternalRec) };
    let Some(cb) = rec.finalize else { return };
    let env = unsafe { &*rec.env };
    if env.dead.get() {
        unsafe { cb(rec.env, rec.data, rec.hint) };
    } else {
        env.finalizers.borrow_mut().push_back(super::env::PendingFinalizer {
            cb: Some(cb),
            data: rec.data,
            hint: rec.hint,
        });
    }
}

// ----- async work -----------------------------------------------------

pub(super) struct AsyncWork {
    pub env: *mut Env,
    pub execute: AsyncExecute,
    pub complete: AsyncComplete,
    pub data: *mut c_void,
    /// 0 created, 1 queued, 2 running, 3 finished.
    pub state: AtomicU8,
    pub in_flight: AtomicBool,
    pub delete_pending: AtomicBool,
}

pub(super) unsafe fn queue_async(work: *mut AsyncWork) -> i32 {
    let w = unsafe { &*work };
    if w.in_flight.swap(true, Ordering::AcqRel) {
        return GENERIC_FAILURE;
    }
    w.state.store(1, Ordering::Release);
    let env = unsafe { &*w.env };
    env.live_async.borrow_mut().insert(work as usize);
    let events = std::sync::Arc::clone(&env.events);
    let ptr = SendPtr(work.cast());
    std::thread::Builder::new()
        .name("tiktools-napi-work".to_owned())
        .spawn(move || {
            let work = ptr.get() as *mut AsyncWork;
            let w = unsafe { &*work };
            let status = if w
                .state
                .compare_exchange(1, 2, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                if let Some(execute) = w.execute {
                    unsafe { execute(w.env, w.data) };
                }
                OK
            } else {
                CANCELLED
            };
            w.state.store(3, Ordering::Release);
            events.post(HostEvent::AsyncDone(ptr, status));
        })
        .map_or_else(
            |_| {
                w.in_flight.store(false, Ordering::Release);
                env.live_async.borrow_mut().remove(&(work as usize));
                GENERIC_FAILURE
            },
            |_| OK,
        )
}

pub(super) unsafe fn complete_async(ptr: *mut c_void, status: i32) {
    let work = ptr as *mut AsyncWork;
    let w = unsafe { &*work };
    let env = unsafe { &*w.env };
    env.live_async.borrow_mut().remove(&(work as usize));
    if let Some(complete) = w.complete {
        let token = env.open_scope(false);
        unsafe { complete(w.env, status, w.data) };
        let _ = env.close_scope(token);
        report_uncaught(env);
    }
    w.in_flight.store(false, Ordering::Release);
    if w.delete_pending.load(Ordering::Acquire) {
        drop(unsafe { Box::from_raw(work) });
    }
}

/// An exception left pending by an addon callback with no JS caller has
/// nowhere to propagate: log it.
pub(super) fn report_uncaught(env: &Env) {
    if let Some(exception) = env.take_pending() {
        let text = unsafe {
            let cstr = qjs::JS_ToCString(env.ctx, exception);
            let text = if cstr.is_null() {
                "exception".to_owned()
            } else {
                let owned = std::ffi::CStr::from_ptr(cstr).to_string_lossy().into_owned();
                qjs::JS_FreeCString(env.ctx, cstr);
                owned
            };
            qjs::JS_FreeValue(env.ctx, exception);
            text
        };
        tracing::warn!(error = %text, "uncaught exception in a native addon callback");
    }
}

// ----- threadsafe functions -------------------------------------------

pub(super) struct TsfnState {
    pub queue: VecDeque<usize>,
    pub threads: usize,
    pub closing: bool,
    pub aborted: bool,
    pub scheduled: bool,
}

pub(super) struct Tsfn {
    pub env: *mut Env,
    pub func: JSValue,
    pub context: *mut c_void,
    pub call_js: TsfnCallJs,
    pub finalize: Finalize,
    pub finalize_data: *mut c_void,
    pub max_queue: usize,
    pub state: Mutex<TsfnState>,
    pub cv: Condvar,
    pub events: std::sync::Arc<HostEvents>,
}

/// Addresses of live threadsafe functions: queued wake events can outlive
/// the function they name, so they are checked here before any dereference.
static LIVE_TSFN: Mutex<Vec<usize>> = Mutex::new(Vec::new());

pub(super) fn register_tsfn(tsfn: *mut Tsfn) {
    LIVE_TSFN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .push(tsfn as usize);
}

fn unregister_tsfn(tsfn: *mut Tsfn) -> bool {
    let mut live = LIVE_TSFN.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    match live.iter().position(|entry| *entry == tsfn as usize) {
        Some(position) => {
            live.swap_remove(position);
            true
        }
        None => false,
    }
}

fn is_live_tsfn(tsfn: *mut Tsfn) -> bool {
    LIVE_TSFN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .contains(&(tsfn as usize))
}

pub(super) unsafe fn tsfn_post(tsfn: *mut Tsfn) {
    let t = unsafe { &*tsfn };
    t.events.post(HostEvent::Tsfn(SendPtr(tsfn.cast())));
}

/// Runs queued calls on the owner thread and finalizes a closing function.
pub(super) unsafe fn dispatch_tsfn(ptr: *mut c_void) {
    let tsfn = ptr as *mut Tsfn;
    if !is_live_tsfn(tsfn) {
        return;
    }
    let t = unsafe { &*tsfn };
    let env = unsafe { &*t.env };
    loop {
        let (item, aborted) = {
            let mut state = t.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            match state.queue.pop_front() {
                Some(item) => {
                    t.cv.notify_all();
                    (item, state.aborted)
                }
                None => {
                    state.scheduled = false;
                    break;
                }
            }
        };
        if aborted {
            // Give the addon a chance to free the data (null env, no JS).
            if let Some(call_js) = t.call_js {
                unsafe { call_js(std::ptr::null_mut(), std::ptr::null_mut(), t.context, item as *mut c_void) };
            }
            continue;
        }
        let token = env.open_scope(false);
        unsafe {
            if let Some(call_js) = t.call_js {
                let callback = if qjs::JS_IsUndefined(t.func) {
                    std::ptr::null_mut()
                } else {
                    env.push_dup(t.func)
                };
                call_js(t.env, callback, t.context, item as *mut c_void);
            } else if !qjs::JS_IsUndefined(t.func) {
                let result = qjs::JS_Call(env.ctx, t.func, qjs::JS_UNDEFINED, 0, std::ptr::null_mut());
                if qjs::JS_IsException(result) {
                    env.capture_exception();
                } else {
                    qjs::JS_FreeValue(env.ctx, result);
                }
            }
        }
        let _ = env.close_scope(token);
        report_uncaught(env);
    }
    let finishing = {
        let state = t.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        state.closing && state.queue.is_empty()
    };
    if finishing {
        unsafe { finish_tsfn(tsfn) };
    }
}

pub(super) unsafe fn finish_tsfn(tsfn: *mut Tsfn) {
    let t = unsafe { &*tsfn };
    let env = unsafe { &*t.env };
    if !unregister_tsfn(tsfn) {
        return;
    }
    env.live_tsfn.borrow_mut().remove(&(tsfn as usize));
    if let Some(finalize) = t.finalize {
        let token = env.open_scope(false);
        unsafe { finalize(t.env, t.finalize_data, t.context) };
        let _ = env.close_scope(token);
        report_uncaught(env);
    }
    unsafe { qjs::JS_FreeValue(env.ctx, t.func) };
    drop(unsafe { Box::from_raw(tsfn) });
}

/// Process-exit path for one env: drop threadsafe functions, then the env.
pub(super) unsafe fn shutdown_env(env: &Env) {
    let live: Vec<usize> = env.live_tsfn.borrow().iter().copied().collect();
    for tsfn in live {
        let t = unsafe { &*(tsfn as *mut Tsfn) };
        {
            let mut state = t.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            state.closing = true;
            state.aborted = true;
        }
        if let Some(call_js) = t.call_js {
            let items: Vec<usize> = t
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .queue
                .drain(..)
                .collect();
            for item in items {
                unsafe { call_js(std::ptr::null_mut(), std::ptr::null_mut(), t.context, item as *mut c_void) };
            }
        }
        unsafe { finish_tsfn(tsfn as *mut Tsfn) };
    }
    env.shutdown();
}
