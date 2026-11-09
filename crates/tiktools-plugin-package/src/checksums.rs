use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf},
};

use tiktools_plugin_api::manifest::is_safe_relative_path;

use crate::archive::{MAX_EXTRACTED_BYTES, MAX_FILES};
use crate::{PackageError, Sha256Digest, Sha256Hasher, CHECKSUMS_FILE, SIGNATURE_FILE};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Checksums(BTreeMap<String, Sha256Digest>);

impl Checksums {
    pub fn parse(bytes: &[u8]) -> Result<Self, PackageError> {
        let raw: BTreeMap<String, String> = serde_json::from_slice(bytes)
            .map_err(|error| PackageError::InvalidChecksums(error.to_string()))?;
        if raw.is_empty() {
            return Err(PackageError::InvalidChecksums(
                "no checksums are listed".to_owned(),
            ));
        }
        raw.into_iter()
            .map(|(path, digest)| {
                if !is_safe_relative_path(&path) {
                    return Err(PackageError::InvalidChecksums(format!(
                        "invalid checksum entry: {path}"
                    )));
                }
                let digest = Sha256Digest::parse(&digest).ok_or_else(|| {
                    PackageError::InvalidChecksums(format!("invalid checksum entry: {path}"))
                })?;
                Ok((path, digest))
            })
            .collect::<Result<_, _>>()
            .map(Self)
    }

    pub fn insert(&mut self, path: impl Into<String>, digest: Sha256Digest) {
        self.0.insert(path.into(), digest);
    }

    pub fn get(&self, path: &str) -> Option<&Sha256Digest> {
        self.0.get(path)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Sha256Digest)> {
        self.0.iter()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let plain: BTreeMap<&str, &str> = self
            .0
            .iter()
            .map(|(path, digest)| (path.as_str(), digest.as_hex()))
            .collect();
        let mut bytes = serde_json::to_vec_pretty(&plain).unwrap_or_default();
        bytes.push(b'\n');
        bytes
    }

    pub fn verify_tree(&self, root: &Path, plugin_id: &str) -> Result<(), PackageError> {
        self.verify_tree_with_directory_limit(root, plugin_id, usize::MAX)
    }

    /// As `verify_tree`, additionally bounding implicit directories. Useful for
    /// inspecting an existing tree whose contents did not come from a freshly
    /// bounded archive extraction. This is not an atomic filesystem snapshot.
    pub fn verify_tree_with_directory_limit(
        &self,
        root: &Path,
        plugin_id: &str,
        max_directories: usize,
    ) -> Result<(), PackageError> {
        // Enumerate before hashing: direct `is_file` and `is_dir` follow
        // symlinks, including symlinks in parent components. Recovery must not
        // treat external files or an incomplete installed tree as a package.
        let files = relative_files_with_limits(root, MAX_FILES, max_directories)?;
        for relative in files.keys() {
            if matches!(relative.as_str(), CHECKSUMS_FILE | SIGNATURE_FILE) {
                continue;
            }
            if !self.0.contains_key(relative) {
                return Err(PackageError::UnlistedFile(relative.clone()));
            }
        }
        for (relative, expected) in &self.0 {
            let size = files
                .get(relative)
                .ok_or_else(|| PackageError::MissingFile(relative.clone()))?;
            if &bounded_digest(&root.join(relative), *size)? != expected {
                return Err(PackageError::ChecksumMismatch {
                    id: plugin_id.to_owned(),
                    path: relative.clone(),
                });
            }
        }
        Ok(())
    }
}

fn bounded_digest(path: &Path, expected_size: u64) -> Result<Sha256Digest, PackageError> {
    let mut file = File::open(path)?;
    let before = file.metadata()?;
    if !before.is_file() || before.len() != expected_size {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "package changed during verification",
        )
        .into());
    }
    let mut hash = Sha256Hasher::new();
    let mut reader = (&mut file).take(expected_size + 1);
    let mut buffer = [0u8; 64 * 1024];
    let mut count = 0u64;
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        count += read as u64;
        hash.update(&buffer[..read]);
    }
    let after = file.metadata()?;
    if count != expected_size
        || after.len() != expected_size
        || before.modified().ok() != after.modified().ok()
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "package changed during verification",
        )
        .into());
    }
    Ok(hash.finish())
}

pub(crate) fn relative_files_with_limits(
    root: &Path,
    max_files: usize,
    max_directories: usize,
) -> Result<BTreeMap<String, u64>, PackageError> {
    let metadata = fs::symlink_metadata(root)?;
    if metadata.file_type().is_symlink() {
        return Err(PackageError::Symlink);
    }
    if !metadata.is_dir() {
        return Err(PackageError::UnsupportedEntry);
    }
    let mut files = BTreeMap::new();
    let mut pending: Vec<PathBuf> = vec![root.to_owned()];
    let mut directories = 0usize;
    let mut bytes = 0u64;
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory)? {
            let path = entry?.path();
            let metadata = fs::symlink_metadata(&path)?;
            if metadata.file_type().is_symlink() {
                return Err(PackageError::Symlink);
            }
            if metadata.is_dir() {
                directories = directories
                    .checked_add(1)
                    .ok_or(PackageError::TooManyFiles)?;
                if directories > max_directories {
                    return Err(PackageError::TooManyFiles);
                }
                pending.push(path);
            } else if metadata.is_file() {
                if files.len() >= max_files {
                    return Err(PackageError::TooManyFiles);
                }
                bytes = bytes
                    .checked_add(metadata.len())
                    .ok_or(PackageError::ExpandsTooLarge)?;
                if bytes > MAX_EXTRACTED_BYTES {
                    return Err(PackageError::ExpandsTooLarge);
                }
                let raw = path
                    .strip_prefix(root)
                    .map_err(|error| PackageError::invalid(error.to_string()))?
                    .to_str()
                    .ok_or_else(|| PackageError::invalid("package contains a non-UTF-8 path"))?;
                // On Unix a backslash is a filename character, not a path
                // separator. Do not silently alias it to a different file.
                #[cfg(unix)]
                if raw.contains('\\') {
                    return Err(PackageError::invalid(
                        "package contains a non-canonical path",
                    ));
                }
                let relative = raw.replace('\\', "/");
                if files.insert(relative, metadata.len()).is_some() {
                    return Err(PackageError::invalid("package contains duplicate paths"));
                }
            } else {
                return Err(PackageError::UnsupportedEntry);
            }
        }
    }
    Ok(files)
}
