//! napi-vm JavaScript runtime for TypeScript-first plugins.
//!
//! Plugins are authored in TypeScript, compiled to JavaScript ahead of time,
//! and executed through `napi_vm::RustPluginHost`. There is no runtime
//! transpiler and no Node.js dependency on this path: guest code runs inside
//! napi-vm's pure-Rust interpreter with deny-by-default host capabilities.
//!
//! Threading: napi-vm's VM is `!Send` (`Rc`/`RefCell` internals), while
//! [`PluginInstance`] must be `Send`. Each plugin therefore runs on one
//! dedicated owner thread that serves the manager's call queue directly —
//! there is no generic worker thread in front of it (see
//! `PluginRuntime::spawn_worker`). The VM value never crosses threads:
//! only plain bytes move over the queue, and only the owner thread runs
//! guest code or drains the event loop.
//!
//! The owner sleeps with no timeout while idle: plugin commands and
//! host-event wakes (native callbacks, async completions, and finalizers
//! posted from other threads) are the only work sources, so an idle plugin
//! consumes zero recurring wakeups.
//!
//! Protocol: unchanged. The host sends `PluginCall` JSON and the guest
//! answers `PluginCallResult` JSON through its `call(request, context)`
//! export. TikTools stays the authority for its own `plugin.json`: the loader
//! parses the manifest, and napi-vm only reads its own compatibility subset
//! (`name`, `version`, `apiVersion`, `entry`) from the same file. Until
//! napi-vm grows a spec-based load entry point, napi-vm plugins carry
//! `"apiVersion": 1` next to the TikTools fields, and request no napi-vm
//! `permissions` object (that key collides with TikTools' own string-list
//! `permissions`, so host capabilities stay deny-by-default in this slice).
//!
//! Native addons (napi-rs `.node` binaries) are opt-in per plugin through
//! the manifest `nativeAddons` list, and only for explicitly trusted
//! manifests (see `native_addons_allowed`). Before `host.load`, the owner
//! thread selects the exact host binary from each declared package root,
//! authorizes that one file through napi-vm (which pins its contents
//! internally), and exposes it to guests as a native `require()` alias
//! under the declared package name; with no declaration the guest stays
//! inside the pure-Rust VM. While idle the owner thread sleeps: native
//! TSFN/async callbacks wake it through a host-event notifier instead of
//! a polling interval.

use std::{
    path::{Path, PathBuf},
    sync::{mpsc as std_mpsc, Arc},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use napi_vm::{RustPluginHost, RustPluginHostOptions, RustPluginPolicy, WakeNotifier};
#[cfg(all(
    feature = "napi-vm-node-api",
    any(target_os = "linux", target_os = "macos", target_os = "windows")
))]
use tiktools_plugin_api::manifest::select_host_native_binary;
use tiktools_plugin_api::{
    NativeAddonDeclaration, PluginManifest, PluginRuntimeKind, MAX_FRAME_BYTES,
};
use tokio::sync::{mpsc, oneshot};

use crate::{
    worker::{QueuedCall, WorkerMsg},
    ManagedWorker, PluginInstance, PluginLoaderError, PluginRuntime, PluginSource,
};

#[derive(Default)]
pub struct NapiVmPluginRuntime;

impl PluginRuntime for NapiVmPluginRuntime {
    fn kind(&self) -> PluginRuntimeKind {
        PluginRuntimeKind::NapiVm
    }

    fn load(
        &self,
        manifest: &PluginManifest,
        directory: &Path,
    ) -> Result<Box<dyn PluginInstance>, PluginLoaderError> {
        check_manifest(manifest)?;
        let (tx, worker) = spawn_owner(manifest, directory)?;
        Ok(Box::new(NapiVmPluginInstance {
            id: manifest.id.clone(),
            tx: Some(tx),
            worker: Some(worker),
        }))
    }

    fn spawn_worker(
        &self,
        manifest: &PluginManifest,
        directory: &Path,
    ) -> Option<Result<ManagedWorker, PluginLoaderError>> {
        if let Err(error) = check_manifest(manifest) {
            return Some(Err(error));
        }
        match spawn_owner(manifest, directory) {
            Ok((tx, worker)) => Some(Ok(ManagedWorker::new(tx, worker))),
            Err(error) => Some(Err(error)),
        }
    }
}

