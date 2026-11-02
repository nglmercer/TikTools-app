use std::sync::Arc;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tiktools_core::AppCore;
pub use tiktools_core::{
    control::{RegistryAddInput, RegistryPackageView, RegistryRootPreview},
    plugin_registry::{RegistryDefaults, RegistryStatus, SearchQuery, SearchResults},
};

use crate::{
    error::ApiError,
    modules::{Empty, OkResult},
    router::ControlRouter,
};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegistryListResult {
    pub registries: Vec<RegistryStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegistryResult {
    pub registry: RegistryStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegistryUrlParams {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegistryNameParams {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegistryEnableParams {
    pub name: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegistryRefreshParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegistryPackageParams {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry: Option<String>,
}

pub fn register(router: &mut ControlRouter) {
    router.register_typed::<Empty, RegistryListResult, _, _>(
        "registry.list",
        "Configured plugin registries with verification and refresh state",
        false,
        |core: Arc<AppCore>, _params: Empty| async move {
            core.registry_list()
                .map(|registries| RegistryListResult { registries })
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<RegistryUrlParams, RegistryRootPreview, _, _>(
        "registry.root",
        "Fetches a registry's root keys so their fingerprints can be confirmed before adding it",
        false,
        |core: Arc<AppCore>, params: RegistryUrlParams| async move {
            core.registry_root(&params.url)
                .await
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<RegistryAddInput, RegistryResult, _, _>(
        "registry.add",
        "Adds a plugin registry, pinning only the root keys whose fingerprints were confirmed",
        true,
        |core: Arc<AppCore>, params: RegistryAddInput| async move {
            core.registry_add(params)
                .await
                .map(|registry| RegistryResult { registry })
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<RegistryNameParams, OkResult, _, _>(
        "registry.remove",
        "Removes a user-added plugin registry and its cached metadata",
        true,
        |core: Arc<AppCore>, params: RegistryNameParams| async move {
            core.registry_remove(&params.name)
                .map(|()| OkResult::ok())
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<RegistryEnableParams, RegistryResult, _, _>(
        "registry.enable",
        "Enables or disables a plugin registry",
        true,
        |core: Arc<AppCore>, params: RegistryEnableParams| async move {
            core.registry_set_enabled(&params.name, params.enabled)
                .map(|registry| RegistryResult { registry })
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<RegistryRefreshParams, RegistryListResult, _, _>(
        "registry.refresh",
        "Re-reads and re-verifies registry keys and configuration",
        false,
        |core: Arc<AppCore>, params: RegistryRefreshParams| async move {
            core.registry_refresh(params.name.as_deref())
                .await
                .map(|registries| RegistryListResult { registries })
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<SearchQuery, SearchResults, _, _>(
        "registry.search",
        "Searches enabled registries' catalogs",
        false,
        |core: Arc<AppCore>, params: SearchQuery| async move {
            core.registry_search(params).await.map_err(ApiError::from)
        },
    );
    router.register_typed::<RegistryPackageParams, RegistryPackageView, _, _>(
        "registry.package",
        "The verified registry document of one plugin",
        false,
        |core: Arc<AppCore>, params: RegistryPackageParams| async move {
            core.registry_package(&params.id, params.registry.as_deref())
                .await
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<Empty, RegistryDefaults, _, _>(
        "registry.defaults.get",
        "Default update channel, auto-update rule, check interval, and sideload policy",
        false,
        |core: Arc<AppCore>, _params: Empty| async move {
            core.registry_defaults().map_err(ApiError::from)
        },
    );
    router.register_typed::<RegistryDefaults, RegistryDefaults, _, _>(
        "registry.defaults.set",
        "Updates the default plugin update and sideload policy",
        true,
        |core: Arc<AppCore>, params: RegistryDefaults| async move {
            core.registry_set_defaults(params).map_err(ApiError::from)
        },
    );
}
