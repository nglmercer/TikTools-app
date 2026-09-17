use std::{collections::BTreeMap, sync::Arc};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tiktools_core::AppCore;

use crate::{error::ApiError, router::ControlRouter};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AppStateGetParams {
    #[serde(default)]
    pub keys: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AppStateSetParams {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AppStateResult {
    pub state: BTreeMap<String, String>,
}

pub fn register(router: &mut ControlRouter) {
    router.register_typed::<AppStateGetParams, AppStateResult, _, _>(
        "app.state.get",
        "Reads app state (all keys, or a filtered subset)",
        false,
        |core: Arc<AppCore>, params: AppStateGetParams| async move {
            core.app_state_get(params.keys.as_deref())
                .map(|state| AppStateResult { state })
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<AppStateSetParams, AppStateResult, _, _>(
        "app.state.set",
        "Writes one app state key",
        true,
        |core: Arc<AppCore>, params: AppStateSetParams| async move {
            core.app_state_set(&params.key, &params.value)
                .map(|state| AppStateResult { state })
                .map_err(ApiError::from)
        },
    );
}
