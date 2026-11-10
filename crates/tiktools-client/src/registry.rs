use super::TikToolsClient;
use tiktools_control_api::modules::registry::{
    CloudCatalogItemPage, CloudCatalogItemQuery, CloudCatalogItemsQuery, CloudCatalogPage,
    CloudCatalogPaginationCapabilities, CloudCatalogQuery, CloudCatalogSummariesPage,
    CloudCatalogSummariesQuery, CloudCatalogVersionsPage, CloudCatalogVersionsQuery,
    RegistryDefaults, RegistryListResult, RegistryPackageParams, RegistryPackageView,
    RegistryRefreshParams, SearchQuery, SearchResults,
};
use tiktools_control_api::modules::Empty;
use tiktools_control_api::ClientError;

pub(crate) const METHODS: &[&str] = &[
    REGISTRY_LIST,
    REGISTRY_REFRESH,
    REGISTRY_SEARCH,
    REGISTRY_CLOUD_CATALOG,
    REGISTRY_CLOUD_CATALOG_CAPABILITIES,
    REGISTRY_CLOUD_CATALOG_SUMMARIES,
    REGISTRY_CLOUD_CATALOG_VERSIONS,
    CATALOG_ITEMS,
    CATALOG_ITEM,
    REGISTRY_PACKAGE,
    REGISTRY_DEFAULTS_GET,
    REGISTRY_DEFAULTS_SET,
];

const CATALOG_ITEMS: &str = "catalog.items";
const CATALOG_ITEM: &str = "catalog.item";
const REGISTRY_LIST: &str = "registry.list";
const REGISTRY_REFRESH: &str = "registry.refresh";
const REGISTRY_SEARCH: &str = "registry.search";
const REGISTRY_CLOUD_CATALOG: &str = "registry.cloudCatalog";
const REGISTRY_CLOUD_CATALOG_CAPABILITIES: &str = "registry.cloudCatalogPaginationCapabilities";
const REGISTRY_CLOUD_CATALOG_SUMMARIES: &str = "registry.cloudCatalogSummaries";
const REGISTRY_CLOUD_CATALOG_VERSIONS: &str = "registry.cloudCatalogVersions";
const REGISTRY_PACKAGE: &str = "registry.package";
const REGISTRY_DEFAULTS_GET: &str = "registry.defaults.get";
const REGISTRY_DEFAULTS_SET: &str = "registry.defaults.set";

impl TikToolsClient {
    pub async fn catalog_items(
        &self,
        params: CloudCatalogItemsQuery,
    ) -> Result<CloudCatalogItemPage, ClientError> {
        self.call(CATALOG_ITEMS, params).await
    }
    pub async fn catalog_item(
        &self,
        params: CloudCatalogItemQuery,
    ) -> Result<serde_json::Value, ClientError> {
        self.call(CATALOG_ITEM, params).await
    }
    pub async fn registry_cloud_catalog_pagination_capabilities(
        &self,
    ) -> Result<CloudCatalogPaginationCapabilities, ClientError> {
        self.call(REGISTRY_CLOUD_CATALOG_CAPABILITIES, Empty::default())
            .await
    }
    pub async fn registry_cloud_catalog_summaries(
        &self,
        params: CloudCatalogSummariesQuery,
    ) -> Result<CloudCatalogSummariesPage, ClientError> {
        self.call(REGISTRY_CLOUD_CATALOG_SUMMARIES, params).await
    }
    pub async fn registry_cloud_catalog_versions(
        &self,
        params: CloudCatalogVersionsQuery,
    ) -> Result<CloudCatalogVersionsPage, ClientError> {
        self.call(REGISTRY_CLOUD_CATALOG_VERSIONS, params).await
    }
    pub async fn registry_list(&self) -> Result<RegistryListResult, ClientError> {
        self.call(REGISTRY_LIST, Empty::default()).await
    }

    pub async fn registry_refresh(
        &self,
        params: RegistryRefreshParams,
    ) -> Result<RegistryListResult, ClientError> {
        self.call(REGISTRY_REFRESH, params).await
    }

    pub async fn registry_search(&self, params: SearchQuery) -> Result<SearchResults, ClientError> {
        self.call(REGISTRY_SEARCH, params).await
    }

    pub async fn registry_cloud_catalog(
        &self,
        params: CloudCatalogQuery,
    ) -> Result<CloudCatalogPage, ClientError> {
        self.call(REGISTRY_CLOUD_CATALOG, params).await
    }

    pub async fn registry_package(
        &self,
        params: RegistryPackageParams,
    ) -> Result<RegistryPackageView, ClientError> {
        self.call(REGISTRY_PACKAGE, params).await
    }

    pub async fn registry_defaults_get(&self) -> Result<RegistryDefaults, ClientError> {
        self.call(REGISTRY_DEFAULTS_GET, Empty::default()).await
    }

    pub async fn registry_defaults_set(
        &self,
        params: RegistryDefaults,
    ) -> Result<RegistryDefaults, ClientError> {
        self.call(REGISTRY_DEFAULTS_SET, params).await
    }
}
