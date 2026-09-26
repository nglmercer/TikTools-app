//! Safe `.plugin` package installation.
//!
//! Installation is deliberately a data operation: no package manager is
//! invoked and no native code is compiled. The archive is extracted to a
//! private staging directory, validated, checksum-checked, and then renamed
//! into the runtime plugin directory.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use sha2::{Digest, Sha256};
use tiktools_plugin_api::{
    manifest::{is_safe_relative_path, ManifestError},
    PluginManifest,
};
use zip::ZipArchive;

use crate::PluginLoaderError;

const MAX_ARCHIVE_BYTES: u64 = 512 * 1024 * 1024;
const MAX_EXTRACTED_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAX_FILES: usize = 50_000;

pub struct PluginInstaller {
    pub plugin_directory: PathBuf,
    pub staging_directory: PathBuf,
    pub replace_existing: bool,
    /// Override fetch bases for declarative native libraries as
    /// `(npm_registry, github_base)`. `None` uses the official
    /// endpoints. Only consulted when an archive declares `nativeLibs`
    /// but lacks the install host's binary; tests point this at a stub
    /// origin, mirrors may point it at a local registry.
    pub provider_endpoints: Option<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct InstalledPluginPackage {
    pub directory: PathBuf,
    pub manifest: PluginManifest,
}

impl PluginInstaller {
    /// Reads and validates the package identity before filesystem mutation.
    /// Full extraction, checksum validation, and atomic replacement still
    /// happen in [`Self::install`]; this lightweight pass only lets the host
    /// stop a running instance belonging to the package being replaced.
    pub fn inspect_manifest(
        &self,
        archive_path: impl AsRef<Path>,
    ) -> Result<PluginManifest, PluginLoaderError> {
        let archive = canonical_archive_path(archive_path.as_ref())?;
        let file = File::open(&archive).map_err(io_error)?;
        let mut archive = ZipArchive::new(file).map_err(|error| {
            PluginLoaderError::Runtime(format!("could not inspect plugin archive: {error}"))
        })?;
        let mut manifest_bytes = None;
        for index in 0..archive.len().min(MAX_FILES) {
            let entry = archive.by_index(index).map_err(zip_error)?;
            let Some(relative) = normalized_archive_path(entry.name())? else {
                continue;
            };
            let is_manifest = relative.file_name().and_then(|name| name.to_str())
                == Some("plugin.json")
                && relative.components().count() <= 2;
            if !is_manifest {
                continue;
            }
            if manifest_bytes.is_some() {
                return Err(PluginLoaderError::Runtime(
                    "plugin archive contains multiple plugin.json manifests".to_owned(),
                ));
            }
            let mut bytes = Vec::new();
            entry
                .take(256 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(io_error)?;
            if bytes.len() > 256 * 1024 {
                return Err(PluginLoaderError::Manifest(ManifestError::TooLarge));
            }
            manifest_bytes = Some(bytes);
        }
        let bytes = manifest_bytes.ok_or_else(|| {
            PluginLoaderError::Runtime(
                "plugin archive must contain plugin.json at its root".to_owned(),
            )
        })?;
        let manifest =
            PluginManifest::from_json_str(std::str::from_utf8(&bytes).map_err(|_| {
                PluginLoaderError::Runtime("plugin manifest is not UTF-8".to_owned())
            })?)
            .map_err(|error| PluginLoaderError::Runtime(error.to_string()))?;
        manifest
            .validate_compatibility()
            .map_err(|error| PluginLoaderError::Runtime(error.to_string()))?;
        Ok(manifest)
    }

    pub fn install(
        &self,
        archive_path: impl AsRef<Path>,
    ) -> Result<InstalledPluginPackage, PluginLoaderError> {
        let archive = canonical_archive_path(archive_path.as_ref())?;
        let archive_size = fs::metadata(&archive).map_err(io_error)?.len();
        if archive_size > MAX_ARCHIVE_BYTES {
            return Err(PluginLoaderError::Runtime(
                "plugin package exceeds the 512 MB limit".to_owned(),
            ));
        }

        fs::create_dir_all(&self.plugin_directory).map_err(io_error)?;
        fs::create_dir_all(&self.staging_directory).map_err(io_error)?;
        let staging = self.staging_path();
        fs::create_dir_all(&staging).map_err(io_error)?;
        let result = self.install_into_staging(&archive, &staging);
        if result.is_err() {
            let _ = fs::remove_dir_all(&staging);
        }
        result
    }

    fn install_into_staging(
        &self,
        archive_path: &Path,
        staging: &Path,
    ) -> Result<InstalledPluginPackage, PluginLoaderError> {
        let file = File::open(archive_path).map_err(io_error)?;
        let mut archive = ZipArchive::new(file).map_err(|error| {
            PluginLoaderError::Runtime(format!("could not inspect plugin archive: {error}"))
        })?;
        if archive.is_empty() || archive.len() > MAX_FILES {
            return Err(PluginLoaderError::Runtime(
                "plugin archive contains an invalid number of entries".to_owned(),
            ));
        }
        let mut extracted_bytes = 0_u64;
        let mut extracted_paths = BTreeSet::new();
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index).map_err(zip_error)?;
            let Some(relative) = normalized_archive_path(entry.name())? else {
                // Some ZIP writers (including bsdtar) include the archive's
                // current-directory marker as `./`. It is not package data.
                continue;
            };
            if !extracted_paths.insert(relative.clone()) {
                return Err(PluginLoaderError::Runtime(format!(
                    "plugin archive contains a duplicate path: {}",
                    entry.name()
                )));
            }
            let destination = staging.join(&relative);
            if entry.is_dir() {
                fs::create_dir_all(&destination).map_err(io_error)?;
                continue;
            }
            extracted_bytes = extracted_bytes.saturating_add(entry.size());
            if extracted_bytes > MAX_EXTRACTED_BYTES {
                return Err(PluginLoaderError::Runtime(
                    "plugin archive expands beyond the 2 GB limit".to_owned(),
                ));
            }
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent).map_err(io_error)?;
            }
            let mut output = File::create(&destination).map_err(io_error)?;
            io::copy(&mut entry, &mut output).map_err(io_error)?;
        }

        let root = find_package_root(staging)?;
        validate_extracted_tree(&root)?;
        let manifest_path = root.join("plugin.json");
        let manifest_text = fs::read_to_string(&manifest_path).map_err(io_error)?;
        let manifest = PluginManifest::from_json_str(&manifest_text)
            .map_err(|error| PluginLoaderError::Runtime(error.to_string()))?;
        manifest
            .validate_compatibility()
            .map_err(|error| PluginLoaderError::Runtime(error.to_string()))?;
        verify_checksums(&root, &manifest)?;
        self.backfill_native_libs(&manifest, &root, staging)?;
        if root.join("signature.json").is_file() {
            return Err(PluginLoaderError::Runtime(
                "signed plugin packages require a configured signature verifier".to_owned(),
            ));
        }

        let target = self.plugin_directory.join(&manifest.id);
        if target.parent() != Some(self.plugin_directory.as_path()) {
            return Err(PluginLoaderError::Runtime(
                "plugin installation target escaped the plugin directory".to_owned(),
            ));
        }
        if target.exists() && !self.replace_existing {
            return Err(PluginLoaderError::Runtime(format!(
                "plugin is already installed: {}",
                manifest.id
            )));
        }

        let backup =
            self.plugin_directory
                .join(format!(".{}.previous-{}", manifest.id, unique_suffix()));
        let mut moved_existing = false;
        if target.exists() {
            fs::rename(&target, &backup).map_err(io_error)?;
            moved_existing = true;
        }
        if let Err(error) = fs::rename(&root, &target) {
            if moved_existing && !target.exists() {
                let _ = fs::rename(&backup, &target);
            }
            return Err(io_error(error));
        }
        if moved_existing {
            fs::remove_dir_all(&backup).map_err(io_error)?;
        }
        let result = InstalledPluginPackage {
            directory: target,
            manifest,
        };
        let _ = fs::remove_dir_all(staging);
        Ok(result)
    }

    fn staging_path(&self) -> PathBuf {
        self.staging_directory
            .join(format!("plugin-{}-{}", std::process::id(), unique_suffix()))
    }

    /// Complete native trees the archive did not ship. For every
    /// declared library whose `nativeAddons` root lacks this host's
    /// `.node` binary, replay the shipped lockfile and fetch exactly
    /// that binary from its pinned provider. Complete trees are never
    /// touched (no network, no rewrite); ambiguous or unreadable trees
    /// are left for the loader's fail-closed selection; libraries
    /// without an addon twin are ignored here (the staging tool rejects
    /// them at build time) and stay unloadable as before.
    #[cfg(feature = "native-providers")]
    fn backfill_native_libs(
        &self,
        manifest: &PluginManifest,
        staging_root: &Path,
        staging: &Path,
    ) -> Result<(), PluginLoaderError> {
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
            match select_host_native_binary(&staging_root.join(&addon.root)) {
                Ok(_) => continue,
                Err(
                    NativeBinarySelectError::NoHostBinary { .. }
                    | NativeBinarySelectError::NotADirectory(_),
                ) => missing.push(declaration.clone()),
                // Ambiguous or unreadable trees keep today's behavior:
                // installation succeeds and loading fails closed.
                Err(_) => continue,
            }
        }
        if missing.is_empty() {
            return Ok(());
        }
        let lockfile_path = staging_root.join("native-libs.lock.json");
        if !lockfile_path.is_file() {
            let names: Vec<_> = missing.iter().map(|entry| entry.package.as_str()).collect();
            return Err(PluginLoaderError::Runtime(format!(
                "plugin archive ships no {} binary for {} and no native-libs.lock.json to fetch it from",
                current_napi_target(),
                names.join(", "),
            )));
        }
        let manifest = PluginManifest {
            native_libs: missing,
            ..manifest.clone()
        };
        let lockfile = read_lockfile(&lockfile_path)
            .map_err(|error| PluginLoaderError::Runtime(error.to_string()))?;
        let endpoints = match &self.provider_endpoints {
            Some((npm_registry, github_base)) => ProviderEndpoints {
                npm_registry: npm_registry.clone(),
                github_base: github_base.clone(),
            },
            None => ProviderEndpoints::officials(),
        };
        let target = current_napi_target();
        let staging_root = staging_root.to_owned();
        let work_dir = staging.to_owned();
        // Installation is synchronous and may run inside an async
        // context, so the fetch gets a dedicated thread with its own
        // current-thread runtime instead of assuming a context.
        std::thread::scope(|scope| {
            scope
                .spawn(|| {
                    let runtime = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .map_err(|error| {
                            PluginLoaderError::Runtime(format!(
                                "cannot start native-library fetch: {error}"
                            ))
                        })?;
                    runtime.block_on(async {
                        let client = provider_client()
                            .map_err(|error| PluginLoaderError::Runtime(error.to_string()))?;
                        fetch_and_stage_native_libs(&FetchAndStage {
                            client: &client,
                            endpoints: &endpoints,
                            manifest: &manifest,
                            lockfile: &lockfile,
                            target: &target,
                            plugin_dir: &staging_root,
                            work_dir: &work_dir,
                            overwrite: true,
                            provider_filter: None,
                        })
                        .await
                        .map_err(|error| PluginLoaderError::Runtime(error.to_string()))?;
                        // Confirm every backfilled root now selects.
                        for declaration in &manifest.native_libs {
                            let addon = manifest
                                .native_addons
                                .iter()
                                .find(|addon| addon.package == declaration.package)
                                .expect("twin checked above");
                            select_host_native_binary(&staging_root.join(&addon.root)).map_err(
                                |error| {
                                    PluginLoaderError::Runtime(format!(
                                        "fetched native library failed validation: {error}"
                                    ))
                                },
                            )?;
                        }
                        Ok::<(), PluginLoaderError>(())
                    })
                })
                .join()
                .unwrap_or_else(|_| {
                    Err(PluginLoaderError::Runtime(
                        "native-library fetch thread failed".to_owned(),
                    ))
                })
        })
    }

    /// Builds without the fetch feature install complete trees
    /// unchanged; archives that need a backfill fail with a clear
    /// rebuild hint instead of a confusing load-time error.
    #[cfg(not(feature = "native-providers"))]
    fn backfill_native_libs(
        &self,
        manifest: &PluginManifest,
        staging_root: &Path,
        _staging: &Path,
    ) -> Result<(), PluginLoaderError> {
        use tiktools_plugin_api::manifest::{select_host_native_binary, NativeBinarySelectError};

        if manifest.native_libs.is_empty() {
            return Ok(());
        }
        for declaration in &manifest.native_libs {
            let Some(addon) = manifest
                .native_addons
                .iter()
                .find(|addon| addon.package == declaration.package)
            else {
                continue;
            };
            if matches!(
                select_host_native_binary(&staging_root.join(&addon.root)),
                Err(NativeBinarySelectError::NoHostBinary { .. }
                    | NativeBinarySelectError::NotADirectory(_))
            ) {
                return Err(PluginLoaderError::Runtime(format!(
                    "plugin archive ships no binary for {} and this build disables native-library fetch (rebuild with the native-providers feature)",
                    declaration.package,
                )));
            }
        }
        Ok(())
    }
}

