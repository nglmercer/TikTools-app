use std::sync::Arc;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
pub use tiktools_core::control::GiftDebugResult;
use tiktools_core::AppCore;

use crate::{error::ApiError, modules::Empty, router::ControlRouter};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GiftDebugParams {
    #[serde(default)]
    pub gift_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GiftCatalogResult {
    pub gifts: Vec<Value>,
}

pub fn register(router: &mut ControlRouter) {
    router.register_typed::<Empty, GiftCatalogResult, _, _>(
        "gifts.list",
        "Persisted gift catalog ordered by diamond count",
        false,
        |core: Arc<AppCore>, _params: Empty| async move {
            let gifts =
                crate::modules::blocking_task("gifts.list", move || core.gift_catalog()).await?;
            Ok::<GiftCatalogResult, ApiError>(GiftCatalogResult { gifts })
        },
    );
    router.register_typed::<GiftDebugParams, GiftDebugResult, _, _>(
        "gifts.debug",
        "Gift icon lookup plus the catalog size",
        false,
        |core: Arc<AppCore>, params: GiftDebugParams| async move {
            let debug = crate::modules::blocking_task("gifts.debug", move || {
                core.gift_debug(params.gift_id.as_deref())
            })
            .await?;
            Ok::<GiftDebugResult, ApiError>(debug)
        },
    );
}
