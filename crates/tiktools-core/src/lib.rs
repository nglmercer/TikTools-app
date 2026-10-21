//! TikTools application core.
//!
//! This crate owns domain orchestration and the host-side message contract. It
//! intentionally has no dependency on Winit, Wry, tray-icon, or any other GUI
//! implementation. The desktop crate supplies a `HostEmitter` and forwards
//! UI work to this crate from its Tokio runtime.

#[cfg(feature = "live-benchmark")]
pub mod benchmark;
pub mod contracts;
pub mod control;
pub mod db;
pub mod events;
pub mod ipc;
pub mod paths;
pub mod platform;
pub mod services;

mod automation_runtime;
mod helpers;
mod hotkey_bindings;
pub mod input_access;
mod ipc_handlers;
mod live_events;
pub mod live_monitor;
mod persistence;
mod plugin_diagnostics;
mod plugin_events;
mod plugin_intents;
mod plugin_invoker;
mod plugin_processors;
mod plugin_runtime;
mod runtime_state;
#[cfg(test)]
mod tests;

pub(crate) use automation_runtime::samples::{sample_automation_event, sample_user};
pub(crate) use helpers::*;
pub(crate) use tiktools_plugin_api::sync::{
    recover_mutex, recover_rwlock_read, recover_rwlock_write,
};

use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex, RwLock,
    },
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use serde_json::{json, Value};
use tiktools_plugin_api::{
    AudioPlayOptions, AudioPlaybackResult, MediaFileRef, MediaPickerOptions, MediaSelection,
};
use tiktools_plugin_loader::{plugin_roots, PluginManager};

#[cfg(feature = "native-tiktok")]
use tiktools_tiktok::{events::TikToolsEvent as NativeLiveEvent, ClientEvent, ConnectRequest};

#[cfg(not(feature = "native-tiktok"))]
#[derive(Debug)]
#[allow(dead_code)]
struct ConnectRequest {
    unique_id: String,
    session_cookie: String,
    room_id: Option<String>,
}

use crate::{
    events::EventBus,
    ipc::messages::{HostMessage, PageMessage},
    paths::AppPaths,
    runtime_state::{
        AutomationRuntimeState, PluginRuntimeState, ProcessorRuntimeState, TransportHealthState,
    },
    services::{
        builtin_action_types, builtin_node_catalog, builtin_translations,
        media_selection_from_path_with_kind, option_sources::OptionSourceService,
        validate_audio_file_ref, validate_media_picker_options, AppStateService, AutomationService,
        AwardOptions, CapabilityBroker, LiveService, PointAction, PointsService,
    },
};

pub use services::{
    MediaApiError, MediaError, MediaHost, MediaHostError, MediaHostFuture, NoopMediaHost,
};

pub trait HostEmitter: Send + Sync {
    fn emit(&self, message: HostMessage);
    /// Hosts can opt out of compatibility visualization pushes. Defaults
    /// preserve existing emitters; accounting/run recording never depends on it.
    fn wants_automation_history(&self) -> bool {
        true
    }
    fn wants_leaderboard_snapshots(&self) -> bool {
        true
    }
    fn wants_automation_context_updates(&self) -> bool {
        true
    }
    /// Optional aggregate-only host transport diagnostics; headless hosts
    /// retain the default null value and need no GUI dependency.
    fn diagnostics(&self) -> serde_json::Value {
        serde_json::Value::Null
    }
}

pub(crate) struct PluginHealth {
    pub(crate) consecutive_failures: u32,
    pub(crate) next_retry_at: Option<Instant>,
}

