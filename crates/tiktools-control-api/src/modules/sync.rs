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
    SyncRecoveryChoice, SyncRecoveryOutcome, SyncRecoveryReview, WorkspaceV2Discovery,
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

fn map_error(error: OperationError) -> ApiError {
    ApiError::from(error)
}

pub fn register(router: &mut ControlRouter) {
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
