//! Cloud assets, overlays and packages (v3). Writes are owner-only and the Cloud
//! never applies a package; preflight is a read-only report for the host.

use std::sync::Arc;

pub use tiktools_core::cloud_bundles::{
    AssetUploadParams, CloudAsset, OverlayCreateParams, OverlayIdParams, OverlayRecord,
    OverlayReviseParams, OverlayWriteResult, PackageCreateParams, PackageCreated, PackageIdParams,
    PackageItemParams, PackagePreflight, PackageRecord, PreflightItem,
};
use tiktools_core::AppCore;

use crate::{error::ApiError, router::ControlRouter};

pub fn register(router: &mut ControlRouter) {
    router.register_typed::<AssetUploadParams, CloudAsset, _, _>(
        "bundles.assets.upload",
        "Uploads a PNG, JPEG or WebP image to the owner's Cloud asset store",
        true,
        |core: Arc<AppCore>, params: AssetUploadParams| async move {
            core.bundles_asset_upload(params)
                .await
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<OverlayCreateParams, OverlayWriteResult, _, _>(
        "bundles.overlays.create",
        "Creates a private overlay from a schema version 1 document",
        true,
        |core: Arc<AppCore>, params: OverlayCreateParams| async move {
            core.bundles_overlay_create(params)
                .await
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<OverlayIdParams, OverlayRecord, _, _>(
        "bundles.overlays.read",
        "Reads one owned overlay with its document",
        false,
        |core: Arc<AppCore>, params: OverlayIdParams| async move {
            core.bundles_overlay_read(params)
                .await
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<OverlayReviseParams, OverlayWriteResult, _, _>(
        "bundles.overlays.revise",
        "Publishes a new overlay revision; requires the current revision",
        true,
        |core: Arc<AppCore>, params: OverlayReviseParams| async move {
            core.bundles_overlay_revise(params)
                .await
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<PackageCreateParams, PackageCreated, _, _>(
        "bundles.packages.create",
        "Creates a package that pins plugin versions and document revisions",
        true,
        |core: Arc<AppCore>, params: PackageCreateParams| async move {
            core.bundles_package_create(params)
                .await
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<PackageIdParams, PackageRecord, _, _>(
        "bundles.packages.read",
        "Reads one owned package",
        false,
        |core: Arc<AppCore>, params: PackageIdParams| async move {
            core.bundles_package_read(params)
                .await
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<PackageIdParams, PackagePreflight, _, _>(
        "bundles.packages.preflight",
        "Reports what applying a package needs; applies nothing",
        false,
        |core: Arc<AppCore>, params: PackageIdParams| async move {
            core.bundles_package_preflight(params)
                .await
                .map_err(ApiError::from)
        },
    );
}
