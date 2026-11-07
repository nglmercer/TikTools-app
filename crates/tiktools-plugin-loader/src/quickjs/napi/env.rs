//! Per-addon Node-API environment: the handle arena, scopes, pending
//! exception, references and everything else a `napi_env` stands for.
//!
//! An `Env` is created when an addon loads and lives for the rest of the
//! process (it is intentionally leaked): GC finalizers that run while the
//! JavaScript runtime shuts down may still reach it, so it must never
//! dangle. All state is only touched from the plugin's owner thread, except
//! the thread-safe [`HostEvents`] queue.

use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet, VecDeque},
    ffi::{c_char, c_void, CString},
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{Arc, Condvar, Mutex},
    time::Duration,
};

use rquickjs::qjs;

use super::{super::runtime::WakeNotifier, types::*};

/// Wrapper to move raw pointers across threads. The pointee's thread
/// safety is the addon's contract (threadsafe functions and async work).
#[derive(Clone, Copy)]
pub(super) struct SendPtr(pub(super) *mut c_void);
unsafe impl Send for SendPtr {}
impl SendPtr {
    pub(super) fn get(self) -> *mut c_void {
        self.0
    }
}

pub(super) enum HostEvent {
    /// A threadsafe function has queued calls.
    Tsfn(SendPtr),
    /// An async work item finished on its worker thread.
    AsyncDone(SendPtr, i32),
}

/// Thread-safe inbox of the owner thread: native callbacks, async
/// completions and finalizers posted from other threads land here and wake
/// the owner thread.
pub(crate) struct HostEvents {
    queue: Mutex<VecDeque<HostEvent>>,
    cv: Condvar,
    wake: WakeNotifier,
}

impl HostEvents {
    pub(super) fn new(wake: WakeNotifier) -> Arc<Self> {
        Arc::new(Self {
            queue: Mutex::new(VecDeque::new()),
            cv: Condvar::new(),
            wake,
        })
    }

    pub(super) fn post(&self, event: HostEvent) {
        self.queue
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push_back(event);
        self.cv.notify_all();
        (self.wake)();
    }

    pub(super) fn take(&self) -> VecDeque<HostEvent> {
        std::mem::take(
            &mut *self
                .queue
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        )
    }

    /// Blocks until an event arrives or the timeout passes. Returns whether
    /// events are waiting.
    pub(super) fn wait(&self, timeout: Duration) -> bool {
        let guard = self
            .queue
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !guard.is_empty() {
            return true;
        }
        let (guard, _) = self
            .cv
            .wait_timeout(guard, timeout)
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        !guard.is_empty()
    }
}

pub(super) struct Scope {
    start: usize,
    /// Handle slot reserved in the parent scope for `napi_escape_handle`.
    escape_slot: Option<usize>,
    escaped: bool,
}

pub(super) struct Reference {
    /// Strong value, or (for weak references to objects) a `WeakRef`.
    pub value: qjs::JSValue,
    pub weak: bool,
    pub count: u32,
}

/// What a finalizer callback needs to run later, on a safe stack.
pub(super) struct PendingFinalizer {
    pub cb: Finalize,
    pub data: *mut c_void,
    pub hint: *mut c_void,
}

pub(crate) struct Env {
    pub(super) ctx: *mut qjs::JSContext,
    pub(super) external_class: qjs::JSClassID,
    pub(super) events: Arc<HostEvents>,
    pub(super) module_url: CString,
    handles: RefCell<Vec<qjs::JSValue>>,
    scopes: RefCell<Vec<Scope>>,
    pending: Cell<Option<qjs::JSValue>>,
    error: RefCell<Box<ExtendedErrorInfo>>,
    pub(super) refs: RefCell<HashMap<usize, Box<Reference>>>,
    pub(super) finalizers: RefCell<VecDeque<PendingFinalizer>>,
    pub(super) cleanup_hooks: RefCell<Vec<(CleanupHook, *mut c_void)>>,
    pub(super) async_cleanup_hooks: RefCell<Vec<(AsyncCleanupHook, *mut c_void, usize)>>,
    pub(super) instance_data: Cell<(*mut c_void, Finalize, *mut c_void)>,
    /// Hidden symbol atoms: wrap pointer, type tag, added finalizers.
    pub(super) wrap_atom: qjs::JSAtom,
    pub(super) tag_atom: qjs::JSAtom,
    pub(super) live_async: RefCell<HashSet<usize>>,
    pub(super) live_tsfn: RefCell<HashSet<usize>>,
    pub(super) dead: Cell<bool>,
    pub(super) external_memory: Cell<i64>,
}

