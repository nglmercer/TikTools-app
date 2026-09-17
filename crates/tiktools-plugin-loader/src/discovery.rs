use std::{
    fs,
    path::{Path, PathBuf},
};

use tiktools_plugin_api::{
    manifest::{is_safe_relative_path, ManifestError},
    PluginManifest, PluginRuntimeKind,
};

use crate::{DiscoveredPlugin, PluginLoaderError, PluginRoot, PluginSource};

const MANIFEST_FILE: &str = "plugin.json";
pub(crate) const MAX_DIRECTORY_ENTRIES: usize = 4_096;
pub(crate) fn read_discovered_plugin(
    directory: &Path,
    source: PluginSource,
) -> Result<DiscoveredPlugin, PluginLoaderError> {
    let manifest_path = directory.join(MANIFEST_FILE);
    let bytes = fs::read(&manifest_path).map_err(|error| {
        PluginLoaderError::InvalidDirectory(format!("{}: {error}", manifest_path.display()))
    })?;
    if bytes.len() > 256 * 1024 {
        return Err(PluginLoaderError::Manifest(ManifestError::TooLarge));
    }
    let manifest =
        PluginManifest::from_json_str(std::str::from_utf8(&bytes).map_err(|_| {
            PluginLoaderError::InvalidDirectory("manifest is not UTF-8".to_owned())
        })?)?;
    let mut available = true;
    let mut reason = None;
    if let Err(error) = manifest.validate_compatibility() {
        available = false;
        reason = Some(error.to_string());
    } else if !manifest.target_matches_current_platform() {
        available = false;
        reason = Some("plugin has no build for this platform".to_owned());
    } else if manifest.runtime == PluginRuntimeKind::Declarative && manifest.entry.is_empty() {
        // Declarative packages have no executable entry; the manifest alone
        // is the integration, so there is no file to inspect.
    } else {
        let entry = manifest.entry.as_str();
        if !is_safe_relative_path(entry) {
            return Err(PluginLoaderError::Manifest(ManifestError::UnsafeEntry));
        }
        let package_root = fs::canonicalize(directory).map_err(|error| {
            PluginLoaderError::InvalidDirectory(format!("{}: {error}", directory.display()))
        })?;
        match fs::canonicalize(directory.join(entry)) {
            Ok(path) if path.starts_with(&package_root) && path.is_file() => {}
            Ok(_) => {
                available = false;
                reason = Some(format!("entry escapes the plugin directory: {entry}"));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                available = false;
                reason = Some(format!("entry does not exist: {entry}"));
            }
            Err(error) => {
                available = false;
                reason = Some(format!("entry could not be inspected: {error}"));
            }
        }
        if available && manifest.runtime == PluginRuntimeKind::Process && is_javascript_entry(entry)
        {
            available = false;
            reason = Some(
                "JavaScript plugin entries are not run by the desktop host; use a Rust ABI or standalone process plugin".to_owned(),
            );
        }
    }

    Ok(DiscoveredPlugin {
        manifest,
        directory: directory.to_owned(),
        source,
        available,
        reason,
        running: false,
    })
}

fn is_javascript_entry(entry: &str) -> bool {
    matches!(
        Path::new(entry)
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "js" | "mjs" | "cjs" | "ts"
    )
}

pub fn plugin_roots(
    builtin: impl Into<PathBuf>,
    user: impl Into<PathBuf>,
    development: Option<PathBuf>,
) -> Vec<PluginRoot> {
    let mut roots = vec![
        PluginRoot {
            path: builtin.into(),
            source: PluginSource::Builtin,
        },
        PluginRoot {
            path: user.into(),
            source: PluginSource::User,
        },
    ];
    if let Some(path) = development {
        roots.push(PluginRoot {
            path,
            source: PluginSource::Development,
        });
    }
    roots
}
