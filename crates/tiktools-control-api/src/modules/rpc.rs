use std::sync::Arc;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tiktools_core::AppCore;

use crate::{
    error::ApiError,
    modules::Empty,
    registry::{MethodMeta, MethodRegistry},
    router::ControlRouter,
};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverResult {
    pub methods: Vec<MethodMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SchemaParams {
    pub method: String,
}

pub fn register(
    router: &mut ControlRouter,
    registry: std::sync::Arc<std::sync::RwLock<MethodRegistry>>,
) {
    let snapshot = registry;
    let discover = std::sync::Arc::clone(&snapshot);
    router.register_typed::<Empty, DiscoverResult, _, _>(
        "rpc.discover",
        "Lists every method with descriptions and JSON Schemas",
        false,
        move |_core: Arc<AppCore>, _params: Empty| {
            let discover = std::sync::Arc::clone(&discover);
            async move {
                let methods = discover
                    .read()
                    .map(|registry| registry.methods.clone())
                    .unwrap_or_default();
                Ok::<DiscoverResult, ApiError>(DiscoverResult { methods })
            }
        },
    );
    router.register_typed::<SchemaParams, MethodMeta, _, _>(
        "rpc.schema",
        "Returns one method metadata entry with its schemas",
        false,
        move |_core: Arc<AppCore>, params: SchemaParams| {
            let snapshot = std::sync::Arc::clone(&snapshot);
            async move {
                snapshot
                    .read()
                    .ok()
                    .and_then(|registry| registry.get(&params.method).cloned())
                    .ok_or_else(|| ApiError::method_not_found(&params.method))
            }
        },
    );
}
