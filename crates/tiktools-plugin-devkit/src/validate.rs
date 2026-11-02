use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_json::json;
use tiktools_plugin_api::{HostCompat, PluginManifest, PluginRuntimeKind};

use crate::args::{print_json, Args, CommandResult};

pub(crate) fn manifest_path(target: Option<&str>) -> PathBuf {
    let path = PathBuf::from(target.unwrap_or("."));
    if path.is_dir() {
        path.join("plugin.json")
    } else {
        path
    }
}

pub(crate) fn load_manifest(path: &Path) -> Result<PluginManifest, String> {
    let text = fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let manifest = PluginManifest::from_json_str(&text)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    manifest
        .validate_compatibility()
        .map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(manifest)
}

pub(crate) fn publish_issues(
    manifest: &PluginManifest,
    directory: &Path,
    legacy_id: bool,
) -> Vec<String> {
    let mut issues = Vec::new();
    if manifest.semver().is_none() {
        issues.push(format!("version `{}` is not semver", manifest.version));
    }
    match manifest.metadata.publisher.as_deref() {
        None => issues.push("`publisher` is required to publish".to_owned()),
        Some(publisher) if !legacy_id && !manifest.id.starts_with(&format!("{publisher}.")) => {
            issues.push(format!(
                "id `{}` must start with `{publisher}.` (pass --legacy-id for grandfathered ids)",
                manifest.id
            ))
        }
        Some(_) => {}
    }
    if manifest.metadata.license.is_none() {
        issues.push("`license` (SPDX) is required to publish".to_owned());
    }
    if manifest.metadata.engine.is_none() {
        issues.push("`engines.tiktools` is required to publish".to_owned());
    }
    if manifest.metadata.categories.is_empty() {
        issues.push("at least one known `categories` entry is required to publish".to_owned());
    }
    if manifest.description.as_deref().is_none_or(str::is_empty) {
        issues.push("`description` is required to publish".to_owned());
    }
    if let Some(changelog) = &manifest.metadata.changelog {
        if !directory.join(changelog).is_file() {
            issues.push(format!("declared changelog `{changelog}` does not exist"));
        }
    }
    issues
}

pub(crate) fn warnings(manifest: &PluginManifest, directory: &Path) -> Vec<String> {
    let mut warnings = Vec::new();
    if manifest.metadata.repository.is_none() {
        warnings.push("`repository` helps reviewers audit the package".to_owned());
    }
    if manifest.runtime != PluginRuntimeKind::Declarative
        && !manifest.entry.is_empty()
        && !directory.join(&manifest.entry).is_file()
        && !directory.join(format!("{}.exe", manifest.entry)).is_file()
    {
        warnings.push(format!(
            "entry `{}` is not built yet; `tiktools-plugin build` or `pack --entry` supplies it",
            manifest.entry
        ));
    }
    if manifest.icon.is_none() {
        warnings.push("`icon` names a host icon shown on the plugin card".to_owned());
    }
    warnings
}

pub(crate) fn run(raw: &[String]) -> CommandResult {
    let args = Args::parse(raw);
    let path = manifest_path(args.positional(0));
    let manifest = load_manifest(&path)?;
    let directory = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let host = HostCompat::new(
        semver::Version::parse(env!("CARGO_PKG_VERSION")).unwrap_or(semver::Version::new(0, 0, 0)),
    );
    let issues = publish_issues(&manifest, &directory, args.has("--legacy-id"));
    let report = json!({
        "manifest": path.display().to_string(),
        "id": manifest.id,
        "version": manifest.version,
        "runtime": manifest.runtime.to_string(),
        "permissions": manifest.permissions,
        "capabilities": manifest.capabilities,
        "hostCompatibility": host.check_manifest(&manifest).err().map(|error| error.to_string()),
        "publishReady": issues.is_empty(),
        "publishIssues": issues,
        "warnings": warnings(&manifest, &directory),
    });
    print_json(&report);
    if args.has("--publish") && !issues.is_empty() {
        return Err("the manifest is not ready to publish".into());
    }
    Ok(())
}
