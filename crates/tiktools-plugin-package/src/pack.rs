use std::{
    collections::BTreeMap,
    env,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use tiktools_plugin_api::{
    manifest::{is_plugin_target, is_safe_relative_path, PLUGIN_TARGETS},
    ui::PluginUiMode,
    PluginManifest, PluginRuntimeKind,
};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

use crate::{
    archive::{normalized_archive_path, parse_manifest},
    Checksums, PackageArchive, PackageError, Sha256Digest, SigningKey, CHECKSUMS_FILE,
    MANIFEST_FILE, PACKAGE_EXTENSION, SIGNATURE_FILE,
};

const ASSET_DIRECTORIES: [&str; 3] = ["assets", "dist", "locales"];
const NATIVE_LIBS_LOCKFILE: &str = "native-libs.lock.json";

#[derive(Debug, Clone)]
pub struct PackRequest {
    pub manifest: PathBuf,
    pub entry: Option<PathBuf>,
    pub output: PathBuf,
    pub target: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PackOutput {
    pub path: PathBuf,
    pub manifest: PluginManifest,
    pub digest: Sha256Digest,
    pub size: u64,
}

pub fn pack(request: &PackRequest) -> Result<PackOutput, PackageError> {
    let has_extension = request
        .output
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case(PACKAGE_EXTENSION));
    if !has_extension {
        return Err(PackageError::WrongExtension);
    }
    let target = request.target.as_deref().map(validate_target).transpose()?;

    let manifest_path = fs::canonicalize(&request.manifest)?;
    let package_directory = manifest_path
        .parent()
        .ok_or_else(|| PackageError::invalid("manifest has no parent directory"))?
        .to_path_buf();
    let mut manifest_value: serde_json::Value = serde_json::from_slice(&fs::read(&manifest_path)?)
        .map_err(|error| PackageError::invalid(format!("manifest is not valid JSON: {error}")))?;
    let manifest = PluginManifest::from_value(manifest_value.clone())?;
    manifest.validate_compatibility()?;
    if let Some(target) = target.as_deref() {
        enforce_target_rules(&manifest, target)?;
    }

    let executable = matches!(
        manifest.runtime,
        PluginRuntimeKind::Native | PluginRuntimeKind::Process
    );
    let mut files = BTreeMap::new();
    let staged_entry = match (&request.entry, manifest.entry.is_empty()) {
        (Some(entry), _) => {
            let metadata = fs::symlink_metadata(entry)?;
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                return Err(PackageError::invalid(
                    "entry must be a regular, non-symlink file",
                ));
            }
            let staged = staged_entry_name(
                &manifest.entry,
                target.as_deref(),
                manifest.runtime == PluginRuntimeKind::Process,
            );
            insert_file(&mut files, &staged, fs::read(entry)?)?;
            Some(staged)
        }
        (None, true) if manifest.runtime == PluginRuntimeKind::Declarative => None,
        (None, _) => {
            return Err(PackageError::invalid(
                "an entry file is required for executable plugin runtimes",
            ))
        }
    };

    let packaged = manifest_value
        .as_object_mut()
        .ok_or_else(|| PackageError::invalid("manifest must be a JSON object"))?;
    if let Some(staged) = &staged_entry {
        packaged.insert(
            "entry".to_owned(),
            serde_json::Value::String(staged.clone()),
        );
    }
    if let Some(target) = target.as_deref() {
        packaged.insert(
            "targets".to_owned(),
            serde_json::Value::Array(vec![serde_json::Value::String(target.to_owned())]),
        );
    }
    let mut manifest_bytes = serde_json::to_vec_pretty(&manifest_value)
        .map_err(|error| PackageError::invalid(error.to_string()))?;
    manifest_bytes.push(b'\n');
    let packaged_manifest = parse_manifest(&manifest_bytes)?;
    insert_file(&mut files, MANIFEST_FILE, manifest_bytes)?;

    for directory in included_directories(&manifest) {
        let source = package_directory.join(&directory);
        if source.is_dir() {
            collect_directory(&source, &directory, staged_entry.as_deref(), &mut files)?;
        }
    }
    let mut loose_files = Vec::new();
    if manifest.runtime == PluginRuntimeKind::NapiVm && !manifest.native_libs.is_empty() {
        loose_files.push(NATIVE_LIBS_LOCKFILE.to_owned());
    }
    if let Some(changelog) = &manifest.metadata.changelog {
        loose_files.push(changelog.clone());
    }
    for relative in loose_files {
        let path = package_directory.join(&relative);
        if path.is_file()
            && !fs::symlink_metadata(&path)?.file_type().is_symlink()
            && !files.contains_key(&relative)
        {
            insert_file(&mut files, &relative, fs::read(&path)?)?;
        }
    }

    let mut checksums = Checksums::default();
    for (path, bytes) in &files {
        checksums.insert(path.clone(), Sha256Digest::of_bytes(bytes));
    }
    insert_file(&mut files, CHECKSUMS_FILE, checksums.to_bytes())?;

    let output = absolutize(&request.output)?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = output.with_extension("plugin.tmp");
    let _ = fs::remove_file(&temporary);
    write_archive(
        &temporary,
        &manifest.id,
        &files,
        staged_entry.as_deref().filter(|_| executable),
    )?;
    replace_file(&temporary, &output)?;
    let archive = PackageArchive::open(&output)?;
    Ok(PackOutput {
        digest: archive.digest()?,
        size: archive.size(),
        path: output,
        manifest: packaged_manifest,
    })
}

