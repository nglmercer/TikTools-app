use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
    time::Duration,
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tiktools_plugin_installer::{
    update_for_document, CommitOptions, Consent, InstallError, InstallErrorCode, InstallOutcome,
    InstallReceipt, InstalledVersions, JobHandle, JobKind, JobReporter, JobSnapshot, JobStage,
    PackageOrigin, PackagePreview, PackageSpec, PackageSpecInput, PreparedPackage, UpdateInfo,
    UpdatePolicy, UpdateReport,
};
use tiktools_plugin_registry::{
    Advisory, AutoUpdate, LiveReleaseStatus, PackageDocument, RegistryDefaults, RegistrySource,
    RegistryStatus, SearchQuery, SearchResults, OFFICIAL_REGISTRY_NAME,
};

use super::OperationError;
use crate::{
    events::DomainEvent,
    plugin_packages::{CoreActivation, PluginPackages},
    AppCore,
};

#[cfg(test)]
#[path = "sideload_policy_tests.rs"]
mod sideload_policy_tests;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PolicyPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_update: Option<AutoUpdate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AvailableRelease {
    pub version: String,
    pub channel: String,
    pub published_at: u64,
    pub yanked: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub advisory: Option<Advisory>,
    pub compatible: bool,
    #[serde(default)]
    pub live_status: LiveReleaseStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub changelog: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginVersions {
    #[serde(flatten)]
    pub installed: InstalledVersions,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry: Option<String>,
    pub available: Vec<AvailableRelease>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegistryRootKey {
    pub key_id: String,
    pub fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegistryRootPreview {
    pub url: String,
    pub keys: Vec<RegistryRootKey>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegistryAddInput {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub fingerprints: Vec<String>,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default)]
    pub priority: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegistryPackageView {
    pub registry: String,
    pub document: PackageDocument,
    #[serde(default)]
    pub live_statuses: BTreeMap<String, LiveReleaseStatus>,
}

enum InstallConsent {
    Reviewed(String),
    Automatic,
}

fn product_package_spec(input: PackageSpecInput) -> Result<PackageSpec, OperationError> {
    let spec = PackageSpec::try_from(input).map_err(OperationError::install)?;
    match &spec {
        PackageSpec::Registry {
            registry: Some(name),
            ..
        } if name != OFFICIAL_REGISTRY_NAME => Err(OperationError::invalid(
            "plugin sources are managed by the application",
        )),
        PackageSpec::Url(_) => Err(OperationError::invalid(
            "install plugins from the configured Cloud instance or a local package",
        )),
        _ => Ok(spec),
    }
}

impl AppCore {
    fn packages(&self) -> Result<Arc<PluginPackages>, OperationError> {
        self.plugin_packages()
            .cloned()
            .map_err(OperationError::install)
    }

    pub async fn plugin_package_inspect(
        self: &Arc<Self>,
        input: PackageSpecInput,
    ) -> Result<PackagePreview, OperationError> {
        let packages = self.packages()?;
        let spec = product_package_spec(input)?;
        let label = spec.to_string();
        let reporter = tiktools_plugin_installer::NoopReporter;
        let (prepared, _trust_guard, _target) = self
            .fetch_prepared(
                &packages,
                &spec,
                &reporter,
                &std::sync::atomic::AtomicBool::new(false),
            )
            .await
            .map_err(OperationError::install)?;
        let preview = packages.installer.preview(&prepared);
        packages.remember_review(&preview.consent_token, &label, prepared);
        Ok(preview)
    }

    pub fn plugin_package_install(
        self: &Arc<Self>,
        input: PackageSpecInput,
        consent_token: String,
        policy: Option<UpdatePolicy>,
    ) -> Result<JobSnapshot, OperationError> {
        let spec = product_package_spec(input)?;
        self.spawn_package_job(
            JobKind::Install,
            spec,
            InstallConsent::Reviewed(consent_token),
            policy,
        )
    }

    pub fn plugin_package_update(
        self: &Arc<Self>,
        plugin_id: &str,
        version: Option<String>,
        consent_token: Option<String>,
    ) -> Result<JobSnapshot, OperationError> {
        let packages = self.packages()?;
        let receipt = packages
            .installer
            .receipt(plugin_id)
            .map_err(OperationError::install)?
            .ok_or_else(|| OperationError::install(InstallError::not_installed(plugin_id)))?;
        let PackageOrigin::Registry { registry, .. } = &receipt.origin else {
            return Err(OperationError::install(InstallError::new(
                InstallErrorCode::Unsupported,
                format!("{plugin_id} was not installed from a registry; install a newer package instead"),
            )));
        };
        if registry != OFFICIAL_REGISTRY_NAME {
            return Err(OperationError::invalid(
                "this plugin was installed from a legacy source; review a local package or the configured Cloud catalog",
            ));
        }
        let spec = PackageSpec::try_from(PackageSpecInput::Registry {
            id: plugin_id.to_owned(),
            version,
            registry: Some(registry.clone()),
        })
        .map_err(OperationError::install)?;
        let consent = match consent_token {
            Some(token) => InstallConsent::Reviewed(token),
            None => InstallConsent::Automatic,
        };
        self.spawn_package_job(JobKind::Update, spec, consent, None)
    }

    pub fn plugin_package_rollback(
        self: &Arc<Self>,
        plugin_id: &str,
        version: Option<String>,
    ) -> Result<JobSnapshot, OperationError> {
        let packages = self.packages()?;
        let job = Arc::new(self.start_job(&packages, JobKind::Rollback, Some(plugin_id), None)?);
        let snapshot = job
            .snapshot()
            .ok_or_else(|| OperationError::internal("job vanished"))?;
        let core = Arc::clone(self);
        let plugin_id = plugin_id.to_owned();
        tokio::spawn(async move {
            let installer = packages.installer.clone();
            let worker_core = Arc::clone(&core);
            let worker_job = Arc::clone(&job);
            let result = tokio::task::spawn_blocking(move || {
                let _sideload = packages.try_sideload_permit()?;
                let _invocation =
                    worker_core
                        .plugin_state
                        .invocation_gate
                        .lock()
                        .map_err(|_| {
                            crate::plugin_packages::unavailable(
                                "plugin invocation lease is unavailable",
                            )
                        })?;
                #[cfg(all(feature = "persistence", feature = "cloud-auth"))]
                let _source_lease = crate::plugin_packages::try_product_mutation_lease(&worker_core)?;
                #[cfg(all(feature = "persistence", feature = "cloud-auth"))]
                crate::plugin_packages::reserve_package_sync_source(&worker_core, &plugin_id)?;
                #[cfg(all(feature = "persistence", not(feature = "cloud-auth")))]
                worker_core
                    .db
                    .invalidate_plugin_package_source(&plugin_id)
                    .map_err(InstallError::io)?;
                let activation = CoreActivation::new(&worker_core);
                let outcome = installer.rollback_local(
                    &plugin_id,
                    version.as_deref(),
                    &activation,
                    worker_job.as_ref(),
                );
                #[cfg(feature = "persistence")]
                if let Err(error) = worker_core.db.invalidate_plugin_package_source(&plugin_id) {
                    worker_core.rebuild_processor_index();
                    worker_core.refresh_automation_snapshot();
                    return Err(InstallError::io(format!(
                        "package rollback may have changed files but its final source fence failed: {error}"
                    )));
                }
                let outcome = outcome?;
                activation.finish()?;
                Ok(outcome)
            })
            .await
            .unwrap_or_else(|error| Err(InstallError::io(error)));
            core.finish_package_job(&job, result);
        });
        Ok(snapshot)
    }

    pub async fn plugin_package_versions(
        self: &Arc<Self>,
        plugin_id: &str,
    ) -> Result<PluginVersions, OperationError> {
        let packages = self.packages()?;
        let installed = packages
            .installer
            .versions(plugin_id)
            .map_err(OperationError::install)?;
        let registry_name = installed
            .receipt
            .as_ref()
            .and_then(|receipt| receipt.origin.registry_name().map(ToOwned::to_owned));
        let host = packages.installer.host().clone();
        let lookup = registry_package_live(&packages, plugin_id, registry_name.as_deref()).await;
        let (registry, available, registry_error) = match lookup {
            Ok((source, document, live)) => {
                let mut available: Vec<AvailableRelease> = document
                    .releases
                    .iter()
                    .map(|release| AvailableRelease {
                        version: release.version.clone(),
                        channel: release.channel.clone(),
                        published_at: release.published_at,
                        yanked: release.yanked,
                        advisory: release.advisory.clone(),
                        compatible: release_is_compatible(release, &host),
                        live_status: live
                            .get(&release.version)
                            .map_or(LiveReleaseStatus::Unknown, |entry| entry.status),
                        changelog: release.changelog.clone(),
                    })
                    .collect();
                available.reverse();
                (Some(source.name), available, None)
            }
            Err(error) => (registry_name, Vec::new(), Some(error.to_string())),
        };
        Ok(PluginVersions {
            installed,
            registry,
            available,
            registry_error,
        })
    }

    pub fn plugin_package_receipts(&self) -> Result<Vec<InstallReceipt>, OperationError> {
        self.packages()?
            .installer
            .receipts()
            .list()
            .map_err(OperationError::install)
    }

    pub fn plugin_package_policy(
        &self,
        plugin_id: &str,
        patch: PolicyPatch,
    ) -> Result<InstallReceipt, OperationError> {
        let packages = self.packages()?;
        let mut store = packages
            .installer
            .try_lock_store()
            .map_err(OperationError::install)?;
        let receipt = store
            .receipt(plugin_id)
            .map_err(OperationError::install)?
            .ok_or_else(|| OperationError::install(InstallError::not_installed(plugin_id)))?;
        let mut policy = receipt.policy;
        if let Some(channel) = patch.channel {
            if !matches!(channel.as_str(), "stable" | "beta") {
                return Err(OperationError::invalid(
                    "channel must be `stable` or `beta`",
                ));
            }
            policy.channel = channel;
        }
        if let Some(auto_update) = patch.auto_update {
            policy.auto_update = auto_update;
        }
        if let Some(pinned) = patch.pinned {
            policy.pinned = pinned;
        }
        store
            .set_policy(plugin_id, policy)
            .map_err(OperationError::install)
    }

    pub async fn plugin_updates_check(self: &Arc<Self>) -> Result<UpdateReport, OperationError> {
        let packages = self.packages()?;
        let _check = packages
            .updates_gate
            .try_lock()
            .map_err(|_| OperationError::conflict("an update check is already in progress"))?;
        let receipts = packages
            .installer
            .receipts()
            .list()
            .map_err(OperationError::install)?;
        let mut report = UpdateReport {
            checked_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            ..UpdateReport::default()
        };
        // Each result is derived from the same signed document used for its
        // live status, not a second registry read from another generation.
        let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
        for receipt in &receipts {
            let PackageOrigin::Registry { registry, .. } = &receipt.origin else {
                continue;
            };
            let lookup = bounded_update_query(
                deadline,
                registry_package_live(&packages, &receipt.id, Some(registry)),
            )
            .await;
            let (source, document, live) = match lookup {
                Some(Ok((source, document, live))) => (source, document, live),
                Some(Err(error)) => {
                    report.errors.push(format!("{}: {error}", receipt.id));
                    report.updates.push(status_only_update(receipt, registry));
                    continue;
                }
                None => {
                    report
                        .errors
                        .push(format!("{}: update check timed out", receipt.id));
                    report.updates.push(status_only_update(receipt, registry));
                    continue;
                }
            };
            let current = installed_live_status(receipt, &source, &document, &live);
            let update =
                update_for_document(receipt, registry, &document, packages.installer.host());
            if update.is_none() && current == LiveReleaseStatus::Approved {
                continue;
            }
            let mut update = update.unwrap_or_else(|| status_only_update(receipt, registry));
            update.current_status = current;
            update.latest_status = update
                .latest
                .as_ref()
                .and_then(|version| live.get(version))
                .map_or(LiveReleaseStatus::Unknown, |entry| entry.status);
            update.auto_applicable &= update.latest_status == LiveReleaseStatus::Approved;
            report.updates.push(update);
        }
        packages.store_update_report(report.clone());
        if let Ok(value) = serde_json::to_value(&report) {
            self.events
                .publish_domain(DomainEvent::PluginUpdates { report: value });
        }
        for update in report
            .updates
            .iter()
            .filter(|update| update.auto_applicable)
        {
            if let Err(error) = self.plugin_package_update(&update.id, update.latest.clone(), None)
            {
                tracing::warn!(plugin = %update.id, %error, "automatic plugin update could not start");
            }
        }
        Ok(report)
    }

    pub fn plugin_updates(&self) -> Result<UpdateReport, OperationError> {
        Ok(self.packages()?.update_report())
    }

    pub fn plugin_jobs(&self) -> Result<Vec<JobSnapshot>, OperationError> {
        Ok(self.packages()?.jobs.list())
    }

    pub fn plugin_job(&self, job_id: &str) -> Result<JobSnapshot, OperationError> {
        self.packages()?
            .jobs
            .get(job_id)
            .ok_or_else(|| OperationError::not_found(format!("Job `{job_id}` does not exist.")))
    }

    pub fn plugin_job_cancel(&self, job_id: &str) -> Result<bool, OperationError> {
        Ok(self.packages()?.jobs.cancel(job_id))
    }

    pub fn registry_list(&self) -> Result<Vec<RegistryStatus>, OperationError> {
        Ok(self.packages()?.registry().statuses())
    }

    pub async fn registry_root(&self, url: &str) -> Result<RegistryRootPreview, OperationError> {
        let packages = self.packages()?;
        let root = packages
            .registry()
            .fetch_root(url)
            .await
            .map_err(registry_error)?;
        let keys = root
            .keys
            .iter()
            .map(|key| {
                Ok(RegistryRootKey {
                    key_id: key.key_id.to_string(),
                    fingerprint: key
                        .fingerprint()
                        .map_err(registry_package_error)?
                        .to_string(),
                })
            })
            .collect::<Result<_, OperationError>>()?;
        Ok(RegistryRootPreview {
            url: url.to_owned(),
            keys,
        })
    }

    pub async fn registry_add(
        &self,
        input: RegistryAddInput,
    ) -> Result<RegistryStatus, OperationError> {
        let packages = self.packages()?;
        let mut source = RegistrySource::new(&input.name, &input.url).map_err(registry_error)?;
        source.scopes = input.scopes;
        source.priority = input.priority.clamp(-100, 99);
        if !input.fingerprints.is_empty() {
            let root = packages
                .registry()
                .fetch_root(&input.url)
                .await
                .map_err(registry_error)?;
            let mut pinned = Vec::new();
            for key in root.keys {
                let fingerprint = key.fingerprint().map_err(registry_package_error)?;
                if input
                    .fingerprints
                    .iter()
                    .any(|expected| expected.eq_ignore_ascii_case(fingerprint.as_hex()))
                {
                    pinned.push(key);
                }
            }
            if pinned.len() != input.fingerprints.len() {
                return Err(OperationError::conflict(
                    "the registry root keys no longer match the confirmed fingerprints",
                ));
            }
            source.root_keys = pinned;
        }
        let name = source.name.clone();
        packages.registry().add(source).map_err(registry_error)?;
        match packages.registry().refresh(&name).await {
            Ok(status) => Ok(status),
            Err(error) => {
                tracing::warn!(registry = %name, %error, "new registry could not be refreshed");
                packages.registry().status(&name).map_err(registry_error)
            }
        }
    }

    pub fn registry_remove(&self, name: &str) -> Result<(), OperationError> {
        self.packages()?
            .registry()
            .remove(name)
            .map_err(registry_error)
    }

    pub fn registry_set_enabled(
        &self,
        name: &str,
        enabled: bool,
    ) -> Result<RegistryStatus, OperationError> {
        self.packages()?
            .registry()
            .set_enabled(name, enabled)
            .map_err(registry_error)
    }

    pub async fn registry_refresh(
        &self,
        name: Option<&str>,
    ) -> Result<Vec<RegistryStatus>, OperationError> {
        let packages = self.packages()?;
        match name {
            Some(name) => Ok(vec![packages
                .registry()
                .refresh(name)
                .await
                .map_err(registry_error)?]),
            None => Ok(packages.registry().refresh_all().await),
        }
    }

    pub async fn registry_search(
        &self,
        query: SearchQuery,
    ) -> Result<SearchResults, OperationError> {
        if query.query.len() > 256 {
            return Err(OperationError::invalid(
                "search queries are at most 256 characters",
            ));
        }
        if query
            .registry
            .as_deref()
            .is_some_and(|name| name != OFFICIAL_REGISTRY_NAME)
        {
            return Err(OperationError::invalid(
                "plugin sources are managed by the application",
            ));
        }
        Ok(self.packages()?.registry().search(&query).await)
    }

    pub async fn registry_package(
        &self,
        id: &str,
        registry: Option<&str>,
    ) -> Result<RegistryPackageView, OperationError> {
        let packages = self.packages()?;
        let (source, document, live) = registry_package_live(&packages, id, registry).await?;
        Ok(RegistryPackageView {
            registry: source.name,
            document,
            live_statuses: live
                .into_iter()
                .map(|(version, entry)| (version, entry.status))
                .collect(),
        })
    }

    pub fn registry_defaults(&self) -> Result<RegistryDefaults, OperationError> {
        self.packages()?
            .registry()
            .try_defaults()
            .map_err(registry_error)
    }

    pub fn registry_set_defaults(
        &self,
        defaults: RegistryDefaults,
    ) -> Result<RegistryDefaults, OperationError> {
        if !matches!(defaults.channel.as_str(), "stable" | "beta") {
            return Err(OperationError::invalid(
                "channel must be `stable` or `beta`",
            ));
        }
        if !(1..=168).contains(&defaults.check_interval_hours) {
            return Err(OperationError::invalid(
                "update checks run every 1..=168 hours",
            ));
        }
        let packages = self.packages()?;
        let _settings = packages
            .try_sideload_settings()
            .map_err(OperationError::install)?;
        packages
            .registry()
            .set_defaults(defaults.clone())
            .map_err(registry_error)?;
        Ok(defaults)
    }

    pub(crate) fn after_package_change(&self, outcome: &InstallOutcome) {
        self.start_enabled_event_subscribers();
        self.rebuild_processor_index();
        self.refresh_automation_snapshot();
        let plugin_id = outcome.receipt.id.clone();
        self.events.publish_domain(match &outcome.previous_version {
            None => DomainEvent::PluginInstalled { plugin_id },
            Some(from) => DomainEvent::PluginUpdated {
                plugin_id,
                from_version: Some(from.clone()),
                to_version: outcome.receipt.version.clone(),
            },
        });
    }

    pub(crate) fn adopt_user_plugin_packages(&self) {
        let Ok(packages) = self.packages() else {
            return;
        };
        let user_plugins: Vec<String> = self
            .plugins
            .list()
            .into_iter()
            .filter(|plugin| plugin.source == tiktools_plugin_loader::PluginSource::User)
            .map(|plugin| plugin.manifest.id)
            .collect();
        match packages
            .installer
            .adopt(user_plugins.iter().map(String::as_str))
        {
            Ok(adopted) if !adopted.is_empty() => {
                tracing::info!(count = adopted.len(), "adopted unmanaged plugin packages")
            }
            Ok(_) => {}
            Err(error) => tracing::warn!(%error, "plugin package adoption failed"),
        }
    }

    pub fn spawn_plugin_update_checker(self: &Arc<Self>, runtime: &tokio::runtime::Handle) {
        if self.plugin_packages().is_err() {
            return;
        }
        let core = Arc::clone(self);
        runtime.spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            loop {
                if core
                    .shutdown_started
                    .load(std::sync::atomic::Ordering::Acquire)
                {
                    break;
                }
                let has_registry_packages = core.plugin_package_receipts().is_ok_and(|receipts| {
                    receipts.iter().any(|receipt| receipt.origin.is_registry())
                });
                if has_registry_packages {
                    if let Err(error) = core.plugin_updates_check().await {
                        tracing::warn!(%error, "plugin update check failed");
                    }
                }
                let hours = core
                    .registry_defaults()
                    .map(|defaults| defaults.check_interval_hours)
                    .unwrap_or(12)
                    .clamp(1, 168);
                let deadline = std::time::Instant::now()
                    + std::time::Duration::from_secs(u64::from(hours) * 3600);
                while std::time::Instant::now() < deadline {
                    if core
                        .shutdown_started
                        .load(std::sync::atomic::Ordering::Acquire)
                    {
                        return;
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                }
            }
        });
    }

    fn spawn_package_job(
        self: &Arc<Self>,
        kind: JobKind,
        spec: PackageSpec,
        consent: InstallConsent,
        policy: Option<UpdatePolicy>,
    ) -> Result<JobSnapshot, OperationError> {
        let packages = self.packages()?;
        let label = spec.to_string();
        let job =
            Arc::new(self.start_job(&packages, kind, spec.plugin_id(), Some(label.clone()))?);
        let snapshot = job
            .snapshot()
            .ok_or_else(|| OperationError::internal("job vanished"))?;
        let core = Arc::clone(self);
        tokio::spawn(async move {
            let result = core
                .run_package_job(&packages, &job, spec, label, consent, policy)
                .await;
            core.finish_package_job(&job, result);
        });
        Ok(snapshot)
    }

    fn start_job(
        &self,
        packages: &PluginPackages,
        kind: JobKind,
        plugin_id: Option<&str>,
        spec: Option<String>,
    ) -> Result<JobHandle, OperationError> {
        if let Some(plugin_id) = plugin_id {
            if packages.installer.is_busy(plugin_id)
                || packages.jobs.active_for(plugin_id).is_some()
            {
                return Err(OperationError::install(InstallError::busy(plugin_id)));
            }
        }
        Ok(packages
            .jobs
            .start(kind, plugin_id.map(ToOwned::to_owned), spec))
    }

    fn finish_package_job(&self, job: &JobHandle, result: Result<InstallOutcome, InstallError>) {
        match &result {
            Ok(outcome) => self.after_package_change(outcome),
            Err(error) => {
                tracing::warn!(job = %job.id(), code = ?error.code, %error, "plugin package job failed")
            }
        }
        job.finish(&result);
    }

    async fn run_package_job(
        self: &Arc<Self>,
        packages: &Arc<PluginPackages>,
        job: &Arc<JobHandle>,
        spec: PackageSpec,
        label: String,
        consent: InstallConsent,
        policy: Option<UpdatePolicy>,
    ) -> Result<InstallOutcome, InstallError> {
        let reviewed = match (&consent, &spec) {
            // A Cloud review never authorizes stale metadata. Resolve the
            // active generation and recheck yanks, keys, digest and the
            // exact consent token immediately before dispatching the commit.
            (InstallConsent::Reviewed(token), PackageSpec::Registry { .. }) => {
                let _ = packages.take_review(token, &label);
                None
            }
            (InstallConsent::Reviewed(token), _) => packages.take_review(token, &label),
            (InstallConsent::Automatic, _) => None,
        };
        let (mut prepared, trust_guard, target) = match reviewed {
            Some(prepared) => (prepared, None, None),
            None => {
                self.fetch_prepared(packages, &spec, job.as_ref(), &job.cancel_flag())
                    .await?
            }
        };
        let preview = packages.installer.preview(&prepared);
        job.describe(
            Some(&prepared.manifest.id),
            preview.installed_version.as_deref(),
            Some(&prepared.manifest.version),
        );
        let token = match consent {
            InstallConsent::Reviewed(token) => token,
            InstallConsent::Automatic => {
                let receipt = packages.installer.receipt(&prepared.manifest.id)?;
                let same_publisher = receipt.as_ref().is_some_and(|receipt| {
                    receipt.signature.is_verified()
                        && receipt.signature.publisher == preview.signature.publisher
                });
                if !preview.signature.is_verified()
                    || !same_publisher
                    || preview.permission_diff.adds_access()
                {
                    return Err(InstallError::new(
                        InstallErrorCode::ConsentRequired,
                        format!(
                            "{} needs review before it can be updated",
                            prepared.manifest.id
                        ),
                    )
                    .with_details(serde_json::to_value(&preview).unwrap_or_default()));
                }
                preview.consent_token.clone()
            }
        };
        job.check()?;
        if let PackageOrigin::Registry { registry, url } = &prepared.origin {
            let guard = trust_guard.as_ref().ok_or_else(|| {
                InstallError::new(
                    InstallErrorCode::Unsupported,
                    "Cloud trust review is unavailable",
                )
            })?;
            let generation = packages
                .registry()
                .check_fixed_generation(guard, registry, url)
                .await?;
            let target = target.as_deref().ok_or_else(|| {
                InstallError::new(
                    InstallErrorCode::Unsupported,
                    "Cloud artifact target is unavailable",
                )
            })?;
            tokio::time::timeout(
                std::time::Duration::from_secs(20),
                verify_current_cloud_release(&prepared, target, generation),
            )
            .await
            .map_err(|_| {
                InstallError::new(
                    InstallErrorCode::Unsupported,
                    "Cloud release status timed out",
                )
            })??;
            packages
                .registry()
                .check_fixed_generation(guard, registry, url)
                .await?;
            prepared.bind_verified_cloud_artifact(target, generation)?;
        }
        job.check()?;
        let installer = packages.installer.clone();
        let packages = Arc::clone(packages);
        let core = Arc::clone(self);
        let worker_job = Arc::clone(job);
        tokio::task::spawn_blocking(move || {
            // For Cloud packages this retains the exact verified trust
            // snapshot until the filesystem transaction completes.
            let _trust_guard = trust_guard;
            let _sideload = match &prepared.origin {
                PackageOrigin::File => Some(packages.try_sideload_permit()?),
                PackageOrigin::Registry { .. } => None,
                _ => return Err(InstallError::new(InstallErrorCode::Unsupported, "unsupported plugin package origin")),
            };
            let _invocation = core.plugin_state.invocation_gate.lock().map_err(|_| {
                crate::plugin_packages::unavailable("plugin invocation lease is unavailable")
            })?;
            #[cfg(all(feature = "persistence", feature = "cloud-auth"))]
            let _source_lease = crate::plugin_packages::try_product_mutation_lease(&core)?;
            #[cfg(all(feature = "persistence", feature = "cloud-auth"))]
            crate::plugin_packages::reserve_package_sync_source(&core, &prepared.manifest.id)?;
            #[cfg(all(feature = "persistence", not(feature = "cloud-auth")))]
            core.db
                .invalidate_plugin_package_source(&prepared.manifest.id)
                .map_err(InstallError::io)?;
            let activation = CoreActivation::new(&core);
            let outcome = installer.commit(
                &prepared,
                &CommitOptions {
                    consent: Consent::Token(token),
                    policy,
                },
                &activation,
                worker_job.as_ref(),
            );
            #[cfg(feature = "persistence")]
            if let Err(error) = core.db.invalidate_plugin_package_source(&prepared.manifest.id) {
                core.rebuild_processor_index();
                core.refresh_automation_snapshot();
                return Err(InstallError::io(format!(
                    "package operation may have changed files but its final source fence failed: {error}"
                )));
            }
            let outcome = outcome?;
            activation.finish()?;
            Ok(outcome)
        })
        .await
        .unwrap_or_else(|error| Err(InstallError::io(error)))
    }

    async fn fetch_prepared(
        &self,
        packages: &Arc<PluginPackages>,
        spec: &PackageSpec,
        reporter: &dyn JobReporter,
        cancel: &std::sync::atomic::AtomicBool,
    ) -> Result<
        (
            PreparedPackage,
            Option<Arc<tiktools_plugin_registry::RegistryTrustGuard>>,
            Option<String>,
        ),
        InstallError,
    > {
        let defaults = packages
            .registry()
            .try_defaults()
            .map_err(InstallError::io)?;
        let channel = match spec.plugin_id() {
            Some(id) => packages
                .installer
                .receipt(id)?
                .map(|receipt| receipt.policy.channel)
                .unwrap_or_else(|| defaults.channel.clone()),
            None => defaults.channel.clone(),
        };
        if !matches!(spec, PackageSpec::Registry { .. }) && !defaults.allow_sideload {
            return Err(InstallError::new(
                InstallErrorCode::Unsupported,
                "installing plugins from local files is disabled in the plugin settings",
            ));
        }
        let resolved = packages
            .sources
            .fetch(spec, &channel, packages.installer.host(), reporter, cancel)
            .await?;
        reporter.stage(JobStage::Verify);
        let installer = packages.installer.clone();
        tokio::task::spawn_blocking(move || {
            // Rust may otherwise capture only individual fields of
            // `resolved`, releasing the fixed-generation trust guard before
            // signature verification completes.
            let trust_guard = resolved.trust_guard;
            let target = resolved.target;
            let prepared = installer.prepare(&resolved.path, resolved.origin, resolved.policy)?;
            if let Some(expected) = &resolved.expected {
                expected.check(&prepared)?;
            }
            Ok((prepared, trust_guard, target))
        })
        .await
        .unwrap_or_else(|error| Err(InstallError::io(error)))
    }
}

fn status_only_update(receipt: &InstallReceipt, registry: &str) -> UpdateInfo {
    UpdateInfo {
        id: receipt.id.clone(),
        registry: registry.to_owned(),
        current: receipt.version.clone(),
        latest: None,
        kind: None,
        channel: receipt.policy.channel.clone(),
        pinned: receipt.policy.pinned,
        permission_diff: Default::default(),
        advisory: None,
        changelog: None,
        auto_applicable: false,
        current_status: LiveReleaseStatus::Unknown,
        latest_status: LiveReleaseStatus::Unknown,
    }
}

async fn bounded_update_query<T>(
    deadline: tokio::time::Instant,
    query: impl std::future::Future<Output = T>,
) -> Option<T> {
    if tokio::time::Instant::now() >= deadline {
        return None;
    }
    tokio::time::timeout_at(deadline, query).await.ok()
}

#[derive(Debug, Clone)]
struct LiveArtifactStatus {
    status: LiveReleaseStatus,
    targets: BTreeMap<String, LiveTargetStatus>,
}

#[derive(Debug, Clone)]
struct LiveTargetStatus {
    status: LiveReleaseStatus,
    digest: String,
    key_id: Option<String>,
}

/// Only a receipt created by the verified install path can identify an
/// original target. Never infer it from the current generation or digest.
fn installed_live_status(
    receipt: &InstallReceipt,
    source: &RegistrySource,
    document: &PackageDocument,
    live: &BTreeMap<String, LiveArtifactStatus>,
) -> LiveReleaseStatus {
    let unknown = LiveReleaseStatus::Unknown;
    let Some(identity) = receipt.cloud_artifact.as_ref() else {
        return unknown;
    };
    let Some(origin) = source.cloud_origin.as_deref() else {
        return unknown;
    };
    let PackageOrigin::Registry { registry, url } = &receipt.origin else {
        return unknown;
    };
    let Ok(generation) = identity.generation.parse::<u64>() else {
        return unknown;
    };
    if registry != &source.name
        || generation == 0
        || generation.to_string() != identity.generation
        || i64::try_from(generation).is_err()
        || !receipt.signature.is_verified()
        || tiktools_plugin_registry::generation_files_url(origin, generation)
            .ok()
            .as_ref()
            != Some(url)
    {
        return unknown;
    }
    if document.publisher.as_deref().is_none_or(str::is_empty)
        || document.publisher != receipt.signature.publisher
    {
        return unknown;
    }
    let Some(release) = document
        .release(&receipt.version)
        .filter(|release| release.version == receipt.version)
    else {
        return unknown;
    };
    let Some(target) = live
        .get(&release.version)
        .and_then(|entry| entry.targets.get(&identity.target))
    else {
        return unknown;
    };
    if Some(target.digest.as_str()) != receipt.archive_digest.as_deref() {
        return unknown;
    }
    if target.key_id.as_deref().is_none_or(str::is_empty)
        || target.key_id != receipt.signature.key_id
    {
        return unknown;
    }
    target.status
}

/// Load one signed document and query current Cloud status while retaining
/// its trust lease. A failed status check still permits displaying the signed
/// document, but no release is described as currently approved.
async fn registry_package_live(
    packages: &PluginPackages,
    id: &str,
    registry: Option<&str>,
) -> Result<
    (
        RegistrySource,
        PackageDocument,
        BTreeMap<String, LiveArtifactStatus>,
    ),
    OperationError,
> {
    let client = packages.registry();
    let (source, document, guard) = client
        .package_guarded(id, registry)
        .await
        .map_err(registry_error)?;
    let host = packages.installer.host();
    let unknown = classify_live_releases(&document, host, None, |_| Ok(false));
    let Some(guard) = guard else {
        return Ok((source, document, unknown));
    };
    let Some(origin) = source.cloud_origin.as_deref() else {
        return Ok((source, document, unknown));
    };
    let query = async {
        let generation = client
            .check_fixed_generation(&guard, &source.name, &source.url)
            .await
            .map_err(|_| ())?;
        let generation_i64 = i64::try_from(generation)
            .ok()
            .filter(|n| *n > 0)
            .ok_or(())?;
        let catalog = cloud_client::CloudCatalog::new(origin).map_err(|_| ())?;
        let records = load_catalog_versions(&catalog, id, &host.target, generation_i64).await?;
        client
            .check_fixed_generation(&guard, &source.name, &source.url)
            .await
            .map_err(|_| ())?;
        Ok::<_, ()>(classify_live_releases(
            &document,
            host,
            Some(&records),
            |digest| client.try_is_revoked(digest).map_err(|_| ()),
        ))
    };
    let live = tokio::time::timeout(Duration::from_secs(20), query)
        .await
        .ok()
        .and_then(Result::ok)
        .unwrap_or(unknown);
    Ok((source, document, live))
}

async fn load_catalog_versions(
    catalog: &cloud_client::CloudCatalog,
    id: &str,
    target: &str,
    generation: i64,
) -> Result<BTreeMap<(String, String), cloud_client::CatalogVersion>, ()> {
    let mut items = BTreeMap::new();
    let mut cursor = None;
    let mut seen = BTreeSet::new();
    for _ in 0..128 {
        let page = catalog
            .plugin_version_page(id, Some(target), Some(generation), cursor.as_deref())
            .await
            .map_err(|_| ())?;
        for item in page.items {
            let key = (item.version.clone(), item.target.clone());
            if items.insert(key, item).is_some() {
                return Err(());
            }
        }
        match page.next_cursor {
            None => return Ok(items),
            Some(next) if seen.insert(next.clone()) => cursor = Some(next),
            Some(_) => return Err(()),
        }
    }
    Err(())
}

fn classify_live_releases(
    document: &PackageDocument,
    host: &tiktools_plugin_api::HostCompat,
    records: Option<&BTreeMap<(String, String), cloud_client::CatalogVersion>>,
    revoked: impl Fn(&tiktools_plugin_package::Sha256Digest) -> Result<bool, ()>,
) -> BTreeMap<String, LiveArtifactStatus> {
    let mut counts = BTreeMap::new();
    for release in &document.releases {
        if let Some(version) = release.semver() {
            *counts.entry(version).or_insert(0usize) += 1;
        }
    }
    let mut result = BTreeMap::new();
    for release in &document.releases {
        let mut entry = LiveArtifactStatus {
            status: LiveReleaseStatus::Unknown,
            targets: BTreeMap::new(),
        };
        if release
            .semver()
            .is_some_and(|version| counts.get(&version) == Some(&1))
        {
            for artifact in &release.artifacts {
                let digest = artifact.sha256.to_string();
                let mut status = LiveReleaseStatus::Unknown;
                if release
                    .artifacts
                    .iter()
                    .filter(|a| a.target == artifact.target)
                    .count()
                    == 1
                {
                    if release.yanked {
                        status = LiveReleaseStatus::Yanked;
                    } else if release.advisory.is_some() {
                        status = LiveReleaseStatus::Advisory;
                    } else if let (Some(record), Ok(is_revoked)) = (
                        records.and_then(|items| {
                            items.get(&(release.version.clone(), artifact.target.clone()))
                        }),
                        revoked(&artifact.sha256),
                    ) {
                        // A live approval describes the publisher/key named
                        // in the signed Cloud snapshot. The install path
                        // requires both identities and binds them to the
                        // verified archive, so missing signer metadata must
                        // never be presented as an approved release.
                        if document.publisher.as_deref().is_some_and(|v| !v.is_empty())
                            && artifact.key_id.as_deref().is_some_and(|v| !v.is_empty())
                            && record.sha256 == digest
                            && record.size == artifact.size.to_string()
                        {
                            status = if record.yanked {
                                LiveReleaseStatus::Yanked
                            } else if record.key_revoked || is_revoked {
                                LiveReleaseStatus::KeyRevoked
                            } else if !record.publisher_approved {
                                LiveReleaseStatus::PublisherUnapproved
                            } else if record.advisory.is_some() {
                                LiveReleaseStatus::Advisory
                            } else {
                                LiveReleaseStatus::Approved
                            };
                        }
                    }
                }
                entry.targets.insert(
                    artifact.target.clone(),
                    LiveTargetStatus {
                        status,
                        digest,
                        key_id: artifact.key_id.clone(),
                    },
                );
            }
            match tiktools_plugin_registry::compatible_artifact(release, host) {
                Err(_) => entry.status = LiveReleaseStatus::Incompatible,
                Ok(artifact) => {
                    entry.status = entry
                        .targets
                        .get(&artifact.target)
                        .map_or(LiveReleaseStatus::Unknown, |item| item.status)
                }
            }
        }
        result.insert(release.version.clone(), entry);
    }
    result
}

/// The signed immutable registry grants trust; the live v2 catalog supplies
/// current revocation/yank state for the exact selected artifact. Missing,
/// changed, unbounded, or advisory-bearing status fails closed pending review.
async fn verify_current_cloud_release(
    prepared: &PreparedPackage,
    target: &str,
    generation: u64,
) -> Result<(), InstallError> {
    let unavailable = || {
        InstallError::new(
            InstallErrorCode::Unsupported,
            "current Cloud release status is unavailable or needs review",
        )
    };
    let origin = tiktools_cloud_auth::resolve_cloud_origin().ok_or_else(unavailable)?;
    let catalog = cloud_client::CloudCatalog::new(&origin).map_err(|_| unavailable())?;
    verify_catalog_release(
        &catalog,
        &prepared.manifest.id,
        &prepared.manifest.version,
        target,
        &prepared.digest.to_string(),
        prepared.size(),
        generation,
    )
    .await
}

async fn verify_catalog_release(
    catalog: &cloud_client::CloudCatalog,
    id: &str,
    version: &str,
    target: &str,
    digest: &str,
    size: u64,
    generation: u64,
) -> Result<(), InstallError> {
    let unavailable = || {
        InstallError::new(
            InstallErrorCode::Unsupported,
            "current Cloud release status is unavailable or needs review",
        )
    };
    let generation = i64::try_from(generation)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(unavailable)?;
    let mut cursor: Option<String> = None;
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..128 {
        let page = catalog
            .plugin_version_page(id, Some(target), Some(generation), cursor.as_deref())
            .await
            .map_err(|_| unavailable())?;
        for item in &page.items {
            if let Some(result) =
                validate_current_cloud_release(item, version, target, digest, size)
            {
                return result;
            }
        }
        let Some(next) = page.next_cursor else {
            return Err(unavailable());
        };
        if !seen.insert(next.clone()) {
            return Err(unavailable());
        }
        cursor = Some(next);
    }
    Err(unavailable())
}

fn validate_current_cloud_release(
    item: &cloud_client::CatalogVersion,
    version: &str,
    target: &str,
    digest: &str,
    size: u64,
) -> Option<Result<(), InstallError>> {
    if item.version != version
        || item.target != target
        || item.sha256 != digest
        || item.size != size.to_string()
    {
        return None;
    }
    if item.yanked || item.key_revoked || !item.publisher_approved {
        return Some(Err(InstallError::new(
            InstallErrorCode::Revoked,
            "Cloud release, publisher or signing key is no longer approved",
        )));
    }
    if item.advisory.is_some() {
        return Some(Err(InstallError::new(
            InstallErrorCode::ConsentRequired,
            "Cloud release has an active advisory; installation is unavailable until it is resolved",
        )));
    }
    Some(Ok(()))
}

fn release_is_compatible(
    release: &tiktools_plugin_registry::Release,
    host: &tiktools_plugin_api::HostCompat,
) -> bool {
    let Some(runtime) = release.runtime_kind() else {
        return false;
    };
    release.artifacts.iter().any(|artifact| {
        if !host.accepts_artifact_target(&artifact.target) {
            return false;
        }
        let targets = if artifact.target == tiktools_plugin_registry::ANY_TARGET {
            Vec::new()
        } else {
            vec![artifact.target.clone()]
        };
        host.check(release.requirements(runtime, &targets)).is_ok()
    })
}

fn registry_error(error: tiktools_plugin_registry::RegistryError) -> OperationError {
    OperationError::install(InstallError::from(error))
}

fn registry_package_error(error: tiktools_plugin_package::PackageError) -> OperationError {
    OperationError::install(InstallError::from(error))
}

#[cfg(test)]
mod fixed_source_tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn update_deadline_bounds_a_stalled_catalog_and_is_shared_by_later_queries() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let catalog =
            cloud_client::CloudCatalog::new(&format!("http://{}", listener.local_addr().unwrap()))
                .unwrap();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
        let task = tokio::spawn(async move {
            bounded_update_query(
                deadline,
                catalog.plugin_version_page("acme.plugin", Some("linux-x64-gnu"), Some(1), None),
            )
            .await
        });
        let (_socket, _) = listener.accept().await.unwrap();
        tokio::time::advance(Duration::from_secs(30)).await;
        // The HTTP client's own timeout may fire before the batch deadline;
        // either timeout is safe as long as it cannot produce approval.
        assert!(!matches!(task.await.unwrap(), Some(Ok(_))));
        assert!(bounded_update_query(deadline, async { 42 }).await.is_none());
    }

