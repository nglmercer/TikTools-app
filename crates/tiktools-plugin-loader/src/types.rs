use std::path::{Path, PathBuf};

use thiserror::Error;
use tiktools_plugin_api::{manifest::ManifestError, PluginManifest, PluginRuntimeKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginSource {
    Builtin,
    User,
    Development,
}

#[derive(Debug, Clone)]
pub struct PluginRoot {
    pub path: PathBuf,
    pub source: PluginSource,
}

#[derive(Debug, Clone)]
pub struct DiscoveredPlugin {
    pub manifest: PluginManifest,
    pub directory: PathBuf,
    pub source: PluginSource,
    pub available: bool,
    pub reason: Option<String>,
    pub running: bool,
}

#[derive(Debug, Error)]
pub enum PluginLoaderError {
    #[error("plugin manifest error: {0}")]
    Manifest(#[from] ManifestError),
    #[error("plugin directory is not valid: {0}")]
    InvalidDirectory(String),
    #[error("plugin runtime is unavailable: {0}")]
    RuntimeUnavailable(String),
    #[error("plugin runtime failed: {0}")]
    Runtime(String),
    #[error("plugin `{0}` was not discovered")]
    NotFound(String),
    #[error("plugin call timed out: {0}")]
    Timeout(String),
    #[error("plugin call queue is full: {0}")]
    Overloaded(String),
    #[error("plugin lock poisoned: {0}")]
    LockPoisoned(String),
}

pub trait PluginInstance: Send {
    fn id(&self) -> &str;
    fn handle_message(&mut self, request: &[u8]) -> Result<Vec<u8>, PluginLoaderError>;
    fn handle_message_with_timeout(
        &mut self,
        request: &[u8],
        _timeout: std::time::Duration,
    ) -> Result<Vec<u8>, PluginLoaderError> {
        self.handle_message(request)
    }
    fn shutdown(&mut self) -> Result<(), PluginLoaderError>;
}

pub trait PluginRuntime: Send + Sync {
    fn kind(&self) -> PluginRuntimeKind;
    fn load(
        &self,
        manifest: &PluginManifest,
        directory: &Path,
    ) -> Result<Box<dyn PluginInstance>, PluginLoaderError>;
    /// Fresh one-shot instance with a host-created per-call data directory.
    /// The default is appropriate for native/VM adapters that have no process
    /// settings contract. Native OS access remains trusted and unrestricted;
    /// this method does not claim to provide an OS sandbox.
    fn load_isolated(
        &self,
        manifest: &PluginManifest,
        directory: &Path,
        data_directory: &Path,
    ) -> Result<Box<dyn PluginInstance>, PluginLoaderError> {
        let _ = data_directory;
        self.load(manifest, directory)
    }
    /// Optionally provide a runtime-owned worker thread that serves the
    /// manager's call queue directly, bypassing the generic
    /// `run_instance_worker` thread. Runtimes whose state is `!Send`
    /// (napi-vm) use this so one owner thread per plugin serves calls,
    /// instead of a generic worker forwarding to a second VM thread.
    /// `None` (the default) keeps the generic worker.
    fn spawn_worker(
        &self,
        manifest: &PluginManifest,
        directory: &Path,
        ctx: &WorkerContext,
    ) -> Option<Result<ManagedWorker, PluginLoaderError>> {
        let _ = (manifest, directory, ctx);
        None
    }
}

/// One guest-pushed event waiting on the loader's push bus. Validated at
/// emit time (shape, declared type, size); core re-validates the publish
/// grant before delivery.
#[derive(Debug, Clone)]
pub struct EmittedEvent {
    pub plugin_id: String,
    pub event_type: String,
    pub data: serde_json::Value,
}

/// Manager-owned resources a runtime-owned worker needs at spawn. Today
/// this is the push-bus sender for guest-emitted events; runtimes that
/// spawn their own worker thread receive it here instead of allocating
/// per-plugin channels the manager could never drain.
pub struct WorkerContext {
    emit_tx: tokio::sync::broadcast::Sender<EmittedEvent>,
    /// Startup-handshake bound the runtime must honour before reporting a
    /// load failure. Owned by the manager so one stuck plugin cannot freeze
    /// every other start/stop for longer than this.
    start_deadline: std::time::Duration,
}

impl WorkerContext {
    pub(crate) fn new(
        emit_tx: tokio::sync::broadcast::Sender<EmittedEvent>,
        start_deadline: std::time::Duration,
    ) -> Self {
        Self {
            emit_tx,
            start_deadline,
        }
    }

    /// Clone the push-bus sender for guest-emitted events. `send` is
    /// synchronous and never blocks, so owner threads use it directly.
    pub fn emit_sender(&self) -> tokio::sync::broadcast::Sender<EmittedEvent> {
        self.emit_tx.clone()
    }

    /// How long a runtime may take to report a finished load (`onLoad`,
    /// native addon init) before the manager treats it as a failed start.
    pub fn start_deadline(&self) -> std::time::Duration {
        self.start_deadline
    }
}

/// A runtime-owned worker: the manager-facing end of a thread the runtime
/// spawned itself. Opaque on purpose: only the runtime that spawned the
/// thread understands its shutdown protocol, which mirrors the generic
/// worker's (`Shutdown` message, then join).
pub struct ManagedWorker {
    tx: tokio::sync::mpsc::UnboundedSender<crate::worker::WorkerMsg>,
    worker: std::thread::JoinHandle<Result<(), PluginLoaderError>>,
}

impl ManagedWorker {
    pub(crate) fn new(
        tx: tokio::sync::mpsc::UnboundedSender<crate::worker::WorkerMsg>,
        worker: std::thread::JoinHandle<Result<(), PluginLoaderError>>,
    ) -> Self {
        Self { tx, worker }
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        tokio::sync::mpsc::UnboundedSender<crate::worker::WorkerMsg>,
        std::thread::JoinHandle<Result<(), PluginLoaderError>>,
    ) {
        (self.tx, self.worker)
    }
}
