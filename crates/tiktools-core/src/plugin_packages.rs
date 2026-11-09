use std::{
    collections::BTreeMap,
    path::Path,
    sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard},
    time::{Duration, Instant},
};

use tiktools_plugin_api::HostCompat;
use tiktools_plugin_installer::{
    ActivationHooks, InstallError, InstallErrorCode, Installer, InstallerConfig, JobRegistry,
    PackageSources, PreparedPackage, UpdateReport, DEFAULT_RETAINED_VERSIONS,
};
use tiktools_plugin_package::{PublicKey, TrustStore};
use tiktools_plugin_registry::{
    cloud_source, Downloader, RegistryClient, RegistrySource, OFFICIAL_REGISTRY_NAME,
};

#[cfg(all(feature = "persistence", feature = "cloud-auth"))]
#[allow(dead_code)] // Native journal orchestration is not enabled in product.
mod mutation_admission;

use crate::{events::DomainEvent, paths::AppPaths, EventBus};

const REVIEW_TTL: Duration = Duration::from_secs(30 * 60);
const MAX_REVIEWS: usize = 16;
const CACHE_LIMIT_BYTES: u64 = 1024 * 1024 * 1024;

pub(crate) struct PluginPackages {
    pub(crate) installer: Arc<Installer>,
    pub(crate) sources: Arc<PackageSources>,
    pub(crate) jobs: JobRegistry,
    pub(crate) updates_gate: tokio::sync::Mutex<()>,
    sideload_gate: RwLock<()>,
    reviews: Mutex<BTreeMap<String, Review>>,
    updates: Mutex<UpdateReport>,
}

struct Review {
    prepared: PreparedPackage,
    spec: String,
    created: Instant,
}

impl PluginPackages {
    pub(crate) fn new(paths: &AppPaths, events: EventBus) -> Result<Self, String> {
        let store = paths.plugin_store();
        let trust = Arc::new(RwLock::new(TrustStore::new()));
        let installer = Installer::new(
            InstallerConfig {
                plugins_dir: paths.plugins.clone(),
                store_dir: store.clone(),
                host: host_compat(),
                retain_versions: DEFAULT_RETAINED_VERSIONS,
                provider_endpoints: None,
            },
            trust.clone(),
        );
        let layout = installer.layout().clone();
        installer.cleanup_staging();
        let registry = RegistryClient::new_fixed(
            layout.registries(),
            layout.registries_config(),
            builtin_registries(),
            trust,
        )
        .map_err(|error| error.to_string())?;
        let downloader =
            Downloader::new_fixed(layout.cache()).map_err(|error| error.to_string())?;
        if let Err(error) = downloader.prune(CACHE_LIMIT_BYTES) {
            tracing::warn!(%error, "plugin download cache could not be pruned");
        }
        let jobs = JobRegistry::new(Some(Arc::new(move |snapshot| {
            if let Ok(job) = serde_json::to_value(snapshot) {
                events.publish_domain(DomainEvent::PluginJob { job });
            }
        })));
        Ok(Self {
            installer: Arc::new(installer),
            sources: Arc::new(PackageSources::new_fixed(Arc::new(registry), downloader)),
            jobs,
            updates_gate: tokio::sync::Mutex::new(()),
            sideload_gate: RwLock::new(()),
            reviews: Mutex::new(BTreeMap::new()),
            updates: Mutex::new(UpdateReport::default()),
        })
    }

    pub(crate) fn registry(&self) -> &Arc<RegistryClient> {
        self.sources.registry()
    }

