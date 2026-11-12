use super::*;

/// The signed immutable registry grants trust; the live v2 catalog supplies
/// current revocation/yank state for the exact selected artifact. Missing,
/// changed, unbounded, or advisory-bearing status fails closed pending review.
pub(super) async fn verify_current_cloud_release(
    prepared: &PreparedPackage,
    target: &str,
    generation: u64,
) -> Result<(), InstallError> {
    let unavailable = || {
        InstallError::new(
            InstallErrorCode::Unsupported,
            "current Cloud release status is unavailable or needs review",
        )
    };
    let origin = tiktools_cloud_auth::resolve_cloud_origin().ok_or_else(unavailable)?;
    let catalog = cloud_client::CloudCatalog::new(&origin).map_err(|_| unavailable())?;
    verify_catalog_release(
        &catalog,
        &prepared.manifest.id,
        &prepared.manifest.version,
        target,
        &prepared.digest.to_string(),
        prepared.size(),
        generation,
    )
    .await
}

pub(super) async fn verify_catalog_release(
    catalog: &cloud_client::CloudCatalog,
    id: &str,
    version: &str,
    target: &str,
    digest: &str,
    size: u64,
    generation: u64,
) -> Result<(), InstallError> {
    let unavailable = || {
        InstallError::new(
            InstallErrorCode::Unsupported,
            "current Cloud release status is unavailable or needs review",
        )
    };
    let generation = i64::try_from(generation)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(unavailable)?;
    let mut cursor: Option<String> = None;
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..128 {
        let page = catalog
            .plugin_version_page(id, Some(target), Some(generation), cursor.as_deref())
            .await
            .map_err(|_| unavailable())?;
        for item in &page.items {
            if let Some(result) =
                validate_current_cloud_release(item, version, target, digest, size)
            {
                return result;
            }
        }
        let Some(next) = page.next_cursor else {
            return Err(unavailable());
        };
        if !seen.insert(next.clone()) {
            return Err(unavailable());
        }
        cursor = Some(next);
    }
    Err(unavailable())
}

pub(super) fn validate_current_cloud_release(
    item: &cloud_client::CatalogVersion,
    version: &str,
    target: &str,
    digest: &str,
    size: u64,
) -> Option<Result<(), InstallError>> {
    if item.version != version
        || item.target != target
        || item.sha256 != digest
        || item.size != size.to_string()
    {
        return None;
    }
    if item.yanked || item.key_revoked || !item.publisher_approved {
        return Some(Err(InstallError::new(
            InstallErrorCode::Revoked,
            "Cloud release, publisher or signing key is no longer approved",
        )));
    }
    if item.advisory.is_some() {
        return Some(Err(InstallError::new(
            InstallErrorCode::ConsentRequired,
            "Cloud release has an active advisory; installation is unavailable until it is resolved",
        )));
    }
    Some(Ok(()))
}
