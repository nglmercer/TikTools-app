use std::{
    collections::BTreeMap,
    path::Path,
    sync::{Arc, Mutex, RwLock},
    time::{Duration, Instant},
};

use tiktools_plugin_api::HostCompat;
use tiktools_plugin_installer::{
    ActivationHooks, InstallError, Installer, InstallerConfig, JobRegistry, PackageSources,
    PreparedPackage, UpdateReport, DEFAULT_RETAINED_VERSIONS,
};
use tiktools_plugin_package::{PublicKey, TrustStore};
use tiktools_plugin_registry::{
    cloud_source, Downloader, RegistryClient, RegistrySource, OFFICIAL_REGISTRY_NAME,
    STAGING_REGISTRY_NAME,
};

#[cfg(all(feature = "persistence", feature = "cloud-auth"))]
#[allow(dead_code)] // Native journal orchestration is not enabled in product.
mod mutation_admission;

use crate::{events::DomainEvent, paths::AppPaths, EventBus};

const REVIEW_TTL: Duration = Duration::from_secs(30 * 60);
const MAX_REVIEWS: usize = 16;
const CACHE_LIMIT_BYTES: u64 = 1024 * 1024 * 1024;
const OFFICIAL_REGISTRY: &str = "official";

pub(crate) struct PluginPackages {
    pub(crate) installer: Arc<Installer>,
    pub(crate) sources: Arc<PackageSources>,
    pub(crate) jobs: JobRegistry,
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
        let registry = RegistryClient::new(
            layout.registries(),
            layout.registries_config(),
            builtin_registries(),
            trust,
        )
        .map_err(|error| error.to_string())?;
        let downloader = Downloader::new(layout.cache()).map_err(|error| error.to_string())?;
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
            sources: Arc::new(PackageSources::new(Arc::new(registry), downloader)),
            jobs,
            reviews: Mutex::new(BTreeMap::new()),
            updates: Mutex::new(UpdateReport::default()),
        })
    }

    pub(crate) fn registry(&self) -> &Arc<RegistryClient> {
        self.sources.registry()
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
    let mut sources = Vec::new();
    // A static registry tree keeps working exactly as before when an explicit
    // URL is pinned; otherwise the official source discovers the active
    // snapshot generation from the Cloud origin on every refresh.
    if let Some(url) = option_env!("TIKTOOLS_OFFICIAL_REGISTRY_URL") {
        match static_source(
            OFFICIAL_REGISTRY,
            url,
            option_env!("TIKTOOLS_OFFICIAL_REGISTRY_ROOT_KEYS"),
            100,
        ) {
            Some(source) => sources.push(source),
            None => return sources,
        }
    } else if let Some(origin) = tiktools_cloud_auth::resolve_cloud_origin() {
        // Same origin the account uses (debug override, build config, or the
        // built-in default). Trust still comes only from the independently
        // pinned root keys: without them the official source stays disabled
        // and the catalog reports it as unavailable instead of trusting keys
        // served by the origin itself.
        match cloud_builtin(
            OFFICIAL_REGISTRY_NAME,
            &origin,
            option_env!("TIKTOOLS_OFFICIAL_REGISTRY_ROOT_KEYS"),
            100,
        ) {
            Some(source) => sources.push(source),
            None => return sources,
        }
    }
    if let Some(origin) = option_env!("TIKTOOLS_STAGING_CLOUD_ORIGIN") {
        // Staging uses its own source name and independently pinned keys so a
        // disposable staging trust anchor can never be mistaken for the
        // official one. Both must be configured; half a staging pair stays
        // disabled instead of running unverified.
        if let Some(source) = cloud_builtin(
            STAGING_REGISTRY_NAME,
            origin,
            option_env!("TIKTOOLS_STAGING_REGISTRY_ROOT_KEYS"),
            50,
        ) {
            sources.push(source);
        }
    }
    sources
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

fn static_source(
    name: &str,
    url: &str,
    keys: Option<&str>,
    priority: i32,
) -> Option<RegistrySource> {
    let root_keys = parse_root_keys(keys, name)?;
    match RegistrySource::new(name, url) {
        Ok(mut source) => {
            source.root_keys = root_keys;
            source.builtin = true;
            source.priority = priority;
            Some(source)
        }
        Err(error) => {
            tracing::warn!(%error, "{name} registry URL is invalid; registry disabled");
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

pub(crate) struct CoreActivation<'a>(pub(crate) &'a crate::AppCore);

impl ActivationHooks for CoreActivation<'_> {
    fn deactivate(&self, plugin_id: &str) -> Result<bool, String> {
        let running = self.0.plugins.is_running(plugin_id);
        if running {
            self.0
                .plugins
                .stop(plugin_id)
                .map_err(|error| error.to_string())?;
        }
        Ok(running)
    }

    fn activate(&self, plugin_id: &str, was_running: bool) -> Result<(), String> {
        self.0.plugins.scan().map_err(|error| error.to_string())?;
        if was_running {
            // A successful package operation must not report a runtime that
            // failed to restart as applied. The installer will restore the
            // previous package where possible and report the error.
            self.0
                .plugins
                .start(plugin_id)
                .map_err(|error| error.to_string())?;
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
            paths,
        ));
        let first = package_fixture(&root, "1.0.0");
        let second = package_fixture(&root, "2.0.0");
        let third = package_fixture(&root, "3.0.0");
        let source = || {
            rusqlite::Connection::open(core.db.paths().automation_database()).unwrap().query_row(
                "SELECT generation FROM plugin_sync_source_versions WHERE plugin_id='acme.package'", [],
                |row| row.get::<_, i64>(0),
            ).unwrap()
        };
        core.db
            .set_plugin_state("acme.package", true, true)
            .unwrap();
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
        core.install_plugin(&first, false).unwrap();
        assert_eq!(source(), 3);
        assert_eq!(
            core.plugins.get("acme.package").unwrap().manifest.version,
            "1.0.0"
        );
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
        core.uninstall_plugin("acme.package", false).unwrap();
        assert_eq!(source(), 10);
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
        let error = CoreActivation(&core)
            .activate("missing.plugin", true)
            .unwrap_err();
        assert!(!error.is_empty());
        assert!(CoreActivation(&core)
            .activate("missing.plugin", false)
            .is_ok());
        core.shutdown().await;
        let _ = std::fs::remove_dir_all(root);
    }
}
