//! Async work and threadsafe functions.

use std::{
    collections::VecDeque,
    ffi::c_void,
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        Condvar, Mutex,
    },
};

use rquickjs::qjs;

use super::super::{
    env::{put, run, run_js, Env},
    functions::{queue_async, tsfn_post, AsyncWork, Tsfn, TsfnState},
    types::*,
};

pub(in super::super) unsafe extern "C" fn api_create_async_work(
    env: Environment,
    _resource: Handle,
    _name: Handle,
    execute: AsyncExecute,
    complete: AsyncComplete,
    data: *mut c_void,
    result: *mut Opaque,
) -> i32 {
    run(env, |env| {
        if execute.is_none() {
            return Err(INVALID_ARG);
        }
        let work = Box::into_raw(Box::new(AsyncWork {
            env: env as *const Env as *mut Env,
            execute,
            complete,
            data,
            state: AtomicU8::new(0),
            in_flight: AtomicBool::new(false),
            delete_pending: AtomicBool::new(false),
        }));
        put(result, work.cast())
    })
}

pub(in super::super) unsafe extern "C" fn api_delete_async_work(env: Environment, work: Opaque) -> i32 {
    run(env, |_| {
        if work.is_null() {
            return Err(INVALID_ARG);
        }
        let ptr = work as *mut AsyncWork;
        let w = unsafe { &*ptr };
        if w.in_flight.load(Ordering::Acquire) {
            w.delete_pending.store(true, Ordering::Release);
        } else {
            drop(unsafe { Box::from_raw(ptr) });
        }
        Ok(())
    })
}

pub(in super::super) unsafe extern "C" fn api_queue_async_work(env: Environment, work: Opaque) -> i32 {
    run(env, |_| {
        if work.is_null() {
            return Err(INVALID_ARG);
        }
        match unsafe { queue_async(work as *mut AsyncWork) } {
            OK => Ok(()),
            status => Err(status),
        }
    })
}

pub(in super::super) unsafe extern "C" fn api_cancel_async_work(env: Environment, work: Opaque) -> i32 {
    run(env, |_| {
        if work.is_null() {
            return Err(INVALID_ARG);
        }
        let w = unsafe { &*(work as *mut AsyncWork) };
        // Only work that has not started can be cancelled.
        w.state
            .compare_exchange(1, 4, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| ())
            .map_err(|_| GENERIC_FAILURE)
    })
}

// ----- threadsafe functions ---------------------------------------------

pub(in super::super) unsafe extern "C" fn api_create_threadsafe_function(
    env: Environment,
    func: Handle,
    _resource: Handle,
    _name: Handle,
    max_queue: usize,
    initial_threads: usize,
    finalize_data: *mut c_void,
    finalize: Finalize,
    context: *mut c_void,
    call_js: TsfnCallJs,
    result: *mut Opaque,
) -> i32 {
    run_js(env, |env| {
        if initial_threads == 0 || (func.is_null() && call_js.is_none()) {
            return Err(INVALID_ARG);
        }
        let function = if func.is_null() {
            qjs::JS_UNDEFINED
        } else {
            let value = env.get(func)?;
            if !unsafe { qjs::JS_IsFunction(env.ctx, value) } {
                return Err(FUNCTION_EXPECTED);
            }
            unsafe { qjs::JS_DupValue(env.ctx, value) }
        };
        let tsfn = Box::into_raw(Box::new(Tsfn {
            env: env as *const Env as *mut Env,
            func: function,
            context,
            call_js,
            finalize,
            finalize_data,
            max_queue,
            state: Mutex::new(TsfnState {
                queue: VecDeque::new(),
                threads: initial_threads,
                closing: false,
                aborted: false,
                scheduled: false,
            }),
            cv: Condvar::new(),
            events: std::sync::Arc::clone(&env.events),
        }));
        env.live_tsfn.borrow_mut().insert(tsfn as usize);
        super::super::functions::register_tsfn(tsfn);
        put(result, tsfn.cast())
    })
}

pub(in super::super) unsafe extern "C" fn api_get_threadsafe_function_context(tsfn: Opaque, result: *mut *mut c_void) -> i32 {
    if tsfn.is_null() || result.is_null() {
        return INVALID_ARG;
    }
    unsafe { result.write((*(tsfn as *mut Tsfn)).context) };
    OK
}

/// Callable from any thread.
pub(in super::super) unsafe extern "C" fn api_call_threadsafe_function(tsfn: Opaque, data: *mut c_void, mode: i32) -> i32 {
    if tsfn.is_null() {
        return INVALID_ARG;
    }
    let ptr = tsfn as *mut Tsfn;
    let t = unsafe { &*ptr };
    let mut state = t.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    loop {
        if state.closing {
            return CLOSING;
        }
        if t.max_queue == 0 || state.queue.len() < t.max_queue {
            break;
        }
        if mode == 0 {
            return QUEUE_FULL;
        }
        state = t.cv.wait(state).unwrap_or_else(std::sync::PoisonError::into_inner);
    }
    state.queue.push_back(data as usize);
    let post = !state.scheduled;
    state.scheduled = true;
    drop(state);
    if post {
        unsafe { tsfn_post(ptr) };
    }
    OK
}

pub(in super::super) unsafe extern "C" fn api_acquire_threadsafe_function(tsfn: Opaque) -> i32 {
    if tsfn.is_null() {
        return INVALID_ARG;
    }
    let t = unsafe { &*(tsfn as *mut Tsfn) };
    let mut state = t.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if state.closing {
        return CLOSING;
    }
    state.threads += 1;
    OK
}

pub(in super::super) unsafe extern "C" fn api_release_threadsafe_function(tsfn: Opaque, mode: i32) -> i32 {
    if tsfn.is_null() {
        return INVALID_ARG;
    }
    let ptr = tsfn as *mut Tsfn;
    let t = unsafe { &*ptr };
    let mut state = t.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if state.threads == 0 {
        return INVALID_ARG;
    }
    state.threads -= 1;
    let abort = mode == 1;
    if state.threads == 0 || abort {
        state.closing = true;
        state.aborted |= abort;
        t.cv.notify_all();
        drop(state);
        unsafe { tsfn_post(ptr) };
    }
    OK
}

pub(in super::super) unsafe extern "C" fn api_ref_threadsafe_function(env: Environment, tsfn: Opaque) -> i32 {
    run(env, |_| if tsfn.is_null() { Err(INVALID_ARG) } else { Ok(()) })
}
pub(in super::super) unsafe extern "C" fn api_unref_threadsafe_function(env: Environment, tsfn: Opaque) -> i32 {
    run(env, |_| if tsfn.is_null() { Err(INVALID_ARG) } else { Ok(()) })
}
