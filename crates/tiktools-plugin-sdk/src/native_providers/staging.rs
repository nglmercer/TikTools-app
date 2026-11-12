use super::*;

pub(super) fn write_source_file(
    dest: &Path,
    relative: &str,
    bytes: &[u8],
) -> Result<(), NativeProviderError> {
    let path = dest.join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| io_error(&path, error))?;
    }
    std::fs::write(&path, bytes).map_err(|error| io_error(&path, error))
}

/// Resolve `target`: an explicit triple, or the host triple when
/// omitted. Unknown triples fail; nothing is ever guessed.
pub fn resolve_target(target: Option<&str>) -> Result<String, NativeProviderError> {
    match target {
        Some(triple) if is_known_napi_target(triple) => Ok(triple.to_owned()),
        Some(triple) => Err(NativeProviderError::UnknownTarget {
            target: triple.to_owned(),
        }),
        None => Ok(current_napi_target()),
    }
}

/// Fetch every `nativeLibs` entry (optionally filtered to one provider)
/// for `target` and stage each tree into its `nativeAddons` root under
/// `plugin_dir`. Each library needs a lockfile entry and a same-package
/// `nativeAddons` twin; anything missing fails the whole operation
/// before anything is staged. `work_dir` holds transient downloads and
/// is cleaned afterwards.
pub async fn fetch_and_stage_native_libs(
    plan: &FetchAndStage<'_>,
) -> Result<Vec<NativeFetchReport>, NativeProviderError> {
    if !is_known_napi_target(plan.target) {
        return Err(NativeProviderError::UnknownTarget {
            target: plan.target.to_owned(),
        });
    }
    let selected = |entry: &&NativeLibDeclaration| {
        plan.provider_filter
            .is_none_or(|provider| entry.provider == provider)
    };
    // Validate the full plan before touching the plugin directory.
    for declaration in plan.manifest.native_libs.iter().filter(selected) {
        let locked = plan
            .lockfile
            .packages
            .get(&declaration.package)
            .ok_or_else(|| NativeProviderError::LockMissing(declaration.package.clone()))?;
        check_lock_matches(&declaration.package, declaration, locked)?;
        if !plan
            .manifest
            .native_addons
            .iter()
            .any(|addon| addon.package == declaration.package)
        {
            return Err(NativeProviderError::MissingAddonRoot {
                package: declaration.package.clone(),
            });
        }
    }
    let fetch_root = plan.work_dir.join("tiktools-native-fetch");
    let _ = std::fs::remove_dir_all(&fetch_root);
    let mut reports = Vec::new();
    for declaration in plan.manifest.native_libs.iter().filter(selected) {
        let locked = plan
            .lockfile
            .packages
            .get(&declaration.package)
            .expect("lockfile entry validated above");
        let root = plan
            .manifest
            .native_addons
            .iter()
            .find(|addon| addon.package == declaration.package)
            .expect("addon root validated above")
            .root
            .clone();
        let source = fetch_root.join(&declaration.package);
        let _ = std::fs::remove_dir_all(&source);
        let staged = fetch_native_lib(
            plan.client,
            plan.endpoints,
            declaration,
            locked,
            plan.target,
            &source,
        )
        .await
        .and_then(|_| {
            stage_native_package(&NativeStageRequest {
                package: &declaration.package,
                source: &source,
                plugin_dir: plan.plugin_dir,
                root: &root,
                overwrite: plan.overwrite,
            })
            .map_err(|error| NativeProviderError::Stage {
                package: declaration.package.clone(),
                message: error.to_string(),
            })
        });
        let _ = std::fs::remove_dir_all(&source);
        let report = staged?;
        reports.push(NativeFetchReport {
            package: declaration.package.clone(),
            version: declaration.version.clone(),
            target: plan.target.to_owned(),
            root,
            files: report.files,
        });
    }
    let _ = std::fs::remove_dir_all(&fetch_root);
    Ok(reports)
}
