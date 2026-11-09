//! Node-API host for QuickJS: loads napi-rs (`.node`) addons in-process.
//!
//! The addon's `napi_*` imports resolve against a tiny C shim (see
//! `shim.rs`) that forwards every call into [`table::TABLE`]; the functions
//! behind that table (in `api/`) implement Node-API on QuickJS values.
//! Only a trusted, explicitly declared addon ever gets here (see
//! `native_aliases` in `runtime.rs`).

mod api;
mod env;
mod functions;
mod shim;
mod table;
mod types;

use std::{
    cell::RefCell,
    collections::VecDeque,
    ffi::{c_void, CString},
    path::Path,
    rc::Rc,
    sync::Arc,
    time::Duration,
};

use rquickjs::{qjs, Ctx, Exception, Value};

use self::{
    env::{Env, HostEvent, HostEvents},
    shim::NodeApiShim,
    types::*,
};
use super::modules::HostShared;

/// Per-plugin native state: one shim handle, one External class, one env per
/// loaded addon.
pub(crate) struct NativeState {
    external_class: qjs::JSClassID,
    events: Arc<HostEvents>,
    shim: Arc<NodeApiShim>,
    envs: Vec<*mut Env>,
}

pub(crate) type NativeSlot = RefCell<Option<NativeState>>;

/// `napi_module` for legacy `napi_module_register` addons.
#[repr(C)]
struct LegacyModule {
    version: i32,
    flags: u32,
    filename: *const std::ffi::c_char,
    register: Option<unsafe extern "C" fn(Environment, Handle) -> Handle>,
}

thread_local! {
    /// Modules registered by addon constructors while a library loads.
    pub(super) static REGISTRATIONS: RefCell<Vec<usize>> = const { RefCell::new(Vec::new()) };
}

const DEFAULT_MODULE_VERSION: i32 = 8;
const MAX_NAPI_VERSION: i32 = 10;

/// Checks that `path` is a regular file whose header matches this platform.
fn preflight(path: &Path) -> Result<(), String> {
    use std::io::Read;
    let metadata = std::fs::metadata(path).map_err(|e| format!("cannot stat native addon: {e}"))?;
    if !metadata.is_file() {
        return Err("native addon is not a regular file".to_owned());
    }
    let mut header = [0_u8; 4];
    std::fs::File::open(path)
        .and_then(|mut file| file.read_exact(&mut header))
        .map_err(|e| format!("cannot read native addon header: {e}"))?;
    let ok = if cfg!(target_os = "linux") {
        header == [0x7f, b'E', b'L', b'F']
    } else if cfg!(target_os = "windows") {
        header[..2] == *b"MZ"
    } else {
        matches!(
            u32::from_le_bytes(header),
            0xfeed_face | 0xfeed_facf | 0xcafe_babe | 0xbeba_feca | 0xcefa_edfe | 0xcffa_edfe
        )
    };
    if ok {
        Ok(())
    } else {
        Err(format!(
            "{} is not a native addon for this platform",
            path.display()
        ))
    }
}

unsafe extern "C" fn external_finalizer(_rt: *mut qjs::JSRuntime, value: qjs::JSValue) {
    unsafe { functions::finalize_external(value) };
}

fn init_state(ctx: &Ctx<'_>, shared: &HostShared) -> Result<NativeState, String> {
    let shim = NodeApiShim::load()?;
    let rt = unsafe { qjs::JS_GetRuntime(ctx.as_raw().as_ptr()) };
    let mut class_id: qjs::JSClassID = 0;
    unsafe { qjs::JS_NewClassID(rt, &mut class_id) };
    let def = qjs::JSClassDef {
        class_name: c"External".as_ptr(),
        finalizer: Some(external_finalizer),
        gc_mark: None,
        call: None,
        exotic: std::ptr::null_mut(),
    };
    if unsafe { qjs::JS_NewClass(rt, class_id, &def) } != 0 {
        return Err("cannot register the External class".to_owned());
    }
    Ok(NativeState {
        external_class: class_id,
        events: HostEvents::new(Arc::clone(&shared.wake)),
        shim,
        envs: Vec::new(),
    })
}

