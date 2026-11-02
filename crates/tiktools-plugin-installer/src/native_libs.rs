use std::path::Path;

use tiktools_plugin_api::PluginManifest;

use crate::{InstallError, InstallErrorCode};

#[cfg(feature = "native-providers")]
pub(crate) fn backfill_native_libs(
    manifest: &PluginManifest,
    package_root: &Path,
    work_dir: &Path,
    endpoints: Option<&(String, String)>,
) -> Result<(), InstallError> {
    use tiktools_plugin_api::manifest::{
        current_napi_target, select_host_native_binary, NativeBinarySelectError,
    };
    use tiktools_plugin_sdk::native_providers::{
        fetch_and_stage_native_libs, provider_client, read_lockfile, FetchAndStage,
        ProviderEndpoints,
    };

    if manifest.native_libs.is_empty() {
        return Ok(());
    }
    let mut missing = Vec::new();
    for declaration in &manifest.native_libs {
        let Some(addon) = manifest
            .native_addons
            .iter()
            .find(|addon| addon.package == declaration.package)
        else {
            continue;
        };
        match select_host_native_binary(&package_root.join(&addon.root)) {
            Ok(_) => continue,
            Err(
                NativeBinarySelectError::NoHostBinary { .. }
                | NativeBinarySelectError::NotADirectory(_),
            ) => missing.push(declaration.clone()),
            Err(_) => continue,
        }
    }
    if missing.is_empty() {
        return Ok(());
    }
    let lockfile_path = package_root.join("native-libs.lock.json");
    if !lockfile_path.is_file() {
        let names: Vec<_> = missing.iter().map(|entry| entry.package.as_str()).collect();
        return Err(invalid(format!(
            "plugin archive ships no {} binary for {} and no native-libs.lock.json to fetch it from",
            current_napi_target(),
            names.join(", "),
        )));
    }
    let manifest = PluginManifest {
        native_libs: missing,
        ..manifest.clone()
    };
    let lockfile = read_lockfile(&lockfile_path).map_err(|error| invalid(error.to_string()))?;
    let endpoints = match endpoints {
        Some((npm_registry, github_base)) => ProviderEndpoints {
            npm_registry: npm_registry.clone(),
            github_base: github_base.clone(),
        },
        None => ProviderEndpoints::officials(),
    };
    let target = current_napi_target();
    std::thread::scope(|scope| {
        scope
            .spawn(|| {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|error| {
                        fetch_error(format!("cannot start native-library fetch: {error}"))
                    })?;
                runtime.block_on(async {
                    let client =
                        provider_client().map_err(|error| fetch_error(error.to_string()))?;
                    fetch_and_stage_native_libs(&FetchAndStage {
                        client: &client,
                        endpoints: &endpoints,
                        manifest: &manifest,
                        lockfile: &lockfile,
                        target: &target,
                        plugin_dir: package_root,
                        work_dir,
                        overwrite: true,
                        provider_filter: None,
                    })
                    .await
                    .map_err(|error| fetch_error(error.to_string()))?;
                    for declaration in &manifest.native_libs {
                        let Some(addon) = manifest
                            .native_addons
                            .iter()
                            .find(|addon| addon.package == declaration.package)
                        else {
                            continue;
                        };
                        select_host_native_binary(&package_root.join(&addon.root)).map_err(
                            |error| {
                                invalid(format!(
                                    "fetched native library failed validation: {error}"
                                ))
                            },
                        )?;
                    }
                    Ok::<(), InstallError>(())
                })
            })
            .join()
            .unwrap_or_else(|_| Err(fetch_error("native-library fetch thread failed".to_owned())))
    })
}

#[cfg(not(feature = "native-providers"))]
pub(crate) fn backfill_native_libs(
    manifest: &PluginManifest,
    package_root: &Path,
    _work_dir: &Path,
    _endpoints: Option<&(String, String)>,
) -> Result<(), InstallError> {
    use tiktools_plugin_api::manifest::{select_host_native_binary, NativeBinarySelectError};

    for declaration in &manifest.native_libs {
        let Some(addon) = manifest
            .native_addons
            .iter()
            .find(|addon| addon.package == declaration.package)
        else {
            continue;
        };
        if matches!(
            select_host_native_binary(&package_root.join(&addon.root)),
            Err(NativeBinarySelectError::NoHostBinary { .. }
                | NativeBinarySelectError::NotADirectory(_))
        ) {
            return Err(invalid(format!(
                "plugin archive ships no binary for {} and this build disables native-library fetch (rebuild with the native-providers feature)",
                declaration.package,
            )));
        }
    }
    Ok(())
}

fn invalid(message: String) -> InstallError {
    InstallError::new(InstallErrorCode::InvalidPackage, message)
}

#[cfg(feature = "native-providers")]
fn fetch_error(message: String) -> InstallError {
    InstallError::new(InstallErrorCode::DownloadFailed, message)
}
