use std::sync::Arc;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tiktools_core::AppCore;
pub use tiktools_core::{
    control::{PluginVersions, PolicyPatch},
    plugin_installer::{
        InstallErrorCode, InstallReceipt, JobSnapshot, PackagePreview, PackageSpecInput,
        UpdatePolicy, UpdateReport,
    },
};

use crate::{error::ApiError, modules::Empty, router::ControlRouter};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PackageInspectParams {
    pub spec: PackageSpecInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PackageInstallParams {
    pub spec: PackageSpecInput,
    pub consent_token: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy: Option<UpdatePolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PackageUpdateParams {
    pub plugin_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consent_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PackageRollbackParams {
    pub plugin_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PackagePluginParams {
    pub plugin_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PackagePolicyParams {
    pub plugin_id: String,
    #[serde(flatten)]
    pub patch: PolicyPatch,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobParams {
    pub job_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobResult {
    pub job: JobSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobListResult {
    pub jobs: Vec<JobSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobCancelResult {
    pub cancelled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptListResult {
    pub receipts: Vec<InstallReceipt>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptResult {
    pub receipt: InstallReceipt,
}

pub fn register(router: &mut ControlRouter) {
    router.register_typed::<PackageInspectParams, PackagePreview, _, _>(
        "plugins.packages.inspect",
        "Resolves, downloads, and verifies a package spec and returns its review (permissions, signature, consent token)",
        false,
        |core: Arc<AppCore>, params: PackageInspectParams| async move {
            core.plugin_package_inspect(params.spec)
                .await
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<PackageInstallParams, JobResult, _, _>(
        "plugins.packages.install",
        "Starts an install job for a reviewed package; the consent token comes from plugins.packages.inspect",
        true,
        |core: Arc<AppCore>, params: PackageInstallParams| async move {
            core.plugin_package_install(params.spec, params.consent_token, params.policy)
                .map(|job| JobResult { job })
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<PackageUpdateParams, JobResult, _, _>(
        "plugins.packages.update",
        "Starts an update job for a registry plugin; omitting the consent token only applies updates that add no access",
        true,
        |core: Arc<AppCore>, params: PackageUpdateParams| async move {
            core.plugin_package_update(&params.plugin_id, params.version, params.consent_token)
                .map(|job| JobResult { job })
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<PackageRollbackParams, JobResult, _, _>(
        "plugins.packages.rollback",
        "Restores a retained previous version of a plugin and pins it",
        true,
        |core: Arc<AppCore>, params: PackageRollbackParams| async move {
            core.plugin_package_rollback(&params.plugin_id, params.version)
                .map(|job| JobResult { job })
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<PackagePluginParams, PluginVersions, _, _>(
        "plugins.packages.versions",
        "Installed, retained, and registry-available versions of one plugin",
        false,
        |core: Arc<AppCore>, params: PackagePluginParams| async move {
            core.plugin_package_versions(&params.plugin_id)
                .await
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<Empty, ReceiptListResult, _, _>(
        "plugins.packages.receipts",
        "Install receipts (origin, signature, granted access, update policy) for managed plugins",
        false,
        |core: Arc<AppCore>, _params: Empty| async move {
            crate::modules::blocking_task("plugins.packages.receipts", move || {
                core.plugin_package_receipts()
            })
            .await?
            .map(|receipts| ReceiptListResult { receipts })
            .map_err(ApiError::from)
        },
    );
    router.register_typed::<PackagePolicyParams, ReceiptResult, _, _>(
        "plugins.packages.policy",
        "Updates a plugin's update channel, auto-update rule, or pin",
        true,
        |core: Arc<AppCore>, params: PackagePolicyParams| async move {
            core.plugin_package_policy(&params.plugin_id, params.patch)
                .map(|receipt| ReceiptResult { receipt })
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<Empty, UpdateReport, _, _>(
        "plugins.updates.check",
        "Checks registries for plugin updates and advisories, applying permitted automatic updates",
        false,
        |core: Arc<AppCore>, _params: Empty| async move {
            core.plugin_updates_check().await.map_err(ApiError::from)
        },
    );
    router.register_typed::<Empty, UpdateReport, _, _>(
        "plugins.updates.list",
        "The most recent plugin update report",
        false,
        |core: Arc<AppCore>, _params: Empty| async move {
            core.plugin_updates().map_err(ApiError::from)
        },
    );
    router.register_typed::<Empty, JobListResult, _, _>(
        "plugins.jobs.list",
        "Recent plugin package jobs, newest first",
        false,
        |core: Arc<AppCore>, _params: Empty| async move {
            core.plugin_jobs()
                .map(|jobs| JobListResult { jobs })
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<JobParams, JobResult, _, _>(
        "plugins.jobs.get",
        "One plugin package job",
        false,
        |core: Arc<AppCore>, params: JobParams| async move {
            core.plugin_job(&params.job_id)
                .map(|job| JobResult { job })
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<JobParams, JobCancelResult, _, _>(
        "plugins.jobs.cancel",
        "Requests cancellation of a running plugin package job",
        true,
        |core: Arc<AppCore>, params: JobParams| async move {
            core.plugin_job_cancel(&params.job_id)
                .map(|cancelled| JobCancelResult { cancelled })
                .map_err(ApiError::from)
        },
    );
}
