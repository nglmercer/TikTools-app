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
    ) -> Option<Result<ManagedWorker, PluginLoaderError>> {
        let _ = (manifest, directory);
        None
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
