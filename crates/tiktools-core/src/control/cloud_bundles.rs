//! Cloud assets, overlays and packages for the host API. Packages are described
//! here and applied by the host; the Cloud never applies them.

use super::OperationError;
use crate::{
    cloud_bundles::{
        self, AssetIdParams, AssetSetLabelParams, AssetUploadParams, CatalogForkParams,
        CatalogForkResult, CloudAsset, CloudAssetPage, CloudDeleted, CloudPageQuery,
        LibraryItemPage, LibraryItemsQuery, OverlayCreateParams, OverlayIdParams, OverlayPage,
        OverlayRecord, OverlayReviseParams, OverlayRevisionPage, OverlayRevisionParams,
        OverlayRevisionRecord, OverlayRevisionsQuery, OverlayWriteResult, PackageCreateParams,
        PackageCreated, PackageIdParams, PackagePreflight, PackageRecord,
    },
    AppCore,
};

impl AppCore {
    pub async fn bundles_asset_upload(
        &self,
        params: AssetUploadParams,
    ) -> Result<CloudAsset, OperationError> {
        cloud_bundles::upload_asset(self.cloud.auth(), params).await
    }
    pub async fn bundles_overlay_create(
        &self,
        params: OverlayCreateParams,
    ) -> Result<OverlayWriteResult, OperationError> {
        cloud_bundles::create_overlay(self.cloud.auth(), params).await
    }
    pub async fn bundles_overlay_read(
        &self,
        params: OverlayIdParams,
    ) -> Result<OverlayRecord, OperationError> {
        cloud_bundles::read_overlay(self.cloud.auth(), params).await
    }
    pub async fn bundles_overlay_revise(
        &self,
        params: OverlayReviseParams,
    ) -> Result<OverlayWriteResult, OperationError> {
        cloud_bundles::revise_overlay(self.cloud.auth(), params).await
    }
    pub async fn bundles_package_create(
        &self,
        params: PackageCreateParams,
    ) -> Result<PackageCreated, OperationError> {
        cloud_bundles::create_package(self.cloud.auth(), params).await
    }
    pub async fn bundles_package_read(
        &self,
        params: PackageIdParams,
    ) -> Result<PackageRecord, OperationError> {
        cloud_bundles::read_package(self.cloud.auth(), params).await
    }
    pub async fn bundles_package_preflight(
        &self,
        params: PackageIdParams,
    ) -> Result<PackagePreflight, OperationError> {
        cloud_bundles::preflight_package(self.cloud.auth(), params).await
    }
    pub async fn bundles_library_list(
        &self,
        query: LibraryItemsQuery,
    ) -> Result<LibraryItemPage, OperationError> {
        cloud_bundles::list_library_items(self.cloud.auth(), query).await
    }
    pub async fn bundles_assets_list(
        &self,
        query: CloudPageQuery,
    ) -> Result<CloudAssetPage, OperationError> {
        cloud_bundles::list_assets(self.cloud.auth(), query).await
    }
    pub async fn bundles_asset_set_label(
        &self,
        params: AssetSetLabelParams,
    ) -> Result<CloudAsset, OperationError> {
        cloud_bundles::set_asset_label(self.cloud.auth(), params).await
    }
    pub async fn bundles_asset_delete(
        &self,
        params: AssetIdParams,
    ) -> Result<CloudDeleted, OperationError> {
        cloud_bundles::delete_asset(self.cloud.auth(), params).await
    }
    pub async fn bundles_overlays_list(
        &self,
        query: CloudPageQuery,
    ) -> Result<OverlayPage, OperationError> {
        cloud_bundles::list_overlays(self.cloud.auth(), query).await
    }
    pub async fn bundles_overlay_revisions(
        &self,
        query: OverlayRevisionsQuery,
    ) -> Result<OverlayRevisionPage, OperationError> {
        cloud_bundles::list_overlay_revisions(self.cloud.auth(), query).await
    }
    pub async fn bundles_overlay_revision(
        &self,
        params: OverlayRevisionParams,
    ) -> Result<OverlayRevisionRecord, OperationError> {
        cloud_bundles::read_overlay_revision(self.cloud.auth(), params).await
    }
    pub async fn bundles_overlay_delete(
        &self,
        params: OverlayIdParams,
    ) -> Result<CloudDeleted, OperationError> {
        cloud_bundles::delete_overlay(self.cloud.auth(), params).await
    }
    pub async fn bundles_package_delete(
        &self,
        params: PackageIdParams,
    ) -> Result<CloudDeleted, OperationError> {
        cloud_bundles::delete_package(self.cloud.auth(), params).await
    }
    pub async fn catalog_fork(
        &self,
        params: CatalogForkParams,
    ) -> Result<CatalogForkResult, OperationError> {
        cloud_bundles::fork_catalog_document(self.cloud.auth(), params).await
    }
}