fn canonical_archive_path(path: &Path) -> Result<PathBuf, PluginLoaderError> {
    let archive = fs::canonicalize(path).map_err(io_error)?;
    if archive
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.eq_ignore_ascii_case("plugin"))
        != Some(true)
    {
        return Err(PluginLoaderError::Runtime(
            "plugin packages must use the .plugin extension".to_owned(),
        ));
    }
    if !archive.is_file() {
        return Err(PluginLoaderError::Runtime(
            "plugin package is not a regular file".to_owned(),
        ));
    }
    let archive_size = fs::metadata(&archive).map_err(io_error)?.len();
    if archive_size > MAX_ARCHIVE_BYTES {
        return Err(PluginLoaderError::Runtime(
            "plugin package exceeds the 512 MB limit".to_owned(),
        ));
    }
    Ok(archive)
}

fn normalized_archive_path(name: &str) -> Result<Option<PathBuf>, PluginLoaderError> {
    let normalized = name.replace('\\', "/");
    let normalized = normalized.strip_prefix("./").unwrap_or(&normalized);
    let normalized = normalized.trim_end_matches('/');
    if normalized.is_empty() {
        return Ok(None);
    }
    if !is_safe_relative_path(normalized) {
        return Err(PluginLoaderError::Runtime(format!(
            "plugin archive contains unsafe path: {name}"
        )));
    }
    Ok(Some(PathBuf::from(normalized)))
}