/// The Rust application service graph. Each subsystem has separate ownership
/// so a future database/live/plugin implementation can be tested in isolation.
pub struct AppCore {
    pub live: Arc<LiveService>,
    live_monitor: live_monitor::LiveMonitor,
    pub points: Arc<PointsService>,
    pub automation: Arc<AutomationService>,
    pub capabilities: Arc<CapabilityBroker>,
    pub media: Arc<dyn MediaHost>,
    pub plugins: Arc<PluginManager>,
    pub db: Arc<db::DatabaseManager>,
    #[cfg(feature = "persistence")]
    analytics_writer: db::AnalyticsWriter,
    pub app_state: Arc<AppStateService>,
    pub events: EventBus,
    emitter: Arc<dyn HostEmitter>,
    pub(crate) automation_state: AutomationRuntimeState,
    #[cfg(all(feature = "persistence", feature = "native-tiktok"))]
    last_analytics_emit_at: AtomicU64,
    /// Monotonic revision of the persisted hotkey behavior projection. The
    /// plugin poll consumes this asynchronously so UI writes never wait for
    /// a process-plugin round trip.
    hotkey_sync_revision: AtomicU64,
    hotkey_synced_revision: AtomicU64,
    last_leaderboard_emit_at: AtomicU64,
    #[cfg(feature = "http")]
    http_client: Option<reqwest::Client>,
    #[cfg(feature = "http")]
    http_client_error: Option<String>,
    #[cfg(feature = "native-tiktok")]
    connection_sequence: AtomicU64,
    #[cfg(feature = "native-tiktok")]
    live_points_work: services::points_work::PointsWorkService,
    connection_context: RwLock<Option<LiveContext>>,
    #[cfg(feature = "native-tiktok")]
    live_pump_started: AtomicBool,
    /// Cancels *new* LIVE event admission only. Cancelling it stops the pump,
    /// which closes the queue sender and lets the worker drain everything
    /// already accepted. It is deliberately not the worker's token.
    #[cfg(feature = "native-tiktok")]
    live_admission: tokio_util::sync::CancellationToken,
    /// Emergency stop for the ordered worker, cancelled only after the
    /// graceful drain deadline expires. The worker then stops between items
    /// and the shutdown ladder accounts for whatever is left.
    #[cfg(feature = "native-tiktok")]
    live_ingress_stop: tokio_util::sync::CancellationToken,
    /// Bounded stage deadlines for `AppCore::stop_live_ingress`.
    #[cfg(feature = "native-tiktok")]
    live_shutdown_deadlines: Mutex<crate::live_events::pump::LiveShutdownDeadlines>,
    #[cfg(feature = "native-tiktok")]
    live_pump_task: Mutex<Option<tokio::task::JoinHandle<()>>>,
    /// Ordered ingress worker: the sole consumer of the bounded LIVE queue.
    /// Joined at shutdown after the pump so accepted events drain.
    #[cfg(feature = "native-tiktok")]
    live_ingress_task: Mutex<Option<tokio::task::JoinHandle<()>>>,
    /// Bounded staged-ingestion admission counters (accepted, shed,
    /// lagged, fenced). Always present with the native transport.
    #[cfg(feature = "native-tiktok")]
    pub(crate) live_ingress: crate::live_events::ingress::LiveIngressStats,
    /// Canonical replay guard: exact `(type, msg_id)` replays are skipped
    /// with a counter before any side effect. Survives reconnects by
    /// design (replays arrive after them); bounded by TTL plus capacity.
    #[cfg(feature = "native-tiktok")]
    pub(crate) live_dedup: crate::live_events::dedup::LiveDedup,
    option_sources: OptionSourceService,
    pub(crate) plugin_state: PluginRuntimeState,
    pub(crate) processor_state: ProcessorRuntimeState,
    pub(crate) plugin_last_events:
        Mutex<BTreeMap<String, crate::plugin_diagnostics::PluginLastEvent>>,
    pub(crate) plugin_event_drops: Mutex<BTreeMap<String, u64>>,
    pub(crate) plugin_delivery:
        Mutex<BTreeMap<String, crate::plugin_events::fanout::PluginDeliveryStats>>,
    pub(crate) hotkey_sync_state: Mutex<crate::plugin_diagnostics::HotkeySyncState>,
    shutdown_started: AtomicBool,
    shutdown_finished: AtomicBool,
    shutdown_complete: tokio::sync::Notify,
    pub(crate) transport_state: TransportHealthState,
}

impl AppCore {
    pub fn new(emitter: Arc<dyn HostEmitter>) -> Self {
        Self::with_media_host(emitter, Arc::new(NoopMediaHost))
    }

