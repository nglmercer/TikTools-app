//! Cloud assets, overlays and packages (v3). Writes are owner-only and the Cloud
//! never applies a package; preflight is a read-only report for the host.

use std::sync::Arc;

pub use tiktools_core::cloud_bundles::{
    AssetIdParams, AssetSetLabelParams, AssetUploadParams, CatalogForkOrigin, CatalogForkParams,
    CatalogForkResult, CloudAsset, CloudAssetPage, CloudDeleted, CloudPageQuery, LibraryItemPage,
    LibraryItemsQuery, OverlayCreateParams, OverlayIdParams, OverlayPage, OverlayRecord,
    OverlayReviseParams, OverlayRevisionPage, OverlayRevisionParams, OverlayRevisionRecord,
    OverlayRevisionSummary, OverlayRevisionsQuery, OverlaySummary, OverlayWriteResult,
    PackageCreateParams, PackageCreated, PackageIdParams, PackageItemParams, PackagePreflight,
    PackageRecord, PreflightItem,
};
use tiktools_core::AppCore;

use crate::{error::ApiError, router::ControlRouter};

pub fn register(router: &mut ControlRouter) {
    router.register_typed::<LibraryItemsQuery, LibraryItemPage, _, _>(
        "catalog.mine",
        "Lists the signed-in owner's templates and profiles in any status (Míos)",
        false,
        |core: Arc<AppCore>, params: LibraryItemsQuery| async move {
            core.bundles_library_list(params)
                .await
                .map_err(ApiError::from)
        },
    );
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
    router.register_typed::<CatalogForkParams, CatalogForkResult, _, _>(
        "catalog.fork",
        "Copies a public template or profile into the signed-in owner's library as a private draft; needs library.fork",
        true,
        |core: Arc<AppCore>, params: CatalogForkParams| async move {
            core.catalog_fork(params).await.map_err(ApiError::from)
        },
    );
    router.register_typed::<CloudPageQuery, CloudAssetPage, _, _>(
        "bundles.assets.list",
        "Lists the owner's Cloud images, newest first; needs media.operations list",
        false,
        |core: Arc<AppCore>, params: CloudPageQuery| async move {
            core.bundles_assets_list(params)
                .await
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<AssetSetLabelParams, CloudAsset, _, _>(
        "bundles.assets.setLabel",
        "Renames a Cloud image, or clears its label with null; needs media.operations rename",
        true,
        |core: Arc<AppCore>, params: AssetSetLabelParams| async move {
            core.bundles_asset_set_label(params)
                .await
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<AssetIdParams, CloudDeleted, _, _>(
        "bundles.assets.delete",
        "Deletes a Cloud image; conflict while an overlay still uses it; needs media.operations delete",
        true,
        |core: Arc<AppCore>, params: AssetIdParams| async move {
            core.bundles_asset_delete(params).await.map_err(ApiError::from)
        },
    );
    router.register_typed::<CloudPageQuery, OverlayPage, _, _>(
        "bundles.overlays.list",
        "Lists the owner's overlays without their documents; needs overlays.operations list",
        false,
        |core: Arc<AppCore>, params: CloudPageQuery| async move {
            core.bundles_overlays_list(params)
                .await
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<OverlayRevisionsQuery, OverlayRevisionPage, _, _>(
        "bundles.overlays.revisions",
        "Lists the revisions of one overlay, newest first; needs overlays.operations revisions",
        false,
        |core: Arc<AppCore>, params: OverlayRevisionsQuery| async move {
            core.bundles_overlay_revisions(params)
                .await
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<OverlayRevisionParams, OverlayRevisionRecord, _, _>(
        "bundles.overlays.revision",
        "Reads one stored overlay revision with its document; needs overlays.operations revision",
        false,
        |core: Arc<AppCore>, params: OverlayRevisionParams| async move {
            core.bundles_overlay_revision(params)
                .await
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<OverlayIdParams, CloudDeleted, _, _>(
        "bundles.overlays.delete",
        "Deletes an overlay and all its revisions; conflict while a package pins it; needs overlays.operations delete",
        true,
        |core: Arc<AppCore>, params: OverlayIdParams| async move {
            core.bundles_overlay_delete(params).await.map_err(ApiError::from)
        },
    );
    router.register_typed::<PackageIdParams, CloudDeleted, _, _>(
        "bundles.packages.delete",
        "Deletes a package, which releases the overlays it pinned; needs packages.operations delete",
        true,
        |core: Arc<AppCore>, params: PackageIdParams| async move {
            core.bundles_package_delete(params).await.map_err(ApiError::from)
        },
    );
}