pub(crate) fn load_addon<'js>(
    ctx: &Ctx<'js>,
    shared: &Rc<HostShared>,
    path: &Path,
) -> rquickjs::Result<Value<'js>> {
    let fail = |message: String| Exception::throw_message(ctx, &message);
    tracing::debug!(plugin = %shared.plugin, addon = %path.display(), "loading native addon");
    preflight(path).map_err(fail)?;
    if shared.native.borrow().is_none() {
        let state = init_state(ctx, shared).map_err(fail)?;
        *shared.native.borrow_mut() = Some(state);
    }
    let (shim, events, external_class) = {
        let guard = shared.native.borrow();
        let state = guard.as_ref().expect("native state was just created");
        (
            Arc::clone(&state.shim),
            Arc::clone(&state.events),
            state.external_class,
        )
    };

    REGISTRATIONS.with(|registrations| registrations.borrow_mut().clear());
    let library = shim.open_addon(path);
    let registered: Vec<usize> = REGISTRATIONS.with(|r| std::mem::take(&mut *r.borrow_mut()));
    let library = library.map_err(|error| {
        fail(format!(
            "cannot load native addon {}: {error}",
            path.display()
        ))
    })?;

    type Register = unsafe extern "C" fn(Environment, Handle) -> Handle;
    let api_version = unsafe {
        library
            .get::<unsafe extern "C" fn() -> i32>(b"node_api_module_get_api_version_v1\0")
            .ok()
            .map(|symbol| *symbol)
    };
    let symbol_init = unsafe {
        library
            .get::<Register>(b"napi_register_module_v1\0")
            .ok()
            .map(|symbol| *symbol)
    };
    let (version, init): (i32, Register) = if let Some(api_version) = api_version {
        let init = symbol_init.ok_or_else(|| {
            fail(
                "addon exports a Node-API version getter but no napi_register_module_v1".to_owned(),
            )
        })?;
        (unsafe { api_version() }, init)
    } else if let (true, Some(init)) = (registered.is_empty(), symbol_init) {
        (DEFAULT_MODULE_VERSION, init)
    } else {
        match registered.as_slice() {
            [module] => {
                let module = unsafe { &*(*module as *const LegacyModule) };
                let init = module
                    .register
                    .ok_or_else(|| fail("registered module has no initializer".to_owned()))?;
                (1, init)
            }
            [] => return Err(fail(format!("{} is not a Node-API addon", path.display()))),
            _ => return Err(fail("addon registered several legacy modules".to_owned())),
        }
    };
    if !(1..=MAX_NAPI_VERSION).contains(&version) {
        return Err(fail(format!(
            "addon requests Node-API version {version}; supported: 1 through {MAX_NAPI_VERSION}"
        )));
    }

    let url = CString::new(format!("file://{}", path.display())).unwrap_or_default();
    let env_ptr = Env::new(ctx.as_raw().as_ptr(), external_class, events, url);
    if let Some(state) = shared.native.borrow_mut().as_mut() {
        state.envs.push(env_ptr);
    }
    let env = unsafe { &*env_ptr };
    let token = env.open_scope(false);
    let exports = unsafe { qjs::JS_NewObject(ctx.as_raw().as_ptr()) };
    let exports_handle = env.push_dup(exports);
    let returned = unsafe { init(env_ptr, exports_handle) };
    let result = if let Some(exception) = env.take_pending() {
        unsafe { qjs::JS_FreeValue(ctx.as_raw().as_ptr(), exports) };
        let thrown = unsafe { qjs::JS_Throw(ctx.as_raw().as_ptr(), exception) };
        let _ = thrown;
        let _ = env.close_scope(token);
        return Err(rquickjs::Error::Exception);
    } else if returned.is_null() {
        exports
    } else {
        match env.get(returned) {
            Ok(value) if !unsafe { qjs::JS_IsUndefined(value) } => {
                unsafe { qjs::JS_FreeValue(ctx.as_raw().as_ptr(), exports) };
                unsafe { qjs::JS_DupValue(ctx.as_raw().as_ptr(), value) }
            }
            _ => exports,
        }
    };
    let _ = env.close_scope(token);
    env.run_finalizers();
    drop(library);
    Ok(unsafe { Value::from_raw(ctx.clone(), result) })
}

impl NativeState {
    fn envs(&self) -> impl Iterator<Item = &Env> {
        self.envs.iter().map(|env| unsafe { &**env })
    }
}

/// Runs queued finalizers, thread-posted callbacks and async completions.
pub(crate) fn pump(shared: &HostShared) {
    let (events, envs): (Arc<HostEvents>, Vec<*mut Env>) = match shared.native.borrow().as_ref() {
        Some(state) => (Arc::clone(&state.events), state.envs.clone()),
        None => return,
    };
    let batch: VecDeque<HostEvent> = events.take();
    for event in batch {
        match event {
            HostEvent::Tsfn(ptr) => unsafe { functions::dispatch_tsfn(ptr.0) },
            HostEvent::AsyncDone(ptr, status) => unsafe {
                functions::complete_async(ptr.0, status)
            },
        }
    }
    for env in envs {
        unsafe { &*env }.run_finalizers();
    }
}

/// Whether native work (async work or threadsafe functions) could still
/// settle a pending promise.
pub(crate) fn has_pending_work(shared: &HostShared) -> bool {
    shared.native.borrow().as_ref().is_some_and(|state| {
        state
            .envs()
            .any(|env| !env.live_async.borrow().is_empty() || !env.live_tsfn.borrow().is_empty())
    })
}

/// Blocks until a host event arrives (or the timeout passes).
pub(crate) fn wait_for_event(shared: &HostShared, timeout: Duration) -> bool {
    let events = shared
        .native
        .borrow()
        .as_ref()
        .map(|state| Arc::clone(&state.events));
    events.is_some_and(|events| events.wait(timeout))
}

/// Tears every addon environment down: cleanup hooks, finalizers, handles.
pub(crate) fn shutdown(shared: &HostShared) {
    let envs: Vec<*mut Env> = shared
        .native
        .borrow()
        .as_ref()
        .map(|state| state.envs.clone())
        .unwrap_or_default();
    for env in envs {
        unsafe { functions::shutdown_env(&*env) };
    }
}

#[allow(dead_code)]
fn _assert_send(_: *mut c_void) {}