    /// Serializes local package activation against product settings changes.
    /// The lease is process-local; independent AppCore instances do not share it.
    pub(crate) fn try_sideload_permit(&self) -> Result<RwLockReadGuard<'_, ()>, InstallError> {
        let guard = self.sideload_gate.try_read().map_err(|_| {
            InstallError::new(
                InstallErrorCode::Busy,
                "plugin installation settings are changing",
            )
        })?;
        let defaults = self.registry().try_defaults().map_err(InstallError::io)?;
        if !defaults.allow_sideload {
            return Err(InstallError::new(
                InstallErrorCode::Unsupported,
                "installing plugins from local files is disabled in the plugin settings",
            ));
        }
        Ok(guard)
    }

    pub(crate) fn try_sideload_settings(&self) -> Result<RwLockWriteGuard<'_, ()>, InstallError> {
        self.sideload_gate.try_write().map_err(|_| {
            InstallError::new(
                InstallErrorCode::Busy,
                "a local plugin installation or rollback is in progress",
            )
        })
    }

    pub(crate) fn remember_review(&self, token: &str, spec: &str, prepared: PreparedPackage) {
        let mut reviews = self
            .reviews
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        reviews.retain(|_, review| review.created.elapsed() < REVIEW_TTL);
        while reviews.len() >= MAX_REVIEWS {
            let Some(oldest) = reviews
                .iter()
                .min_by_key(|(_, review)| review.created)
                .map(|(token, _)| token.clone())
            else {
                break;
            };
            reviews.remove(&oldest);
        }
        reviews.insert(
            token.to_owned(),
            Review {
                prepared,
                spec: spec.to_owned(),
                created: Instant::now(),
            },
        );
    }

    pub(crate) fn take_review(&self, token: &str, spec: &str) -> Option<PreparedPackage> {
        let mut reviews = self
            .reviews
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let review = reviews.remove(token)?;
        (review.spec == spec && review.created.elapsed() < REVIEW_TTL).then_some(review.prepared)
    }

    pub(crate) fn update_report(&self) -> UpdateReport {
        self.updates
            .lock()
            .map(|report| report.clone())
            .unwrap_or_default()
    }

    pub(crate) fn store_update_report(&self, report: UpdateReport) {
        if let Ok(mut current) = self.updates.lock() {
            *current = report;
        }
    }
}

pub(crate) fn host_compat() -> HostCompat {
    HostCompat::new(
        semver::Version::parse(env!("CARGO_PKG_VERSION")).unwrap_or(semver::Version::new(0, 0, 0)),
    )
}

fn builtin_registries() -> Vec<RegistrySource> {
    // The account, catalog, and signed registry use the same provisioned
    // origin. Trust anchors remain independently provisioned at build time.
    tiktools_cloud_auth::resolve_cloud_origin()
        .and_then(|origin| {
            cloud_builtin(
                OFFICIAL_REGISTRY_NAME,
                &origin,
                option_env!("TIKTOOLS_OFFICIAL_REGISTRY_ROOT_KEYS"),
                100,
            )
        })
        .into_iter()
        .collect()
}

fn parse_root_keys(keys: Option<&str>, what: &str) -> Option<Vec<PublicKey>> {
    let keys = keys?;
    match serde_json::from_str(keys) {
        Ok(keys) => Some(keys),
        Err(error) => {
            tracing::warn!(%error, "{what} root keys are invalid; registry disabled");
            None
        }
    }
}

fn cloud_builtin(
    name: &str,
    origin: &str,
    keys: Option<&str>,
    priority: i32,
) -> Option<RegistrySource> {
    let root_keys = parse_root_keys(keys, name)?;
    match cloud_source(name, origin, root_keys) {
        Ok(mut source) => {
            source.priority = priority;
            if source.root_keys.is_empty() {
                tracing::warn!("{name} cloud registry has no pinned root keys; registry disabled");
                return None;
            }
            Some(source)
        }
        Err(error) => {
            tracing::warn!(%error, "{name} cloud origin is invalid; registry disabled");
            None
        }
    }
}

pub(crate) struct CoreActivation<'a> {
    core: &'a crate::AppCore,
    lease: std::sync::Mutex<Option<(String, tiktools_plugin_loader::PluginPackageLease)>>,
}

impl<'a> CoreActivation<'a> {
    pub(crate) fn new(core: &'a crate::AppCore) -> Self {
        Self {
            core,
            lease: std::sync::Mutex::new(None),
        }
    }

    /// Release only after the complete package operation and its durable fences.
    pub(crate) fn finish(&self) -> Result<(), InstallError> {
        let lease = self
            .lease
            .lock()
            .map_err(|_| unavailable("package lease is poisoned"))?
            .take();
        if let Some((_, lease)) = lease {
            lease.finish().map_err(InstallError::io)?;
        }
        Ok(())
    }
}

