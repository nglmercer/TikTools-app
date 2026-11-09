use super::TikToolsClient;
use tiktools_control_api::modules::sync::{
    CloudLibraryDocument, CloudLibraryPage, CloudLibraryQuery, CloudLibraryRead, SyncNowResult,
    SyncRecoveryOutcome, SyncRecoveryReview, SyncResolveParams, SyncStatusResult,
    SyncUncertainResolveParams, SyncUncertainReviewParams, WorkspaceV2Discovery,
    WorkspaceV2SyncResult,
};
use tiktools_control_api::modules::Empty;
use tiktools_control_api::ClientError;

pub(crate) const METHODS: &[&str] = &[
    SYNC_STATUS,
    SYNC_ENABLE,
    SYNC_DISABLE,
    SYNC_NOW,
    SYNC_RESOLVE,
    "sync.uncertain.list",
    "sync.uncertain.review",
    "sync.uncertain.resolve",
    "sync.v2.discover",
    "sync.v2.now",
    "cloudLibrary.list",
    "cloudLibrary.read",
];

const SYNC_STATUS: &str = "sync.status";
const SYNC_ENABLE: &str = "sync.enable";
const SYNC_DISABLE: &str = "sync.disable";
const SYNC_NOW: &str = "sync.now";
const SYNC_RESOLVE: &str = "sync.resolve";

impl TikToolsClient {
    pub async fn cloud_library_list(
        &self,
        query: CloudLibraryQuery,
    ) -> Result<CloudLibraryPage, ClientError> {
        self.call("cloudLibrary.list", query).await
    }

    pub async fn cloud_library_read(
        &self,
        request: CloudLibraryRead,
    ) -> Result<CloudLibraryDocument, ClientError> {
        self.call("cloudLibrary.read", request).await
    }

    pub async fn sync_v2_now(&self) -> Result<WorkspaceV2SyncResult, ClientError> {
        self.call("sync.v2.now", Empty::default()).await
    }

    pub async fn sync_v2_discover(&self) -> Result<WorkspaceV2Discovery, ClientError> {
        self.call("sync.v2.discover", Empty::default()).await
    }

    pub async fn sync_uncertain_list(&self) -> Result<Vec<String>, ClientError> {
        self.call("sync.uncertain.list", Empty::default()).await
    }

    pub async fn sync_uncertain_review(
        &self,
        params: SyncUncertainReviewParams,
    ) -> Result<SyncRecoveryReview, ClientError> {
        self.call("sync.uncertain.review", params).await
    }

    pub async fn sync_uncertain_resolve(
        &self,
        params: SyncUncertainResolveParams,
    ) -> Result<SyncRecoveryOutcome, ClientError> {
        self.call("sync.uncertain.resolve", params).await
    }
    pub async fn sync_status(&self) -> Result<SyncStatusResult, ClientError> {
        self.call(SYNC_STATUS, Empty::default()).await
    }

    pub async fn sync_enable(&self) -> Result<SyncStatusResult, ClientError> {
        self.call(SYNC_ENABLE, Empty::default()).await
    }

    pub async fn sync_disable(&self) -> Result<SyncStatusResult, ClientError> {
        self.call(SYNC_DISABLE, Empty::default()).await
    }

    pub async fn sync_now(&self) -> Result<SyncNowResult, ClientError> {
        self.call(SYNC_NOW, Empty::default()).await
    }

    pub async fn sync_resolve(
        &self,
        params: SyncResolveParams,
    ) -> Result<SyncStatusResult, ClientError> {
        self.call(SYNC_RESOLVE, params).await
    }
}
