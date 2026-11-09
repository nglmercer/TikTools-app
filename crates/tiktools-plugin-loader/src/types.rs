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
    #[error("plugin startup outcome is unknown: {0}")]
    StartupUncertain(String),
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
    /// The host-assigned worker generation that emitted this event.
    pub generation: u64,
    pub event_type: String,
    pub data: serde_json::Value,
}

/// A host-bound event sender. Startup events are held until the manager has
/// registered this generation; failed startup closes and discards the buffer.
#[derive(Clone)]
pub struct PluginEventSender {
    inner: std::sync::Arc<std::sync::Mutex<EventSenderState>>,
}

enum EventPhase {
    Starting(Vec<EmittedEvent>),
    Live,
    Closed,
}

struct EventSenderState {
    plugin_id: String,
    generation: u64,
    tx: tokio::sync::broadcast::Sender<EmittedEvent>,
    phase: EventPhase,
}

impl PluginEventSender {
    pub(crate) fn starting(
        tx: tokio::sync::broadcast::Sender<EmittedEvent>,
        plugin_id: String,
        generation: u64,
    ) -> Self {
        Self {
            inner: std::sync::Arc::new(std::sync::Mutex::new(EventSenderState {
                plugin_id,
                generation,
                tx,
                phase: EventPhase::Starting(Vec::new()),
            })),
        }
    }

    pub(crate) fn live(
        tx: tokio::sync::broadcast::Sender<EmittedEvent>,
        plugin_id: String,
        generation: u64,
    ) -> Self {
        let sender = Self::starting(tx, plugin_id, generation);
        let _ = sender.activate_with(|| {});
        sender
    }

    /// The host overrides identity/generation, so plugins cannot assign their
    /// own producer identity. Errors, including buffer overflow, are explicit.
    pub fn emit(
        &self,
        event_type: impl Into<String>,
        data: serde_json::Value,
    ) -> Result<usize, String> {
        self.emit_many(vec![(event_type.into(), data)])
    }

    /// Emit a batch without supplying host-owned identity fields. A batch
    /// admitted to the startup buffer is accepted or rejected as a whole.
    pub fn emit_many(&self, events: Vec<(String, serde_json::Value)>) -> Result<usize, String> {
        self.send_many(
            events
                .into_iter()
                .map(|(event_type, data)| EmittedEvent {
                    plugin_id: String::new(),
                    generation: 0,
                    event_type,
                    data,
                })
                .collect(),
        )
    }

    pub fn send(&self, event: EmittedEvent) -> Result<usize, String> {
        self.send_many(vec![event])
    }

    /// Check and reserve an entire startup batch under one lock.
    pub fn send_many(&self, mut events: Vec<EmittedEvent>) -> Result<usize, String> {
        if events.len() > 64 {
            return Err("plugin event batch exceeds 64 events".into());
        }
        if events.is_empty() {
            return Ok(0);
        }
        for event in &events {
            if serde_json::to_vec(&(&event.event_type, &event.data))
                .map_or(true, |v| v.len() > 64 * 1024)
            {
                return Err("plugin event exceeds 64 KiB".into());
            }
        }
        let mut state = self
            .inner
            .lock()
            .map_err(|_| "plugin event sender is unavailable".to_owned())?;
        if state.tx.receiver_count() == 0 {
            return Err("plugin event bus unavailable".into());
        }
        for event in &mut events {
            event.plugin_id = state.plugin_id.clone();
            event.generation = state.generation;
        }
        let count = events.len();
        match &mut state.phase {
            EventPhase::Starting(buffer) => {
                if count > 64 - buffer.len() {
                    return Err("plugin startup event buffer is full".into());
                }
                buffer.extend(events);
                Ok(count)
            }
            EventPhase::Live => {
                for event in events {
                    state
                        .tx
                        .send(event)
                        .map_err(|_| "plugin event bus unavailable".to_owned())?;
                }
                Ok(count)
            }
            EventPhase::Closed => Err("plugin startup was cancelled".into()),
        }
    }

    pub(crate) fn activate_with(&self, before: impl FnOnce()) -> Result<(), String> {
        let mut state = self
            .inner
            .lock()
            .map_err(|_| "plugin event sender is unavailable".to_owned())?;
        if !matches!(state.phase, EventPhase::Starting(_)) {
            return Err("plugin startup was cancelled".into());
        }
        before();
        let EventPhase::Starting(buffer) = std::mem::replace(&mut state.phase, EventPhase::Live)
        else {
            unreachable!();
        };
        for event in buffer {
            if state.tx.send(event).is_err() {
                tracing::warn!(plugin = %state.plugin_id, "startup event could not be delivered; event subscribers left");
            }
        }
        Ok(())
    }

    pub(crate) fn close_if_starting(&self) {
        if let Ok(mut state) = self.inner.lock() {
            if matches!(state.phase, EventPhase::Starting(_)) {
                state.phase = EventPhase::Closed;
            }
        }
    }

    pub(crate) fn close(&self) {
        if let Ok(mut state) = self.inner.lock() {
            state.phase = EventPhase::Closed;
        }
    }
}

/// Manager-owned resources a runtime-owned worker needs at spawn. Today
/// this is the push-bus sender for guest-emitted events; runtimes that
/// spawn their own worker thread receive it here instead of allocating
/// per-plugin channels the manager could never drain.
pub struct WorkerContext {
    emit_tx: PluginEventSender,
    generation: u64,
    /// Startup-handshake bound the runtime must honour before reporting a
    /// load failure. Owned by the manager so one stuck plugin cannot freeze
    /// every other start/stop for longer than this.
    start_deadline: std::time::Duration,
    retain_uncertain_start: Option<StartupUncertainHandler>,
}

pub(crate) type StartupUncertainHandler =
    std::sync::Arc<dyn Fn(std::thread::JoinHandle<Result<(), PluginLoaderError>>) + Send + Sync>;

impl WorkerContext {
    pub(crate) fn new(
        emit_tx: tokio::sync::broadcast::Sender<EmittedEvent>,
        start_deadline: std::time::Duration,
        plugin_id: String,
        generation: u64,
        retain_uncertain_start: Option<StartupUncertainHandler>,
    ) -> Self {
        Self {
            emit_tx: PluginEventSender::starting(emit_tx, plugin_id, generation),
            start_deadline,
            generation,
            retain_uncertain_start,
        }
    }

    /// Clone the push-bus sender for guest-emitted events. `send` is
    /// synchronous and never blocks, so owner threads use it directly.
    pub fn emit_sender(&self) -> PluginEventSender {
        self.emit_tx.clone()
    }

    /// Identity to attach to every event produced by this worker.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub(crate) fn uncertain_start_handler(&self) -> Option<StartupUncertainHandler> {
        self.retain_uncertain_start.clone()
    }

    pub(crate) fn activate_events(&self, before: impl FnOnce()) -> Result<(), String> {
        self.emit_tx.activate_with(before)
    }

    /// How long a runtime may take to report a finished load (`onLoad`,
    /// native addon init) before the manager treats it as a failed start.
    pub fn start_deadline(&self) -> std::time::Duration {
        self.start_deadline
    }
}

impl Drop for WorkerContext {
    fn drop(&mut self) {
        self.emit_tx.close_if_starting();
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