/// Shared load prelude: runtime-kind match plus the trusted-native choke
/// point (see `native_addons_allowed`). Production loads additionally pass
/// install provenance through `PluginManager::start`, which checks first;
/// direct loads carry development-embedder provenance here.
fn check_manifest(manifest: &PluginManifest) -> Result<(), PluginLoaderError> {
    if manifest.runtime != PluginRuntimeKind::NapiVm {
        return Err(PluginLoaderError::Runtime(
            "runtime kind mismatch".to_owned(),
        ));
    }
    if !manifest.native_addons.is_empty()
        && !native_addons_allowed(manifest, PluginSource::Development)
    {
        return Err(untrusted_native_addons(&manifest.id));
    }
    Ok(())
}

/// Whether this plugin may enable its `nativeAddons` declarations. This is
/// the single choke point for the trusted-native decision: callers pass
/// the manifest plus the provenance of the installed package, and no other
/// loader code makes trust distinctions for native code.
///
/// Native addons execute with TikTools process privileges and are not
/// sandboxed. The default napi-vm manifest is `Sandboxed`, so enabling
/// native code requires the manifest to opt out explicitly with
/// `"trust": "trusted"`. Internally pinned hashes and `checksums.json`
/// are integrity metadata — they prove the files are the ones that were
/// packaged, never that the native code is trustworthy — so install-time
/// provenance narrowing (quarantine or consent for user-installed
/// archives) belongs in this function when TikTools adds it.
pub fn native_addons_allowed(manifest: &PluginManifest, source: PluginSource) -> bool {
    use tiktools_plugin_api::PluginTrust;

    if !matches!(manifest.trust, PluginTrust::Trusted) {
        return false;
    }
    match source {
        // The operator's own checkout: same trust as running a local build.
        PluginSource::Development => true,
        // Shipped with TikTools, or installed from a checksummed archive
        // whose manifest explicitly opts into trusted native code.
        PluginSource::Builtin | PluginSource::User => true,
    }
}

pub(crate) fn untrusted_native_addons(id: &str) -> PluginLoaderError {
    PluginLoaderError::Runtime(format!(
        "napi-vm plugin `{id}` declares native addons but is not trusted; \
         native addons execute with TikTools process privileges and are not sandboxed"
    ))
}

struct NapiVmPluginInstance {
    id: String,
    tx: Option<mpsc::UnboundedSender<WorkerMsg>>,
    worker: Option<JoinHandle<Result<(), PluginLoaderError>>>,
}

impl PluginInstance for NapiVmPluginInstance {
    fn id(&self) -> &str {
        &self.id
    }

    /// Direct (unmanaged) call: enqueue on the owner thread and block for
    /// the answer. Sync-only: `blocking_recv` panics inside a Tokio
    /// runtime, so async callers must go through `PluginManager` instead.
    fn handle_message(&mut self, request: &[u8]) -> Result<Vec<u8>, PluginLoaderError> {
        let tx = self.tx.as_ref().ok_or_else(|| {
            PluginLoaderError::Runtime(format!("napi-vm plugin `{}` stopped", self.id))
        })?;
        let (respond, answer) = oneshot::channel();
        tx.send(WorkerMsg::Call(QueuedCall {
            request: request.to_vec(),
            timeout: DIRECT_CALL_TIMEOUT,
            deadline: Instant::now() + DIRECT_CALL_TIMEOUT,
            respond,
        }))
        .map_err(|_| {
            PluginLoaderError::Runtime(format!("napi-vm plugin `{}` worker is gone", self.id))
        })?;
        answer.blocking_recv().map_err(|_| {
            PluginLoaderError::Runtime(format!("napi-vm plugin `{}` worker died", self.id))
        })?
    }