impl ActivationHooks for CoreActivation<'_> {
    fn deactivate(&self, plugin_id: &str) -> Result<bool, String> {
        let mut held = self
            .lease
            .lock()
            .map_err(|_| "package lease is poisoned".to_owned())?;
        if held.is_some() {
            return Err("package lease already acquired".into());
        }
        let lease = self
            .core
            .plugins
            .begin_package_mutation(plugin_id)
            .map_err(|error| error.to_string())?;
        let result = lease.stop_if_present().map_err(|error| error.to_string());
        *held = Some((plugin_id.to_owned(), lease));
        result
    }

    fn activate(&self, plugin_id: &str, was_running: bool) -> Result<(), String> {
        self.core
            .plugins
            .ensure_startup_known(plugin_id)
            .map_err(|error| error.to_string())?;
        self.core
            .plugins
            .scan()
            .map_err(|error| error.to_string())?;
        self.core
            .plugins
            .ensure_startup_known(plugin_id)
            .map_err(|error| error.to_string())?;
        if was_running {
            if self.core.plugin_apply_quarantined(plugin_id) {
                return Err("Plugin application requires recovery before restart.".to_owned());
            }
            // A successful package operation must not report a runtime that
            // failed to restart as applied. The installer will restore the
            // previous package where possible and report the error.
            let held = self
                .lease
                .lock()
                .map_err(|_| "package lease is poisoned".to_owned())?;
            let (id, lease) = held
                .as_ref()
                .ok_or_else(|| "package lease is missing".to_owned())?;
            if id != plugin_id {
                return Err("package lease identity mismatch".into());
            }
            lease.start().map_err(|error| error.to_string())?;
        }
        Ok(())
    }
}

pub(crate) fn plugin_data_dir(paths: &AppPaths, plugin_id: &str) -> Option<std::path::PathBuf> {
    let directory = paths.plugin_data.join(plugin_id);
    (directory.parent() == Some(Path::new(&paths.plugin_data))).then_some(directory)
}

pub(crate) fn unavailable(error: &str) -> InstallError {
    InstallError::new(
        tiktools_plugin_installer::InstallErrorCode::Unsupported,
        format!("plugin package management is unavailable: {error}"),
    )
}

#[cfg(all(feature = "persistence", feature = "cloud-auth"))]
pub(crate) fn try_product_mutation_lease(
    core: &crate::AppCore,
) -> Result<tiktools_cloud_sync::ProductEmissionLease, InstallError> {
    use tiktools_cloud_sync::ProtocolFenceError;
    let fence = tiktools_cloud_sync::ProductEmissionFence::open(
        core.db.paths().data.join("plugin-sync-emission"),
    )
    .map_err(InstallError::io)?;
    fence.try_untracked_edit().map_err(|error| match error {
        ProtocolFenceError::Busy => InstallError::new(
            InstallErrorCode::Busy,
            "plugin sync emission is in progress",
        ),
        other => InstallError::io(other),
    })
}

