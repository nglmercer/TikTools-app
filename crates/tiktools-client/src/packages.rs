use super::TikToolsClient;
use tiktools_control_api::modules::packages::{
    JobCancelResult, JobListResult, JobParams, JobResult, PackageInspectParams,
    PackageInstallParams, PackagePluginParams, PackagePolicyParams, PackagePreview,
    PackageRollbackParams, PackageUpdateParams, PluginVersions, ReceiptListResult, ReceiptResult,
    UpdateReport,
};
use tiktools_control_api::modules::Empty;
use tiktools_control_api::ClientError;

pub(crate) const METHODS: &[&str] = &[
    PACKAGES_INSPECT,
    PACKAGES_INSTALL,
    PACKAGES_UPDATE,
    PACKAGES_ROLLBACK,
    PACKAGES_VERSIONS,
    PACKAGES_RECEIPTS,
    PACKAGES_POLICY,
    UPDATES_CHECK,
    UPDATES_LIST,
    JOBS_LIST,
    JOBS_GET,
    JOBS_CANCEL,
];

const PACKAGES_INSPECT: &str = "plugins.packages.inspect";
const PACKAGES_INSTALL: &str = "plugins.packages.install";
const PACKAGES_UPDATE: &str = "plugins.packages.update";
const PACKAGES_ROLLBACK: &str = "plugins.packages.rollback";
const PACKAGES_VERSIONS: &str = "plugins.packages.versions";
const PACKAGES_RECEIPTS: &str = "plugins.packages.receipts";
const PACKAGES_POLICY: &str = "plugins.packages.policy";
const UPDATES_CHECK: &str = "plugins.updates.check";
const UPDATES_LIST: &str = "plugins.updates.list";
const JOBS_LIST: &str = "plugins.jobs.list";
const JOBS_GET: &str = "plugins.jobs.get";
const JOBS_CANCEL: &str = "plugins.jobs.cancel";

impl TikToolsClient {
    pub async fn plugins_packages_inspect(
        &self,
        params: PackageInspectParams,
    ) -> Result<PackagePreview, ClientError> {
        self.call(PACKAGES_INSPECT, params).await
    }

    pub async fn plugins_packages_install(
        &self,
        params: PackageInstallParams,
    ) -> Result<JobResult, ClientError> {
        self.call(PACKAGES_INSTALL, params).await
    }

    pub async fn plugins_packages_update(
        &self,
        params: PackageUpdateParams,
    ) -> Result<JobResult, ClientError> {
        self.call(PACKAGES_UPDATE, params).await
    }

    pub async fn plugins_packages_rollback(
        &self,
        params: PackageRollbackParams,
    ) -> Result<JobResult, ClientError> {
        self.call(PACKAGES_ROLLBACK, params).await
    }

    pub async fn plugins_packages_versions(
        &self,
        params: PackagePluginParams,
    ) -> Result<PluginVersions, ClientError> {
        self.call(PACKAGES_VERSIONS, params).await
    }

    pub async fn plugins_packages_receipts(&self) -> Result<ReceiptListResult, ClientError> {
        self.call(PACKAGES_RECEIPTS, Empty::default()).await
    }

    pub async fn plugins_packages_policy(
        &self,
        params: PackagePolicyParams,
    ) -> Result<ReceiptResult, ClientError> {
        self.call(PACKAGES_POLICY, params).await
    }

    pub async fn plugins_updates_check(&self) -> Result<UpdateReport, ClientError> {
        self.call(UPDATES_CHECK, Empty::default()).await
    }

    pub async fn plugins_updates_list(&self) -> Result<UpdateReport, ClientError> {
        self.call(UPDATES_LIST, Empty::default()).await
    }

    pub async fn plugins_jobs_list(&self) -> Result<JobListResult, ClientError> {
        self.call(JOBS_LIST, Empty::default()).await
    }

    pub async fn plugins_jobs_get(&self, params: JobParams) -> Result<JobResult, ClientError> {
        self.call(JOBS_GET, params).await
    }

    pub async fn plugins_jobs_cancel(
        &self,
        params: JobParams,
    ) -> Result<JobCancelResult, ClientError> {
        self.call(JOBS_CANCEL, params).await
    }
}
