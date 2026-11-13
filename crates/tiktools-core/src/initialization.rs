//! Construction of the headless application service graph.
use super::*;

impl AppCore {
    pub fn new(emitter: Arc<dyn HostEmitter>) -> Self {
        Self::with_media_host(emitter, Arc::new(NoopMediaHost))
    }

    /// Builds the core with an explicit native capability implementation.
    /// The desktop crate supplies this for file dialogs and audio output;
    /// headless callers can keep using [`AppCore::new`].
    pub fn with_media_host(emitter: Arc<dyn HostEmitter>, media: Arc<dyn MediaHost>) -> Self {
        Self::build_with_paths(
            emitter,
            media,
            AppPaths::from_environment(),
            std::env::var_os("TIKTOOLS_BUILTIN_PLUGINS_DIR").is_none(),
            #[cfg(feature = "cloud-auth")]
            None,
        )
    }

    /// Constructs an owner host with explicit application paths. Embedders and
    /// isolated network fixtures can avoid process-wide path overrides. Remote
    /// controllers use the client transport and must not construct a core.
    pub fn with_paths(
        emitter: Arc<dyn HostEmitter>,
        media: Arc<dyn MediaHost>,
        paths: AppPaths,
    ) -> Self {
        Self::build_with_paths(
            emitter,
            media,
            paths,
            false,
            #[cfg(feature = "cloud-auth")]
            None,
        )
    }

    /// Native embedders can supply an explicit Cloud account/token store (for
    /// example memory-only headless credentials). The same context observer and
    /// sync services are installed. This constructor is never a remote RPC.
    #[cfg(feature = "cloud-auth")]
    pub fn with_paths_and_cloud_auth(
        emitter: Arc<dyn HostEmitter>,
        media: Arc<dyn MediaHost>,
        paths: AppPaths,
        auth: tiktools_cloud_auth::CloudAuth,
    ) -> Self {
        Self::build_with_paths(emitter, media, paths, false, Some(auth))
    }

    fn build_with_paths(
        emitter: Arc<dyn HostEmitter>,
        media: Arc<dyn MediaHost>,
        paths: AppPaths,
        optional_default_builtin: bool,
        #[cfg(feature = "cloud-auth")] auth: Option<tiktools_cloud_auth::CloudAuth>,
    ) -> Self {
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

        let roots = runtime_plugin_roots(&paths, optional_default_builtin);
        let plugins = Arc::new(PluginManager::new(roots));
        match plugins.scan() {
            Ok(entries) => tracing::info!(count = entries.len(), "runtime plugin scan complete"),
            Err(error) => tracing::warn!(%error, "runtime plugin scan failed"),
        }

        let events = EventBus::new(256);
        #[cfg(feature = "plugin-install")]
        let packages = plugin_packages::PluginPackages::new(&paths, events.clone()).map(Arc::new);
        #[cfg(feature = "cloud-auth")]
        let cloud = Arc::new(match auth {
            Some(auth) => cloud_account::CloudAccount::with_auth(auth, &events),
            None => cloud_account::CloudAccount::new(&paths, &events),
        });
        #[cfg(feature = "cloud-auth")]
        let sync = Arc::new(cloud_sync::CloudSync::new(&paths, cloud.auth()));
        #[cfg(all(feature = "cloud-auth", feature = "http", feature = "plugin-install"))]
        let publisher = Arc::new(cloud_publisher::CloudPublisher::new(&paths));
        #[cfg(all(feature = "cloud-auth", feature = "http"))]
        let teams = Arc::new(cloud_teams::CloudTeams::new(&paths));
        #[cfg(all(feature = "cloud-auth", feature = "http"))]
        let remote_host = Arc::new(cloud_remote_host::RemoteHostLink::default());
        #[cfg(feature = "plugin-install")]
        if let Err(error) = &packages {
            tracing::warn!(%error, "plugin package management is unavailable");
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
        let plugin_activation = match db.load_behavior_runtime_approval_snapshot() {
            Ok(((snapshot, provenance), approvals)) => {
                let authorities = provenance
                    .into_iter()
                    .map(|(scope, id, p)| ((scope, id), p.into()))
                    .collect();
                automation.replace_snapshot_trusted_with_approvals(
                    &snapshot.value,
                    &snapshot.resource_versions,
                    authorities,
                    approvals,
                );
                crate::plugin_runtime::activation_from_snapshot(&snapshot.value)
            }
            Err(_) => {
                automation.quarantine();
                db.load_behavior_snapshot_versioned()
                    .map(|snapshot| {
                        crate::plugin_runtime::activation_from_snapshot(&snapshot.value)
                    })
                    .unwrap_or_default()
            }
        };
        #[cfg(not(feature = "persistence"))]
        let plugin_activation: BTreeMap<String, crate::plugin_runtime::PluginActivation> =
            BTreeMap::new();
        #[cfg(feature = "persistence")]
        let plugin_apply_quarantine = match db.plugin_apply_quarantine() {
            Ok(ids) => Some(ids),
            Err(error) => {
                tracing::warn!(%error, "plugin apply recovery is unavailable; plugin execution is blocked");
                None
            }
        };
        #[cfg(not(feature = "persistence"))]
        let plugin_apply_quarantine = Some(Default::default());
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
            collaboration: Mutex::new(None),
            collaboration_gateway_available: AtomicBool::new(false),
            collaboration_peer_available: AtomicBool::new(false),
            coordination_events: tokio::sync::broadcast::channel(1024).0,
            coordination_streams: collaboration::StreamHub::default(),
            automation_mutations: Mutex::new(()),
            automation_write_epoch: AtomicU64::new(0),
            automation_write_settled: tokio::sync::Notify::new(),
            #[cfg(feature = "persistence")]
            coordination_gate: Mutex::new(()),
            #[cfg(all(test, feature = "persistence"))]
            approved_points_coordination_wait: Mutex::new(None),
            #[cfg(feature = "persistence")]
            owner_adoption_gate: Mutex::new(()),
            #[cfg(feature = "persistence")]
            coordination_worker: control::CoordinationWorker::default(),
            #[cfg(feature = "app-update")]
            app_updater: RwLock::new(None),
            live: Arc::new(live),
            live_monitor: live_monitor::LiveMonitor::default(),
            points,
            #[cfg(feature = "native-tiktok")]
            live_points_work,
            automation,
            #[cfg(feature = "persistence")]
            effects: services::effects::EffectJournal::open(db.clone())
                .map_err(|error| error.to_string()),
            capabilities,
            media,
            plugins,
            #[cfg(feature = "persistence")]
            analytics_writer: db::AnalyticsWriter::new(Arc::clone(&db)),
            db: db.clone(),
            app_state: Arc::new(AppStateService::default()),
            events,
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
            #[cfg(feature = "native-tiktok")]
            live_totals: crate::live_events::totals::LiveTotals::default(),
            #[cfg(feature = "persistence")]
            aggregations: services::aggregations::AggregationService::restore(&db),
            #[cfg(not(feature = "persistence"))]
            aggregations: services::aggregations::AggregationService::default(),
            #[cfg(feature = "persistence")]
            competitions: services::competitions::CompetitionService::restore(&db),
            #[cfg(not(feature = "persistence"))]
            competitions: services::competitions::CompetitionService::default(),
            option_sources: OptionSourceService::new(),
            plugin_state: PluginRuntimeState {
                activation: RwLock::new(plugin_activation),
                apply_quarantine: RwLock::new(plugin_apply_quarantine),
                ..Default::default()
            },
            processor_state: ProcessorRuntimeState::default(),
            plugin_last_events: Mutex::new(BTreeMap::new()),
            plugin_event_drops: Mutex::new(BTreeMap::new()),
            plugin_delivery: Mutex::new(BTreeMap::new()),
            hotkey_sync_state: Mutex::new(crate::plugin_diagnostics::HotkeySyncState::default()),
            shutdown_started: AtomicBool::new(false),
            shutdown_finished: AtomicBool::new(false),
            shutdown_update_safe: AtomicBool::new(true),
            shutdown_complete: tokio::sync::Notify::new(),
            transport_state: TransportHealthState::default(),
            #[cfg(feature = "plugin-install")]
            packages,
            #[cfg(feature = "cloud-auth")]
            cloud,
            #[cfg(feature = "cloud-auth")]
            sync,
            #[cfg(all(feature = "cloud-auth", feature = "http", feature = "plugin-install"))]
            publisher,
            #[cfg(all(feature = "cloud-auth", feature = "http"))]
            teams,
            #[cfg(all(feature = "cloud-auth", feature = "http"))]
            remote_host,
        };
        #[cfg(feature = "http")]
        if let Some(message) = core.http_client_error.clone() {
            core.emit(HostMessage::AutomationError { message });
        }
        core.rebuild_processor_index();
        core
    }
}