const ERROR_MESSAGES: [&[u8]; 24] = [
    b"\0",
    b"Invalid argument\0",
    b"An object was expected\0",
    b"A string was expected\0",
    b"A string or symbol was expected\0",
    b"A function was expected\0",
    b"A number was expected\0",
    b"A boolean was expected\0",
    b"An array was expected\0",
    b"Unknown failure\0",
    b"An exception is pending\0",
    b"The async work item was cancelled\0",
    b"napi_escape_handle already called on scope\0",
    b"Invalid handle scope usage\0",
    b"Invalid callback scope usage\0",
    b"Thread-safe function queue is full\0",
    b"Thread-safe function handle is closing\0",
    b"A bigint was expected\0",
    b"A date was expected\0",
    b"An arraybuffer was expected\0",
    b"A detachable arraybuffer was expected\0",
    b"Main thread would deadlock\0",
    b"External buffers are not allowed\0",
    b"Cannot run JavaScript\0",
];

impl Env {
    pub(super) fn new(
        ctx: *mut qjs::JSContext,
        external_class: qjs::JSClassID,
        events: Arc<HostEvents>,
        module_url: CString,
    ) -> *mut Env {
        let wrap_atom = new_private_atom(ctx, "tiktools.napi.wrap");
        let tag_atom = new_private_atom(ctx, "tiktools.napi.type-tag");
        Box::into_raw(Box::new(Env {
            ctx,
            external_class,
            events,
            module_url,
            handles: RefCell::new(Vec::new()),
            scopes: RefCell::new(Vec::new()),
            pending: Cell::new(None),
            error: RefCell::new(Box::new(ExtendedErrorInfo {
                error_message: std::ptr::null(),
                engine_reserved: std::ptr::null_mut(),
                engine_error_code: 0,
                error_code: OK,
            })),
            refs: RefCell::default(),
            finalizers: RefCell::default(),
            cleanup_hooks: RefCell::default(),
            async_cleanup_hooks: RefCell::default(),
            instance_data: Cell::new((std::ptr::null_mut(), None, std::ptr::null_mut())),
            wrap_atom,
            tag_atom,
            live_async: RefCell::default(),
            live_tsfn: RefCell::default(),
            dead: Cell::new(false),
            external_memory: Cell::new(0),
        }))
    }

    // ----- handles -------------------------------------------------

    /// Stores an owned value and returns its handle.
    pub(super) fn push(&self, value: qjs::JSValue) -> Handle {
        let mut handles = self.handles.borrow_mut();
        handles.push(value);
        handles.len() as Handle
    }

    /// Stores a duplicate of a borrowed value.
    pub(super) fn push_dup(&self, value: qjs::JSValue) -> Handle {
        self.push(unsafe { qjs::JS_DupValue(self.ctx, value) })
    }

    /// Borrows the value behind a handle (no reference is taken).
    pub(super) fn get(&self, handle: Handle) -> Result<qjs::JSValue, i32> {
        let index = (handle as usize).wrapping_sub(1);
        self.handles
            .borrow()
            .get(index)
            .copied()
            .ok_or(INVALID_ARG)
    }