    /// Runs guest `onUnload` on the VM thread, then joins it. Every guest
    /// call gets a fresh loop budget inside napi-vm, so spinning guest
    /// JavaScript terminates with a `RangeError` instead of wedging this
    /// join; only trusted native (`.node`/capability) code could block it,
    /// exactly like a native plugin could. An `onUnload` failure propagates,
    /// like the generic worker's shutdown errors.
    fn shutdown(&mut self) -> Result<(), PluginLoaderError> {
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(WorkerMsg::Shutdown);
        }
        if let Some(worker) = self.worker.take() {
            worker
                .join()
                .map_err(|_| {
                    PluginLoaderError::Runtime(format!("plugin `{}` worker panicked", self.id))
                })
                .and_then(|inner| inner)?;
        }
        Ok(())
    }
}

impl Drop for NapiVmPluginInstance {
    /// Backstop for a worker that never ran `shutdown` (for example after a
    /// manager-side panic): ask the VM thread to unload and detach. The
    /// thread exits on its own; dropping the join handle never blocks here.
    fn drop(&mut self) {
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(WorkerMsg::Shutdown);
        }
    }
}

/// Effective deadline for direct (unmanaged) `load()` calls. The direct
/// path predates worker deadlines and blocks until the guest answers; the
/// distant deadline preserves that while letting the owner share one
/// deadline-enforcing loop with the managed path.
const DIRECT_CALL_TIMEOUT: Duration = Duration::from_secs(365 * 24 * 60 * 60);

/// Guest-visible context for `call(request, context)`: plugin identity only.
/// Host capabilities arrive through Rust capability modules in a follow-up;
/// this object stays additive so guests can ignore unknown fields.
fn guest_context(manifest: &PluginManifest) -> serde_json::Value {
    serde_json::json!({
        "pluginId": manifest.id,
        "name": manifest.name,
        "version": manifest.version,
    })
}

/// Spawns the dedicated VM owner thread and waits for the guest `onLoad`
/// to settle, so callers fail fast instead of reporting a dead instance.
/// Shared by the direct `load()` path and the manager's owned worker: in
/// both cases exactly one thread owns the VM.
#[allow(clippy::type_complexity)]
fn spawn_owner(
    manifest: &PluginManifest,
    directory: &Path,
) -> Result<
    (
        mpsc::UnboundedSender<WorkerMsg>,
        JoinHandle<Result<(), PluginLoaderError>>,
    ),
    PluginLoaderError,
> {
    let id = manifest.id.clone();
    let name = manifest.name.clone();
    let context = guest_context(manifest);
    let native_addons = manifest.native_addons.clone();
    let directory = directory.to_owned();
    let (tx, rx) = mpsc::unbounded_channel();
    let (ready_tx, ready_rx) = std_mpsc::channel();
    let thread_id = id.clone();
    let wake_tx = tx.clone();
    let worker = thread::Builder::new()
        .name(format!("tiktools-napi-vm-{thread_id}"))
        .spawn(move || {
            run_vm_owner(
                name,
                directory,
                context,
                native_addons,
                rx,
                wake_tx,
                ready_tx,
            )
        })
        .map_err(|error| {
            PluginLoaderError::Runtime(format!("could not start napi-vm thread: {error}"))
        })?;
    // The VM thread always answers the handshake before serving calls: a
    // dropped sender means it died during startup, which is also a load
    // failure, never a half-alive instance.
    ready_rx.recv().map_err(|_| {
        PluginLoaderError::Runtime(format!("napi-vm plugin `{id}` failed during startup"))
    })??;
    Ok((tx, worker))
}