/// Called while holding the invocation and product emission leases. The
/// dirty marker is durable before package I/O and carries no Cloud identity.
#[cfg(all(feature = "persistence", feature = "cloud-auth"))]
pub(crate) fn reserve_package_sync_source(
    core: &crate::AppCore,
    id: &str,
) -> Result<i64, InstallError> {
    let Ok(sync) = core.sync.coordinator() else {
        return core
            .db
            .invalidate_plugin_package_source_untracked(id)
            .map_err(InstallError::io);
    };
    let attempted = std::cell::Cell::new(false);
    match sync.with_package_sync_marker(|mark_untracked| {
        attempted.set(true);
        if mark_untracked {
            core.db.invalidate_plugin_package_source_untracked(id)
        } else {
            core.db.invalidate_plugin_package_source(id)
        }
    }) {
        Ok(result) => result.map_err(InstallError::io),
        Err(_) if !attempted.get() => core
            .db
            .invalidate_plugin_package_source_untracked(id)
            .map_err(InstallError::io),
        Err(error) => Err(InstallError::io(error)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ipc::messages::HostMessage, AppCore, HostEmitter, NoopMediaHost};

    struct NoopEmitter;
    impl HostEmitter for NoopEmitter {
        fn emit(&self, _: HostMessage) {}
    }

    #[cfg(feature = "persistence")]
    fn package_fixture(root: &std::path::Path, version: &str) -> std::path::PathBuf {
        use tiktools_plugin_package::{pack, PackRequest};
        let source = root.join(format!("source-{version}"));
        std::fs::create_dir_all(&source).unwrap();
        let manifest = source.join("plugin.json");
        let entry = source.join("demo-host");
        std::fs::write(&entry, format!("fixture binary {version}")).unwrap();
        std::fs::write(
            &manifest,
            serde_json::to_vec(&serde_json::json!({
                "schemaVersion": 3,
                "id": "acme.package",
                "name": "Demo",
                "version": version,
                "runtime": "process",
                "entry": "demo-host"
            }))
            .unwrap(),
        )
        .unwrap();
        let output = root.join(format!("package-{version}.plugin"));
        pack(&PackRequest {
            manifest,
            entry: Some(entry),
            output: output.clone(),
            target: None,
        })
        .unwrap();
        output
    }

    #[cfg(feature = "persistence")]
    #[tokio::test]
    async fn real_package_replacement_fences_racing_runtime_start() {
        use tiktools_plugin_api::{PluginManifest, PluginRuntimeKind};
        use tiktools_plugin_installer::{
            CommitOptions, Consent, NoopReporter, PackageOrigin, SignaturePolicy,
        };
        use tiktools_plugin_loader::{
            PluginInstance, PluginLoaderError, PluginRoot, PluginRuntime, PluginSource,
            RuntimeRegistry,
        };
        struct Runtime;
        struct Instance;
        impl PluginRuntime for Runtime {
            fn kind(&self) -> PluginRuntimeKind {
                PluginRuntimeKind::Process
            }
            fn load(
                &self,
                _: &PluginManifest,
                _: &std::path::Path,
            ) -> Result<Box<dyn PluginInstance>, PluginLoaderError> {
                Ok(Box::new(Instance))
            }
        }
        impl PluginInstance for Instance {
            fn id(&self) -> &str {
                "acme.package"
            }
            fn handle_message(&mut self, _: &[u8]) -> Result<Vec<u8>, PluginLoaderError> {
                Ok(b"null".to_vec())
            }
            fn shutdown(&mut self) -> Result<(), PluginLoaderError> {
                Ok(())
            }
        }
        struct Pausing<'a> {
            inner: CoreActivation<'a>,
            entered: std::sync::mpsc::Sender<()>,
            resume: std::sync::Mutex<std::sync::mpsc::Receiver<()>>,
        }
        impl ActivationHooks for Pausing<'_> {
            fn deactivate(&self, id: &str) -> Result<bool, String> {
                let running = self.inner.deactivate(id)?;
                self.entered.send(()).unwrap();
                self.resume.lock().unwrap().recv().unwrap();
                Ok(running)
            }
            fn activate(&self, id: &str, running: bool) -> Result<(), String> {
                self.inner.activate(id, running)
            }
        }
        let root = std::env::temp_dir().join(format!(
            "plugin-package-race-{}-{}",
            std::process::id(),
            fastrand::u64(..)
        ));
        let paths = AppPaths {
            data: root.join("data"),
            plugins: root.join("plugins"),
            plugin_data: root.join("plugin-data"),
            builtin_plugins: root.join("absent"),
            development_plugins: None,
            logs: root.join("logs"),
            temp: root.join("temp"),
            root: root.clone(),
        };
        paths.ensure_directories().unwrap();
        let mut core = AppCore::with_paths(
            Arc::new(NoopEmitter),
            Arc::new(NoopMediaHost),
            paths.clone(),
        );
        core.install_plugin(package_fixture(&root, "1.0.0"), false)
            .unwrap();
        let mut runtimes = RuntimeRegistry::default();
        runtimes.register(Arc::new(Runtime));
        let manager = Arc::new(tiktools_plugin_loader::PluginManager::with_runtimes(
            vec![PluginRoot {
                path: paths.plugins.clone(),
                source: PluginSource::User,
            }],
            runtimes,
        ));
        manager.scan().unwrap();
        manager.start("acme.package").unwrap();
        core.plugins = manager.clone();
        let core = Arc::new(core);
        let installer = core.plugin_packages().unwrap().installer.clone();
        let prepared = installer
            .prepare(
                &package_fixture(&root, "2.0.0"),
                PackageOrigin::File,
                SignaturePolicy::AllowWithConsent,
            )
            .unwrap();
        let (entered_tx, entered_rx) = std::sync::mpsc::channel();
        let (resume_tx, resume_rx) = std::sync::mpsc::channel();
        let worker_core = Arc::clone(&core);
        let worker = std::thread::spawn(move || {
            let hooks = Pausing {
                inner: CoreActivation::new(&worker_core),
                entered: entered_tx,
                resume: std::sync::Mutex::new(resume_rx),
            };
            let result = installer.commit(
                &prepared,
                &CommitOptions {
                    consent: Consent::Implicit {
                        replace_existing: true,
                    },
                    policy: None,
                },
                &hooks,
                &NoopReporter,
            );
            if result.is_ok() {
                hooks.inner.finish().unwrap();
            }
            result
        });
        entered_rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        assert!(manager.start("acme.package").is_err());
        assert_eq!(
            manager.get("acme.package").unwrap().manifest.version,
            "1.0.0"
        );
        resume_tx.send(()).unwrap();
        worker.join().unwrap().unwrap();
        assert_eq!(
            manager.get("acme.package").unwrap().manifest.version,
            "2.0.0"
        );
        assert!(manager.is_running("acme.package"));
        assert!(manager.start("acme.package").is_ok());
        core.shutdown().await;
        let _ = std::fs::remove_dir_all(root);
    }

    #[cfg(feature = "persistence")]
    #[tokio::test]
    async fn uncertain_startup_refuses_real_reinstall_and_uninstall() {
        use tiktools_plugin_api::{PluginManifest, PluginRuntimeKind};
        use tiktools_plugin_loader::{
            ManagedWorker, PluginInstance, PluginLoaderError, PluginRoot, PluginRuntime,
            PluginSource, RuntimeRegistry, WorkerContext,
        };
        struct UnknownRuntime;
        impl PluginRuntime for UnknownRuntime {
            fn kind(&self) -> PluginRuntimeKind {
                PluginRuntimeKind::Process
            }
            fn load(
                &self,
                _: &PluginManifest,
                _: &std::path::Path,
            ) -> Result<Box<dyn PluginInstance>, PluginLoaderError> {
                unreachable!()
            }
            fn spawn_worker(
                &self,
                _: &PluginManifest,
                _: &std::path::Path,
                _: &WorkerContext,
            ) -> Option<Result<ManagedWorker, PluginLoaderError>> {
                Some(Err(PluginLoaderError::StartupUncertain("fixture".into())))
            }
        }
        let root = std::env::temp_dir().join(format!(
            "plugin-unknown-package-{}-{}",
            std::process::id(),
            fastrand::u64(..)
        ));
        let paths = AppPaths {
            data: root.join("data"),
            plugins: root.join("plugins"),
            plugin_data: root.join("plugin-data"),
            builtin_plugins: root.join("absent"),
            development_plugins: None,
            logs: root.join("logs"),
            temp: root.join("temp"),
            root: root.clone(),
        };
        paths.ensure_directories().unwrap();
        let mut core = AppCore::with_paths(
            Arc::new(NoopEmitter),
            Arc::new(NoopMediaHost),
            paths.clone(),
        );
        let first = package_fixture(&root, "1.0.0");
        let second = package_fixture(&root, "2.0.0");
        core.install_plugin(&first, false).unwrap();
        let manifest = paths.plugins.join("acme.package/plugin.json");
        let before = std::fs::read(&manifest).unwrap();
        let mut runtimes = RuntimeRegistry::default();
        runtimes.register(Arc::new(UnknownRuntime));
        let manager = tiktools_plugin_loader::PluginManager::with_runtimes(
            vec![PluginRoot {
                path: paths.plugins.clone(),
                source: PluginSource::User,
            }],
            runtimes,
        );
        manager.scan().unwrap();
        core.plugins = Arc::new(manager);
        assert!(matches!(
            core.plugins.start("acme.package"),
            Err(PluginLoaderError::StartupUncertain(_))
        ));
        assert!(CoreActivation::new(&core)
            .activate("acme.package", false)
            .is_err());
        assert!(core.install_plugin(&second, true).is_err());
        assert_eq!(std::fs::read(&manifest).unwrap(), before);
        assert!(core.uninstall_plugin("acme.package", false).is_err());
        assert_eq!(std::fs::read(&manifest).unwrap(), before);
        let core = Arc::new(core);
        core.shutdown().await;
        let _ = std::fs::remove_dir_all(root);
    }

    #[cfg(feature = "persistence")]
    #[tokio::test]
    async fn plugin_source_real_package_mutations_fence_files_and_uninstall() {
        let root = std::env::temp_dir().join(format!(
            "plugin-package-source-{}-{}",
            std::process::id(),
            fastrand::u64(..)
        ));
        let paths = AppPaths {
            data: root.join("data"),
            plugins: root.join("plugins"),
            plugin_data: root.join("plugin-data"),
            builtin_plugins: root.join("absent"),
            development_plugins: None,
            logs: root.join("logs"),
            temp: root.join("temp"),
            root: root.clone(),
        };
        paths.ensure_directories().unwrap();
        let core = Arc::new(AppCore::with_paths(
            Arc::new(NoopEmitter),
            Arc::new(NoopMediaHost),
            paths.clone(),
        ));
        #[cfg(feature = "cloud-auth")]
        {
            core.sync.coordinator().unwrap().set_enabled(false).unwrap();
            assert!(!core
                .sync
                .coordinator()
                .unwrap()
                .with_package_sync_marker(|mark| mark)
                .unwrap());
            let marker = paths.data.join("cloud-sync/protocol-other-account.json");
            std::fs::write(&marker, "{}").unwrap();
            assert!(core
                .sync
                .coordinator()
                .unwrap()
                .with_package_sync_marker(|mark| mark)
                .unwrap());
            std::fs::remove_file(marker).unwrap();
            std::fs::write(paths.data.join("cloud-sync/cloud-sync.json"), serde_json::to_vec(&serde_json::json!({"version":2,"enabled":true,"generation":42,"scope":vec![1u8; 32]})).unwrap()).unwrap();
            core.sync.coordinator().unwrap().set_enabled(false).unwrap();
            assert!(core
                .sync
                .coordinator()
                .unwrap()
                .with_package_sync_marker(|mark| mark)
                .unwrap());
        }
        let first = package_fixture(&root, "1.0.0");
        let second = package_fixture(&root, "2.0.0");
        let third = package_fixture(&root, "3.0.0");
        let database_path = core.db.paths().automation_database();
        let source = || {
            rusqlite::Connection::open(&database_path).unwrap().query_row(
                "SELECT generation FROM plugin_sync_source_versions WHERE plugin_id='acme.package'", [],
                |row| row.get::<_, i64>(0),
            ).unwrap()
        };
        core.db
            .set_plugin_state("acme.package", true, true)
            .unwrap();
        #[cfg(feature = "cloud-auth")]
        {
            let fence = tiktools_cloud_sync::ProductEmissionFence::open(
                core.db.paths().data.join("plugin-sync-emission"),
            )
            .unwrap();
            let in_flight = fence.try_emission().unwrap();
            let error = core.install_plugin(&first, false).unwrap_err();
            assert_eq!(error.code, InstallErrorCode::Busy);
            assert_eq!(source(), 1);
            assert!(!core.db.paths().plugins.join("acme.package").exists());
            drop(in_flight);
        }
        let db = rusqlite::Connection::open(core.db.paths().automation_database()).unwrap();
        db.execute_batch("CREATE TRIGGER fail_package_fence BEFORE UPDATE ON plugin_sync_source_versions BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
        drop(db);
        assert!(core.install_plugin(&first, false).is_err());
        assert_eq!(source(), 1);
        assert!(!core.db.paths().plugins.join("acme.package").exists());
        let db = rusqlite::Connection::open(core.db.paths().automation_database()).unwrap();
        db.execute_batch("DROP TRIGGER fail_package_fence;")
            .unwrap();
        drop(db);
        #[cfg(feature = "cloud-auth")]
        {
            let db = rusqlite::Connection::open(&database_path).unwrap();
            db.execute_batch("CREATE TRIGGER fail_package_marker BEFORE INSERT ON plugin_sync_untracked_local BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
            drop(db);
            assert!(core.install_plugin(&first, false).is_err());
            assert_eq!(source(), 1);
            assert!(!core.db.paths().plugins.join("acme.package").exists());
            let db = rusqlite::Connection::open(&database_path).unwrap();
            db.execute_batch("DROP TRIGGER fail_package_marker")
                .unwrap();
        }
        core.install_plugin(&first, false).unwrap();
        assert_eq!(source(), 3);
        #[cfg(feature = "cloud-auth")]
        {
            let connection = rusqlite::Connection::open(&database_path).unwrap();
            let marker: i64 = connection.query_row("SELECT generation FROM plugin_sync_untracked_local WHERE plugin_id='acme.package'", [], |row| row.get(0)).unwrap();
            assert_eq!(marker, source());
        }
        assert_eq!(
            core.plugins.get("acme.package").unwrap().manifest.version,
            "1.0.0"
        );
        #[cfg(feature = "cloud-auth")]
        {
            let fence = tiktools_cloud_sync::ProductEmissionFence::open(
                core.db.paths().data.join("plugin-sync-emission"),
            )
            .unwrap();
            let in_flight = fence.try_emission().unwrap();
            assert_eq!(
                core.install_plugin(&second, true).unwrap_err().code,
                InstallErrorCode::Busy
            );
            assert_eq!(
                core.uninstall_plugin("acme.package", false)
                    .unwrap_err()
                    .code,
                InstallErrorCode::Busy
            );
            assert_eq!(source(), 3);
            assert_eq!(
                core.plugins.get("acme.package").unwrap().manifest.version,
                "1.0.0"
            );
            drop(in_flight);
        }
        // A rejected replacement retains old files but still fences reviews.
        assert!(core.install_plugin(&second, false).is_err());
        assert_eq!(source(), 5);
        assert_eq!(
            core.plugins.get("acme.package").unwrap().manifest.version,
            "1.0.0"
        );
        core.install_plugin(&second, true).unwrap();
        assert_eq!(source(), 7);
        assert_eq!(
            core.plugins.get("acme.package").unwrap().manifest.version,
            "2.0.0"
        );
        // The filesystem can commit before the final fence fails. Surface
        // this as an uncertain operation instead of reporting success.
        let db = rusqlite::Connection::open(core.db.paths().automation_database()).unwrap();
        db.execute_batch("CREATE TRIGGER fail_final_fence BEFORE UPDATE ON plugin_sync_source_versions WHEN NEW.generation=9 BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
        drop(db);
        let error = core.install_plugin(&third, true).unwrap_err();
        assert!(error.message.contains("final source fence failed"));
        assert_eq!(source(), 8);
        assert_eq!(
            core.plugins.get("acme.package").unwrap().manifest.version,
            "3.0.0"
        );
        let db = rusqlite::Connection::open(core.db.paths().automation_database()).unwrap();
        db.execute_batch("DROP TRIGGER fail_final_fence;").unwrap();
        drop(db);
        assert!(core.uninstall_plugin("acme.package", false).is_err());
        core.shutdown().await;
        drop(core);
        let core = Arc::new(AppCore::with_paths(
            Arc::new(NoopEmitter),
            Arc::new(NoopMediaHost),
            paths,
        ));
        core.uninstall_plugin("acme.package", false).unwrap();
        assert_eq!(source(), 12);
        #[cfg(feature = "cloud-auth")]
        {
            let connection = rusqlite::Connection::open(&database_path).unwrap();
            let marker: i64 = connection.query_row("SELECT generation FROM plugin_sync_untracked_local WHERE plugin_id='acme.package'", [], |row| row.get(0)).unwrap();
            assert_eq!(marker, source());
        }
        assert!(!core.db.paths().plugins.join("acme.package").exists());
        let db = rusqlite::Connection::open(core.db.paths().automation_database()).unwrap();
        let count: i64 = db
            .query_row(
                "SELECT COUNT(*) FROM behavior_plugins WHERE id='acme.package'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
        drop(db);
        core.shutdown().await;
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn activation_reports_failed_restart_to_installer() {
        let root = std::env::temp_dir().join(format!(
            "plugin-activation-failed-{}-{}",
            std::process::id(),
            fastrand::u64(..)
        ));
        let paths = AppPaths {
            data: root.join("data"),
            plugins: root.join("plugins"),
            plugin_data: root.join("plugin-data"),
            builtin_plugins: root.join("absent"),
            development_plugins: None,
            logs: root.join("logs"),
            temp: root.join("temp"),
            root: root.clone(),
        };
        paths.ensure_directories().unwrap();
        let core = Arc::new(AppCore::with_paths(
            Arc::new(NoopEmitter),
            Arc::new(NoopMediaHost),
            paths,
        ));
        // A successful scan cannot make a missing running plugin start. The
        // installer must see that failure and retain its rollback path.
        let error = CoreActivation::new(&core)
            .activate("missing.plugin", true)
            .unwrap_err();
        assert!(!error.is_empty());
        *core.plugin_state.apply_quarantine.write().unwrap() = None;
        let error = CoreActivation::new(&core)
            .activate("missing.plugin", true)
            .unwrap_err();
        assert!(error.contains("requires recovery"));
        assert!(CoreActivation::new(&core)
            .activate("missing.plugin", false)
            .is_ok());
        core.shutdown().await;
        let _ = std::fs::remove_dir_all(root);
    }
}
