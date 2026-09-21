use std::sync::Arc;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
pub use tiktools_core::{
    ipc::messages::{PartialPointsConfig, PointsConfig},
    services::PointAward,
};
use tiktools_core::AppCore;

use crate::{
    error::ApiError,
    modules::{Empty, OkResult},
    router::ControlRouter,
};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PointsViewerParams {
    pub unique_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PointsAdjustParams {
    pub unique_id: String,
    pub delta: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PointsLeaderboardParams {
    #[serde(default)]
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PointsLeaderboardResult {
    pub viewers: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PointsViewerResult {
    pub viewer: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PointsResetParams {
    #[serde(default)]
    pub unique_id: Option<String>,
}

pub fn register(router: &mut ControlRouter) {
    router.register_typed::<Empty, PointsConfig, _, _>(
        "points.config.get",
        "Current points configuration",
        false,
        |core: Arc<AppCore>, _params: Empty| async move {
            Ok::<PointsConfig, ApiError>(core.points_config())
        },
    );
    // Params ARE the partial config object, e.g. {"pointsPerChat": 2}.
    router.register_typed::<PartialPointsConfig, PointsConfig, _, _>(
        "points.config.set",
        "Updates points configuration (partial object)",
        true,
        |core: Arc<AppCore>, params: PartialPointsConfig| async move {
            // Synchronous SQLite write: off the Tokio workers.
            crate::modules::blocking_task("points.config.set", move || {
                core.points_update_config(params)
            })
            .await
        },
    );
    router.register_typed::<PointsViewerParams, PointsViewerResult, _, _>(
        "points.viewer.get",
        "One viewer points record by uniqueId",
        false,
        |core: Arc<AppCore>, params: PointsViewerParams| async move {
            core.points_viewer(&params.unique_id)
                .map(|viewer| PointsViewerResult { viewer })
                .map_err(|error| ApiError::from(error).scoped_not_found("viewer_not_found"))
        },
    );
    router.register_typed::<PointsAdjustParams, PointAward, _, _>(
        "points.adjust",
        "Manual points adjustment (creates the viewer record if missing)",
        true,
        |core: Arc<AppCore>, params: PointsAdjustParams| async move {
            // Synchronous SQLite write: off the Tokio workers.
            crate::modules::blocking_task("points.adjust", move || {
                core.points_adjust(&params.unique_id, params.delta)
            })
            .await?
            .map_err(ApiError::from)
        },
    );
    router.register_typed::<PointsLeaderboardParams, PointsLeaderboardResult, _, _>(
        "points.leaderboard",
        "Points leaderboard, highest first",
        false,
        |core: Arc<AppCore>, params: PointsLeaderboardParams| async move {
            Ok::<PointsLeaderboardResult, ApiError>(PointsLeaderboardResult {
                viewers: core.points_leaderboard(params.limit),
            })
        },
    );
    router.register_typed::<PointsResetParams, OkResult, _, _>(
        "points.reset",
        "Resets points for one viewer, or all viewers when uniqueId is omitted",
        true,
        |core: Arc<AppCore>, params: PointsResetParams| async move {
            // Synchronous SQLite write: off the Tokio workers.
            crate::modules::blocking_task("points.reset", move || {
                core.points_reset(params.unique_id.as_deref());
            })
            .await?;
            Ok::<OkResult, ApiError>(OkResult::ok())
        },
    );
}