    /// Opens a handle scope; the token is the arena length to restore.
    pub(super) fn open_scope(&self, escapable: bool) -> usize {
        let escape_slot = escapable.then(|| {
            let slot = self.handles.borrow().len();
            self.push(qjs::JS_UNDEFINED);
            slot
        });
        let mut scopes = self.scopes.borrow_mut();
        scopes.push(Scope {
            start: self.handles.borrow().len(),
            escape_slot,
            escaped: false,
        });
        scopes.len()
    }

    /// Closes the scope with this token (LIFO), freeing its handles.
    pub(super) fn close_scope(&self, token: usize) -> Result<(), i32> {
        let scope = {
            let mut scopes = self.scopes.borrow_mut();
            if scopes.len() != token || token == 0 {
                return Err(HANDLE_SCOPE_MISMATCH);
            }
            scopes.pop().ok_or(HANDLE_SCOPE_MISMATCH)?
        };
        self.truncate_handles(scope.start);
        Ok(())
    }

    /// Promotes `value` into the slot reserved by an escapable scope.
    pub(super) fn escape(&self, token: usize, value: qjs::JSValue) -> Result<Handle, i32> {
        let mut scopes = self.scopes.borrow_mut();
        let scope = scopes
            .get_mut(token.wrapping_sub(1))
            .ok_or(INVALID_ARG)?;
        let slot = scope.escape_slot.ok_or(INVALID_ARG)?;
        if scope.escaped {
            return Err(ESCAPE_CALLED_TWICE);
        }
        scope.escaped = true;
        let mut handles = self.handles.borrow_mut();
        let old = std::mem::replace(
            &mut handles[slot],
            unsafe { qjs::JS_DupValue(self.ctx, value) },
        );
        unsafe { qjs::JS_FreeValue(self.ctx, old) };
        Ok((slot + 1) as Handle)
    }

    fn truncate_handles(&self, len: usize) {
        loop {
            let value = {
                let mut handles = self.handles.borrow_mut();
                if handles.len() <= len {
                    return;
                }
                handles.pop()
            };
            if let Some(value) = value {
                unsafe { qjs::JS_FreeValue(self.ctx, value) };
            }
        }
    }

    // ----- exceptions ----------------------------------------------

    pub(super) fn has_pending(&self) -> bool {
        let value = self.pending.take();
        let pending = value.is_some();
        self.pending.set(value);
        pending
    }

    /// Records a thrown value (takes ownership) as the pending exception.
    pub(super) fn set_pending(&self, exception: qjs::JSValue) {
        if let Some(previous) = self.pending.replace(Some(exception)) {
            unsafe { qjs::JS_FreeValue(self.ctx, previous) };
        }
    }

    pub(super) fn take_pending(&self) -> Option<qjs::JSValue> {
        self.pending.take()
    }

    /// Moves the engine's current exception into the pending slot.
    pub(super) fn capture_exception(&self) -> i32 {
        let exception = unsafe { qjs::JS_GetException(self.ctx) };
        self.set_pending(exception);
        PENDING_EXCEPTION
    }

    // ----- status ---------------------------------------------------

    pub(super) fn set_status(&self, status: i32) {
        let mut error = self.error.borrow_mut();
        error.error_code = status;
        error.error_message = if status == OK {
            std::ptr::null()
        } else {
            ERROR_MESSAGES
                .get(status as usize)
                .map_or(std::ptr::null(), |text| text.as_ptr().cast::<c_char>())
        };
    }

    pub(super) fn error_info(&self) -> *const ExtendedErrorInfo {
        let error = self.error.borrow();
        &**error as *const ExtendedErrorInfo
    }

    // ----- lifecycle ------------------------------------------------

