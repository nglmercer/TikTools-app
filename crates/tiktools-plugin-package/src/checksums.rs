use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use tiktools_plugin_api::manifest::is_safe_relative_path;

use crate::{PackageError, Sha256Digest, CHECKSUMS_FILE, SIGNATURE_FILE};

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
        for (relative, expected) in &self.0 {
            let path = root.join(relative);
            if !path.is_file() {
                return Err(PackageError::MissingFile(relative.clone()));
            }
            if &Sha256Digest::of_file(&path)? != expected {
                return Err(PackageError::ChecksumMismatch {
                    id: plugin_id.to_owned(),
                    path: relative.clone(),
                });
            }
        }
        for relative in relative_files(root)? {
            if matches!(relative.as_str(), CHECKSUMS_FILE | SIGNATURE_FILE) {
                continue;
            }
            if !self.0.contains_key(&relative) {
                return Err(PackageError::UnlistedFile(relative));
            }
        }
        Ok(())
    }
}

pub(crate) fn relative_files(root: &Path) -> Result<Vec<String>, PackageError> {
    let mut files = Vec::new();
    let mut pending: Vec<PathBuf> = vec![root.to_owned()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory)? {
            let path = entry?.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.is_file() {
                let relative = path
                    .strip_prefix(root)
                    .map_err(|error| PackageError::invalid(error.to_string()))?
                    .to_string_lossy()
                    .replace('\\', "/");
                files.push(relative);
            }
        }
    }
    files.sort();
    Ok(files)
}
