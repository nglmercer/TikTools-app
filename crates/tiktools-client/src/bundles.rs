use super::TikToolsClient;
use tiktools_control_api::modules::bundles::{
    AssetUploadParams, CloudAsset, LibraryItemPage, LibraryItemsQuery, OverlayCreateParams,
    OverlayIdParams, OverlayRecord, OverlayReviseParams, OverlayWriteResult, PackageCreateParams,
    PackageCreated, PackageIdParams, PackagePreflight, PackageRecord,
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
];

const CATALOG_MINE: &str = "catalog.mine";
const BUNDLES_ASSETS_UPLOAD: &str = "bundles.assets.upload";
const BUNDLES_OVERLAYS_CREATE: &str = "bundles.overlays.create";
const BUNDLES_OVERLAYS_READ: &str = "bundles.overlays.read";
const BUNDLES_OVERLAYS_REVISE: &str = "bundles.overlays.revise";
const BUNDLES_PACKAGES_CREATE: &str = "bundles.packages.create";
const BUNDLES_PACKAGES_READ: &str = "bundles.packages.read";
const BUNDLES_PACKAGES_PREFLIGHT: &str = "bundles.packages.preflight";

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
}
