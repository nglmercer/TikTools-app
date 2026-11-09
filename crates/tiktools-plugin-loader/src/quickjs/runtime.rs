//! Plugin runtime adapter for the QuickJS host (`runtime: "napi-vm"`).
//!
//! The manifest runtime name stays `napi-vm` so installed plugins keep
//! working; the engine behind it is QuickJS (see the module docs in
//! `quickjs/mod.rs`).

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc as std_mpsc, Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

#[cfg(all(
    feature = "napi-vm-node-api",
    any(target_os = "linux", target_os = "macos", target_os = "windows")
))]
use tiktools_plugin_api::manifest::select_host_native_binary;
use tiktools_plugin_api::{
    capabilities::EVENTS_PUBLISH, manifest::declared_event_types, NativeAddonDeclaration,
    PluginManifest, PluginRuntimeKind, MAX_FRAME_BYTES,
};
use tokio::sync::{broadcast, mpsc, oneshot};

use super::{
    events::EventsPush,
    host::{HostConfig, QuickJsHost},
    modules::NativeAliases,
};
use crate::{
    worker::{QueuedCall, WorkerMsg},
    ManagedWorker, PluginEventSender, PluginInstance, PluginLoaderError, PluginRuntime,
    PluginSource, WorkerContext,
};

/// Cross-thread wake callback: native callbacks, async completions and
/// finalizers posted from other threads nudge the owner thread through it.
pub(crate) type WakeNotifier = Arc<dyn Fn() + Send + Sync>;

#[derive(Default)]
pub struct QuickJsPluginRuntime;

impl PluginRuntime for QuickJsPluginRuntime {
    fn kind(&self) -> PluginRuntimeKind {
        PluginRuntimeKind::NapiVm
    }

    fn load(
        &self,
        manifest: &PluginManifest,
        directory: &Path,
    ) -> Result<Box<dyn PluginInstance>, PluginLoaderError> {
        check_manifest(manifest)?;
        // Direct loads have no manager bus: emits fail loudly instead of
        // vanishing, so embedders learn push needs the managed path.
        let (orphan_tx, orphan_rx) = broadcast::channel(1);
        drop(orphan_rx);
        // Direct loads carry no manager context, so they use the default
        // startup bound; the managed path passes its own through
        // `WorkerContext`.
        let (tx, worker) = spawn_owner(
            manifest,
            directory,
            PluginEventSender::live(orphan_tx, manifest.id.clone(), 0),
            crate::manager::PLUGIN_START_DEADLINE,
            0,
        )?;
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
        ctx: &WorkerContext,
    ) -> Option<Result<ManagedWorker, PluginLoaderError>> {
        if let Err(error) = check_manifest(manifest) {
            return Some(Err(error));
        }
        match spawn_owner(
            manifest,
            directory,
            ctx.emit_sender(),
            ctx.start_deadline(),
            ctx.generation(),
        ) {
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
            admission: None,
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
/// Host capabilities arrive through Rust capability modules such as
/// [`EVENTS_CAPABILITY`]; this object stays additive so guests can ignore
/// unknown fields.
fn guest_context(manifest: &PluginManifest) -> serde_json::Value {
    serde_json::json!({
        "pluginId": manifest.id,
        "name": manifest.name,
        "version": manifest.version,
    })
}

/// Translates the TikTools manifest into the events request: a plugin that
/// declares the publish capability gets the `tiktools:events` module. The
/// guest import still requires core's per-event validation on delivery.
fn events_push_for(
    manifest: &PluginManifest,
    emit_tx: PluginEventSender,
    generation: u64,
) -> Option<Arc<EventsPush>> {
    if !manifest
        .capabilities
        .iter()
        .any(|capability| capability == EVENTS_PUBLISH)
    {
        return None;
    }
    Some(Arc::new(EventsPush {
        plugin_id: manifest.id.clone(),
        generation,
        declared: declared_event_types(manifest)
            .into_iter()
            .collect::<HashSet<_>>(),
        tx: emit_tx,
    }))
}

/// Spawns the dedicated VM owner thread and waits for the guest `onLoad`
/// to settle, so callers fail fast instead of reporting a dead instance.
/// Shared by the direct `load()` path and the manager's owned worker: in
/// both cases exactly one thread owns the VM.
#[allow(clippy::type_complexity)]
fn spawn_owner(
    manifest: &PluginManifest,
    directory: &Path,
    emit_tx: PluginEventSender,
    start_deadline: std::time::Duration,
    generation: u64,
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
    let version = manifest.version.clone();
    let entry = manifest.entry.clone();
    let native_addons = manifest.native_addons.clone();
    let directory = directory.to_owned();
    let events = events_push_for(manifest, emit_tx, generation);
    let (tx, rx) = mpsc::unbounded_channel();
    let (ready_tx, ready_rx) = std_mpsc::channel();
    let thread_id = id.clone();
    let wake_tx = tx.clone();
    let worker = thread::Builder::new()
        .name(format!("tiktools-napi-vm-{thread_id}"))
        // The guest stack limit (2 MiB) plus native frames must fit.
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            run_vm_owner(VmOwnerArgs {
                name,
                version,
                entry,
                directory,
                context,
                native_addons,
                events,
                rx,
                wake_tx,
                ready: ready_tx,
            })
        })
        .map_err(|error| {
            PluginLoaderError::Runtime(format!("could not start napi-vm thread: {error}"))
        })?;
    // The VM thread always answers the handshake before serving calls: a
    // dropped sender means it died during startup, which is also a load
    // failure, never a half-alive instance. The wait is bounded: a guest
    // wedged in `onLoad` (or a native addon hanging during load) must fail
    // the start instead of freezing every other plugin's lifecycle behind
    // the manager's lifecycle lock. A timeout asks the thread to unload
    // itself and then drops its handle (detach) — reported explicitly, so
    // the abandonment is on record rather than silent.
    match ready_rx.recv_timeout(start_deadline) {
        Ok(result) => result?,
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
            let _ = tx.send(WorkerMsg::Shutdown);
            tracing::error!(
                id = %id,
                deadline_ms = start_deadline.as_millis() as u64,
                "napi-vm startup missed its deadline; detaching the worker thread (outcome unknown)"
            );
            return Err(PluginLoaderError::Runtime(format!(
                "napi-vm plugin `{id}` did not finish loading within {}ms",
                start_deadline.as_millis()
            )));
        }
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
            return Err(PluginLoaderError::Runtime(format!(
                "napi-vm plugin `{id}` failed during startup"
            )));
        }
    }
    Ok((tx, worker))
}

