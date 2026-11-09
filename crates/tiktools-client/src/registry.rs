use super::TikToolsClient;
use tiktools_control_api::modules::registry::{
    CloudCatalogPage, CloudCatalogQuery, RegistryAddInput, RegistryDefaults, RegistryEnableParams,
    RegistryListResult, RegistryNameParams, RegistryPackageParams, RegistryPackageView,
    RegistryRefreshParams, RegistryResult, RegistryRootPreview, RegistryUrlParams, SearchQuery,
    SearchResults,
};
use tiktools_control_api::modules::{Empty, OkResult};
use tiktools_control_api::ClientError;

pub(crate) const METHODS: &[&str] = &[
    REGISTRY_LIST,
    REGISTRY_ROOT,
    REGISTRY_ADD,
    REGISTRY_REMOVE,
    REGISTRY_ENABLE,
    REGISTRY_REFRESH,
    REGISTRY_SEARCH,
    REGISTRY_CLOUD_CATALOG,
    REGISTRY_PACKAGE,
    REGISTRY_DEFAULTS_GET,
    REGISTRY_DEFAULTS_SET,
];

const REGISTRY_LIST: &str = "registry.list";
const REGISTRY_ROOT: &str = "registry.root";
const REGISTRY_ADD: &str = "registry.add";
const REGISTRY_REMOVE: &str = "registry.remove";
const REGISTRY_ENABLE: &str = "registry.enable";
const REGISTRY_REFRESH: &str = "registry.refresh";
const REGISTRY_SEARCH: &str = "registry.search";
const REGISTRY_CLOUD_CATALOG: &str = "registry.cloudCatalog";
const REGISTRY_PACKAGE: &str = "registry.package";
const REGISTRY_DEFAULTS_GET: &str = "registry.defaults.get";
const REGISTRY_DEFAULTS_SET: &str = "registry.defaults.set";

impl TikToolsClient {
    pub async fn registry_list(&self) -> Result<RegistryListResult, ClientError> {
        self.call(REGISTRY_LIST, Empty::default()).await
    }

    pub async fn registry_root(
        &self,
        params: RegistryUrlParams,
    ) -> Result<RegistryRootPreview, ClientError> {
        self.call(REGISTRY_ROOT, params).await
    }

    pub async fn registry_add(
        &self,
        params: RegistryAddInput,
    ) -> Result<RegistryResult, ClientError> {
        self.call(REGISTRY_ADD, params).await
    }

    pub async fn registry_remove(
        &self,
        params: RegistryNameParams,
    ) -> Result<OkResult, ClientError> {
        self.call(REGISTRY_REMOVE, params).await
    }

    pub async fn registry_enable(
        &self,
        params: RegistryEnableParams,
    ) -> Result<RegistryResult, ClientError> {
        self.call(REGISTRY_ENABLE, params).await
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