/// VM owner-thread main loop. Creates the `RustPluginHost` here so the VM
/// never exists anywhere else, authorizes declared native addons, reports
/// the `onLoad` outcome through the handshake channel, then serves the
/// manager's call queue until `Shutdown` unloads the guest.
///
/// Shutdown order: stop accepting calls, run guest `onUnload`, dispose the
/// napi-vm plugin and shut down its native runtime, then exit this thread.
/// The host never terminates threads an addon detached itself: persistent
/// addon resources must expose their own stop API, which the guest calls
/// from `onUnload`.
fn run_vm_owner(
    name: String,
    directory: PathBuf,
    context: serde_json::Value,
    native_addons: Vec<NativeAddonDeclaration>,
    mut rx: mpsc::UnboundedReceiver<WorkerMsg>,
    wake_tx: mpsc::UnboundedSender<WorkerMsg>,
    ready: std_mpsc::Sender<Result<(), PluginLoaderError>>,
) -> Result<(), PluginLoaderError> {
    // Deny-by-default: no filesystem, no `node:path`, no capability modules
    // in this slice. TikTools-owned host APIs arrive as Rust capability
    // modules granted per plugin in a follow-up. Native selection is
    // host-side (see below): guests receive no platform or libc facts and
    // load the addon through its package alias instead of a loader.
    let mut host = RustPluginHost::new(RustPluginHostOptions {
        policy: RustPluginPolicy::default(),
        ..RustPluginHostOptions::default()
    });
    // Guest load entry point: `host.load` evaluates the guest module and
    // settles its `onLoad` (including native addon calls). A guest stuck
    // there shows as `load started` with no matching `loaded`, while the
    // manager-side `plugin starting` line stays open too.
    let load_started = Instant::now();
    tracing::info!(
        plugin = %name,
        native_addons = native_addons.len(),
        "napi-vm guest load started"
    );
    if let Err(error) = configure_host_napi_addons(&mut host, &name, &directory, &native_addons) {
        tracing::warn!(
            plugin = %name,
            elapsed_ms = load_started.elapsed().as_millis() as u64,
            %error,
            "napi-vm guest load failed"
        );
        let _ = ready.send(Err(error));
        return Ok(());
    }
    if let Err(error) = host.load(&directory).map(|_| ()) {
        let error =
            PluginLoaderError::Runtime(format!("napi-vm plugin `{name}` failed to load: {error}"));
        tracing::warn!(
            plugin = %name,
            elapsed_ms = load_started.elapsed().as_millis() as u64,
            %error,
            "napi-vm guest load failed"
        );
        let _ = ready.send(Err(error));
        return Ok(());
    }
    tracing::info!(
        plugin = %name,
        elapsed_ms = load_started.elapsed().as_millis() as u64,
        "napi-vm guest loaded (onLoad settled)"
    );
    if ready.send(Ok(())).is_err() {
        return Ok(());
    }
    // Host-event wake: native callbacks, async completions, and finalizers
    // posted from other threads wake this loop through the command queue.
    // Bridges without threaded ingress ignore the registration.
    if let Some(plugin) = host.get_mut(&name) {
        let wake = wake_tx.clone();
        let notifier: WakeNotifier = Arc::new(move || {
            let _ = wake.send(WorkerMsg::HostEvent);
        });
        plugin.interpreter_mut().set_host_wake_notifier(notifier);
    }
    // Initial pump: run anything load queued (native handshake callbacks)
    // before sleeping, so the first idle state is fully drained.
    pump_vm_event_loop(&mut host, &name);
    loop {
        // Block with no timeout: commands and host-event wakes are the only
        // work sources — guest timers always drain inside the call that
        // scheduled them, and native callbacks arrive as host events — so
        // an idle plugin sleeps with zero recurring wakeups.
        match rx.blocking_recv() {
            Some(WorkerMsg::Call(call)) => {
                // Same queue-deadline rule as the generic worker: expire
                // here so a timed-out caller never causes stale guest work.
                if Instant::now() >= call.deadline {
                    let _ = call.respond.send(Err(PluginLoaderError::Timeout(format!(
                        "plugin `{name}` call expired while queued"
                    ))));
                    continue;
                }
                let kind = request_kind(&call.request);
                let call_started = Instant::now();
                let outcome = handle_vm_call(&mut host, &name, &call.request, &context);
                let elapsed_ms = call_started.elapsed().as_millis() as u64;
                // Steady-state poll ticks land here every second per plugin,
                // so only slow calls warn; the rest stay at debug.
                if elapsed_ms > SLOW_GUEST_CALL_MS {
                    tracing::warn!(
                        plugin = %name,
                        kind,
                        elapsed_ms,
                        "napi-vm guest call is slow; a blocking guest wedges its poll/action"
                    );
                } else {
                    tracing::debug!(
                        plugin = %name,
                        kind,
                        elapsed_ms,
                        "napi-vm guest call finished"
                    );
                }
                let _ = call.respond.send(outcome);
            }
            // A native callback, async completion, or finalizer arrived:
            // fall through to the pump below.
            Some(WorkerMsg::HostEvent) => {}
            Some(WorkerMsg::Shutdown) | None => break,
        }
        // Pump after every call and every wake so queued native callbacks
        // execute promptly without an arriving plugin request.
        pump_vm_event_loop(&mut host, &name);
    }
    // Fail waiters queued behind the shutdown instead of leaving them on
    // their deadlines; mirrors the generic worker.
    while let Ok(message) = rx.try_recv() {
        if let WorkerMsg::Call(call) = message {
            let _ = call.respond.send(Err(PluginLoaderError::Runtime(format!(
                "plugin `{name}` stopped"
            ))));
        }
    }
    // `onUnload` errors propagate like the generic worker's shutdown
    // errors: the VM is revoked by dropping the host either way. The
    // started/finished pair brackets a hook stuck in native teardown.
    let unload_started = Instant::now();
    tracing::info!(plugin = %name, "napi-vm guest unload started (onUnload)");
    match host.unload(&name) {
        Ok(_) => {
            tracing::info!(
                plugin = %name,
                elapsed_ms = unload_started.elapsed().as_millis() as u64,
                "napi-vm guest unloaded (onUnload settled)"
            );
            Ok(())
        }
        Err(error) => {
            let error = PluginLoaderError::Runtime(format!(
                "napi-vm plugin `{name}` failed in onUnload: {error}"
            ));
            tracing::warn!(
                plugin = %name,
                elapsed_ms = unload_started.elapsed().as_millis() as u64,
                %error,
                "napi-vm plugin failed in onUnload"
            );
            Err(error)
        }
    }
}