pub fn sign_archive(path: &Path, key: &SigningKey) -> Result<PackOutput, PackageError> {
    let archive = PackageArchive::open(path)?;
    let inspection = archive.inspect()?;
    let checksums = inspection
        .checksums
        .ok_or_else(|| PackageError::MissingChecksums(inspection.manifest.id.clone()))?;
    Checksums::parse(&checksums)?;
    let signature = key.sign_package(&checksums)?;

    let mut source = ZipArchive::new(File::open(archive.path())?)?;
    let mut prefix = None;
    for index in 0..source.len() {
        let entry = source.by_index(index)?;
        if let Some(relative) = normalized_archive_path(&entry.name()?)? {
            if relative.file_name().and_then(|name| name.to_str()) == Some(CHECKSUMS_FILE)
                && relative.components().count() <= 2
            {
                prefix = Some(relative.parent().map(Path::to_path_buf).unwrap_or_default());
                break;
            }
        }
    }
    let prefix =
        prefix.ok_or_else(|| PackageError::MissingChecksums(inspection.manifest.id.clone()))?;
    let signature_path = prefix.join(SIGNATURE_FILE);

    let temporary = archive.path().with_extension("plugin.tmp");
    let _ = fs::remove_file(&temporary);
    {
        let mut writer = ZipWriter::new(File::create(&temporary)?);
        for index in 0..source.len() {
            let entry = source.by_index_raw(index)?;
            if normalized_archive_path(&entry.name()?)?.as_deref() == Some(signature_path.as_path())
            {
                continue;
            }
            writer.raw_copy_file(entry)?;
        }
        writer.start_file(
            signature_path.to_string_lossy().replace('\\', "/"),
            SimpleFileOptions::default()
                .compression_method(CompressionMethod::Deflated)
                .unix_permissions(0o644),
        )?;
        writer.write_all(&signature.to_bytes())?;
        writer.finish()?;
    }
    drop(source);
    replace_file(&temporary, archive.path())?;
    let signed = PackageArchive::open(archive.path())?;
    Ok(PackOutput {
        digest: signed.digest()?,
        size: signed.size(),
        path: path.to_path_buf(),
        manifest: inspection.manifest,
    })
}

pub(crate) fn validate_target(value: &str) -> Result<String, PackageError> {
    let normalized = value.trim();
    if !is_plugin_target(normalized) {
        return Err(PackageError::invalid(format!(
            "unsupported target {value}; expected one of {}",
            PLUGIN_TARGETS.join(", ")
        )));
    }
    Ok(normalized.to_owned())
}

fn included_directories(manifest: &PluginManifest) -> Vec<String> {
    let mut directories: Vec<String> = ASSET_DIRECTORIES
        .iter()
        .map(|value| (*value).to_owned())
        .collect();
    if manifest.runtime == PluginRuntimeKind::NapiVm {
        directories.push("node_modules".to_owned());
    }
    let ui_directory = manifest
        .ui
        .as_ref()
        .filter(|ui| ui.mode == PluginUiMode::Webview)
        .and_then(|ui| ui.entry.as_deref())
        .and_then(|entry| Path::new(entry).parent())
        .map(|parent| parent.to_string_lossy().replace('\\', "/"))
        .filter(|parent| !parent.is_empty() && is_safe_relative_path(parent));
    if let Some(directory) = ui_directory {
        if !directories.iter().any(|existing| {
            directory.starts_with(&format!("{existing}/")) || *existing == directory
        }) {
            directories.push(directory);
        }
    }
    directories
}

