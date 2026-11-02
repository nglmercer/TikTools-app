use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf},
};

use tiktools_plugin_api::{manifest::is_safe_relative_path, PluginManifest};
use zip::ZipArchive;

use crate::{
    Checksums, PackageError, PackageSignature, Sha256Digest, CHECKSUMS_FILE, MANIFEST_FILE,
    PACKAGE_EXTENSION, SIGNATURE_FILE,
};

pub const MAX_ARCHIVE_BYTES: u64 = 512 * 1024 * 1024;
pub const MAX_EXTRACTED_BYTES: u64 = 2 * 1024 * 1024 * 1024;
pub const MAX_FILES: usize = 50_000;
pub const MAX_MANIFEST_BYTES: usize = 256 * 1024;
pub const MAX_CHANGELOG_BYTES: usize = 64 * 1024;
const MAX_CHECKSUMS_BYTES: usize = 8 * 1024 * 1024;
const MAX_SIGNATURE_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone)]
pub struct PackageArchive {
    path: PathBuf,
    size: u64,
}

#[derive(Debug, Clone)]
pub struct PackageInspection {
    pub manifest: PluginManifest,
    pub checksums: Option<Vec<u8>>,
    pub signature: Option<PackageSignature>,
    pub changelog: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExtractedPackage {
    pub root: PathBuf,
    pub manifest: PluginManifest,
    pub checksums: Vec<u8>,
    pub signature: Option<PackageSignature>,
}

impl PackageArchive {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, PackageError> {
        let path = fs::canonicalize(path.as_ref())?;
        let has_extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case(PACKAGE_EXTENSION));
        if !has_extension {
            return Err(PackageError::WrongExtension);
        }
        if !path.is_file() {
            return Err(PackageError::NotAFile);
        }
        let size = fs::metadata(&path)?.len();
        if size > MAX_ARCHIVE_BYTES {
            return Err(PackageError::ArchiveTooLarge);
        }
        Ok(Self { path, size })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn digest(&self) -> Result<Sha256Digest, PackageError> {
        Ok(Sha256Digest::of_file(&self.path)?)
    }

    pub fn inspect(&self) -> Result<PackageInspection, PackageError> {
        let mut archive = self.zip()?;
        let mut manifest_entry: Option<(usize, PathBuf)> = None;
        for index in 0..archive.len().min(MAX_FILES) {
            let entry = archive.by_index(index)?;
            let name = entry.name()?;
            let Some(relative) = normalized_archive_path(&name)? else {
                continue;
            };
            let is_manifest = relative.file_name().and_then(|name| name.to_str())
                == Some(MANIFEST_FILE)
                && relative.components().count() <= 2;
            if !is_manifest {
                continue;
            }
            if manifest_entry.is_some() {
                return Err(PackageError::MultipleManifests);
            }
            manifest_entry = Some((index, relative));
        }
        let (manifest_index, manifest_path) =
            manifest_entry.ok_or(PackageError::MissingManifest)?;
        let prefix = manifest_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default();
        let manifest_bytes = read_entry(&mut archive, manifest_index, MAX_MANIFEST_BYTES)?;
        let manifest = parse_manifest(&manifest_bytes)?;
        manifest.validate_compatibility()?;

        let checksums = find_entry(&mut archive, &prefix.join(CHECKSUMS_FILE))?
            .map(|index| read_entry(&mut archive, index, MAX_CHECKSUMS_BYTES))
            .transpose()?;
        let signature = find_entry(&mut archive, &prefix.join(SIGNATURE_FILE))?
            .map(|index| read_entry(&mut archive, index, MAX_SIGNATURE_BYTES))
            .transpose()?
            .map(|bytes| PackageSignature::parse(&bytes))
            .transpose()?;
        let changelog = match manifest.metadata.changelog.as_deref() {
            Some(path) => find_entry(&mut archive, &prefix.join(path))?
                .map(|index| read_entry(&mut archive, index, MAX_CHANGELOG_BYTES))
                .transpose()?
                .and_then(|bytes| String::from_utf8(bytes).ok()),
            None => None,
        };
        Ok(PackageInspection {
            manifest,
            checksums,
            signature,
            changelog,
        })
    }

    pub fn extract(&self, staging: &Path) -> Result<ExtractedPackage, PackageError> {
        let mut archive = self.zip()?;
        if archive.is_empty() || archive.len() > MAX_FILES {
            return Err(PackageError::InvalidEntryCount);
        }
        let mut extracted_bytes = 0_u64;
        let mut extracted_paths = BTreeSet::new();
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index)?;
            let name = entry.name()?;
            let Some(relative) = normalized_archive_path(&name)? else {
                continue;
            };
            if !extracted_paths.insert(relative.clone()) {
                return Err(PackageError::DuplicatePath(name.into_owned()));
            }
            let destination = staging.join(&relative);
            if entry.is_dir() {
                fs::create_dir_all(&destination)?;
                continue;
            }
            extracted_bytes = extracted_bytes.saturating_add(entry.size());
            if extracted_bytes > MAX_EXTRACTED_BYTES {
                return Err(PackageError::ExpandsTooLarge);
            }
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut output = File::create(&destination)?;
            let copied = io::copy(&mut (&mut entry).take(MAX_EXTRACTED_BYTES + 1), &mut output)?;
            if copied > entry.size() {
                return Err(PackageError::ExpandsTooLarge);
            }
            apply_stored_unix_mode(&destination, entry.unix_mode())?;
        }