/// Guest calls slower than this warn instead of debug-logging. The poll
/// deadline is 5 s; 2 s leaves headroom while catching wedged guests.
const SLOW_GUEST_CALL_MS: u64 = 2000;

/// Best-effort `PluginCall` discriminator (`poll`, `action`, ...) for
/// call timing lines. Malformed requests keep their typed error
/// downstream; here they just log as `unknown`.
fn request_kind(request: &[u8]) -> String {
    serde_json::from_slice::<serde_json::Value>(request)
        .ok()
        .and_then(|value| {
            value
                .get("type")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_else(|| "unknown".to_owned())
}

/// Runs one non-blocking napi-vm event-loop turn so native TSFN/async
/// callbacks posted while the VM is idle execute promptly. A failure is
/// logged and never breaks the owner loop: one throwing callback must not
/// kill the plugin thread.
fn pump_vm_event_loop(host: &mut RustPluginHost, name: &str) {
    let Some(plugin) = host.get_mut(name) else {
        return;
    };
    if let Err(error) = plugin.interpreter_mut().run_event_loop_once(Duration::ZERO) {
        tracing::warn!(plugin = %name, %error, "napi-vm event-loop pump failed");
    }
}

/// Selects the exact host binary from each declared package root and
/// authorizes it before `host.load`: one `.node` per package, exposed to
/// guests as a native `require()` alias under the declared package name.
/// With no declaration this is a no-op and the guest stays inside the
/// pure-Rust VM. Only the selected file is ever authorized; arbitrary
/// `.node` files stay refused even when they sit inside the plugin
/// directory.
#[cfg(all(
    feature = "napi-vm-node-api",
    any(target_os = "linux", target_os = "macos", target_os = "windows")
))]
fn configure_host_napi_addons(
    host: &mut RustPluginHost,
    name: &str,
    directory: &Path,
    native_addons: &[NativeAddonDeclaration],
) -> Result<(), PluginLoaderError> {
    use napi_vm::RustPluginNapiOptions;

    if native_addons.is_empty() {
        return Ok(());
    }
    let root = std::fs::canonicalize(directory).map_err(|error| {
        PluginLoaderError::Runtime(format!(
            "napi-vm plugin `{name}` directory is not usable: {error}"
        ))
    })?;
    let mut options = RustPluginNapiOptions::default();
    for declaration in native_addons {
        // The manifest parser guarantees a safe-relative root; every
        // canonical containment check below still runs so a hand-built
        // manifest value can never authorize an escape.
        let package_root = std::fs::canonicalize(root.join(&declaration.root)).map_err(|_| {
            PluginLoaderError::Runtime(format!(
                "napi-vm plugin `{name}` native package `{}` is missing: {}",
                declaration.package,
                root.join(&declaration.root).display(),
            ))
        })?;
        if !package_root.starts_with(&root) {
            return Err(PluginLoaderError::Runtime(format!(
                "napi-vm plugin `{name}` native package `{}` escapes the plugin directory",
                declaration.package,
            )));
        }
        let selected = select_host_native_binary(&package_root).map_err(|error| {
            PluginLoaderError::Runtime(format!(
                "napi-vm plugin `{name}` package `{}`: {error}",
                declaration.package,
            ))
        })?;
        let canonical = std::fs::canonicalize(&selected).map_err(|_| {
            PluginLoaderError::Runtime(format!(
                "napi-vm plugin `{name}` native binary is missing: {}",
                selected.display(),
            ))
        })?;
        if !canonical.starts_with(&root) {
            return Err(PluginLoaderError::Runtime(format!(
                "napi-vm plugin `{name}` native binary escapes the plugin directory: {}",
                selected.display(),
            )));
        }
        tracing::debug!(
            plugin = %name,
            package = %declaration.package,
            path = %canonical.display(),
            "authorizing native addon binary",
        );
        options = options
            .allow_addon(&canonical)
            .allow_addon_alias(declaration.package.clone(), &canonical);
    }
    host.configure_napi_addons(name, options).map_err(|error| {
        PluginLoaderError::Runtime(format!(
            "napi-vm plugin `{name}` native addon configuration failed: {error}"
        ))
    })
}

