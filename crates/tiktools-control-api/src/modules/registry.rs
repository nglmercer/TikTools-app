use std::sync::Arc;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tiktools_core::AppCore;
pub use tiktools_core::{
    control::{
        CloudCatalogPage, CloudCatalogPaginationCapabilities, CloudCatalogQuery,
        CloudCatalogSummariesPage, CloudCatalogSummariesQuery, CloudCatalogVersionsPage,
        CloudCatalogVersionsQuery, RegistryPackageView,
    },
    plugin_registry::{RegistryDefaults, RegistryStatus, SearchQuery, SearchResults},
};

use crate::{error::ApiError, modules::Empty, router::ControlRouter};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegistryListResult {
    pub registries: Vec<RegistryStatus>,
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
    router.register_typed::<Empty, CloudCatalogPaginationCapabilities, _, _>(
        "registry.cloudCatalogPaginationCapabilities",
        "Discovers bounded Cloud catalog pagination capabilities",
        false,
        |core: Arc<AppCore>, _: Empty| async move {
            core.cloud_catalog_pagination_capabilities()
                .await
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<CloudCatalogSummariesQuery, CloudCatalogSummariesPage, _, _>(
        "registry.cloudCatalogSummaries",
        "Reads bounded Cloud plugin summaries without granting installation trust",
        false,
        |core: Arc<AppCore>, params: CloudCatalogSummariesQuery| async move {
            core.cloud_catalog_summaries(params)
                .await
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<CloudCatalogVersionsQuery, CloudCatalogVersionsPage, _, _>(
        "registry.cloudCatalogVersions",
        "Reads one bounded Cloud plugin version page",
        false,
        |core: Arc<AppCore>, params: CloudCatalogVersionsQuery| async move {
            core.cloud_catalog_versions(params)
                .await
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<CloudCatalogQuery, CloudCatalogPage, _, _>(
        "registry.cloudCatalog",
        "Discovers plugin metadata from the configured Cloud instance; installation requires separate signature verification",
        false,
        |core: Arc<AppCore>, params: CloudCatalogQuery| async move {
            core.cloud_catalog_plugins(params).await.map_err(ApiError::from)
        },
    );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn product_does_not_register_source_enrollment_or_toggle_methods() {
        let mut router = ControlRouter::new();
        register(&mut router);
        for method in [
            "registry.root",
            "registry.add",
            "registry.remove",
            "registry.enable",
        ] {
            assert!(
                router.get(method).is_none(),
                "unexpected source control: {method}"
            );
        }
        assert!(router.get("registry.list").is_some());
        assert!(router.get("registry.defaults.set").is_some());
    }
}
