use super::TikToolsClient;
use tiktools_control_api::modules::sync::{
    CloudLibraryDocument, CloudLibraryPage, CloudLibraryQuery, CloudLibraryRead, SyncNowResult,
    SyncRecoveryOutcome, SyncRecoveryReview, SyncResolveParams, SyncStatusResult,
    SyncUncertainResolveParams, SyncUncertainReviewParams, WorkspaceV2Discovery,
    WorkspaceV2Preview, WorkspaceV2SyncResult,
};
use tiktools_control_api::modules::sync::{
    SyncV2ConflictResolveParams, SyncV2ConflictReviewParams, SyncV2StagePluginSettingsParams,
    V2ChangeEntry, V2ConflictOutcome, V2ConflictReview, V2ConflictTarget, WorkspaceV2Migration,
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
    "sync.v2.selected",
    "sync.v2.preview",
    "sync.v2.now",
    "sync.v2.migrate",
    "sync.v2.rebuild",
    "sync.v2.stagePreferences",
    "sync.v2.stagePluginSettings",
    "sync.v2.changes.list",
    "sync.v2.conflict.list",
    "sync.v2.conflict.review",
    "sync.v2.conflict.resolve",
    "cloudLibrary.list",
    "cloudLibrary.read",
];

const SYNC_STATUS: &str = "sync.status";
const SYNC_ENABLE: &str = "sync.enable";
const SYNC_DISABLE: &str = "sync.disable";
const SYNC_NOW: &str = "sync.now";
const SYNC_RESOLVE: &str = "sync.resolve";

impl TikToolsClient {
    pub async fn sync_v2_preview(&self) -> Result<WorkspaceV2Preview, ClientError> {
        self.call("sync.v2.preview", Empty::default()).await
    }

    pub async fn sync_v2_selected(&self) -> Result<bool, ClientError> {
        self.call("sync.v2.selected", Empty::default()).await
    }

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

impl TikToolsClient {
    pub async fn sync_v2_migrate(&self) -> Result<WorkspaceV2Migration, ClientError> {
        self.call("sync.v2.migrate", Empty::default()).await
    }

    pub async fn sync_v2_rebuild(&self) -> Result<(), ClientError> {
        self.call("sync.v2.rebuild", Empty::default()).await
    }

    pub async fn sync_v2_stage_preferences(
        &self,
        preferences: serde_json::Value,
    ) -> Result<(), ClientError> {
        self.call("sync.v2.stagePreferences", preferences).await
    }

    pub async fn sync_v2_stage_plugin_settings(
        &self,
        params: SyncV2StagePluginSettingsParams,
    ) -> Result<(), ClientError> {
        self.call("sync.v2.stagePluginSettings", params).await
    }

    pub async fn sync_v2_changes_list(&self) -> Result<Vec<V2ChangeEntry>, ClientError> {
        self.call("sync.v2.changes.list", Empty::default()).await
    }

    pub async fn sync_v2_conflict_list(&self) -> Result<Vec<V2ConflictTarget>, ClientError> {
        self.call("sync.v2.conflict.list", Empty::default()).await
    }

    pub async fn sync_v2_conflict_review(
        &self,
        params: SyncV2ConflictReviewParams,
    ) -> Result<V2ConflictReview, ClientError> {
        self.call("sync.v2.conflict.review", params).await
    }

    pub async fn sync_v2_conflict_resolve(
        &self,
        params: SyncV2ConflictResolveParams,
    ) -> Result<V2ConflictOutcome, ClientError> {
        self.call("sync.v2.conflict.resolve", params).await
    }
}
