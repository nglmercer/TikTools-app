//! napi-vm JavaScript runtime for TypeScript-first plugins.
//!
//! Plugins are authored in TypeScript, compiled to JavaScript ahead of time,
//! and executed through `napi_vm::RustPluginHost`. There is no runtime
//! transpiler and no Node.js dependency on this path: guest code runs inside
//! napi-vm's pure-Rust interpreter with deny-by-default host capabilities.
//!
//! Threading: napi-vm's VM is `!Send` (`Rc`/`RefCell` internals), while
//! [`PluginInstance`] must be `Send`. The VM therefore lives on one dedicated
//! owner thread per plugin instance; the `Send` handle only carries a command
//! channel and the thread's join handle. The VM value never crosses threads.
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
//! inside the pure-Rust VM. While idle the owner thread pumps the napi-vm
//! event loop so native TSFN/async callbacks run without an arriving
//! plugin request.

use std::{
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
    time::Duration,
};

use napi_vm::{RustPluginHost, RustPluginHostOptions, RustPluginPolicy};
#[cfg(all(
    feature = "napi-vm-node-api",
    any(target_os = "linux", target_os = "macos", target_os = "windows")
))]
use tiktools_plugin_api::manifest::select_host_native_binary;
use tiktools_plugin_api::{
    NativeAddonDeclaration, PluginManifest, PluginRuntimeKind, MAX_FRAME_BYTES,
};

use crate::{PluginInstance, PluginLoaderError, PluginRuntime, PluginSource};

/// Commands the `Send` instance handle forwards to its dedicated VM thread.
/// The VM thread is the sole owner of the `RustPluginHost`; nothing
/// VM-backed ever crosses this channel, only plain bytes.
enum VmCommand {
    Call {
        request: Vec<u8>,
        respond: Sender<Result<Vec<u8>, PluginLoaderError>>,
    },
    Shutdown,
}

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
        if manifest.runtime != PluginRuntimeKind::NapiVm {
            return Err(PluginLoaderError::Runtime(
                "runtime kind mismatch".to_owned(),
            ));
        }
        if !manifest.native_addons.is_empty()
            // Direct runtime loads carry development-embedder provenance:
            // production loads go through `PluginManager::start`, which
            // passes the discovered source.
            && !native_addons_allowed(manifest, PluginSource::Development)
        {
            return Err(untrusted_native_addons(&manifest.id));
        }
        Ok(Box::new(NapiVmPluginInstance::spawn(manifest, directory)?))
    }
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
    tx: Option<Sender<VmCommand>>,
    worker: Option<JoinHandle<()>>,
}