    /// Builds the core with an explicit native capability implementation.
    /// The desktop crate supplies this for file dialogs and audio output;
    /// headless callers can keep using [`AppCore::new`].
    pub fn with_media_host(emitter: Arc<dyn HostEmitter>, media: Arc<dyn MediaHost>) -> Self {
        let paths = AppPaths::from_environment();
        if let Err(error) = paths.ensure_directories() {
            tracing::warn!(%error, "could not create all Rust host directories");
        }
        // Process-plugin children inherit only an explicit env contract;
        // export the resolved plugin-data root (unless the operator set
        // one) so backends read the same `<root>/<id>/settings.json` the
        // host serves. Without this the loader default (`<package>/.data`)
        // and the host default diverge and backends silently miss settings.
        if std::env::var_os("TIKTOOLS_PLUGIN_DATA_DIR").is_none() {
            std::env::set_var("TIKTOOLS_PLUGIN_DATA_DIR", paths.plugin_data.as_os_str());
        }

        let roots = plugin_roots(
            paths.builtin_plugins.clone(),
            paths.plugins.clone(),
            paths.development_plugins.clone(),
        );
        let plugins = Arc::new(PluginManager::new(roots));
        match plugins.scan() {
            Ok(entries) => tracing::info!(count = entries.len(), "runtime plugin scan complete"),
            Err(error) => tracing::warn!(%error, "runtime plugin scan failed"),
        }

        let db = Arc::new(db::DatabaseManager::new(paths));
        let capabilities = Arc::new(CapabilityBroker::new(db.paths().plugin_data.clone()));

        let live = {
            #[cfg(feature = "native-tiktok")]
            {
                let config = tiktools_tiktok::NativeTikTokConfig {
                    bundle_cache_path: Some(db.paths().data.join("webmssdk.js")),
                    ..Default::default()
                };
                LiveService::with_native_config(config)
            }
            #[cfg(not(feature = "native-tiktok"))]
            {
                LiveService::new()
            }
        };

        let automation = Arc::new(AutomationService::default());
        #[cfg(feature = "persistence")]
        let plugin_activation = match db.load_behavior_snapshot() {
            Ok(snapshot) => {
                automation.replace_snapshot(&snapshot);
                crate::plugin_runtime::activation_from_snapshot(&snapshot)
            }
            Err(_) => BTreeMap::new(),
        };
        #[cfg(not(feature = "persistence"))]
        let plugin_activation: BTreeMap<String, crate::plugin_runtime::PluginActivation> =
            BTreeMap::new();
        #[cfg(feature = "http")]
        let (http_client, http_client_error) = match tiktools_plugin_sdk::http::client_builder()
            .and_then(|builder| {
                builder
                    .redirect(reqwest::redirect::Policy::none())
                    .build()
                    .map_err(|error| error.to_string())
            }) {
            Ok(client) => (Some(client), None),
            Err(error) => {
                let message = format!(
                        "HTTP automation is disabled because the hardened client could not be created: {error}"
                    );
                tracing::error!(%error, "hardened HTTP client unavailable; refusing insecure fallback");
                (None, Some(message))
            }
        };

        let points = Arc::new(PointsService::new(db.clone()));
        #[cfg(feature = "native-tiktok")]
        let live_points_work = services::points_work::PointsWorkService::new(Arc::clone(&points));
        let core = Self {
            live: Arc::new(live),
            live_monitor: live_monitor::LiveMonitor::default(),
            points,
            #[cfg(feature = "native-tiktok")]
            live_points_work,
            automation,
            capabilities,
            media,
            plugins,
            #[cfg(feature = "persistence")]
            analytics_writer: db::AnalyticsWriter::new(Arc::clone(&db)),
            db,
            app_state: Arc::new(AppStateService::default()),
            events: EventBus::new(256),
            emitter,
            automation_state: AutomationRuntimeState::default(),
            #[cfg(all(feature = "persistence", feature = "native-tiktok"))]
            last_analytics_emit_at: AtomicU64::new(0),
            hotkey_sync_revision: AtomicU64::new(1),
            hotkey_synced_revision: AtomicU64::new(0),
            last_leaderboard_emit_at: AtomicU64::new(0),
            #[cfg(feature = "http")]
            http_client,
            #[cfg(feature = "http")]
            http_client_error,
            #[cfg(feature = "native-tiktok")]
            connection_sequence: AtomicU64::new(0),
            connection_context: RwLock::new(None),
            #[cfg(feature = "native-tiktok")]
            live_pump_started: AtomicBool::new(false),
            #[cfg(feature = "native-tiktok")]
            live_admission: tokio_util::sync::CancellationToken::new(),
            #[cfg(feature = "native-tiktok")]
            live_ingress_stop: tokio_util::sync::CancellationToken::new(),
            #[cfg(feature = "native-tiktok")]
            live_shutdown_deadlines: Mutex::new(
                crate::live_events::pump::LiveShutdownDeadlines::from_env(),
            ),
            #[cfg(feature = "native-tiktok")]
            live_pump_task: Mutex::new(None),
            #[cfg(feature = "native-tiktok")]
            live_ingress_task: Mutex::new(None),
            #[cfg(feature = "native-tiktok")]
            live_ingress: crate::live_events::ingress::LiveIngressStats::default(),
            #[cfg(feature = "native-tiktok")]
            live_dedup: crate::live_events::dedup::LiveDedup::default(),
            option_sources: OptionSourceService::new(),
            plugin_state: PluginRuntimeState {
                activation: RwLock::new(plugin_activation),
                ..Default::default()
            },
            processor_state: ProcessorRuntimeState::default(),
            plugin_last_events: Mutex::new(BTreeMap::new()),
            plugin_event_drops: Mutex::new(BTreeMap::new()),
            plugin_delivery: Mutex::new(BTreeMap::new()),
            hotkey_sync_state: Mutex::new(crate::plugin_diagnostics::HotkeySyncState::default()),
            shutdown_started: AtomicBool::new(false),
            shutdown_finished: AtomicBool::new(false),
            shutdown_complete: tokio::sync::Notify::new(),
            transport_state: TransportHealthState::default(),
        };
        #[cfg(feature = "http")]
        if let Some(message) = core.http_client_error.clone() {
            core.emit(HostMessage::AutomationError { message });
        }
        core.rebuild_processor_index();
        core
    }