/// Builds without the native backend: any `nativeAddons` declaration fails
/// closed instead of silently running without its addons.
#[cfg(not(all(
    feature = "napi-vm-node-api",
    any(target_os = "linux", target_os = "macos", target_os = "windows")
)))]
fn configure_host_napi_addons(
    _host: &mut RustPluginHost,
    name: &str,
    _directory: &Path,
    native_addons: &[NativeAddonDeclaration],
) -> Result<(), PluginLoaderError> {
    if native_addons.is_empty() {
        return Ok(());
    }
    Err(PluginLoaderError::Runtime(format!(
        "napi-vm plugin `{name}` declares native addons but this build lacks napi-vm-node-api support"
    )))
}

/// Runs one `PluginCall` through the guest `call(request, context)` export
/// and returns the raw `PluginCallResult` JSON bytes, like every other
/// runtime. Promise results are awaited inside `call_json`, which drains
/// the VM job queue before returning.
fn handle_vm_call(
    host: &mut RustPluginHost,
    name: &str,
    request: &[u8],
    context: &serde_json::Value,
) -> Result<Vec<u8>, PluginLoaderError> {
    let request_text = std::str::from_utf8(request).map_err(|_| {
        PluginLoaderError::Runtime("napi-vm plugin request is not UTF-8".to_owned())
    })?;
    // Decode once: the guest function receives converted values, never
    // source text, so a non-JSON caller gets a typed error up front.
    let request_value: serde_json::Value = serde_json::from_str(request_text).map_err(|error| {
        PluginLoaderError::Runtime(format!("plugin request is not JSON: {error}"))
    })?;
    let plugin = host.get_mut(name).ok_or_else(|| {
        PluginLoaderError::Runtime(format!("napi-vm plugin `{name}` is not loaded"))
    })?;
    let response = plugin
        .call_json(&request_value, context)
        .map_err(|error| PluginLoaderError::Runtime(format!("napi-vm call failed: {error}")))?;
    let serialized = serde_json::to_string(&response).map_err(|error| {
        PluginLoaderError::Runtime(format!("napi-vm call result is not serializable: {error}"))
    })?;
    if serialized.len() > MAX_FRAME_BYTES {
        return Err(PluginLoaderError::Runtime(
            "napi-vm plugin returned an oversized result".to_owned(),
        ));
    }
    Ok(serialized.into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guest_context_carries_plugin_identity() {
        let manifest = PluginManifest::from_json_str(
            r#"{"schemaVersion":3,"id":"example.plugin","name":"Example","version":"1.0.0","runtime":"napi-vm","entry":"dist/index.js"}"#,
        )
        .unwrap();
        assert_eq!(
            guest_context(&manifest),
            serde_json::json!({
                "pluginId": "example.plugin",
                "name": "Example",
                "version": "1.0.0",
            })
        );
    }

    #[test]
    fn request_kind_names_calls_and_degrades() {
        assert_eq!(request_kind(br#"{"type":"poll"}"#), "poll");
        assert_eq!(request_kind(br#"{"type":"action","action":{}}"#), "action");
        assert_eq!(request_kind(b"not json"), "unknown");
        assert_eq!(request_kind(br#"{"nope":true}"#), "unknown");
    }

    #[test]
    fn owned_worker_serves_calls_and_shuts_down() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/napi-vm-echo");
        let bytes = std::fs::read(dir.join("plugin.json")).unwrap();
        let manifest = PluginManifest::from_json_str(std::str::from_utf8(&bytes).unwrap()).unwrap();
        let owned = NapiVmPluginRuntime
            .spawn_worker(&manifest, &dir)
            .expect("napi-vm provides an owned worker")
            .expect("owned worker spawns");
        let (tx, worker) = owned.into_parts();
        let (respond, answer) = oneshot::channel();
        tx.send(WorkerMsg::Call(QueuedCall {
            request: br#"{"type":"poll"}"#.to_vec(),
            timeout: Duration::from_secs(30),
            deadline: Instant::now() + Duration::from_secs(30),
            respond,
        }))
        .unwrap();
        let response = answer.blocking_recv().unwrap().unwrap();
        let result: serde_json::Value = serde_json::from_slice(&response).unwrap();
        assert_eq!(
            result.get("events"),
            Some(
                &serde_json::json!([{ "type": "echo.tick", "data": { "plugin": "tiktools.napi-vm-echo" } }])
            ),
            "{result}"
        );
        tx.send(WorkerMsg::Shutdown).unwrap();
        worker.join().unwrap().unwrap();
    }

    fn trust_manifest(trust: &str) -> PluginManifest {
        PluginManifest::from_json_str(&format!(
            r#"{{"schemaVersion":3,"id":"example.plugin","name":"Example","version":"1.0.0","runtime":"napi-vm","entry":"dist/index.js","trust":"{trust}"}}"#,
        ))
        .unwrap()
    }

    #[test]
    fn native_addons_require_explicit_trust() {
        use tiktools_plugin_api::PluginTrust;

        let trusted = trust_manifest("trusted");
        assert_eq!(trusted.trust, PluginTrust::Trusted);
        for source in [
            PluginSource::Builtin,
            PluginSource::User,
            PluginSource::Development,
        ] {
            assert!(native_addons_allowed(&trusted, source), "{source:?}");
        }
        // Sandboxed (the napi-vm default) and Untrusted manifests fail
        // closed on every source: enabling native code is an explicit
        // opt-out of the sandbox.
        for trust in ["sandboxed", "untrusted"] {
            let manifest = trust_manifest(trust);
            for source in [
                PluginSource::Builtin,
                PluginSource::User,
                PluginSource::Development,
            ] {
                assert!(
                    !native_addons_allowed(&manifest, source),
                    "{trust} {source:?}"
                );
            }
        }
        // The default napi-vm manifest carries no trust label, hence no
        // native code.
        let default = PluginManifest::from_json_str(
            r#"{"schemaVersion":3,"id":"example.plugin","name":"Example","version":"1.0.0","runtime":"napi-vm","entry":"dist/index.js"}"#,
        )
        .unwrap();
        assert_eq!(default.trust, PluginTrust::Sandboxed);
        assert!(!native_addons_allowed(&default, PluginSource::Development));
    }
}