        let root = find_package_root(staging)?;
        validate_extracted_tree(&root)?;
        let manifest_bytes = fs::read(root.join(MANIFEST_FILE))?;
        if manifest_bytes.len() > MAX_MANIFEST_BYTES {
            return Err(PackageError::Manifest(
                tiktools_plugin_api::manifest::ManifestError::TooLarge,
            ));
        }
        let manifest = parse_manifest(&manifest_bytes)?;
        manifest.validate_compatibility()?;
        ensure_entry_executable(&root, &manifest)?;
        let checksums_bytes = fs::read(root.join(CHECKSUMS_FILE))
            .map_err(|_| PackageError::MissingChecksums(manifest.id.clone()))?;
        Checksums::parse(&checksums_bytes)?.verify_tree(&root, &manifest.id)?;
        let signature = match fs::read(root.join(SIGNATURE_FILE)) {
            Ok(bytes) => Some(PackageSignature::parse(&bytes)?),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        Ok(ExtractedPackage {
            root,
            manifest,
            checksums: checksums_bytes,
            signature,
        })
    }

    fn zip(&self) -> Result<ZipArchive<File>, PackageError> {
        Ok(ZipArchive::new(File::open(&self.path)?)?)
    }
}

pub(crate) fn parse_manifest(bytes: &[u8]) -> Result<PluginManifest, PackageError> {
    let text = std::str::from_utf8(bytes).map_err(|_| PackageError::ManifestEncoding)?;
    Ok(PluginManifest::from_json_str(text)?)
}

fn read_entry(
    archive: &mut ZipArchive<File>,
    index: usize,
    limit: usize,
) -> Result<Vec<u8>, PackageError> {
    let entry = archive.by_index(index)?;
    let mut bytes = Vec::new();
    entry.take(limit as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(PackageError::Manifest(
            tiktools_plugin_api::manifest::ManifestError::TooLarge,
        ));
    }
    Ok(bytes)
}

fn find_entry(
    archive: &mut ZipArchive<File>,
    wanted: &Path,
) -> Result<Option<usize>, PackageError> {
    for index in 0..archive.len().min(MAX_FILES) {
        let entry = archive.by_index(index)?;
        let name = entry.name()?;
        if normalized_archive_path(&name)?.as_deref() == Some(wanted) && !entry.is_dir() {
            return Ok(Some(index));
        }
    }
    Ok(None)
}

pub(crate) fn normalized_archive_path(name: &str) -> Result<Option<PathBuf>, PackageError> {
    let normalized = name.replace('\\', "/");
    let normalized = normalized.strip_prefix("./").unwrap_or(&normalized);
    let normalized = normalized.trim_end_matches('/');
    if normalized.is_empty() {
        return Ok(None);
    }
    if !is_safe_relative_path(normalized) {
        return Err(PackageError::UnsafePath(name.to_owned()));
    }
    Ok(Some(PathBuf::from(normalized)))
}

fn find_package_root(staging: &Path) -> Result<PathBuf, PackageError> {
    if staging.join(MANIFEST_FILE).is_file() {
        return Ok(staging.to_owned());
    }
    let directories: Vec<_> = fs::read_dir(staging)?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false))
        .collect();
    if directories.len() == 1 && directories[0].path().join(MANIFEST_FILE).is_file() {
        return Ok(directories[0].path());
    }
    Err(PackageError::MissingManifest)
}

fn validate_extracted_tree(root: &Path) -> Result<(), PackageError> {
    let root = fs::canonicalize(root)?;
    let mut files = 0_usize;
    let mut bytes = 0_u64;
    let mut pending = vec![root.clone()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory)? {
            let path = entry?.path();
            let metadata = fs::symlink_metadata(&path)?;
            if metadata.file_type().is_symlink() {
                return Err(PackageError::Symlink);
            }
            if metadata.is_dir() {
                pending.push(path);
                continue;
            }
            if !metadata.is_file() {
                return Err(PackageError::UnsupportedEntry);
            }
            if !path.starts_with(&root) {
                return Err(PackageError::Traversal);
            }
            files = files.saturating_add(1);
            bytes = bytes.saturating_add(metadata.len());
            if files > MAX_FILES {
                return Err(PackageError::TooManyFiles);
            }
            if bytes > MAX_EXTRACTED_BYTES {
                return Err(PackageError::ExpandsTooLarge);
            }
        }
    }
    Ok(())
}

#[cfg(unix)]
fn apply_stored_unix_mode(destination: &Path, mode: Option<u32>) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let Some(bits) = mode.map(|mode| mode & 0o777).filter(|bits| *bits != 0) else {
        return Ok(());
    };
    fs::set_permissions(destination, fs::Permissions::from_mode(bits))
}

#[cfg(not(unix))]
fn apply_stored_unix_mode(_destination: &Path, _mode: Option<u32>) -> io::Result<()> {
    Ok(())
}

#[cfg(unix)]
fn ensure_entry_executable(root: &Path, manifest: &PluginManifest) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    use tiktools_plugin_api::PluginRuntimeKind;
    if !matches!(
        manifest.runtime,
        PluginRuntimeKind::Native | PluginRuntimeKind::Process
    ) {
        return Ok(());
    }
    let entry = root.join(manifest.entry.as_str());
    let mut permissions = fs::metadata(&entry)?.permissions();
    permissions.set_mode(permissions.mode() | 0o111);
    fs::set_permissions(&entry, permissions)
}

#[cfg(not(unix))]
fn ensure_entry_executable(_root: &Path, _manifest: &PluginManifest) -> io::Result<()> {
    Ok(())
}