    /// Frees every retained value and runs the registered cleanup hooks in
    /// reverse order. After this the env only tolerates late finalizers.
    pub(super) fn shutdown(&self) {
        // Cleanup hooks run LIFO, like Node.
        loop {
            let hook = self.cleanup_hooks.borrow_mut().pop();
            match hook {
                Some((Some(hook), arg)) => unsafe { hook(arg) },
                Some(_) => {}
                None => break,
            }
        }
        loop {
            let hook = self.async_cleanup_hooks.borrow_mut().pop();
            match hook {
                Some((Some(hook), arg, handle)) => {
                    if !async_hook_removed(handle) {
                        unsafe { hook(handle as Opaque, arg) }
                    }
                }
                Some(_) => {}
                None => break,
            }
        }
        self.run_finalizers();
        let (data, finalize, hint) = self.instance_data.replace((
            std::ptr::null_mut(),
            None,
            std::ptr::null_mut(),
        ));
        if let Some(finalize) = finalize {
            let this = self as *const Env as *mut Env;
            unsafe { finalize(this, data, hint) };
        }
        let references: Vec<_> = self.refs.borrow_mut().drain().collect();
        for (_, reference) in references {
            unsafe { qjs::JS_FreeValue(self.ctx, reference.value) };
        }
        if let Some(pending) = self.pending.take() {
            unsafe { qjs::JS_FreeValue(self.ctx, pending) };
        }
        self.truncate_handles(0);
        self.scopes.borrow_mut().clear();
        self.dead.set(true);
    }

    /// Runs finalizers queued by the garbage collector.
    pub(super) fn run_finalizers(&self) {
        let this = self as *const Env as *mut Env;
        loop {
            let next = self.finalizers.borrow_mut().pop_front();
            let Some(item) = next else { break };
            let Some(cb) = item.cb else { continue };
            let token = self.open_scope(false);
            unsafe { cb(this, item.data, item.hint) };
            let _ = self.close_scope(token);
        }
    }
}

fn new_private_atom(ctx: *mut qjs::JSContext, description: &str) -> qjs::JSAtom {
    let description = CString::new(description).unwrap_or_default();
    unsafe {
        let symbol = qjs::JS_NewSymbol(ctx, description.as_ptr(), false);
        let atom = qjs::JS_ValueToAtom(ctx, symbol);
        qjs::JS_FreeValue(ctx, symbol);
        atom
    }
}

static REMOVED_ASYNC_HOOKS: Mutex<Vec<usize>> = Mutex::new(Vec::new());

pub(super) fn mark_async_hook_removed(handle: usize) {
    REMOVED_ASYNC_HOOKS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .push(handle);
}

fn async_hook_removed(handle: usize) -> bool {
    REMOVED_ASYNC_HOOKS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .contains(&handle)
}

/// Runs an API body: validates the env, converts panics into
/// `napi_generic_failure` and records the last error.
pub(super) fn run(env: Environment, body: impl FnOnce(&Env) -> Result<(), i32>) -> i32 {
    // SAFETY: the env pointer was handed to the addon by this host and is
    // leaked for the process lifetime.
    let Some(env) = (unsafe { env.as_ref() }) else {
        return INVALID_ARG;
    };
    let status = match catch_unwind(AssertUnwindSafe(|| {
        if env.dead.get() {
            return Err(CANNOT_RUN_JS);
        }
        body(env)
    })) {
        Ok(Ok(())) => OK,
        Ok(Err(status)) => status,
        Err(_) => GENERIC_FAILURE,
    };
    env.set_status(status);
    status
}

/// Like [`run`] for calls that may execute JavaScript: they refuse to run
/// while an exception is pending (`NAPI_PREAMBLE`).
pub(super) fn run_js(env: Environment, body: impl FnOnce(&Env) -> Result<(), i32>) -> i32 {
    run(env, |env| {
        if env.has_pending() {
            return Err(PENDING_EXCEPTION);
        }
        body(env)
    })
}

/// Writes an out-parameter, failing with `napi_invalid_arg` on NULL.
pub(super) fn put<T>(slot: *mut T, value: T) -> Result<(), i32> {
    if slot.is_null() {
        return Err(INVALID_ARG);
    }
    unsafe { slot.write(value) };
    Ok(())
}
