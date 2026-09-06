//! TikTools application core.
//!
//! This crate owns domain orchestration and the host-side message contract. It
//! intentionally has no dependency on Winit, Wry, tray-icon, or any other GUI
//! implementation. The desktop crate supplies a `HostEmitter` and forwards
//! UI work to this crate from its Tokio runtime.

pub mod contracts;
pub mod db;
pub mod events;
pub mod ipc;
pub mod paths;
pub mod services;

mod automation_runtime;
mod helpers;
mod hotkey_bindings;
mod ipc_handlers;
mod live_events;
mod persistence;
mod plugin_intents;
mod plugin_runtime;
#[cfg(test)]
mod tests;

pub(crate) use helpers::*;

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
use tokio::sync::Notify;
#[cfg(feature = "native-tiktok")]
use tokio::sync::Semaphore;

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
    events::{AppEvent, EventBus},
    ipc::messages::{HostMessage, PageMessage},
    paths::AppPaths,
    services::{
        builtin_action_types, builtin_node_catalog, builtin_translations,
        media_selection_from_path_with_kind, validate_audio_file_ref,
        validate_media_picker_options, AppStateService, AutomationService, AwardOptions,
        CapabilityBroker, LiveService, PointAction, PointsService,
    },
};

pub use services::{
    MediaApiError, MediaError, MediaHost, MediaHostError, MediaHostFuture, NoopMediaHost,
};

pub trait HostEmitter: Send + Sync {
    fn emit(&self, message: HostMessage);
}

pub(crate) struct PluginHealth {
    pub(crate) consecutive_failures: u32,
    pub(crate) next_retry_at: Option<Instant>,
}

/// The Rust application service graph. Each subsystem has separate ownership
/// so a future database/live/plugin implementation can be tested in isolation.
pub struct AppCore {
    pub live: Arc<LiveService>,
    pub points: Arc<PointsService>,
    pub automation: Arc<AutomationService>,
    pub capabilities: Arc<CapabilityBroker>,
    pub media: Arc<dyn MediaHost>,
    pub plugins: Arc<PluginManager>,
    pub db: Arc<db::DatabaseManager>,
    pub app_state: Arc<AppStateService>,
    pub events: EventBus,
    emitter: Arc<dyn HostEmitter>,
    last_automation_event: RwLock<Option<serde_json::Value>>,
    last_automation_event_at: RwLock<Option<u64>>,
    last_automation_context_emit_at: AtomicU64,
    automation_sequence: AtomicU64,
    /// Monotonic revision of the persisted hotkey behavior projection. The
    /// plugin poll consumes this asynchronously so UI writes never wait for
    /// a process-plugin round trip.
    hotkey_sync_revision: AtomicU64,
    hotkey_synced_revision: AtomicU64,
    /// Bounds native-live automation work. Events arriving while all slots
    /// are occupied are intentionally dropped; live delivery must remain
    /// responsive and disposable events must not create an unbounded task
    /// backlog.
    #[cfg(feature = "native-tiktok")]
    pub(crate) automation_slots: Arc<Semaphore>,
    last_leaderboard_emit_at: AtomicU64,
    #[cfg(feature = "http")]
    http_client: Option<reqwest::Client>,
    #[cfg(feature = "http")]
    http_client_error: Option<String>,
    #[cfg(feature = "native-tiktok")]
    connection_sequence: AtomicU64,
    connection_context: RwLock<Option<LiveContext>>,
    #[cfg(feature = "native-tiktok")]
    live_pump_started: AtomicBool,
    plugin_health: Mutex<BTreeMap<String, PluginHealth>>,
    plugin_poll_started: AtomicBool,
    plugin_poll_shutdown: Arc<Notify>,
    plugin_poll_task: Mutex<Option<tokio::task::JoinHandle<()>>>,
    plugin_install_lock: Mutex<()>,
    shutdown_started: AtomicBool,
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
        if let Ok(snapshot) = db.load_behavior_snapshot() {
            automation.replace_snapshot(&snapshot);
        }
        #[cfg(feature = "http")]
        let (http_client, http_client_error) = match reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
        {
            Ok(client) => (Some(client), None),
            Err(error) => {
                let message = format!(
                        "HTTP automation is disabled because the hardened client could not be created: {error}"
                    );
                tracing::error!(%error, "hardened HTTP client unavailable; refusing insecure fallback");
                (None, Some(message))
            }
        };

        let core = Self {
            live: Arc::new(live),
            points: Arc::new(PointsService::new(db.clone())),
            automation,
            capabilities,
            media,
            plugins,
            db,
            app_state: Arc::new(AppStateService::default()),
            events: EventBus::new(256),
            emitter,
            last_automation_event: RwLock::new(None),
            last_automation_event_at: RwLock::new(None),
            last_automation_context_emit_at: AtomicU64::new(0),
            automation_sequence: AtomicU64::new(0),
            hotkey_sync_revision: AtomicU64::new(1),
            hotkey_synced_revision: AtomicU64::new(0),
            #[cfg(feature = "native-tiktok")]
            automation_slots: Arc::new(Semaphore::new(32)),
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
            plugin_health: Mutex::new(BTreeMap::new()),
            plugin_poll_started: AtomicBool::new(false),
            plugin_poll_shutdown: Arc::new(Notify::new()),
            plugin_poll_task: Mutex::new(None),
            plugin_install_lock: Mutex::new(()),
            shutdown_started: AtomicBool::new(false),
        };
        #[cfg(feature = "http")]
        if let Some(message) = core.http_client_error.clone() {
            core.emit(HostMessage::AutomationError { message });
        }
        core
    }

    pub fn emit(&self, message: HostMessage) {
        self.emitter.emit(message);
    }

    /// Publishes a leaderboard snapshot at a bounded rate. Callers that need
    /// an immediate snapshot (for example an explicit UI request) still emit
    /// directly; high-rate live events use this coalescing boundary.
    pub(crate) fn emit_leaderboard_if_due(&self) {
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
        let _install_lock = self
            .plugin_install_lock
            .lock()
            .expect("plugin install lock poisoned");
        let paths = self.db.paths();
        let installer = tiktools_plugin_loader::PluginInstaller {
            plugin_directory: paths.plugins.clone(),
            staging_directory: paths.temp.join("plugin-install"),
            replace_existing,
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
        Ok(installed)
    }

    #[cfg(feature = "plugin-install")]
    pub fn uninstall_plugin(
        &self,
        id: &str,
    ) -> Result<(), tiktools_plugin_loader::PluginLoaderError> {
        let _install_lock = self
            .plugin_install_lock
            .lock()
            .expect("plugin install lock poisoned");
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
        self.plugins.scan().map(|_| ())?;
        Ok(())
    }

    pub(crate) fn next_sequence(&self) -> u64 {
        self.automation_sequence.fetch_add(1, Ordering::AcqRel) + 1
    }

    pub async fn shutdown(self: &Arc<Self>) {
        if self.shutdown_started.swap(true, Ordering::AcqRel) {
            return;
        }
        // Keep a notification queued if the polling task is in a plugin call
        // when shutdown begins; `notify_waiters` alone would be lost before
        // the task reaches its select point.
        self.plugin_poll_shutdown.notify_one();
        self.events.publish(AppEvent::Shutdown);
        self.publish_disconnected_event().await;
        self.live.disconnect().await;
        let task = self
            .plugin_poll_task
            .lock()
            .expect("plugin poll task lock poisoned")
            .take();
        if let Some(task) = task {
            let _ = task.await;
        }
        self.plugins.stop_all();
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