/// Releases ship external plugins. An absent automatic resource directory is
/// therefore not a configured inventory root. Explicit roots, unreadable roots
/// and dangling symlinks remain mandatory and make a partial scan fail closed.
fn runtime_plugin_roots(
    paths: &AppPaths,
    optional_default_builtin: bool,
) -> Vec<tiktools_plugin_loader::PluginRoot> {
    let mut roots = plugin_roots(
        paths.builtin_plugins.clone(),
        paths.plugins.clone(),
        paths.development_plugins.clone(),
    );
    if optional_default_builtin
        && std::fs::symlink_metadata(&paths.builtin_plugins)
            .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)
    {
        roots.retain(|root| root.source != tiktools_plugin_loader::PluginSource::Builtin);
    }
    roots
}

#[cfg(test)]
mod inventory_root_tests {
    use super::*;
    #[test]
    fn only_the_absent_automatic_builtin_root_is_optional() {
        let root = std::env::temp_dir().join(format!(
            "tiktools-roots-{}-{}",
            std::process::id(),
            fastrand::u64(..)
        ));
        let paths = AppPaths {
            data: root.join("data"),
            plugins: root.join("plugins"),
            plugin_data: root.join("plugin-data"),
            builtin_plugins: root.join("builtins"),
            development_plugins: Some(root.join("development")),
            logs: root.join("logs"),
            temp: root.join("temp"),
            root: root.clone(),
        };
        assert_eq!(runtime_plugin_roots(&paths, false).len(), 3);
        let automatic = runtime_plugin_roots(&paths, true);
        assert_eq!(automatic.len(), 2);
        assert!(automatic.iter().any(|entry| entry.path == paths.plugins));
        assert!(automatic
            .iter()
            .any(|entry| Some(&entry.path) == paths.development_plugins.as_ref()));
        std::fs::create_dir_all(&paths.builtin_plugins).unwrap();
        assert_eq!(runtime_plugin_roots(&paths, true).len(), 3);
        std::fs::remove_dir_all(root).unwrap();
    }
}