/// Arguments for the VM owner-thread entry point, bundled so the thread
/// body takes one parameter instead of a long positional list.
struct VmOwnerArgs {
    name: String,
    version: String,
    entry: String,
    directory: PathBuf,
    context: serde_json::Value,
    native_addons: Vec<NativeAddonDeclaration>,
    events: Option<Arc<EventsPush>>,
    rx: mpsc::UnboundedReceiver<WorkerMsg>,
    wake_tx: mpsc::UnboundedSender<WorkerMsg>,
    ready: std_mpsc::Sender<Result<(), PluginLoaderError>>,
}

/// VM owner-thread main loop. Creates the QuickJS host here so the runtime
/// never exists anywhere else, authorizes declared native addons, reports
/// the `onLoad` outcome through the handshake channel, then serves the
/// manager's call queue until `Shutdown` unloads the guest.
///
/// Shutdown order: stop accepting calls, run guest `onUnload`, release the
/// runtime and its native addons, then exit this thread. The host never
/// terminates threads an addon detached itself: persistent addon resources
/// must expose their own stop API, which the guest calls from `onUnload`.
fn run_vm_owner(args: VmOwnerArgs) -> Result<(), PluginLoaderError> {
    let VmOwnerArgs {
        name,
        version,
        entry,
        directory,
        context,
        native_addons,
        events,
        mut rx,
        wake_tx,
        ready,
    } = args;
    // Guest load entry point: evaluates the guest module and settles its
    // `onLoad` (including native addon calls). A guest stuck there shows as
    // `load started` with no matching `loaded`, while the manager-side
    // `plugin starting` line stays open too.
    let load_started = Instant::now();
    tracing::info!(
        plugin = %name,
        native_addons = native_addons.len(),
        "napi-vm guest load started"
    );
    let fail = |ready: &std_mpsc::Sender<Result<(), PluginLoaderError>>,
                error: PluginLoaderError| {
        tracing::warn!(
            plugin = %name,
            elapsed_ms = load_started.elapsed().as_millis() as u64,
            %error,
            "napi-vm guest load failed"
        );
        let _ = ready.send(Err(error));
    };
    let aliases = match native_aliases(&name, &directory, &native_addons) {
        Ok(aliases) => aliases,
        Err(error) => {
            fail(&ready, error);
            return Ok(());
        }
    };
    let wake_pending = Arc::new(AtomicBool::new(false));
    let notifier = coalesced_wake(wake_tx.clone(), Arc::clone(&wake_pending));
    let mut host = match QuickJsHost::load(HostConfig {
        name: name.clone(),
        version,
        directory,
        entry,
        aliases,
        events,
        wake: notifier,
    }) {
        Ok(host) => host,
        Err(error) => {
            fail(
                &ready,
                PluginLoaderError::Runtime(format!(
                    "napi-vm plugin `{name}` failed to load: {error}"
                )),
            );
            return Ok(());
        }
    };
    tracing::info!(
        plugin = %name,
        elapsed_ms = load_started.elapsed().as_millis() as u64,
        "napi-vm guest loaded (onLoad settled)"
    );
    if ready.send(Ok(())).is_err() {
        return Ok(());
    }
    // Initial pump: run anything load queued (native handshake callbacks)
    // before sleeping, so the first idle state is fully drained.
    host.pump();
    loop {
        // Block with no timeout: commands and host-event wakes are the only
        // work sources — guest timers always drain inside the call that
        // scheduled them, and native callbacks arrive as host events — so
        // an idle plugin sleeps with zero recurring wakeups.
        match rx.blocking_recv() {
            Some(WorkerMsg::Call(call)) => {
                if call.respond.is_closed()
                    || call
                        .admission
                        .as_ref()
                        .is_some_and(|permit| permit.semaphore().is_closed())
                {
                    continue;
                }
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
                let outcome = handle_vm_call(&mut host, &call.request, &context);
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
            Some(WorkerMsg::HostEvent) => {
                // Clear before pumping: callbacks arriving during the pump
                // enqueue one further wake, so the reset cannot strand work.
                wake_pending.store(false, Ordering::Release);
            }
            Some(WorkerMsg::Shutdown) | None => break,
        }
        // Pump after every call and every wake so queued native callbacks
        // execute promptly without an arriving plugin request.
        host.pump();
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
    // errors: the runtime is released by dropping the host either way. The
    // started/finished pair brackets a hook stuck in native teardown.
    let unload_started = Instant::now();
    tracing::info!(plugin = %name, "napi-vm guest unload started (onUnload)");
    match host.unload() {
        Ok(()) => {
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

fn coalesced_wake(
    wake: mpsc::UnboundedSender<WorkerMsg>,
    pending: Arc<AtomicBool>,
) -> WakeNotifier {
    Arc::new(move || {
        if !pending.swap(true, Ordering::AcqRel) && wake.send(WorkerMsg::HostEvent).is_err() {
            pending.store(false, Ordering::Release);
        }
    })
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

/// Selects the exact host binary from each declared package root and
/// authorizes it before load: one `.node` per package, exposed to guests as
/// a native `require()` alias under the declared package name. With no
/// declaration this is empty and the guest runs without any native code.
/// Only the selected file is ever authorized; arbitrary `.node` files stay
/// refused even when they sit inside the plugin directory.
#[cfg(all(
    feature = "napi-vm-node-api",
    any(target_os = "linux", target_os = "macos", target_os = "windows")
))]
fn native_aliases(
    name: &str,
    directory: &Path,
    native_addons: &[NativeAddonDeclaration],
) -> Result<NativeAliases, PluginLoaderError> {
    let mut aliases = NativeAliases::new();
    if native_addons.is_empty() {
        return Ok(aliases);
    }
    let root = std::fs::canonicalize(directory).map_err(|error| {
        PluginLoaderError::Runtime(format!(
            "napi-vm plugin `{name}` directory is not usable: {error}"
        ))
    })?;
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
        aliases.insert(declaration.package.clone(), canonical);
    }
    Ok(aliases)
}

/// Builds without the native backend: any `nativeAddons` declaration fails
/// closed instead of silently running without its addons.
#[cfg(not(all(
    feature = "napi-vm-node-api",
    any(target_os = "linux", target_os = "macos", target_os = "windows")
)))]
fn native_aliases(
    name: &str,
    _directory: &Path,
    native_addons: &[NativeAddonDeclaration],
) -> Result<NativeAliases, PluginLoaderError> {
    if native_addons.is_empty() {
        return Ok(NativeAliases::new());
    }
    if cfg!(target_os = "android") {
        return Err(PluginLoaderError::RuntimeUnavailable(format!(
            "napi-vm plugin `{name}` requires native .node modules; Android ABI validation is not implemented"
        )));
    }
    Err(PluginLoaderError::Runtime(format!(
        "napi-vm plugin `{name}` declares native addons but this build lacks napi-vm-node-api support"
    )))
}