fn enforce_target_rules(manifest: &PluginManifest, _target: &str) -> Result<(), PackageError> {
    match manifest.runtime {
        PluginRuntimeKind::Native | PluginRuntimeKind::Process => Ok(()),
        PluginRuntimeKind::Wasm => Err(PackageError::invalid(
            "--target must not be used for wasm plugins; keep targets empty for portable WASM",
        )),
        PluginRuntimeKind::NapiVm if manifest.native_addons.is_empty() => Err(PackageError::invalid(
            "--target must not be used for pure-JavaScript napi-vm plugins; keep targets empty for portable JavaScript",
        )),
        PluginRuntimeKind::NapiVm => Ok(()),
        PluginRuntimeKind::Declarative => Err(PackageError::invalid(
            "--target must not be used for declarative plugins; they ship no executable entry",
        )),
    }
}

pub(crate) fn staged_entry_name(entry: &str, target: Option<&str>, executable: bool) -> String {
    if !executable {
        return entry.to_owned();
    }
    let wants_exe = match target {
        Some(value) => value.starts_with("win32-"),
        None => cfg!(target_os = "windows"),
    };
    if wants_exe && !entry.to_ascii_lowercase().ends_with(".exe") {
        format!("{entry}.exe")
    } else {
        entry.to_owned()
    }
}

fn collect_directory(
    directory: &Path,
    archive_prefix: &str,
    staged_entry: Option<&str>,
    files: &mut BTreeMap<String, Vec<u8>>,
) -> Result<(), PackageError> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        let metadata = fs::symlink_metadata(&path)?;
        let relative = path
            .strip_prefix(directory)
            .map_err(|error| PackageError::invalid(error.to_string()))?
            .to_string_lossy()
            .replace('\\', "/");
        let archive_path = format!("{archive_prefix}/{relative}");
        if metadata.file_type().is_symlink() {
            return Err(PackageError::invalid(format!(
                "asset tree contains a symlink: {}",
                path.display()
            )));
        }
        if metadata.is_dir() {
            collect_directory(&path, &archive_path, staged_entry, files)?;
        } else if metadata.is_file() {
            if Some(archive_path.as_str()) == staged_entry || files.contains_key(&archive_path) {
                continue;
            }
            insert_file(files, &archive_path, fs::read(&path)?)?;
        } else {
            return Err(PackageError::invalid(format!(
                "unsupported asset entry: {}",
                path.display()
            )));
        }
    }
    Ok(())
}

fn insert_file(
    files: &mut BTreeMap<String, Vec<u8>>,
    path: &str,
    bytes: Vec<u8>,
) -> Result<(), PackageError> {
    if !is_safe_relative_path(path) {
        return Err(PackageError::UnsafePath(path.to_owned()));
    }
    if files.insert(path.to_owned(), bytes).is_some() {
        return Err(PackageError::DuplicatePath(path.to_owned()));
    }
    Ok(())
}

fn write_archive(
    path: &Path,
    plugin_id: &str,
    files: &BTreeMap<String, Vec<u8>>,
    executable_entry: Option<&str>,
) -> Result<(), PackageError> {
    let mut writer = ZipWriter::new(File::create(path)?);
    for (relative, bytes) in files {
        let mode = if Some(relative.as_str()) == executable_entry {
            0o755
        } else {
            0o644
        };
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .unix_permissions(mode);
        writer.start_file(format!("{plugin_id}/{relative}"), options)?;
        writer.write_all(bytes)?;
    }
    writer.finish()?;
    Ok(())
}

fn replace_file(temporary: &Path, destination: &Path) -> Result<(), PackageError> {
    if destination.exists() {
        fs::remove_file(destination)?;
    }
    fs::rename(temporary, destination)?;
    Ok(())
}

fn absolutize(path: &Path) -> Result<PathBuf, PackageError> {
    if path.is_absolute() {
        Ok(path.to_owned())
    } else {
        Ok(env::current_dir()?.join(path))
    }
}
