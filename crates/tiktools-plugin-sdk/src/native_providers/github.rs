use super::*;

fn github_asset_url(endpoints: &ProviderEndpoints, repo: &str, tag: &str, asset: &str) -> String {
    format!(
        "{}/{repo}/releases/download/{tag}/{asset}",
        trim_base(&endpoints.github_base),
    )
}

pub(super) async fn pin_github(
    client: &Client,
    endpoints: &ProviderEndpoints,
    declaration: &NativeLibDeclaration,
) -> Result<LockedNativeLib, NativeProviderError> {
    let package = &declaration.package;
    let (Some(repo), Some(tag), Some(binary)) = (
        declaration.repo.as_deref(),
        declaration.tag.as_deref(),
        declaration.binary.as_deref(),
    ) else {
        return Err(metadata_error(
            package,
            "github declaration needs repo, tag, and binary".to_owned(),
        ));
    };
    let mut assets = BTreeMap::new();
    for companion in GITHUB_COMPANION_FILES {
        let url = github_asset_url(endpoints, repo, tag, companion);
        let bytes = fetch_capped(client, &url, MAX_DOWNLOAD_BYTES).await?;
        assets.insert(companion.to_owned(), sha256_pin(&bytes));
    }
    for target in NAPI_TARGETS {
        let asset = format!("{binary}.{target}.node");
        let url = github_asset_url(endpoints, repo, tag, &asset);
        match fetch_capped(client, &url, MAX_DOWNLOAD_BYTES).await {
            Ok(bytes) => {
                assets.insert(asset, sha256_pin(&bytes));
            }
            Err(NativeProviderError::NotFound(_)) => continue,
            Err(error) => return Err(error),
        }
    }
    if !assets.keys().any(|name| name.ends_with(".node")) {
        return Err(NativeProviderError::NoAssetsFound {
            package: package.clone(),
        });
    }
    Ok(LockedNativeLib {
        version: declaration.version.clone(),
        provider: NativeLibProvider::Github,
        tarball: None,
        integrity: None,
        repo: Some(repo.to_owned()),
        tag: Some(tag.to_owned()),
        binary: Some(binary.to_owned()),
        assets,
    })
}

/// Minimal `package.json` for a GitHub-assembled tree. Releases carry
/// no manifest, so the provider writes one in the napi-rs shape
/// (`binaryName`, plus a `files` allowlist matching what was fetched).
fn github_package_manifest(declaration: &NativeLibDeclaration, binary: &str) -> String {
    let manifest = serde_json::json!({
        "name": declaration.package,
        "version": declaration.version,
        "main": "index.js",
        "types": "index.d.ts",
        "files": ["index.js", "index.d.ts", "*.node"],
        "napi": {"binaryName": binary},
    });
    format!(
        "{}\n",
        serde_json::to_string_pretty(&manifest).unwrap_or_default()
    )
}

pub(super) async fn fetch_github(
    client: &Client,
    endpoints: &ProviderEndpoints,
    declaration: &NativeLibDeclaration,
    locked: &LockedNativeLib,
    target: &str,
    dest: &Path,
) -> Result<Vec<String>, NativeProviderError> {
    let package = &declaration.package;
    let (Some(repo), Some(tag), Some(binary)) = (
        locked.repo.as_deref(),
        locked.tag.as_deref(),
        locked.binary.as_deref(),
    ) else {
        return Err(NativeProviderError::LockMismatch {
            package: package.clone(),
            message: "github entry lacks its repo pins; re-pin".to_owned(),
        });
    };
    // The lockfile was checked against the declaration, so its
    // coordinates are authoritative here.
    let mut wanted = Vec::with_capacity(GITHUB_COMPANION_FILES.len() + 1);
    wanted.extend(GITHUB_COMPANION_FILES.iter().map(ToString::to_string));
    wanted.push(format!("{binary}.{target}.node"));
    let mut staged = Vec::with_capacity(wanted.len() + 1);
    for file in &wanted {
        if !locked.assets.contains_key(file) {
            if file.ends_with(".node") {
                return Err(NativeProviderError::NoBinaryForTarget {
                    package: package.clone(),
                    target: target.to_owned(),
                });
            }
            return Err(NativeProviderError::PinMissing {
                package: package.clone(),
                file: file.clone(),
            });
        }
        let url = github_asset_url(endpoints, repo, tag, file);
        let bytes = fetch_capped(client, &url, MAX_DOWNLOAD_BYTES).await?;
        verify_asset(package, locked, file, &bytes)?;
        write_source_file(dest, file, &bytes)?;
        staged.push(file.clone());
    }
    write_source_file(
        dest,
        "package.json",
        github_package_manifest(declaration, binary).as_bytes(),
    )?;
    staged.push("package.json".to_owned());
    Ok(staged)
}

/// Companion sources every GitHub release must carry next to the
/// `.node` assets: the loader entry and its types.
const GITHUB_COMPANION_FILES: [&str; 2] = ["index.js", "index.d.ts"];