    #[test]
    fn live_status_matches_the_exact_host_artifact_and_fails_closed() {
        let host = tiktools_plugin_api::HostCompat::new(semver::Version::new(1, 0, 0))
            .with_target("linux-x64-gnu");
        let mut document: PackageDocument = serde_json::from_value(serde_json::json!({
            "id": "acme.plugin", "name": "Example", "publisher":"acme", "releases": [{
                "version":"1.2.3", "runtime":"process", "schemaVersion":3, "protocolVersion":1,
                "artifacts":[
                    {"target":"any", "url":"any.plugin", "size":1, "sha256":"a".repeat(64), "keyId":"key-a"},
                    {"target":"linux-x64-gnu", "url":"native.plugin", "size":2, "sha256":"b".repeat(64), "keyId":"key-a"}
                ]
            }]
        })).unwrap();
        let any: cloud_client::CatalogVersion = serde_json::from_value(serde_json::json!({
            "version":"1.2.3", "target":"any", "sha256":"a".repeat(64), "size":"1", "yanked":false,
            "keyRevoked":false, "publisherApproved":true, "advisory":null
        }))
        .unwrap();
        let mut native = any.clone();
        native.target = "linux-x64-gnu".into();
        native.sha256 = "b".repeat(64);
        native.size = "2".into();
        let mut records = BTreeMap::from([((any.version.clone(), any.target.clone()), any)]);
        let status = |records: &BTreeMap<_, _>| {
            classify_live_releases(&document, &host, Some(records), |_| Ok(false))["1.2.3"].status
        };
        assert_eq!(
            status(&records),
            LiveReleaseStatus::Unknown,
            "an approved any artifact cannot approve the native artifact"
        );
        records.insert(
            (native.version.clone(), native.target.clone()),
            native.clone(),
        );
        assert_eq!(status(&records), LiveReleaseStatus::Approved);
        let mut missing_publisher = document.clone();
        missing_publisher.publisher = None;
        assert_eq!(
            classify_live_releases(&missing_publisher, &host, Some(&records), |_| Ok(false))
                ["1.2.3"]
                .status,
            LiveReleaseStatus::Unknown,
            "a live catalog row cannot approve a release with no publisher"
        );
        missing_publisher.publisher = Some(String::new());
        assert_eq!(
            classify_live_releases(&missing_publisher, &host, Some(&records), |_| Ok(false))
                ["1.2.3"]
                .status,
            LiveReleaseStatus::Unknown
        );
        let mut missing_key = document.clone();
        missing_key.releases[0].artifacts[1].key_id = None;
        assert_eq!(
            classify_live_releases(&missing_key, &host, Some(&records), |_| Ok(false))["1.2.3"]
                .status,
            LiveReleaseStatus::Unknown,
            "a live catalog row cannot approve an artifact with no signing key"
        );
        missing_key.releases[0].artifacts[1].key_id = Some(String::new());
        assert_eq!(
            classify_live_releases(&missing_key, &host, Some(&records), |_| Ok(false))["1.2.3"]
                .status,
            LiveReleaseStatus::Unknown
        );
        native.yanked = true;
        records.insert(
            (native.version.clone(), native.target.clone()),
            native.clone(),
        );
        assert_eq!(
            status(&records),
            LiveReleaseStatus::Yanked,
            "status may change within one generation"
        );
        native.yanked = false;
        native.key_revoked = true;
        records.insert(
            (native.version.clone(), native.target.clone()),
            native.clone(),
        );
        assert_eq!(status(&records), LiveReleaseStatus::KeyRevoked);
        native.key_revoked = false;
        native.publisher_approved = false;
        records.insert(
            (native.version.clone(), native.target.clone()),
            native.clone(),
        );
        assert_eq!(status(&records), LiveReleaseStatus::PublisherUnapproved);
        native.publisher_approved = true;
        native.sha256 = "c".repeat(64);
        records.insert((native.version.clone(), native.target.clone()), native);
        assert_eq!(
            status(&records),
            LiveReleaseStatus::Unknown,
            "digest mismatch must not approve"
        );
        assert_eq!(
            classify_live_releases(&document, &host, None, |_| Ok(false))["1.2.3"].status,
            LiveReleaseStatus::Unknown
        );
        // Equal digests on multiple targets may have different live states.
        // Old receipts cannot choose one; new receipts bind the exact target.
        document.releases[0].artifacts[0].sha256 = document.releases[0].artifacts[1].sha256.clone();
        records
            .get_mut(&("1.2.3".into(), "linux-x64-gnu".into()))
            .unwrap()
            .sha256 = "b".repeat(64);
        records
            .get_mut(&("1.2.3".into(), "any".into()))
            .unwrap()
            .sha256 = "b".repeat(64);
        records
            .get_mut(&("1.2.3".into(), "any".into()))
            .unwrap()
            .yanked = true;
        let ambiguous = classify_live_releases(&document, &host, Some(&records), |_| Ok(false));
        assert_eq!(ambiguous["1.2.3"].status, LiveReleaseStatus::Approved);
        let origin = "https://cloud.example";
        let historical_url = tiktools_plugin_registry::generation_files_url(origin, 3).unwrap();
        let mut source = RegistrySource::new("official", &historical_url).unwrap();
        source.cloud_origin = Some(origin.into());
        let mut receipt: InstallReceipt = serde_json::from_value(serde_json::json!({
            "id":"acme.plugin", "version":"1.2.3",
            "origin":{"kind":"registry", "registry":"official", "url":historical_url},
            "archiveDigest":"b".repeat(64), "signature":{"status":"verified", "publisher":"acme", "keyId":"key-a"},
            "runtime":"process", "installedAt":1, "updatedAt":1
        }))
        .unwrap();
        assert_eq!(
            installed_live_status(&receipt, &source, &document, &ambiguous),
            LiveReleaseStatus::Unknown
        );
        receipt.cloud_artifact = Some(tiktools_plugin_installer::CloudArtifactIdentity {
            target: "any".into(),
            generation: "3".into(),
        });
        assert_eq!(
            installed_live_status(&receipt, &source, &document, &ambiguous),
            LiveReleaseStatus::Yanked
        );
        receipt.cloud_artifact.as_mut().unwrap().target = "linux-x64-gnu".into();
        assert_eq!(
            installed_live_status(&receipt, &source, &document, &ambiguous),
            LiveReleaseStatus::Approved
        );
        receipt.signature.key_id = Some("other-key".into());
        assert_eq!(
            installed_live_status(&receipt, &source, &document, &ambiguous),
            LiveReleaseStatus::Unknown
        );
        receipt.signature.key_id = Some("key-a".into());
        receipt.signature.publisher = Some("other-publisher".into());
        assert_eq!(
            installed_live_status(&receipt, &source, &document, &ambiguous),
            LiveReleaseStatus::Unknown
        );
        receipt.signature.publisher = Some("acme".into());
        receipt.cloud_artifact.as_mut().unwrap().generation = "03".into();
        assert_eq!(
            installed_live_status(&receipt, &source, &document, &ambiguous),
            LiveReleaseStatus::Unknown
        );
        receipt.cloud_artifact.as_mut().unwrap().generation = "4".into();
        assert_eq!(
            installed_live_status(&receipt, &source, &document, &ambiguous),
            LiveReleaseStatus::Unknown
        );
        document.releases.push(document.releases[0].clone());
        assert_eq!(
            classify_live_releases(&document, &host, Some(&records), |_| Ok(false))["1.2.3"].status,
            LiveReleaseStatus::Unknown,
            "duplicate signed versions are ambiguous"
        );
    }

