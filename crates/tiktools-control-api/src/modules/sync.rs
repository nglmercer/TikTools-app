//! Cloud plugin-selection sync exposed through the shared control plane.
//!
//! Everything here is nonsecret: the opt-in flag, pending/conflict counts,
//! and error text. Applying remote records never installs plugins; unknown
//! plugins are skipped and only the enabled flag of installed ones changes.

use std::sync::Arc;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
pub use tiktools_core::control::SyncNowResult;
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

fn map_error(error: OperationError) -> ApiError {
    ApiError::from(error)
}

pub fn register(router: &mut ControlRouter) {
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
        "Pulls and pushes plugin selections, applying remote changes to installed plugins",
        true,
        |core: Arc<AppCore>, _params: Empty| async move {
            core.sync_now().await.map_err(map_error)
        },
    );
    router.register_typed::<SyncResolveParams, SyncStatusResult, _, _>(
        "sync.resolve",
        "Resolves one conflicted selection, keeping the local edit or taking the remote record",
        true,
        |core: Arc<AppCore>, params: SyncResolveParams| async move {
            core.sync_resolve_selection(&params.key, params.keep_local)
                .await
                .map(|status| SyncStatusResult { status })
                .map_err(map_error)
        },
    );
}
