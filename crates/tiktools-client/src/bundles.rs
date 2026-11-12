use super::TikToolsClient;
use tiktools_control_api::modules::bundles::{
    AssetIdParams, AssetSetLabelParams, AssetUploadParams, CatalogForkParams, CatalogForkResult,
    CloudAsset, CloudAssetPage, CloudDeleted, CloudPageQuery, LibraryItemPage, LibraryItemsQuery,
    OverlayCreateParams, OverlayIdParams, OverlayPage, OverlayRecord, OverlayReviseParams,
    OverlayRevisionPage, OverlayRevisionParams, OverlayRevisionRecord, OverlayRevisionsQuery,
    OverlayWriteResult, PackageCreateParams, PackageCreated, PackageIdParams, PackagePreflight,
    PackageRecord,
};
use tiktools_control_api::ClientError;

pub(crate) const METHODS: &[&str] = &[
    CATALOG_MINE,
    BUNDLES_ASSETS_UPLOAD,
    BUNDLES_OVERLAYS_CREATE,
    BUNDLES_OVERLAYS_READ,
    BUNDLES_OVERLAYS_REVISE,
    BUNDLES_PACKAGES_CREATE,
    BUNDLES_PACKAGES_READ,
    BUNDLES_PACKAGES_PREFLIGHT,
    CATALOG_FORK,
    BUNDLES_ASSETS_LIST,
    BUNDLES_ASSETS_SET_LABEL,
    BUNDLES_ASSETS_DELETE,
    BUNDLES_OVERLAYS_LIST,
    BUNDLES_OVERLAYS_REVISIONS,
    BUNDLES_OVERLAYS_REVISION,
    BUNDLES_OVERLAYS_DELETE,
    BUNDLES_PACKAGES_DELETE,
];

const CATALOG_MINE: &str = "catalog.mine";
const BUNDLES_ASSETS_UPLOAD: &str = "bundles.assets.upload";
const BUNDLES_OVERLAYS_CREATE: &str = "bundles.overlays.create";
const BUNDLES_OVERLAYS_READ: &str = "bundles.overlays.read";
const BUNDLES_OVERLAYS_REVISE: &str = "bundles.overlays.revise";
const BUNDLES_PACKAGES_CREATE: &str = "bundles.packages.create";
const BUNDLES_PACKAGES_READ: &str = "bundles.packages.read";
const BUNDLES_PACKAGES_PREFLIGHT: &str = "bundles.packages.preflight";
const CATALOG_FORK: &str = "catalog.fork";
const BUNDLES_ASSETS_LIST: &str = "bundles.assets.list";
const BUNDLES_ASSETS_SET_LABEL: &str = "bundles.assets.setLabel";
const BUNDLES_ASSETS_DELETE: &str = "bundles.assets.delete";
const BUNDLES_OVERLAYS_LIST: &str = "bundles.overlays.list";
const BUNDLES_OVERLAYS_REVISIONS: &str = "bundles.overlays.revisions";
const BUNDLES_OVERLAYS_REVISION: &str = "bundles.overlays.revision";
const BUNDLES_OVERLAYS_DELETE: &str = "bundles.overlays.delete";
const BUNDLES_PACKAGES_DELETE: &str = "bundles.packages.delete";

impl TikToolsClient {
    pub async fn bundles_assets_upload(
        &self,
        params: AssetUploadParams,
    ) -> Result<CloudAsset, ClientError> {
        self.call(BUNDLES_ASSETS_UPLOAD, params).await
    }
    pub async fn bundles_overlays_create(
        &self,
        params: OverlayCreateParams,
    ) -> Result<OverlayWriteResult, ClientError> {
        self.call(BUNDLES_OVERLAYS_CREATE, params).await
    }
    pub async fn bundles_overlays_read(
        &self,
        params: OverlayIdParams,
    ) -> Result<OverlayRecord, ClientError> {
        self.call(BUNDLES_OVERLAYS_READ, params).await
    }
    pub async fn bundles_overlays_revise(
        &self,
        params: OverlayReviseParams,
    ) -> Result<OverlayWriteResult, ClientError> {
        self.call(BUNDLES_OVERLAYS_REVISE, params).await
    }
    pub async fn bundles_packages_create(
        &self,
        params: PackageCreateParams,
    ) -> Result<PackageCreated, ClientError> {
        self.call(BUNDLES_PACKAGES_CREATE, params).await
    }
    pub async fn bundles_packages_read(
        &self,
        params: PackageIdParams,
    ) -> Result<PackageRecord, ClientError> {
        self.call(BUNDLES_PACKAGES_READ, params).await
    }
    pub async fn bundles_packages_preflight(
        &self,
        params: PackageIdParams,
    ) -> Result<PackagePreflight, ClientError> {
        self.call(BUNDLES_PACKAGES_PREFLIGHT, params).await
    }
    pub async fn catalog_mine(
        &self,
        params: LibraryItemsQuery,
    ) -> Result<LibraryItemPage, ClientError> {
        self.call(CATALOG_MINE, params).await
    }
    pub async fn catalog_fork(
        &self,
        params: CatalogForkParams,
    ) -> Result<CatalogForkResult, ClientError> {
        self.call(CATALOG_FORK, params).await
    }
    pub async fn bundles_assets_list(
        &self,
        params: CloudPageQuery,
    ) -> Result<CloudAssetPage, ClientError> {
        self.call(BUNDLES_ASSETS_LIST, params).await
    }
    pub async fn bundles_assets_set_label(
        &self,
        params: AssetSetLabelParams,
    ) -> Result<CloudAsset, ClientError> {
        self.call(BUNDLES_ASSETS_SET_LABEL, params).await
    }
    pub async fn bundles_assets_delete(
        &self,
        params: AssetIdParams,
    ) -> Result<CloudDeleted, ClientError> {
        self.call(BUNDLES_ASSETS_DELETE, params).await
    }
    pub async fn bundles_overlays_list(
        &self,
        params: CloudPageQuery,
    ) -> Result<OverlayPage, ClientError> {
        self.call(BUNDLES_OVERLAYS_LIST, params).await
    }
    pub async fn bundles_overlays_revisions(
        &self,
        params: OverlayRevisionsQuery,
    ) -> Result<OverlayRevisionPage, ClientError> {
        self.call(BUNDLES_OVERLAYS_REVISIONS, params).await
    }
    pub async fn bundles_overlays_revision(
        &self,
        params: OverlayRevisionParams,
    ) -> Result<OverlayRevisionRecord, ClientError> {
        self.call(BUNDLES_OVERLAYS_REVISION, params).await
    }
    pub async fn bundles_overlays_delete(
        &self,
        params: OverlayIdParams,
    ) -> Result<CloudDeleted, ClientError> {
        self.call(BUNDLES_OVERLAYS_DELETE, params).await
    }
    pub async fn bundles_packages_delete(
        &self,
        params: PackageIdParams,
    ) -> Result<CloudDeleted, ClientError> {
        self.call(BUNDLES_PACKAGES_DELETE, params).await
    }
}