fn find_package_root(staging: &Path) -> Result<PathBuf, PluginLoaderError> {
    if staging.join("plugin.json").is_file() {
        return Ok(staging.to_owned());
    }
    let directories: Vec<_> = fs::read_dir(staging)
        .map_err(io_error)?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false))
        .collect();
    if directories.len() == 1 && directories[0].path().join("plugin.json").is_file() {
        return Ok(directories[0].path());
    }
    Err(PluginLoaderError::Runtime(
        "plugin archive must contain plugin.json at its root".to_owned(),
    ))
}

fn validate_extracted_tree(root: &Path) -> Result<(), PluginLoaderError> {
    let root = fs::canonicalize(root).map_err(io_error)?;
    let mut files = 0_usize;
    let mut bytes = 0_u64;
    visit_tree(&root, &root, &mut files, &mut bytes)
}

fn visit_tree(
    root: &Path,
    directory: &Path,
    files: &mut usize,
    bytes: &mut u64,
) -> Result<(), PluginLoaderError> {
    for entry in fs::read_dir(directory).map_err(io_error)? {
        let entry = entry.map_err(io_error)?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(io_error)?;
        if metadata.file_type().is_symlink() {
            return Err(PluginLoaderError::Runtime(
                "plugin archive contains a symbolic link".to_owned(),
            ));
        }
        if metadata.is_dir() {
            visit_tree(root, &path, files, bytes)?;
            continue;
        }
        if !metadata.is_file() {
            return Err(PluginLoaderError::Runtime(
                "plugin archive contains an unsupported entry".to_owned(),
            ));
        }
        if !path.starts_with(root) {
            return Err(PluginLoaderError::Runtime(
                "plugin archive contains a path traversal entry".to_owned(),
            ));
        }
        *files = files.saturating_add(1);
        *bytes = bytes.saturating_add(metadata.len());
        if *files > MAX_FILES {
            return Err(PluginLoaderError::Runtime(
                "plugin archive contains too many files".to_owned(),
            ));
        }
        if *bytes > MAX_EXTRACTED_BYTES {
            return Err(PluginLoaderError::Runtime(
                "plugin archive expands beyond the 2 GB limit".to_owned(),
            ));
        }
    }
    Ok(())
}

