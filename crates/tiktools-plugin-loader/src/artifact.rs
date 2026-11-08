//! Bounded identity of executable package contents. Mutable `.data` belongs to
//! the host settings/storage contract and must never contribute secret hashes.
use std::{
    fs,
    io::{self, Read},
    path::Path,
};

use sha2::{Digest, Sha256};

use crate::{DiscoveredPlugin, PluginLoaderError};

const MAX_FILES: usize = 16_384;
const MAX_BYTES: u64 = 512 * 1024 * 1024;
const MAX_DEPTH: usize = 32;

/// Freeze reviewed bytes before admission. The destination must be a fresh,
/// host-owned private directory. The result is checked using the same bounded
/// complete-package digest; no original mutable `.data` is copied.
pub(crate) fn freeze_plugin_artifact(
    plugin: &DiscoveredPlugin,
    destination: &Path,
    expected: &str,
) -> Result<(), PluginLoaderError> {
    fn copy(
        source: &Path,
        target: &Path,
        depth: usize,
        files: &mut usize,
        bytes: &mut u64,
    ) -> Result<(), String> {
        if depth > MAX_DEPTH {
            return Err("package snapshot depth limit exceeded".into());
        }
        for entry in fs::read_dir(source).map_err(|_| "package snapshot source is unavailable")? {
            let entry = entry.map_err(|_| "package snapshot entry is unavailable")?;
            if depth == 0 && entry.file_name() == ".data" {
                continue;
            }
            *files += 1;
            if *files > MAX_FILES {
                return Err("package snapshot entry limit exceeded".into());
            }
            let kind = entry
                .file_type()
                .map_err(|_| "package snapshot entry kind is unavailable")?;
            let output = target.join(entry.file_name());
            if kind.is_dir() {
                fs::create_dir(&output)
                    .map_err(|_| "package snapshot directory could not be created")?;
                copy(&entry.path(), &output, depth + 1, files, bytes)?;
            } else if kind.is_file() {
                let input = fs::File::open(entry.path())
                    .map_err(|_| "package snapshot file is unavailable")?;
                let metadata = input
                    .metadata()
                    .map_err(|_| "package snapshot metadata is unavailable")?;
                if !metadata.is_file() {
                    return Err("package snapshot entry changed kind".into());
                }
                let size = metadata.len();
                *bytes = bytes
                    .checked_add(size)
                    .filter(|bytes| *bytes <= MAX_BYTES)
                    .ok_or("package snapshot byte limit exceeded")?;
                let mut output = fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(output)
                    .map_err(|_| "package snapshot output could not be created")?;
                let copied = io::copy(&mut input.take(size + 1), &mut output)
                    .map_err(|_| "package snapshot file could not be copied")?;
                if copied != size {
                    return Err("package snapshot file changed while copying".into());
                }
                output
                    .set_permissions(metadata.permissions())
                    .map_err(|_| "package snapshot file permissions could not be preserved")?;
            } else {
                return Err("package snapshot cannot contain links or special files".into());
            }
        }
        Ok(())
    }
    if plugin_artifact_fingerprint(plugin)? != expected {
        return Err(PluginLoaderError::RuntimeUnavailable(
            "reviewed plugin artifact changed".into(),
        ));
    }
    let source_root = fs::canonicalize(&plugin.directory).map_err(|_| {
        PluginLoaderError::RuntimeUnavailable("plugin package is unavailable".into())
    })?;
    let target_parent = destination
        .parent()
        .and_then(|parent| fs::canonicalize(parent).ok())
        .ok_or_else(|| {
            PluginLoaderError::RuntimeUnavailable(
                "plugin snapshot parent must already exist".into(),
            )
        })?;
    if target_parent.starts_with(&source_root) {
        return Err(PluginLoaderError::RuntimeUnavailable(
            "plugin snapshot cannot be nested inside its source package".into(),
        ));
    }
    #[allow(unused_mut)] // mode is a Unix-only builder method.
    let mut options = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        options.mode(0o700);
    }
    options.create(destination).map_err(|_| {
        PluginLoaderError::RuntimeUnavailable(
            "plugin invocation snapshot must be a fresh private directory".into(),
        )
    })?;
    let result = (|| {
        copy(&plugin.directory, destination, 0, &mut 0, &mut 0)
            .map_err(PluginLoaderError::RuntimeUnavailable)?;
        let mut frozen = plugin.clone();
        frozen.directory = destination.to_path_buf();
        if plugin_artifact_fingerprint(&frozen)? != expected {
            return Err(PluginLoaderError::RuntimeUnavailable(
                "plugin artifact changed while freezing invocation".into(),
            ));
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(destination);
    }
    result
}

/// Hash the complete confined package tree, including manifest, scripts and
/// native libraries. Symlinks and special files fail closed. This is an
/// identity check for trusted installed code, not an OS sandbox: trusted
/// native code may still load libraries or data outside its package.
pub fn plugin_artifact_fingerprint(plugin: &DiscoveredPlugin) -> Result<String, PluginLoaderError> {
    let mutable_entry = |path: &str| {
        path.split(['/', '\\'])
            .find(|part| !part.is_empty() && *part != ".")
            == Some(".data")
    };
    if mutable_entry(&plugin.manifest.entry)
        || plugin
            .manifest
            .native_addons
            .iter()
            .any(|addon| mutable_entry(&addon.root))
    {
        return Err(PluginLoaderError::RuntimeUnavailable(
            "plugin executable entries cannot live in mutable .data storage".into(),
        ));
    }
    fn walk(
        directory: &Path,
        root: &Path,
        depth: usize,
        files: &mut usize,
        bytes: &mut u64,
        digest: &mut Sha256,
    ) -> Result<(), String> {
        if depth > MAX_DEPTH {
            return Err("package depth exceeds the trusted artifact limit".into());
        }
        let mut entries = fs::read_dir(directory)
            .map_err(|_| "package contents could not be read")?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| "package entry could not be read")?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            // Process runtimes place mutable settings/storage here. It is not
            // executable package metadata and may contain secrets.
            if depth == 0 && entry.file_name() == ".data" {
                continue;
            }
            *files += 1;
            if *files > MAX_FILES {
                return Err("package entry count exceeds the trusted artifact limit".into());
            }
            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .ok()
                .and_then(Path::to_str)
                .filter(|path| path.len() <= 4096)
                .ok_or("package path is not a bounded UTF-8 path")?;
            let kind = entry
                .file_type()
                .map_err(|_| "package entry kind is unavailable")?;
            digest.update((relative.len() as u64).to_be_bytes());
            digest.update(relative.as_bytes());
            if kind.is_dir() {
                digest.update(b"directory\0");
                walk(&path, root, depth + 1, files, bytes, digest)?;
            } else if kind.is_file() {
                digest.update(b"file\0");
                let mut file = fs::File::open(&path).map_err(|_| "package file is unavailable")?;
                let size = file
                    .metadata()
                    .map_err(|_| "package file metadata is unavailable")?
                    .len();
                *bytes = bytes
                    .checked_add(size)
                    .filter(|size| *size <= MAX_BYTES)
                    .ok_or("package bytes exceed the trusted artifact limit")?;
                digest.update(size.to_be_bytes());
                let mut read = 0_u64;
                let mut buffer = [0_u8; 64 * 1024];
                loop {
                    let count = file
                        .read(&mut buffer)
                        .map_err(|_| "package file could not be read")?;
                    if count == 0 {
                        break;
                    }
                    read = read
                        .checked_add(count as u64)
                        .filter(|read| *read <= size)
                        .ok_or("package file changed during artifact verification")?;
                    digest.update(&buffer[..count]);
                }
                if read != size {
                    return Err("package file changed during artifact verification".into());
                }
            } else {
                return Err("package symlinks and special files cannot be approved".into());
            }
        }
        Ok(())
    }
    let metadata = fs::symlink_metadata(&plugin.directory).map_err(|_| {
        PluginLoaderError::RuntimeUnavailable("plugin package is unavailable".into())
    })?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(PluginLoaderError::RuntimeUnavailable(
            "plugin package must be a real directory".into(),
        ));
    }
    let mut digest = Sha256::new();
    digest.update(b"tiktools/trusted-plugin-artifact/v1\0");
    walk(
        &plugin.directory,
        &plugin.directory,
        0,
        &mut 0,
        &mut 0,
        &mut digest,
    )
    .map_err(PluginLoaderError::RuntimeUnavailable)?;
    Ok(digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PluginSource;
    use tiktools_plugin_api::PluginManifest;

    #[test]
    fn artifact_covers_manifest_dependencies_and_native_bytes_but_not_mutable_settings() {
        let root = std::env::temp_dir().join(format!(
            "tiktools-artifact-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("native")).unwrap();
        fs::write(root.join("entry.bin"), b"entry").unwrap();
        fs::write(root.join("native/addon.node"), b"native-a").unwrap();
        let manifest = PluginManifest::from_json_str(r#"{"schemaVersion":3,"id":"artifact","name":"Artifact","version":"1.0.0","runtime":"process","entry":"entry.bin"}"#).unwrap();
        let plugin = DiscoveredPlugin {
            manifest,
            directory: root.clone(),
            source: PluginSource::Development,
            available: true,
            reason: None,
            running: false,
        };
        let initial = plugin_artifact_fingerprint(&plugin).unwrap();
        fs::create_dir_all(root.join(".data")).unwrap();
        fs::write(
            root.join(".data/settings.json"),
            b"secret settings must never be hashed",
        )
        .unwrap();
        assert_eq!(initial, plugin_artifact_fingerprint(&plugin).unwrap());
        fs::write(root.join("native/addon.node"), b"native-b").unwrap();
        assert_ne!(initial, plugin_artifact_fingerprint(&plugin).unwrap());
        let current = plugin_artifact_fingerprint(&plugin).unwrap();
        assert!(freeze_plugin_artifact(&plugin, &root.join("nested-snapshot"), &current).is_err());
        for entry in [".data/entry.bin", "./.data/entry.bin", ".data\\entry.bin"] {
            let mut mutable = plugin.clone();
            mutable.manifest.entry = entry.into();
            assert!(plugin_artifact_fingerprint(&mutable).is_err());
        }
        let mut mutable = plugin.clone();
        mutable
            .manifest
            .native_addons
            .push(tiktools_plugin_api::NativeAddonDeclaration {
                package: "addon".into(),
                root: ".data/addon".into(),
            });
        assert!(plugin_artifact_fingerprint(&mutable).is_err());
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn artifact_rejects_links_outside_or_inside_package() {
        let root = std::env::temp_dir().join(format!(
            "tiktools-artifact-link-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("entry.bin"), b"entry").unwrap();
        std::os::unix::fs::symlink("entry.bin", root.join("link.bin")).unwrap();
        let manifest = PluginManifest::from_json_str(r#"{"schemaVersion":3,"id":"artifact","name":"Artifact","version":"1.0.0","runtime":"process","entry":"entry.bin"}"#).unwrap();
        let plugin = DiscoveredPlugin {
            manifest,
            directory: root.clone(),
            source: PluginSource::Development,
            available: true,
            reason: None,
            running: false,
        };
        assert!(plugin_artifact_fingerprint(&plugin).is_err());
        let _ = fs::remove_dir_all(root);
    }
}
