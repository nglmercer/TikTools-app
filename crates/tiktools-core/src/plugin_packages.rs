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
use tiktools_plugin_registry::{Downloader, RegistryClient, RegistrySource};

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
    let (Some(url), Some(keys)) = (
        option_env!("TIKTOOLS_OFFICIAL_REGISTRY_URL"),
        option_env!("TIKTOOLS_OFFICIAL_REGISTRY_ROOT_KEYS"),
    ) else {
        return Vec::new();
    };
    let root_keys: Vec<PublicKey> = match serde_json::from_str(keys) {
        Ok(keys) => keys,
        Err(error) => {
            tracing::warn!(%error, "official registry root keys are invalid; registry disabled");
            return Vec::new();
        }
    };
    match RegistrySource::new(OFFICIAL_REGISTRY, url) {
        Ok(mut source) => {
            source.root_keys = root_keys;
            source.builtin = true;
            source.priority = 100;
            vec![source]
        }
        Err(error) => {
            tracing::warn!(%error, "official registry URL is invalid; registry disabled");
            Vec::new()
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
            if let Err(error) = self.0.plugins.start(plugin_id) {
                tracing::warn!(plugin = %plugin_id, %error, "plugin runtime could not be restarted after package operation");
            }
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