    #[test]
    fn product_specs_cannot_choose_another_registry_or_network_url() {
        for input in [
            PackageSpecInput::Spec {
                spec: "other:acme.plugin@1.0.0".into(),
            },
            PackageSpecInput::Registry {
                id: "acme.plugin".into(),
                version: None,
                registry: Some("other".into()),
            },
            PackageSpecInput::Url {
                url: "https://other.example/plugin.plugin".into(),
            },
            PackageSpecInput::Spec {
                spec: "https://other.example/plugin.plugin".into(),
            },
        ] {
            assert!(product_package_spec(input).is_err());
        }
        assert!(matches!(
            product_package_spec(PackageSpecInput::Spec {
                spec: "acme.plugin@1.0.0".into()
            }),
            Ok(PackageSpec::Registry { .. })
        ));
        assert!(matches!(
            product_package_spec(PackageSpecInput::File {
                path: "/tmp/example.plugin".into()
            }),
            Ok(PackageSpec::File(_))
        ));
    }

    #[test]
    fn current_cloud_status_must_match_exact_artifact_and_remain_approved() {
        let digest = "a".repeat(64);
        let mut item: cloud_client::CatalogVersion = serde_json::from_value(serde_json::json!({
            "version": "1.2.3", "target": "linux-x64-gnu", "sha256": digest,
            "size": "42", "yanked": false, "keyRevoked": false,
            "publisherApproved": true, "advisory": null
        }))
        .unwrap();
        let check = |item: &cloud_client::CatalogVersion, target: &str, size| {
            validate_current_cloud_release(item, "1.2.3", target, &"a".repeat(64), size)
        };
        assert!(check(&item, "linux-x64-gnu", 42).unwrap().is_ok());
        assert!(check(&item, "any", 42).is_none());
        assert!(check(&item, "linux-x64-gnu", 43).is_none());
        item.yanked = true;
        assert!(check(&item, "linux-x64-gnu", 42).unwrap().is_err());
        item.yanked = false;
        item.key_revoked = true;
        assert!(check(&item, "linux-x64-gnu", 42).unwrap().is_err());
        item.key_revoked = false;
        item.publisher_approved = false;
        assert!(check(&item, "linux-x64-gnu", 42).unwrap().is_err());
        item.publisher_approved = true;
        item.advisory = Some(cloud_client::CatalogAdvisory {
            severity: "high".into(),
            message: None,
            url: None,
        });
        assert!(check(&item, "linux-x64-gnu", 42).unwrap().is_err());
    }

