//! Cloud plugin-selection sync exposed through the shared control plane.
//!
//! Everything here is nonsecret: the opt-in flag, pending/conflict counts,
//! and error text. Remote records stay in the durable Cloud snapshot; local
//! plugin application and conflict resolution wait for a reviewed recovery path.

use std::sync::Arc;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
pub use tiktools_core::control::{
    CloudLibraryDocument, CloudLibraryDocumentMeta, CloudLibraryKind, CloudLibraryPage,
    CloudLibraryQuery, CloudLibraryRead, CloudLibraryScope, CloudLibraryStatus, SyncNowResult,
    SyncRecoveryChoice, SyncRecoveryOutcome, SyncRecoveryReview, SyncSection,
    SyncV2StageVariablesParams, SyncV2StageVariablesResult, V2ChangeEntry, V2ConflictChoice,
    V2ConflictOutcome, V2ConflictReview, V2ConflictTarget, V2PendingChange,
    WorkspaceInventoryRepairParams, WorkspaceInventoryReview, WorkspaceV2Discovery,
    WorkspaceV2Migration, WorkspaceV2Preview, WorkspaceV2SyncResult,
};
use tiktools_core::{cloud_sync::SyncStatus, control::OperationError, AppCore};

use super::Empty;
use crate::ApiError;
use crate::ControlRouter;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatusResult {
    #[serde(flatten)]
    pub status: SyncStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SyncResolveParams {
    pub key: String,
    #[serde(default)]
    pub keep_local: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyncUncertainReviewParams {
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyncUncertainResolveParams {
    pub handle: String,
    pub choice: SyncRecoveryChoice,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyncV2ConflictReviewParams {
    pub collection: String,
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyncV2ConflictResolveParams {
    pub handle: String,
    pub choice: V2ConflictChoice,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyncV2StagePluginSettingsParams {
    pub plugin_id: String,
    pub settings: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyncSectionParams {
    pub section: SyncSection,
}

fn map_error(error: OperationError) -> ApiError {
    ApiError::from(error)
}

pub fn register(router: &mut ControlRouter) {
    router.register_typed::<Empty, WorkspaceInventoryReview, _, _>(
        "sync.v2.inventory.review",
        "Reviews absent local packages without changing sync queues",
        false,
        |core: Arc<AppCore>, _| async move {
            core.workspace_inventory_review(None)
                .await
                .map_err(map_error)
        },
    );
    router.register_typed::<WorkspaceInventoryRepairParams, WorkspaceInventoryReview, _, _>(
        "sync.v2.inventory.repair", "Archives and clears only reviewed absent local install flags; keeps Cloud queues and plugin data", true,
        |core: Arc<AppCore>, params| async move { core.workspace_inventory_review(Some(params.review)).await.map_err(map_error) },
    );
    router.register_typed::<Empty, WorkspaceV2Preview, _, _>(
        "sync.v2.preview",
        "Reviews local migration inputs without changing or enabling synchronization",
        false,
        |core: Arc<AppCore>, _| async move { core.workspace_v2_preview().await.map_err(map_error) },
    );
    router.register_typed::<Empty, WorkspaceV2Migration, _, _>(
        "sync.v2.migrate",
        "Migrates this account from v1 to v2 and activates the server write-epoch policy",
        true,
        |core: Arc<AppCore>, _| async move { core.workspace_v2_migrate().await.map_err(map_error) },
    );
    router.register_typed::<Empty, bool, _, _>(
        "sync.v2.selected",
        "Reads the selected Cloud protocol without contacting the server",
        false,
        |core: Arc<AppCore>, _| async move { core.workspace_v2_selected().await.map_err(map_error) },
    );
    router.register_typed::<CloudLibraryQuery, CloudLibraryPage, _, _>(
        "cloudLibrary.list",
        "Lists published or private Cloud documents without importing them",
        false,
        |core: Arc<AppCore>, query: CloudLibraryQuery| async move {
            core.cloud_library_list(query).await.map_err(map_error)
        },
    );
    router.register_typed::<CloudLibraryRead, CloudLibraryDocument, _, _>(
        "cloudLibrary.read",
        "Reads a Cloud document revision without importing it",
        false,
        |core: Arc<AppCore>, request: CloudLibraryRead| async move {
            core.cloud_library_read(request).await.map_err(map_error)
        },
    );
    router.register_typed::<Empty, WorkspaceV2Discovery, _, _>(
        "sync.v2.discover",
        "Checks Cloud v2 compatibility without enabling synchronization",
        false,
        |core: Arc<AppCore>, _| async move { core.workspace_v2_discover().await.map_err(map_error) },
    );
    router.register_typed::<Empty, Vec<String>, _, _>(
        "sync.uncertain.list",
        "Lists selections with an uncertain upload outcome",
        false,
        |core: Arc<AppCore>, _| async move { core.sync_uncertain_keys().await.map_err(map_error) },
    );
    router.register_typed::<SyncUncertainReviewParams, SyncRecoveryReview, _, _>(
        "sync.uncertain.review",
        "Reads the current Cloud value for an uncertain upload",
        false,
        |core: Arc<AppCore>, params: SyncUncertainReviewParams| async move {
            core.sync_review_uncertain(&params.key)
                .await
                .map_err(map_error)
        },
    );
    router.register_typed::<SyncUncertainResolveParams, SyncRecoveryOutcome, _, _>(
        "sync.uncertain.resolve",
        "Resolves a reviewed uncertain upload",
        true,
        |core: Arc<AppCore>, params: SyncUncertainResolveParams| async move {
            core.sync_resolve_uncertain(&params.handle, params.choice)
                .await
                .map_err(map_error)
        },
    );
    router.register_typed::<Empty, SyncStatusResult, _, _>(
        "sync.status",
        "Shows the Cloud sync state without exposing any credentials",
        false,
        |core: Arc<AppCore>, _params: Empty| async move {
            core.sync_status()
                .await
                .map(|status| SyncStatusResult { status })
                .map_err(map_error)
        },
    );
    router.register_typed::<Empty, SyncStatusResult, _, _>(
        "sync.enable",
        "Opts this device into Cloud plugin-selection sync",
        true,
        |core: Arc<AppCore>, _params: Empty| async move {
            core.sync_set_enabled(true)
                .map(|status| SyncStatusResult { status })
                .map_err(map_error)
        },
    );
    router.register_typed::<Empty, SyncStatusResult, _, _>(
        "sync.disable",
        "Opts this device out of Cloud plugin-selection sync",
        true,
        |core: Arc<AppCore>, _params: Empty| async move {
            core.sync_set_enabled(false)
                .map(|status| SyncStatusResult { status })
                .map_err(map_error)
        },
    );
    router.register_typed::<Empty, WorkspaceV2SyncResult, _, _>(
        "sync.v2.now",
        "Synchronize an already selected v2 workspace without applying Cloud records",
        true,
        |core: Arc<AppCore>, _| async move { core.sync_v2_now().await.map_err(map_error) },
    );
    router.register_typed::<SyncSectionParams, WorkspaceV2SyncResult, _, _>(
        "sync.v2.section",
        "Publishes only the requested section and returns its application candidates; other downloaded records remain cached",
        true,
        |core: Arc<AppCore>, params| async move {
            core.sync_v2_section(params.section).await.map_err(map_error)
        },
    );
    router.register_typed::<Empty, (), _, _>(
        "sync.v2.rebuild",
        "Rebuilds the fetched Cloud baseline after cursor expiry",
        true,
        |core: Arc<AppCore>, _| async move { core.sync_v2_rebuild().await.map_err(map_error) },
    );
    router.register_typed::<serde_json::Value, (), _, _>(
        "sync.v2.stagePreferences",
        "Stages desktop preferences for v2 sync",
        true,
        |core: Arc<AppCore>, native: serde_json::Value| async move {
            core.sync_v2_stage_preferences(native)
                .await
                .map_err(map_error)
        },
    );
    router.register_typed::<SyncV2StagePluginSettingsParams, (), _, _>(
        "sync.v2.stagePluginSettings",
        "Stages plugin settings for v2 sync",
        true,
        |core: Arc<AppCore>, params: SyncV2StagePluginSettingsParams| async move {
            core.sync_v2_stage_plugin_settings(&params.plugin_id, params.settings)
                .await
                .map_err(map_error)
        },
    );
    router.register_typed::<SyncV2StageVariablesParams, SyncV2StageVariablesResult, _, _>(
        "sync.v2.stageVariables",
        "Stages runtime variables for v2 sync; secrets are excluded and never leave the host; needs the Cloud variables collection",
        true,
        |core: Arc<AppCore>, params: SyncV2StageVariablesParams| async move {
            core.sync_v2_stage_variables(params).await.map_err(map_error)
        },
    );
    router.register_typed::<Empty, Vec<V2ConflictTarget>, _, _>(
        "sync.v2.conflict.list",
        "Lists v2 conflicts for the current account without contacting Cloud",
        false,
        |core: Arc<AppCore>, _| async move { core.sync_v2_conflict_list().await.map_err(map_error) },
    );
    router.register_typed::<Empty, Vec<V2ChangeEntry>, _, _>(
        "sync.v2.changes.list",
        "Reads the durable v2 workspace (cached Cloud values and pending edits) without contacting Cloud",
        false,
        |core: Arc<AppCore>, _| async move { core.sync_v2_changes_list().await.map_err(map_error) },
    );
    router.register_typed::<SyncV2ConflictReviewParams, V2ConflictReview, _, _>(
        "sync.v2.conflict.review",
        "Reads the Cloud comparison for one reviewed v2 conflict",
        false,
        |core: Arc<AppCore>, params: SyncV2ConflictReviewParams| async move {
            core.sync_v2_review_conflict(&params.collection, &params.key)
                .await
                .map_err(map_error)
        },
    );
    router.register_typed::<SyncV2ConflictResolveParams, V2ConflictOutcome, _, _>(
        "sync.v2.conflict.resolve",
        "Resolves a reviewed v2 conflict",
        true,
        |core: Arc<AppCore>, params: SyncV2ConflictResolveParams| async move {
            core.sync_v2_resolve_conflict(&params.handle, params.choice)
                .await
                .map_err(map_error)
        },
    );
    router.register_typed::<Empty, SyncNowResult, _, _>(
        "sync.now",
        "Pulls and pushes plugin selections without automatically applying remote changes",
        true,
        |core: Arc<AppCore>, _params: Empty| async move {
            core.sync_now().await.map_err(map_error)
        },
    );
    router.register_typed::<SyncResolveParams, SyncStatusResult, _, _>(
        "sync.resolve",
        "Conflict resolution is unavailable until local recovery is protected",
        true,
        |core: Arc<AppCore>, params: SyncResolveParams| async move {
            core.sync_resolve_selection(&params.key, params.keep_local)
                .await
                .map(|status| SyncStatusResult { status })
                .map_err(map_error)
        },
    );
}