    pub fn emit(&self, message: HostMessage) {
        self.emitter.emit(message);
    }

    /// Publishes a leaderboard snapshot at a bounded rate. Callers that need
    /// an immediate snapshot (for example an explicit UI request) still emit
    /// directly; high-rate live events use this coalescing boundary.
    pub(crate) fn emit_leaderboard_if_due(&self) {
        if !self.emitter.wants_leaderboard_snapshots() {
            return;
        }
        const MIN_INTERVAL_MS: u64 = 250;
        let now = now_millis();
        let last = self.last_leaderboard_emit_at.load(Ordering::Acquire);
        if last != 0 && now.saturating_sub(last) < MIN_INTERVAL_MS {
            return;
        }
        if self
            .last_leaderboard_emit_at
            .compare_exchange(last, now, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            self.emit(HostMessage::Leaderboard {
                viewers: self.points.leaderboard(Some(50)),
            });
        }
    }

    /// Opens the host-owned native picker and returns a validated reference to
    /// the existing file/directory. The result contains metadata and a
    /// canonical path only; the host never copies the selected bytes.
    pub async fn open_media_picker(
        &self,
        options: MediaPickerOptions,
    ) -> Result<Option<MediaSelection>, MediaApiError> {
        validate_media_picker_options(&options)?;
        let mode = options.mode;
        let kind = options.kind;
        let path = self.media.open_picker(options).await?;
        path.map(|path| media_selection_from_path_with_kind(&path, mode, kind))
            .transpose()
            .map_err(MediaApiError::from)
    }

