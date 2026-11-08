use std::sync::Arc;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tiktools_plugin_installer::{
    check_updates, CommitOptions, Consent, InstallError, InstallErrorCode, InstallOutcome,
    InstallReceipt, InstalledVersions, JobHandle, JobKind, JobReporter, JobSnapshot, JobStage,
    PackageOrigin, PackagePreview, PackageSpec, PackageSpecInput, PreparedPackage, UpdatePolicy,
    UpdateReport,
};
use tiktools_plugin_registry::{
    Advisory, AutoUpdate, PackageDocument, RegistryDefaults, RegistrySource, RegistryStatus,
    SearchQuery, SearchResults,
};

use super::OperationError;
use crate::{
    events::DomainEvent,
    plugin_packages::{CoreActivation, PluginPackages},
    AppCore,
};

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
}

enum InstallConsent {
    Reviewed(String),
    Automatic,
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
        let spec = PackageSpec::try_from(input).map_err(OperationError::install)?;
        let label = spec.to_string();
        let reporter = tiktools_plugin_installer::NoopReporter;
        let prepared = self
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
        let spec = PackageSpec::try_from(input).map_err(OperationError::install)?;
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
                installer.rollback(
                    &plugin_id,
                    version.as_deref(),
                    &CoreActivation(&worker_core),
                    worker_job.as_ref(),
                )
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
        let lookup = packages
            .registry()
            .package(plugin_id, registry_name.as_deref())
            .await;
        let (registry, available, registry_error) = match lookup {
            Ok((source, document)) => {
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
        let receipt = packages
            .installer
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
        packages
            .installer
            .set_policy(plugin_id, policy)
            .map_err(OperationError::install)
    }

    pub async fn plugin_updates_check(self: &Arc<Self>) -> Result<UpdateReport, OperationError> {
        let packages = self.packages()?;
        let receipts = packages
            .installer
            .receipts()
            .list()
            .map_err(OperationError::install)?;
        let report = check_updates(&receipts, packages.registry(), packages.installer.host()).await;
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
        Ok(self.packages()?.registry().search(&query).await)
    }

    pub async fn registry_package(
        &self,
        id: &str,
        registry: Option<&str>,
    ) -> Result<RegistryPackageView, OperationError> {
        let (source, document) = self
            .packages()?
            .registry()
            .package(id, registry)
            .await
            .map_err(registry_error)?;
        Ok(RegistryPackageView {
            registry: source.name,
            document,
        })
    }

    pub fn registry_defaults(&self) -> Result<RegistryDefaults, OperationError> {
        Ok(self.packages()?.registry().defaults())
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
        let reviewed = match &consent {
            InstallConsent::Reviewed(token) => packages.take_review(token, &label),
            InstallConsent::Automatic => None,
        };
        let prepared = match reviewed {
            Some(prepared) => prepared,
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
        let installer = packages.installer.clone();
        let core = Arc::clone(self);
        let worker_job = Arc::clone(job);
        tokio::task::spawn_blocking(move || {
            let _invocation = core.plugin_state.invocation_gate.lock().map_err(|_| {
                crate::plugin_packages::unavailable("plugin invocation lease is unavailable")
            })?;
            installer.commit(
                &prepared,
                &CommitOptions {
                    consent: Consent::Token(token),
                    policy,
                },
                &CoreActivation(&core),
                worker_job.as_ref(),
            )
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
    ) -> Result<PreparedPackage, InstallError> {
        let channel = match spec.plugin_id() {
            Some(id) => packages
                .installer
                .receipt(id)?
                .map(|receipt| receipt.policy.channel)
                .unwrap_or_else(|| packages.registry().defaults().channel),
            None => packages.registry().defaults().channel,
        };
        if !matches!(spec, PackageSpec::Registry { .. })
            && !packages.registry().defaults().allow_sideload
        {
            return Err(InstallError::new(
                InstallErrorCode::Unsupported,
                "installing plugins from files and URLs is disabled in the plugin source settings",
            ));
        }
        let resolved = packages
            .sources
            .fetch(spec, &channel, packages.installer.host(), reporter, cancel)
            .await?;
        reporter.stage(JobStage::Verify);
        let installer = packages.installer.clone();
        tokio::task::spawn_blocking(move || {
            let prepared = installer.prepare(&resolved.path, resolved.origin, resolved.policy)?;
            if let Some(expected) = &resolved.expected {
                expected.check(&prepared)?;
            }
            Ok(prepared)
        })
        .await
        .unwrap_or_else(|error| Err(InstallError::io(error)))
    }
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
