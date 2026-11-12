use super::*;

/// Load one signed document and query current Cloud status while retaining
/// its trust lease. A failed status check still permits displaying the signed
/// document, but no release is described as currently approved.
pub(super) async fn registry_package_live(
    packages: &PluginPackages,
    id: &str,
    registry: Option<&str>,
) -> Result<
    (
        RegistrySource,
        PackageDocument,
        BTreeMap<String, LiveArtifactStatus>,
    ),
    OperationError,
> {
    let client = packages.registry();
    let (source, document, guard) = client
        .package_guarded(id, registry)
        .await
        .map_err(registry_error)?;
    let host = packages.installer.host();
    let unknown = classify_live_releases(&document, host, None, |_| Ok(false));
    let Some(guard) = guard else {
        return Ok((source, document, unknown));
    };
    let Some(origin) = source.cloud_origin.as_deref() else {
        return Ok((source, document, unknown));
    };
    let query = async {
        let generation = client
            .check_fixed_generation(&guard, &source.name, &source.url)
            .await
            .map_err(|_| ())?;
        let generation_i64 = i64::try_from(generation)
            .ok()
            .filter(|n| *n > 0)
            .ok_or(())?;
        let catalog = cloud_client::CloudCatalog::new(origin).map_err(|_| ())?;
        let records = load_catalog_versions(&catalog, id, &host.target, generation_i64).await?;
        client
            .check_fixed_generation(&guard, &source.name, &source.url)
            .await
            .map_err(|_| ())?;
        Ok::<_, ()>(classify_live_releases(
            &document,
            host,
            Some(&records),
            |digest| client.try_is_revoked(digest).map_err(|_| ()),
        ))
    };
    let live = tokio::time::timeout(Duration::from_secs(20), query)
        .await
        .ok()
        .and_then(Result::ok)
        .unwrap_or(unknown);
    Ok((source, document, live))
}

pub(super) async fn load_catalog_versions(
    catalog: &cloud_client::CloudCatalog,
    id: &str,
    target: &str,
    generation: i64,
) -> Result<BTreeMap<(String, String), cloud_client::CatalogVersion>, ()> {
    let mut items = BTreeMap::new();
    let mut cursor = None;
    let mut seen = BTreeSet::new();
    for _ in 0..128 {
        let page = catalog
            .plugin_version_page(id, Some(target), Some(generation), cursor.as_deref())
            .await
            .map_err(|_| ())?;
        for item in page.items {
            let key = (item.version.clone(), item.target.clone());
            if items.insert(key, item).is_some() {
                return Err(());
            }
        }
        match page.next_cursor {
            None => return Ok(items),
            Some(next) if seen.insert(next.clone()) => cursor = Some(next),
            Some(_) => return Err(()),
        }
    }
    Err(())
}