impl NapiVmPluginInstance {
    /// Spawns the dedicated VM owner thread and waits for the guest `onLoad`
    /// to settle, so `load` fails fast instead of reporting a dead instance.
    fn spawn(manifest: &PluginManifest, directory: &Path) -> Result<Self, PluginLoaderError> {
        let id = manifest.id.clone();
        let name = manifest.name.clone();
        let context = guest_context(manifest);
        let native_addons = manifest.native_addons.clone();
        let directory = directory.to_owned();
        let (tx, rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel();
        let thread_id = id.clone();
        let worker = thread::Builder::new()
            .name(format!("tiktools-napi-vm-{thread_id}"))
            .spawn(move || {
                run_vm_owner(name, directory, context, native_addons, rx, ready_tx);
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
        Ok(Self {
            id,
            tx: Some(tx),
            worker: Some(worker),
        })
    }

    fn send_call(&mut self, request: &[u8]) -> Result<Vec<u8>, PluginLoaderError> {
        let tx = self.tx.as_ref().ok_or_else(|| {
            PluginLoaderError::Runtime(format!("napi-vm plugin `{}` stopped", self.id))
        })?;
        let (respond, answer) = mpsc::channel();
        tx.send(VmCommand::Call {
            request: request.to_vec(),
            respond,
        })
        .map_err(|_| {
            PluginLoaderError::Runtime(format!("napi-vm plugin `{}` worker is gone", self.id))
        })?;
        answer.recv().map_err(|_| {
            PluginLoaderError::Runtime(format!("napi-vm plugin `{}` worker died", self.id))
        })?
    }
}

impl PluginInstance for NapiVmPluginInstance {
    fn id(&self) -> &str {
        &self.id
    }

    fn handle_message(&mut self, request: &[u8]) -> Result<Vec<u8>, PluginLoaderError> {
        self.send_call(request)
    }

    /// Runs guest `onUnload` on the VM thread, then joins it. Every
    /// `eval_source` gets a fresh loop budget inside napi-vm, so spinning
    /// guest JavaScript terminates with a `RangeError` instead of wedging
    /// this join; only trusted native (`.node`/capability) code could block
    /// it, exactly like a native plugin could.
    fn shutdown(&mut self) -> Result<(), PluginLoaderError> {
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(VmCommand::Shutdown);
        }
        if let Some(worker) = self.worker.take() {
            worker.join().map_err(|_| {
                PluginLoaderError::Runtime(format!("plugin `{}` worker panicked", self.id))
            })?;
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
            let _ = tx.send(VmCommand::Shutdown);
        }
    }
}

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

/// How long the VM owner waits for the next command before pumping the
/// napi-vm event loop for native TSFN/async callbacks. Short enough for
/// interactive native workloads, long enough to sleep instead of spin.
const VM_EVENT_LOOP_POLL: Duration = Duration::from_millis(10);

/// VM owner-thread main loop. Creates the `RustPluginHost` here so the VM
/// never exists anywhere else, authorizes declared native addons, reports
/// the `onLoad` outcome through the handshake channel, then serves calls
/// until `Shutdown` unloads the guest.
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
    rx: Receiver<VmCommand>,
    ready: Sender<Result<(), PluginLoaderError>>,
) {
    // Deny-by-default: no filesystem, no `node:path`, no capability modules
    // in this slice. TikTools-owned host APIs arrive as Rust capability
    // modules granted per plugin in a follow-up. Native selection is
    // host-side (see below): guests receive no platform or libc facts and
    // load the addon through its package alias instead of a loader.
    let mut host = RustPluginHost::new(RustPluginHostOptions {
        policy: RustPluginPolicy::default(),
        ..RustPluginHostOptions::default()
    });
    if let Err(error) = configure_host_napi_addons(&mut host, &name, &directory, &native_addons) {
        let _ = ready.send(Err(error));
        return;
    }
    if let Err(error) = host.load(&directory).map(|_| ()) {
        let _ = ready.send(Err(PluginLoaderError::Runtime(format!(
            "napi-vm plugin `{name}` failed to load: {error}"
        ))));
        return;
    }
    if ready.send(Ok(())).is_err() {
        return;
    }
    loop {
        match rx.recv_timeout(VM_EVENT_LOOP_POLL) {
            Ok(VmCommand::Call { request, respond }) => {
                let _ = respond.send(handle_vm_call(&mut host, &name, &request, &context));
            }
            Ok(VmCommand::Shutdown) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        // Pump after every call and while idle so queued native callbacks
        // execute promptly without an arriving plugin request.
        pump_vm_event_loop(&mut host, &name);
    }
    // Best-effort `onUnload`: the VM is revoked by dropping the host either
    // way, so a failing hook is reported but never blocks teardown.
    if let Err(error) = host.unload(&name) {
        tracing::warn!(plugin = %name, %error, "napi-vm plugin failed in onUnload");
    }
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
/// runtime. Promise results are awaited inside the envelope; napi-vm drains
/// the VM job queue before `eval_source` returns.
fn handle_vm_call(
    host: &mut RustPluginHost,
    name: &str,
    request: &[u8],
    context: &serde_json::Value,
) -> Result<Vec<u8>, PluginLoaderError> {
    let request_text = std::str::from_utf8(request).map_err(|_| {
        PluginLoaderError::Runtime("napi-vm plugin request is not UTF-8".to_owned())
    })?;
    // Validate before embedding so a non-JSON caller gets a typed error
    // instead of a guest syntax failure.
    let _: serde_json::Value = serde_json::from_str(request_text).map_err(|error| {
        PluginLoaderError::Runtime(format!("plugin request is not JSON: {error}"))
    })?;
    let context_text = serde_json::to_string(context).map_err(|error| {
        PluginLoaderError::Runtime(format!("could not encode plugin context: {error}"))
    })?;
    let source = call_envelope(request_text, &context_text);
    let plugin = host.get_mut(name).ok_or_else(|| {
        PluginLoaderError::Runtime(format!("napi-vm plugin `{name}` is not loaded"))
    })?;
    let value = plugin
        .interpreter_mut()
        .eval_source(&source)
        .map_err(|error| PluginLoaderError::Runtime(format!("napi-vm call failed: {error}")))?;
    let napi_vm::Value::String(envelope) = &value else {
        return Err(PluginLoaderError::Runtime(
            "napi-vm call returned an invalid host result".to_owned(),
        ));
    };
    let envelope: serde_json::Value = serde_json::from_str(envelope).map_err(|error| {
        PluginLoaderError::Runtime(format!("napi-vm call returned invalid JSON: {error}"))
    })?;
    let serialized = envelope
        .get("serialized")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            PluginLoaderError::Runtime("napi-vm call result is not serializable".to_owned())
        })?;
    if serialized.len() > MAX_FRAME_BYTES {
        return Err(PluginLoaderError::Runtime(
            "napi-vm plugin returned an oversized result".to_owned(),
        ));
    }
    Ok(serialized.as_bytes().to_vec())
}

/// Builds the one-shot guest program for a call. Both fragments are
/// serializer-produced JSON, so they embed as safe JavaScript literals.
/// `__pluginInstance` is napi-vm's current guest-handle global; the envelope
/// fails closed with a TikTools-typed error when it is absent, and the whole
/// bridge is slated to move to an upstream `call_json` API (see docs).
fn call_envelope(request_json: &str, context_json: &str) -> String {
    format!(
        r#"await (async () => {{
  const request = {request_json};
  const context = {context_json};
  if (typeof __pluginInstance === "undefined" || __pluginInstance === null)
    throw new Error("tiktools: napi-vm guest instance is unavailable");
  if (typeof __pluginInstance.call !== "function")
    throw new Error("tiktools: napi-vm guest must export call(request, context)");
  const value = await __pluginInstance.call(request, context);
  if (value === undefined)
    throw new TypeError("tiktools: call must return a PluginCallResult object");
  const serialized = JSON.stringify(value);
  if (typeof serialized !== "string")
    throw new TypeError("tiktools: PluginCallResult must be JSON serializable");
  return JSON.stringify({{ defined: true, serialized }});
}})()"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_embeds_request_and_context_as_literals() {
        let source = call_envelope(r#"{"type":"poll"}"#, r#"{"pluginId":"demo"}"#);
        assert!(
            source.contains(r#"const request = {"type":"poll"};"#),
            "{source}"
        );
        assert!(
            source.contains(r#"const context = {"pluginId":"demo"};"#),
            "{source}"
        );
        assert!(
            source.contains("__pluginInstance.call(request, context)"),
            "{source}"
        );
        assert!(source.starts_with("await (async () => {"), "{source}");
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