fn verify_checksums(root: &Path, manifest: &PluginManifest) -> Result<(), PluginLoaderError> {
    let checksum_path = root.join("checksums.json");
    let checksums_text = fs::read_to_string(&checksum_path).map_err(|_| {
        PluginLoaderError::Runtime(format!("plugin {} is missing checksums.json", manifest.id))
    })?;
    let checksums: BTreeMap<String, String> =
        serde_json::from_str(&checksums_text).map_err(|error| {
            PluginLoaderError::Runtime(format!("checksums.json is invalid: {error}"))
        })?;
    if checksums.is_empty() {
        return Err(PluginLoaderError::Runtime(format!(
            "plugin {} has no checksums",
            manifest.id
        )));
    }
    for (relative, expected) in &checksums {
        if !is_safe_relative_path(relative)
            || expected.len() != 64
            || !expected.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(PluginLoaderError::Runtime(format!(
                "invalid checksum entry: {relative}"
            )));
        }
        let path = root.join(relative);
        if !path.is_file() {
            return Err(PluginLoaderError::Runtime(format!(
                "checksum target is not a file: {relative}"
            )));
        }
        let digest = digest_file(&path)?;
        if !digest.eq_ignore_ascii_case(expected) {
            return Err(PluginLoaderError::Runtime(format!(
                "checksum mismatch in {}: {relative}",
                manifest.id
            )));
        }
    }
    for relative in all_files(root)? {
        if matches!(relative.as_str(), "checksums.json" | "signature.json") {
            continue;
        }
        if !checksums.contains_key(&relative) {
            return Err(PluginLoaderError::Runtime(format!(
                "plugin checksum is missing for {relative}"
            )));
        }
    }
    Ok(())
}

fn all_files(root: &Path) -> Result<Vec<String>, PluginLoaderError> {
    let mut files = Vec::new();
    collect_files(root, root, &mut files)?;
    Ok(files)
}

fn collect_files(
    root: &Path,
    directory: &Path,
    files: &mut Vec<String>,
) -> Result<(), PluginLoaderError> {
    for entry in fs::read_dir(directory).map_err(io_error)? {
        let path = entry.map_err(io_error)?.path();
        if path.is_dir() {
            collect_files(root, &path, files)?;
        } else if path.is_file() {
            let relative = path
                .strip_prefix(root)
                .map_err(|error| PluginLoaderError::Runtime(error.to_string()))?
                .to_string_lossy()
                .replace('\\', "/");
            files.push(relative);
        }
    }
    Ok(())
}

fn digest_file(path: &Path) -> Result<String, PluginLoaderError> {
    let mut file = File::open(path).map_err(io_error)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(io_error)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn unique_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
}

fn io_error(error: io::Error) -> PluginLoaderError {
    PluginLoaderError::Runtime(error.to_string())
}

