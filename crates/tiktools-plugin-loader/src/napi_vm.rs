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

use std::{
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
};

use napi_vm::{RustPluginHost, RustPluginHostOptions, RustPluginPolicy};
use tiktools_plugin_api::{PluginManifest, PluginRuntimeKind, MAX_FRAME_BYTES};

use crate::{PluginInstance, PluginLoaderError, PluginRuntime};

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
        Ok(Box::new(NapiVmPluginInstance::spawn(manifest, directory)?))
    }
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
        let directory = directory.to_owned();
        let (tx, rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel();
        let thread_id = id.clone();
        let worker = thread::Builder::new()
            .name(format!("tiktools-napi-vm-{thread_id}"))
            .spawn(move || run_vm_owner(name, directory, context, rx, ready_tx))
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

/// VM owner-thread main loop. Creates the `RustPluginHost` here so the VM
/// never exists anywhere else, reports the `onLoad` outcome through the
/// handshake channel, then serves calls until `Shutdown` unloads the guest.
fn run_vm_owner(
    name: String,
    directory: PathBuf,
    context: serde_json::Value,
    rx: Receiver<VmCommand>,
    ready: Sender<Result<(), PluginLoaderError>>,
) {
    // Deny-by-default: no filesystem, no `node:path`, no capability modules
    // in this slice. TikTools-owned host APIs arrive as Rust capability
    // modules granted per plugin in a follow-up.
    let mut host = RustPluginHost::new(RustPluginHostOptions {
        policy: RustPluginPolicy::default(),
        ..RustPluginHostOptions::default()
    });
    if let Err(error) = host.load(&directory).map(|_| ()) {
        let _ = ready.send(Err(PluginLoaderError::Runtime(format!(
            "napi-vm plugin `{name}` failed to load: {error}"
        ))));
        return;
    }
    if ready.send(Ok(())).is_err() {
        return;
    }
    for command in rx {
        match command {
            VmCommand::Call { request, respond } => {
                let _ = respond.send(handle_vm_call(&mut host, &name, &request, &context));
            }
            VmCommand::Shutdown => break,
        }
    }
    // Best-effort `onUnload`: the VM is revoked by dropping the host either
    // way, so a failing hook is reported but never blocks teardown.
    if let Err(error) = host.unload(&name) {
        tracing::warn!(plugin = %name, %error, "napi-vm plugin failed in onUnload");
    }
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
}