    /// Validates a media reference immediately before playback and then hands
    /// the canonical path to the desktop audio backend. This is the only core
    /// entry point used by automation and runtime plugins for local audio.
    pub async fn play_audio(
        &self,
        file: MediaFileRef,
        options: AudioPlayOptions,
    ) -> Result<AudioPlaybackResult, MediaApiError> {
        if !options.volume.is_finite() {
            return Err(MediaApiError::Validation(MediaError::InvalidOption(
                "volume",
            )));
        }
        let file = validate_audio_file_ref(&file, self.db.paths().data.as_path())?;
        self.media
            .play_audio(
                file,
                AudioPlayOptions {
                    volume: options.volume.clamp(0.0, 1.0),
                    ..options
                },
            )
            .await
            .map_err(MediaApiError::from)
    }

    #[cfg(feature = "plugin-install")]
    pub fn install_plugin(
        &self,
        archive: impl AsRef<std::path::Path>,
        replace_existing: bool,
    ) -> Result<
        tiktools_plugin_loader::InstalledPluginPackage,
        tiktools_plugin_loader::PluginLoaderError,
    > {
        let _install_lock = recover_mutex(&self.plugin_state.install_lock, "plugin install");
        let paths = self.db.paths();
        let installer = tiktools_plugin_loader::PluginInstaller {
            plugin_directory: paths.plugins.clone(),
            staging_directory: paths.temp.join("plugin-install"),
            replace_existing,
            provider_endpoints: None,
        };
        let archive = archive.as_ref();
        let manifest = installer.inspect_manifest(archive)?;
        let old_running = replace_existing && self.plugins.is_running(&manifest.id);
        if old_running {
            self.plugins.stop(&manifest.id)?;
        }
        let installed = match installer.install(archive) {
            Ok(installed) => installed,
            Err(error) => {
                if old_running {
                    restart_plugin(&self.plugins, &manifest.id);
                }
                return Err(error);
            }
        };
        if let Err(error) = self.plugins.scan().map(|_| ()) {
            if old_running {
                restart_plugin(&self.plugins, &manifest.id);
            }
            return Err(error);
        }
        if old_running {
            restart_plugin(&self.plugins, &manifest.id);
        }
        self.rebuild_processor_index();
        Ok(installed)
    }