    #[tokio::test]
    async fn current_cloud_status_rejects_live_yank_without_generation_change() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let yanked = Arc::new(AtomicBool::new(false));
        let state = Arc::clone(&yanked);
        let server = tokio::spawn(async move {
            for _ in 0..4 {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = [0u8; 4096];
                let read = socket.read(&mut request).await.unwrap();
                assert!(String::from_utf8_lossy(&request[..read])
                    .contains("/api/v2/catalog/plugins/acme.plugin/version-pages"));
                let body = serde_json::json!({
                    "generation": "1", "id": "acme.plugin",
                    "metadataPath": "/api/v1/registry/generations/1/files/index/ac/me/acme.plugin.json",
                    "items": [{"version": "1.2.3", "target": "linux-x64-gnu", "sha256": "a".repeat(64), "size": "42", "yanked": state.load(Ordering::Acquire), "keyRevoked": false, "publisherApproved": true, "advisory": null}],
                    "nextCursor": null
                }).to_string();
                let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body);
                socket.write_all(response.as_bytes()).await.unwrap();
            }
        });
        let catalog = cloud_client::CloudCatalog::new(&origin).unwrap();
        let digest = "a".repeat(64);
        assert!(verify_catalog_release(
            &catalog,
            "acme.plugin",
            "1.2.3",
            "linux-x64-gnu",
            &digest,
            42,
            1
        )
        .await
        .is_ok());
        yanked.store(true, Ordering::Release);
        assert!(verify_catalog_release(
            &catalog,
            "acme.plugin",
            "1.2.3",
            "linux-x64-gnu",
            &digest,
            42,
            1
        )
        .await
        .is_err());
        yanked.store(false, Ordering::Release);
        let listed = load_catalog_versions(&catalog, "acme.plugin", "linux-x64-gnu", 1)
            .await
            .unwrap();
        assert!(!listed[&("1.2.3".into(), "linux-x64-gnu".into())].yanked);
        yanked.store(true, Ordering::Release);
        let listed = load_catalog_versions(&catalog, "acme.plugin", "linux-x64-gnu", 1)
            .await
            .unwrap();
        assert!(listed[&("1.2.3".into(), "linux-x64-gnu".into())].yanked);
        server.await.unwrap();
    }
}