/// Runs one `PluginCall` through the guest `call(request, context)` export
/// and returns the raw `PluginCallResult` JSON bytes, like every other
/// runtime. Promise results are awaited inside `QuickJsHost::call`, which
/// drains the job queue and timers before returning.
fn handle_vm_call(
    host: &mut QuickJsHost,
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
    let response = host
        .call(&request_value, context)
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
    fn callback_flood_has_one_wake_and_reset_allows_next_callback() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let pending = Arc::new(AtomicBool::new(false));
        let wake = coalesced_wake(tx, Arc::clone(&pending));
        let threads: Vec<_> = (0..8)
            .map(|_| {
                let wake = Arc::clone(&wake);
                std::thread::spawn(move || {
                    for _ in 0..10_000 {
                        wake();
                    }
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
        assert_eq!(rx.len(), 1);
        assert!(matches!(rx.try_recv(), Ok(WorkerMsg::HostEvent)));
        pending.store(false, Ordering::Release);
        wake();
        assert_eq!(rx.len(), 1);
        drop(rx);
        pending.store(false, Ordering::Release);
        wake();
        assert!(!pending.load(Ordering::Acquire));
    }

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
        let (emit_tx, _emit_rx) = broadcast::channel(8);
        let ctx = WorkerContext::new(
            emit_tx,
            std::time::Duration::from_secs(15),
            manifest.id.clone(),
            1,
        );
        let owned = QuickJsPluginRuntime
            .spawn_worker(&manifest, &dir, &ctx)
            .expect("napi-vm provides an owned worker")
            .expect("owned worker spawns");
        let (tx, worker) = owned.into_parts();
        let (respond, answer) = oneshot::channel();
        tx.send(WorkerMsg::Call(QueuedCall {
            admission: None,
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
