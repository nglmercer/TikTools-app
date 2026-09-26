//! Typed `globals.*` methods: runtime global values for automations.

use std::{collections::BTreeMap, sync::Arc};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tiktools_core::AppCore;

use crate::{error::ApiError, modules::Empty, router::ControlRouter};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GlobalsGetParams {
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GlobalsSetParams {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GlobalsDeleteParams {
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GlobalsListResult {
    pub globals: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GlobalsValueResult {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GlobalsDeleteResult {
    pub key: String,
    pub deleted: bool,
}

pub fn register(router: &mut ControlRouter) {
    router.register_typed::<Empty, GlobalsListResult, _, _>(
        "globals.list",
        "Lists every runtime global (bare key to text value)",
        false,
        |core: Arc<AppCore>, _params: Empty| async move {
            crate::modules::blocking_task("globals.list", move || core.globals_list())
                .await?
                .map(|globals| GlobalsListResult { globals })
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<GlobalsGetParams, GlobalsValueResult, _, _>(
        "globals.get",
        "Reads one runtime global by bare key",
        false,
        |core: Arc<AppCore>, params: GlobalsGetParams| async move {
            crate::modules::blocking_task("globals.get", move || {
                core.globals_get(&params.key)
                    .map(|value| (params.key, value))
            })
            .await?
            .map(|(key, value)| GlobalsValueResult { key, value })
            .map_err(ApiError::from)
        },
    );
    router.register_typed::<GlobalsSetParams, GlobalsValueResult, _, _>(
        "globals.set",
        "Creates or replaces one runtime global",
        true,
        |core: Arc<AppCore>, params: GlobalsSetParams| async move {
            crate::modules::blocking_task("globals.set", move || {
                core.globals_set(&params.key, &params.value)
            })
            .await?
            .map(|(key, value)| GlobalsValueResult { key, value })
            .map_err(ApiError::from)
        },
    );
    router.register_typed::<GlobalsDeleteParams, GlobalsDeleteResult, _, _>(
        "globals.delete",
        "Deletes one runtime global",
        true,
        |core: Arc<AppCore>, params: GlobalsDeleteParams| async move {
            crate::modules::blocking_task("globals.delete", move || {
                core.globals_delete(&params.key)
                    .map(|deleted| (params.key, deleted))
            })
            .await?
            .map(|(key, deleted)| GlobalsDeleteResult { key, deleted })
            .map_err(ApiError::from)
        },
    );
}
