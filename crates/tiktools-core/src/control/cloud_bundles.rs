//! Cloud assets, overlays and packages for the host API. Packages are described
//! here and applied by the host; the Cloud never applies them.

use super::OperationError;
use crate::{
    cloud_bundles::{
        self, AssetUploadParams, CloudAsset, OverlayCreateParams, OverlayIdParams, OverlayRecord,
        OverlayReviseParams, OverlayWriteResult, PackageCreateParams, PackageCreated,
        PackageIdParams, PackagePreflight, PackageRecord,
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
}
