use std::sync::Arc;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tiktools_core::AppCore;

use crate::{
    error::ApiError,
    modules::{Empty, OkResult},
    router::ControlRouter,
};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreatorGetParams {
    #[serde(default)]
    pub unique_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreatorGetResult {
    pub creator: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreatorsRecentParams {
    #[serde(default)]
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreatorsRecentResult {
    pub creators: Vec<Value>,
}

pub fn register(router: &mut ControlRouter) {
    router.register_typed::<CreatorGetParams, CreatorGetResult, _, _>(
        "creators.get",
        "Creator record by uniqueId (defaults to the last connected creator)",
        false,
        |core: Arc<AppCore>, params: CreatorGetParams| async move {
            Ok::<CreatorGetResult, ApiError>(CreatorGetResult {
                creator: core.creator_get(params.unique_id.as_deref()),
            })
        },
    );
    router.register_typed::<CreatorsRecentParams, CreatorsRecentResult, _, _>(
        "creators.recent",
        "Recently connected creators, most recent first",
        false,
        |core: Arc<AppCore>, params: CreatorsRecentParams| async move {
            Ok::<CreatorsRecentResult, ApiError>(CreatorsRecentResult {
                creators: core.creator_recent(params.limit),
            })
        },
    );
    router.register_typed::<Empty, OkResult, _, _>(
        "creators.history.clear",
        "Clears creator history and the last-creator pointer",
        true,
        |core: Arc<AppCore>, _params: Empty| async move {
            core.creator_history_clear();
            Ok::<OkResult, ApiError>(OkResult::ok())
        },
    );
}