    #[cfg(feature = "plugin-install")]
    pub fn uninstall_plugin(
        &self,
        id: &str,
    ) -> Result<(), tiktools_plugin_loader::PluginLoaderError> {
        let _install_lock = recover_mutex(&self.plugin_state.install_lock, "plugin install");
        let plugin = self
            .plugins
            .get(id)
            .ok_or_else(|| tiktools_plugin_loader::PluginLoaderError::NotFound(id.to_owned()))?;
        if plugin.source != tiktools_plugin_loader::PluginSource::User {
            return Err(tiktools_plugin_loader::PluginLoaderError::Runtime(
                "only user-installed plugin packages can be uninstalled".to_owned(),
            ));
        }
        let root = std::fs::canonicalize(&self.db.paths().plugins).map_err(|error| {
            tiktools_plugin_loader::PluginLoaderError::Runtime(format!(
                "could not resolve the user plugin directory: {error}"
            ))
        })?;
        let metadata = std::fs::symlink_metadata(&plugin.directory).map_err(|error| {
            tiktools_plugin_loader::PluginLoaderError::Runtime(format!(
                "could not inspect plugin package: {error}"
            ))
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(tiktools_plugin_loader::PluginLoaderError::Runtime(
                "plugin package is not a regular directory".to_owned(),
            ));
        }
        let package = std::fs::canonicalize(&plugin.directory).map_err(|error| {
            tiktools_plugin_loader::PluginLoaderError::Runtime(format!(
                "could not resolve plugin package: {error}"
            ))
        })?;
        if !is_strict_plugin_child(&root, &package) {
            return Err(tiktools_plugin_loader::PluginLoaderError::Runtime(
                "plugin package is outside the user plugin directory".to_owned(),
            ));
        }
        let was_running = self.plugins.is_running(id);
        if was_running {
            self.plugins.stop(id)?;
        }
        if let Err(error) = std::fs::remove_dir_all(&package) {
            if was_running {
                restart_plugin(&self.plugins, id);
            }
            return Err(tiktools_plugin_loader::PluginLoaderError::Runtime(format!(
                "could not remove plugin package: {error}"
            )));
        }
        #[cfg(feature = "persistence")]
        if let Err(error) = self.db.remove_plugin_state(id) {
            tracing::warn!(plugin = %id, %error, "plugin package was removed but persisted state could not be cleared");
        }
        self.clear_plugin_activation(id);
        self.plugins.scan().map(|_| ())?;
        self.rebuild_processor_index();
        Ok(())
    }

    pub async fn shutdown(self: &Arc<Self>) {
        if self.shutdown_started.swap(true, Ordering::AcqRel) {
            loop {
                let finished = self.shutdown_complete.notified();
                tokio::pin!(finished);
                finished.as_mut().enable();
                if self.shutdown_finished.load(Ordering::Acquire) {
                    return;
                }
                finished.await;
            }
        }
        {
            let _operation = self.live_monitor.operation.lock().await;
            self.stop_live_monitor().await;
        }
        // Keep a notification queued if the polling task is in a plugin call
        // when shutdown begins; `notify_waiters` alone would be lost before
        // the task reaches its select point.
        self.plugin_state.poll_shutdown.notify_one();
        // LIVE ingestion stops first and in its own order: admission closes,
        // the pump exits (dropping the queue sender), and the worker drains
        // every event already accepted before it is joined. The observer
        // token stays open during that drain so drained events keep their
        // automation side effects instead of being silently dropped here.
        #[cfg(feature = "native-tiktok")]
        self.stop_live_ingress().await;
        // Persistent cancellation: every supervisor and worker observes
        // shutdown no matter when it reaches its select point, and no
        // worker can steal the supervisor's wakeup.
        self.plugin_state.observer_shutdown.cancel();
        self.events
            .publish_domain(crate::events::DomainEvent::Shutdown);
        self.publish_disconnected_event().await;
        self.live.disconnect().await;
        let task = recover_mutex(&self.plugin_state.poll_task, "plugin poll task").take();
        if let Some(task) = task {
            let _ = task.await;
        }
        let push_task = recover_mutex(&self.plugin_state.push_task, "plugin push task").take();
        if let Some(task) = push_task {
            let _ = task.await;
        }
        let observer_task = recover_mutex(
            &self.plugin_state.observer_task,
            "plugin event observer task",
        )
        .take();
        if let Some(task) = observer_task {
            let _ = task.await;
        }
        self.plugins.stop_all();
        #[cfg(feature = "persistence")]
        self.analytics_writer.shutdown().await;
        // Join the owned points writer last: every earlier shutdown step may
        // still award points (disconnect handling included), and accepted
        // writes must reach SQLite before the process exits.
        #[cfg(feature = "native-tiktok")]
        self.live_points_work.shutdown().await;
        // Bounded: a wedged SQLite must not block process shutdown
        // forever. The deadline is an explicit failure policy — the error
        // below is the report that durable work may remain unresolved, and
        // the writer thread is joined (or abandoned) with that on record.
        let points = Arc::clone(&self.points);
        let flush = tokio::task::spawn_blocking(move || points.shutdown());
        match tokio::time::timeout(std::time::Duration::from_secs(30), flush).await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => tracing::error!(%error, "points writer shutdown failed"),
            Err(_) => tracing::error!(
                "points writer failed to stop within 30s; durable points work may remain unresolved"
            ),
        }
        self.shutdown_finished.store(true, Ordering::Release);
        self.shutdown_complete.notify_waiters();
    }
}

fn restart_plugin(plugins: &PluginManager, id: &str) {
    if let Err(error) = plugins.start(id) {
        tracing::warn!(plugin = %id, %error, "plugin runtime could not be restarted after package operation");
    }
}

fn is_strict_plugin_child(root: &std::path::Path, candidate: &std::path::Path) -> bool {
    if cfg!(target_os = "windows") {
        let root = root.to_string_lossy().to_ascii_lowercase();
        let candidate = candidate.to_string_lossy().to_ascii_lowercase();
        candidate.starts_with(&(root.clone() + "\\"))
            && candidate[root.len() + 1..].find('\\').is_none()
            && candidate[root.len() + 1..].find('/').is_none()
    } else {
        candidate.parent() == Some(root)
    }
}