fn zip_error(error: zip::result::ZipError) -> PluginLoaderError {
    PluginLoaderError::Runtime(format!("could not read plugin archive: {error}"))
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        io::Write,
        time::{SystemTime, UNIX_EPOCH},
    };

    use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

    use super::*;

    fn temp_root() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let count = COUNTER.fetch_add(1, Ordering::AcqRel);
        std::env::temp_dir().join(format!(
            "tiktools-plugin-installer-{}-{}-{count}",
            std::process::id(),
            suffix
        ))
    }

    fn digest_bytes(bytes: &[u8]) -> String {
        format!("{:x}", Sha256::digest(bytes))
    }

    fn write_archive(path: &Path, files: &[(&str, &[u8])]) {
        let file = File::create(path).unwrap();
        let mut writer = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        for (name, bytes) in files {
            writer.start_file(*name, options).unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap();
    }

    fn demo_package(manifest: &[u8], entry: &[u8]) -> (Vec<u8>, Vec<u8>, String) {
        let checksums = format!(
            r#"{{"plugin.json":"{}","index.js":"{}"}}"#,
            digest_bytes(manifest),
            digest_bytes(entry)
        );
        (manifest.to_vec(), entry.to_vec(), checksums)
    }

    #[test]
    fn rejects_archive_path_traversal() {
        assert!(normalized_archive_path("../../outside").is_err());
        assert!(normalized_archive_path("plugin\\..\\outside").is_err());
        assert!(normalized_archive_path("./plugin.json").unwrap().is_some());
        assert!(normalized_archive_path("./").unwrap().is_none());
    }

    #[test]
    fn installs_checksum_checked_package_atomically() {
        let root = temp_root();
        fs::create_dir_all(&root).unwrap();
        let archive_path = root.join("demo.plugin");
        let manifest = br#"{"schemaVersion":2,"id":"demo","name":"Demo","version":"1.0.0","runtime":"process","entry":"index.js"}"#;
        let entry = br#"{"ready":true}"#;
        let checksums = format!(
            r#"{{"plugin.json":"{}","index.js":"{}"}}"#,
            digest_bytes(manifest),
            digest_bytes(entry)
        );

        let file = File::create(&archive_path).unwrap();
        let mut writer = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        writer.start_file("plugin.json", options).unwrap();
        writer.write_all(manifest).unwrap();
        writer.start_file("index.js", options).unwrap();
        writer.write_all(entry).unwrap();
        writer.start_file("checksums.json", options).unwrap();
        writer.write_all(checksums.as_bytes()).unwrap();
        writer.finish().unwrap();

        let installed = PluginInstaller {
            plugin_directory: root.join("plugins"),
            staging_directory: root.join("staging"),
            replace_existing: false,
            provider_endpoints: None,
        }
        .install(&archive_path)
        .unwrap();

        assert_eq!(installed.manifest.id, "demo");
        assert_eq!(installed.directory, root.join("plugins/demo"));
        assert!(installed.directory.join("plugin.json").is_file());
        assert!(installed.directory.join("index.js").is_file());
        assert!(!root.join("staging").join("plugin.json").exists());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_wrong_extension() {
        let root = temp_root();
        fs::create_dir_all(&root).unwrap();
        let archive_path = root.join("demo.zip");
        let manifest = br#"{"schemaVersion":2,"id":"demo","name":"Demo","version":"1.0.0","runtime":"process","entry":"index.js"}"#;
        let entry = br#"{"ready":true}"#;
        let (_, _, checksums) = demo_package(manifest, entry);
        write_archive(
            &archive_path,
            &[
                ("plugin.json", manifest),
                ("index.js", entry),
                ("checksums.json", checksums.as_bytes()),
            ],
        );

        let result = PluginInstaller {
            plugin_directory: root.join("plugins"),
            staging_directory: root.join("staging"),
            replace_existing: false,
            provider_endpoints: None,
        }
        .install(&archive_path);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains(".plugin extension"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_missing_manifest_and_cleans_staging() {
        let root = temp_root();
        fs::create_dir_all(&root).unwrap();
        let archive_path = root.join("demo.plugin");
        write_archive(&archive_path, &[("index.js", b"{}")]);

        let staging = root.join("staging");
        let result = PluginInstaller {
            plugin_directory: root.join("plugins"),
            staging_directory: staging.clone(),
            replace_existing: false,
            provider_endpoints: None,
        }
        .install(&archive_path);
        assert!(result.is_err());
        assert!(!root.join("plugins/demo").exists());
        // Staging extracts are removed on failure; only the staging root may remain.
        let leftover: Vec<_> = fs::read_dir(&staging)
            .map(|entries| entries.filter_map(Result::ok).collect())
            .unwrap_or_default();
        assert!(leftover.is_empty(), "staging was not cleaned: {leftover:?}");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_unsafe_traversal_entries() {
        let root = temp_root();
        fs::create_dir_all(&root).unwrap();
        let archive_path = root.join("evil.plugin");
        let manifest = br#"{"schemaVersion":2,"id":"demo","name":"Demo","version":"1.0.0","runtime":"process","entry":"index.js"}"#;
        let entry = br#"{}"#;
        let (_, _, checksums) = demo_package(manifest, entry);
        write_archive(
            &archive_path,
            &[
                ("plugin.json", manifest),
                ("../evil.txt", b"evil"),
                ("index.js", entry),
                ("checksums.json", checksums.as_bytes()),
            ],
        );

        let result = PluginInstaller {
            plugin_directory: root.join("plugins"),
            staging_directory: root.join("staging"),
            replace_existing: false,
            provider_endpoints: None,
        }
        .install(&archive_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unsafe path"));
        assert!(!root.join("plugins/demo").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn existing_plugin_requires_replace_flag() {
        let root = temp_root();
        fs::create_dir_all(&root).unwrap();
        let manifest_v1 = br#"{"schemaVersion":2,"id":"demo","name":"Demo","version":"1.0.0","runtime":"process","entry":"index.js"}"#.as_slice();
        let manifest_v2 = br#"{"schemaVersion":2,"id":"demo","name":"Demo","version":"2.0.0","runtime":"process","entry":"index.js"}"#.as_slice();
        let entry = br#"{"ready":true}"#.as_slice();
        let (_, _, checksums_v1) = demo_package(manifest_v1, entry);
        let (_, _, checksums_v2) = demo_package(manifest_v2, entry);

        let first = root.join("demo-v1.plugin");
        write_archive(
            &first,
            &[
                ("plugin.json", manifest_v1),
                ("index.js", entry),
                ("checksums.json", checksums_v1.as_bytes()),
            ],
        );
        PluginInstaller {
            plugin_directory: root.join("plugins"),
            staging_directory: root.join("staging"),
            replace_existing: false,
            provider_endpoints: None,
        }
        .install(&first)
        .unwrap();

        let second = root.join("demo-v2.plugin");
        write_archive(
            &second,
            &[
                ("plugin.json", manifest_v2),
                ("index.js", entry),
                ("checksums.json", checksums_v2.as_bytes()),
            ],
        );
        let without_replace = PluginInstaller {
            plugin_directory: root.join("plugins"),
            staging_directory: root.join("staging"),
            replace_existing: false,
            provider_endpoints: None,
        }
        .install(&second);
        let message = without_replace.unwrap_err().to_string();
        assert!(message.contains("already installed"), "{message}");
        // The original install is untouched until replacement is confirmed.
        let installed_manifest = fs::read_to_string(root.join("plugins/demo/plugin.json")).unwrap();
        assert!(installed_manifest.contains("1.0.0"));

        let replaced = PluginInstaller {
            plugin_directory: root.join("plugins"),
            staging_directory: root.join("staging"),
            replace_existing: true,
            provider_endpoints: None,
        }
        .install(&second)
        .unwrap();
        assert_eq!(replaced.manifest.version, "2.0.0");
        let installed_manifest = fs::read_to_string(root.join("plugins/demo/plugin.json")).unwrap();
        assert!(installed_manifest.contains("2.0.0"));

        fs::remove_dir_all(root).unwrap();
    }

    /// Stub npm origin plus a fixture tarball serving this host's
    /// binary. The backfill tests run on a multi-thread runtime: the
    /// stub serves on one worker while the synchronous installer (with
    /// its dedicated fetch thread) runs on another.
    #[cfg(feature = "native-providers")]
    struct BackfillFixture {
        base: String,
        tarball: Vec<u8>,
        routes: std::sync::Arc<std::sync::Mutex<std::collections::BTreeMap<String, Vec<u8>>>>,
        _task: tokio::task::JoinHandle<()>,
    }

    #[cfg(feature = "native-providers")]
    impl BackfillFixture {
        async fn start(host_node: &str, host_bytes: &[u8]) -> Self {
            use std::collections::BTreeMap;
            use std::sync::{Arc, Mutex};

            // The tarball always carries a foreign twin the fetch must
            // drop; it must differ from the host file on every platform.
            let foreign_node = if host_node.ends_with("win32-x64-msvc.node") {
                "package/node-stem.linux-x64-gnu.node"
            } else {
                "package/node-stem.win32-x64-msvc.node"
            };
            let tarball = Self::tarball(host_node, host_bytes, foreign_node);
            let routes: Arc<Mutex<BTreeMap<String, Vec<u8>>>> =
                Arc::new(Mutex::new(BTreeMap::new()));
            let serving = routes.clone();
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let base = format!("http://{}", listener.local_addr().unwrap());
            let task = tokio::spawn(async move {
                loop {
                    let Ok((mut socket, _)) = listener.accept().await else {
                        break;
                    };
                    let routes = serving.clone();
                    tokio::spawn(async move {
                        use tokio::io::{AsyncReadExt, AsyncWriteExt};
                        let mut buffer = vec![0u8; 8192];
                        let Ok(read) = socket.read(&mut buffer).await else {
                            return;
                        };
                        let request = String::from_utf8_lossy(&buffer[..read]);
                        let path = request
                            .lines()
                            .next()
                            .and_then(|line| line.split_whitespace().nth(1))
                            .unwrap_or("/");
                        let body = routes
                            .lock()
                            .ok()
                            .and_then(|routes| routes.get(path).cloned());
                        let (status, reason, body) = body.map_or_else(
                            || (404, "Not Found", b"not found".to_vec()),
                            |body| (200, "OK", body),
                        );
                        let header = format!(
                            "HTTP/1.1 {status} {reason}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                            body.len()
                        );
                        let _ = socket.write_all(header.as_bytes()).await;
                        let _ = socket.write_all(&body).await;
                    });
                }
            });
            Self {
                base,
                tarball,
                routes,
                _task: task,
            }
        }

        fn tarball(host_node: &str, host_bytes: &[u8], foreign_node: &str) -> Vec<u8> {
            let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
            let mut builder = tar::Builder::new(encoder);
            for (path, bytes) in [
                (
                    "package/package.json",
                    br#"{"name":"fixture-pkg","version":"1.2.3","main":"index.js","files":["index.js","*.node"]}"#.as_slice(),
                ),
                ("package/index.js", b"module.exports = {};".as_slice()),
                (host_node, host_bytes),
                (foreign_node, b"foreign-bytes".as_slice()),
            ] {
                let mut header = tar::Header::new_gnu();
                header.set_entry_type(tar::EntryType::Regular);
                header.set_size(bytes.len() as u64);
                header.set_mode(0o644);
                header.set_cksum();
                builder.append_data(&mut header, path, bytes).unwrap();
            }
            builder.into_inner().unwrap().finish().unwrap()
        }

        /// Serve metadata + tarball, then pin through the real provider
        /// path to build the shipped lockfile.
        async fn pin_lockfile(&self) -> String {
            use tiktools_plugin_api::{NativeLibDeclaration, NativeLibProvider};
            use tiktools_plugin_sdk::native_providers::{
                pin_native_lib, provider_client, NativeLibsLockfile, ProviderEndpoints,
                LOCKFILE_VERSION,
            };

            let integrity = {
                use base64::Engine;
                use sha2::Digest;
                format!(
                    "sha512-{}",
                    base64::engine::general_purpose::STANDARD
                        .encode(sha2::Sha512::digest(&self.tarball))
                )
            };
            self.routes.lock().unwrap().extend([
                (
                    "/fixture-pkg/1.2.3".to_owned(),
                    serde_json::to_vec(&serde_json::json!({
                        "name": "fixture-pkg",
                        "version": "1.2.3",
                        "dist": {
                            "tarball": format!("{}/t.tgz", self.base),
                            "integrity": integrity,
                        },
                    }))
                    .unwrap(),
                ),
                ("/t.tgz".to_owned(), self.tarball.clone()),
            ]);
            let endpoints = ProviderEndpoints {
                npm_registry: self.base.clone(),
                github_base: self.base.clone(),
            };
            let declaration = NativeLibDeclaration {
                package: "fixture-pkg".to_owned(),
                version: "1.2.3".to_owned(),
                provider: NativeLibProvider::Npm,
                repo: None,
                tag: None,
                binary: None,
            };
            let client = provider_client().unwrap();
            let locked = pin_native_lib(&client, &endpoints, &declaration)
                .await
                .unwrap();
            let lockfile = NativeLibsLockfile {
                version: LOCKFILE_VERSION,
                packages: std::collections::BTreeMap::from([("fixture-pkg".to_owned(), locked)]),
            };
            serde_json::to_string_pretty(&lockfile).unwrap()
        }
    }

    #[cfg(feature = "native-providers")]
    fn backfill_manifest() -> Vec<u8> {
        serde_json::to_vec_pretty(&serde_json::json!({
            "schemaVersion": 3,
            "id": "backfill",
            "name": "Backfill",
            "version": "1.0.0",
            "runtime": "napi-vm",
            "entry": "dist/index.js",
            "nativeAddons": [{"package": "fixture-pkg", "root": "node_modules/fixture-pkg"}],
            "nativeLibs": [{"package": "fixture-pkg", "version": "1.2.3"}],
        }))
        .unwrap()
    }

    #[cfg(feature = "native-providers")]
    fn archive_with_checksums(path: &Path, files: &[(&str, &[u8])]) {
        let checksums: std::collections::BTreeMap<String, String> = files
            .iter()
            .map(|(name, bytes)| ((*name).to_owned(), digest_bytes(bytes)))
            .collect();
        let checksums = serde_json::to_string_pretty(&checksums).unwrap();
        let file = File::create(path).unwrap();
        let mut writer = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        for (name, bytes) in files {
            writer.start_file(*name, options).unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.start_file("checksums.json", options).unwrap();
        writer.write_all(checksums.as_bytes()).unwrap();
        writer.finish().unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[cfg(feature = "native-providers")]
    async fn installer_backfills_missing_host_binary_from_lockfile() {
        use tiktools_plugin_api::manifest::current_napi_target;

        let host_node = format!("package/node-stem.{}.node", current_napi_target());
        let fixture = BackfillFixture::start(&host_node, b"host-bytes").await;
        let lockfile = fixture.pin_lockfile().await;

        let root = temp_root();
        fs::create_dir_all(&root).unwrap();
        let archive_path = root.join("backfill.plugin");
        let manifest = backfill_manifest();
        // The shipped tree carries sources but no binary: the hook must
        // fetch exactly the host file from the stub origin.
        let package_json =
            br#"{"name":"fixture-pkg","version":"1.2.3","main":"index.js","files":["index.js","*.node"]}"#;
        archive_with_checksums(
            &archive_path,
            &[
                ("plugin.json", &manifest),
                ("dist/index.js", b"export default {};"),
                (
                    "node_modules/fixture-pkg/package.json",
                    package_json as &[u8],
                ),
                (
                    "node_modules/fixture-pkg/index.js",
                    b"module.exports = {};" as &[u8],
                ),
                ("native-libs.lock.json", lockfile.as_bytes()),
            ],
        );

        let installed = PluginInstaller {
            plugin_directory: root.join("plugins"),
            staging_directory: root.join("staging"),
            replace_existing: false,
            provider_endpoints: Some((fixture.base.clone(), fixture.base.clone())),
        }
        .install(&archive_path)
        .unwrap();
        let tree = installed.directory.join("node_modules/fixture-pkg");
        let host_file = format!("node-stem.{}.node", current_napi_target());
        assert_eq!(
            fs::read(tree.join(&host_file)).unwrap(),
            b"host-bytes",
            "host binary was not backfilled"
        );
        let foreign_file = if current_napi_target() == "win32-x64-msvc" {
            "node-stem.linux-x64-gnu.node"
        } else {
            "node-stem.win32-x64-msvc.node"
        };
        assert!(
            !tree.join(foreign_file).exists(),
            "foreign twin must not be staged"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[cfg(feature = "native-providers")]
    async fn installer_skips_fetch_for_complete_trees() {
        use tiktools_plugin_api::manifest::current_napi_target;

        let host_node = format!("package/node-stem.{}.node", current_napi_target());
        let fixture = BackfillFixture::start(&host_node, b"host-bytes").await;
        let lockfile = fixture.pin_lockfile().await;

        let root = temp_root();
        fs::create_dir_all(&root).unwrap();
        let archive_path = root.join("complete.plugin");
        let manifest = backfill_manifest();
        let package_json =
            br#"{"name":"fixture-pkg","version":"1.2.3","main":"index.js","files":["index.js","*.node"]}"#;
        // Complete tree: the host binary ships, so the dead endpoint
        // below must never be contacted (any attempt fails the install).
        let host_name = format!(
            "node_modules/fixture-pkg/node-stem.{}.node",
            current_napi_target()
        );
        let files: Vec<(&str, &[u8])> = vec![
            ("plugin.json", &manifest),
            ("dist/index.js", b"export default {};"),
            (
                "node_modules/fixture-pkg/package.json",
                package_json as &[u8],
            ),
            (
                "node_modules/fixture-pkg/index.js",
                b"module.exports = {};" as &[u8],
            ),
            (host_name.as_str(), b"host-bytes" as &[u8]),
            ("native-libs.lock.json", lockfile.as_bytes()),
        ];
        archive_with_checksums(&archive_path, &files);

        let installed = PluginInstaller {
            plugin_directory: root.join("plugins"),
            staging_directory: root.join("staging"),
            replace_existing: false,
            provider_endpoints: Some((
                "http://127.0.0.1:9".to_owned(),
                "http://127.0.0.1:9".to_owned(),
            )),
        }
        .install(&archive_path)
        .unwrap();
        assert_eq!(
            fs::read(
                installed
                    .directory
                    .join("node_modules/fixture-pkg")
                    .join(format!("node-stem.{}.node", current_napi_target()))
            )
            .unwrap(),
            b"host-bytes"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[cfg(feature = "native-providers")]
    fn installer_rejects_missing_binary_without_lockfile() {
        let root = temp_root();
        fs::create_dir_all(&root).unwrap();
        let archive_path = root.join("nolock.plugin");
        let manifest = backfill_manifest();
        let package_json =
            br#"{"name":"fixture-pkg","version":"1.2.3","main":"index.js","files":["index.js","*.node"]}"#;
        archive_with_checksums(
            &archive_path,
            &[
                ("plugin.json", &manifest),
                ("dist/index.js", b"export default {};"),
                (
                    "node_modules/fixture-pkg/package.json",
                    package_json as &[u8],
                ),
            ],
        );
        let result = PluginInstaller {
            plugin_directory: root.join("plugins"),
            staging_directory: root.join("staging"),
            replace_existing: false,
            provider_endpoints: Some((
                "http://127.0.0.1:9".to_owned(),
                "http://127.0.0.1:9".to_owned(),
            )),
        }
        .install(&archive_path);
        let message = result.unwrap_err().to_string();
        assert!(
            message.contains("native-libs.lock.json"),
            "unexpected error: {message}"
        );
        assert!(!root.join("plugins/backfill").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
