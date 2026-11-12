use super::*;

pub(super) async fn pin_npm(
    client: &Client,
    endpoints: &ProviderEndpoints,
    declaration: &NativeLibDeclaration,
) -> Result<LockedNativeLib, NativeProviderError> {
    let package = &declaration.package;
    let url = format!(
        "{}/{}/{}",
        trim_base(&endpoints.npm_registry),
        registry_package_path(package),
        declaration.version,
    );
    let body = fetch_capped(client, &url, MAX_METADATA_BYTES).await?;
    let metadata: serde_json::Value = serde_json::from_slice(&body)
        .map_err(|error| metadata_error(package, format!("metadata is not JSON: {error}")))?;
    let name = metadata
        .get("name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if name != package {
        return Err(metadata_error(
            package,
            format!("registry answered for package {name:?}"),
        ));
    }
    let answered = metadata
        .get("version")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if answered != declaration.version {
        return Err(metadata_error(
            package,
            format!("registry answered version {answered:?}"),
        ));
    }
    let dist = metadata
        .get("dist")
        .ok_or_else(|| metadata_error(package, "metadata has no dist object".to_owned()))?;
    let tarball = dist
        .get("tarball")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| metadata_error(package, "metadata has no dist.tarball".to_owned()))?;
    let integrity = dist
        .get("integrity")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| metadata_error(package, "metadata has no dist.integrity".to_owned()))?;
    if tarball.is_empty() || integrity.is_empty() {
        return Err(metadata_error(
            package,
            "metadata dist fields are empty".to_owned(),
        ));
    }
    let archive = fetch_capped(client, tarball, MAX_DOWNLOAD_BYTES).await?;
    if !verify_integrity(&archive, integrity) {
        return Err(NativeProviderError::IntegrityMismatch {
            what: format!("{package} tarball"),
        });
    }
    let files = extract_npm_tarball(package, &archive)?;
    let manifest = files
        .iter()
        .find(|file| file.relative == "package.json")
        .ok_or_else(|| NativeProviderError::Tar {
            package: package.clone(),
            message: "archive holds no package.json".to_owned(),
        })?;
    let manifest_json: serde_json::Value =
        serde_json::from_slice(&manifest.bytes).map_err(|error| NativeProviderError::Tar {
            package: package.clone(),
            message: format!("package.json is not JSON: {error}"),
        })?;
    let found = manifest_json
        .get("name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if found != package {
        return Err(NativeProviderError::PackageNameMismatch {
            expected: package.clone(),
            found: found.to_owned(),
        });
    }
    let mut assets = BTreeMap::new();
    for file in files.iter().filter(|file| file.relative.ends_with(".node")) {
        assets.insert(file.relative.clone(), sha256_pin(&file.bytes));
    }
    if assets.is_empty() {
        return Err(NativeProviderError::NoAssetsFound {
            package: package.clone(),
        });
    }
    Ok(LockedNativeLib {
        version: declaration.version.clone(),
        provider: NativeLibProvider::Npm,
        tarball: Some(tarball.to_owned()),
        integrity: Some(integrity.to_owned()),
        repo: None,
        tag: None,
        binary: None,
        assets,
    })
}

pub(super) async fn fetch_npm(
    client: &Client,
    declaration: &NativeLibDeclaration,
    locked: &LockedNativeLib,
    target: &str,
    dest: &Path,
) -> Result<Vec<String>, NativeProviderError> {
    let package = &declaration.package;
    let (Some(tarball), Some(integrity)) = (locked.tarball.as_deref(), locked.integrity.as_deref())
    else {
        return Err(NativeProviderError::LockMismatch {
            package: package.clone(),
            message: "npm entry lacks its tarball pin; re-pin".to_owned(),
        });
    };
    let archive = fetch_capped(client, tarball, MAX_DOWNLOAD_BYTES).await?;
    if !verify_integrity(&archive, integrity) {
        return Err(NativeProviderError::IntegrityMismatch {
            what: format!("{package} tarball"),
        });
    }
    let files = extract_npm_tarball(package, &archive)?;
    let mut staged = Vec::new();
    let mut saw_target_binary = false;
    for file in &files {
        if file.relative.ends_with(".node") {
            if !node_serves_target(&file.relative, target) {
                continue;
            }
            saw_target_binary = true;
            verify_asset(package, locked, &file.relative, &file.bytes)?;
        }
        write_source_file(dest, &file.relative, &file.bytes)?;
        staged.push(file.relative.clone());
    }
    if !saw_target_binary {
        return Err(NativeProviderError::NoBinaryForTarget {
            package: package.clone(),
            target: target.to_owned(),
        });
    }
    if !staged.iter().any(|file| file == "package.json") {
        return Err(NativeProviderError::Tar {
            package: package.clone(),
            message: "archive holds no package.json".to_owned(),
        });
    }
    Ok(staged)
}

/// Registry path segment for a package: scoped slashes escape as
/// `%2F` (`@scope/pkg` -> `@scope%2Fpkg`).
fn registry_package_path(package: &str) -> String {
    package.replace('/', "%2F")
}

/// Whether a top-level `.node` filename serves `target`
/// (`{stem}.{target}.node`, dot boundary required).
pub(super) fn node_serves_target(file: &str, target: &str) -> bool {
    if file.contains('/') || !file.ends_with(".node") {
        return false;
    }
    file.strip_suffix(".node").is_some_and(|stem| {
        stem.len() > target.len()
            && stem.ends_with(target)
            && stem.as_bytes()[stem.len() - target.len() - 1] == b'.'
    })
}
