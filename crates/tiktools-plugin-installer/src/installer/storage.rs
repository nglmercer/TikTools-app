//! Confined store paths, retained-version migration, retention pruning and version inventory.
//! Ambiguous entries remain preserved.

use super::{
    evidence::{read_observed_file, required_observed_file, MAX_INSTALLED_RECEIPT_BYTES},
    InstalledVersions, Installer,
};
use crate::{
    layout::sanitize_version, receipt::write_atomic, InstallError, InstallErrorCode, InstallReceipt,
};
use std::{
    fs, io,
    path::{Path, PathBuf},
};
use tiktools_plugin_api::{manifest::is_valid_plugin_id, PluginManifest};
use tiktools_plugin_package::{MANIFEST_FILE, MAX_MANIFEST_BYTES};

pub(super) fn remove_optional_file(path: &Path) -> Result<(), InstallError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

impl Installer {
    pub(super) fn ensure_store_child(
        &self,
        relative: &Path,
        create: bool,
    ) -> Result<bool, InstallError> {
        let store = match fs::symlink_metadata(&self.layout.store) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound && !create => return Ok(false),
            Err(error) => return Err(error.into()),
        };
        if !store.file_type().is_dir() {
            return Err(InstallError::new(
                InstallErrorCode::InvalidPackage,
                "package store is not a real directory",
            ));
        }
        let mut current = self.layout.store.clone();
        for component in relative.components() {
            let std::path::Component::Normal(name) = component else {
                return Err(InstallError::new(
                    InstallErrorCode::InvalidPackage,
                    "invalid package store path",
                ));
            };
            current.push(name);
            let metadata = match fs::symlink_metadata(&current) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == io::ErrorKind::NotFound && !create => {
                    return Ok(false)
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    match fs::create_dir(&current) {
                        Ok(()) => {}
                        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                        Err(error) => return Err(error.into()),
                    }
                    fs::symlink_metadata(&current)?
                }
                Err(error) => return Err(error.into()),
            };
            if !metadata.file_type().is_dir() {
                return Err(InstallError::new(
                    InstallErrorCode::InvalidPackage,
                    "package store path is not a real directory",
                ));
            }
        }
        Ok(true)
    }

    pub(super) fn ensure_versions_dir(
        &self,
        plugin_id: &str,
        create: bool,
    ) -> Result<bool, InstallError> {
        if !is_valid_plugin_id(plugin_id) {
            return Err(InstallError::new(
                InstallErrorCode::InvalidPackage,
                "invalid plugin ID",
            ));
        }
        self.ensure_store_child(&Path::new("versions").join(plugin_id), create)
    }

    pub(super) fn legacy_retained(&self, plugin_id: &str, version: &str) -> Option<PathBuf> {
        let component = sanitize_version(version);
        if matches!(component.as_str(), "" | "." | "..") {
            return None;
        }
        Some(self.layout.versions(plugin_id).join(component))
    }

    pub(super) fn retained_matches(path: &Path, plugin_id: &str, version: &str) -> bool {
        if !fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_dir()) {
            return false;
        }
        required_observed_file(&path.join(MANIFEST_FILE), MAX_MANIFEST_BYTES)
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .and_then(|text| PluginManifest::from_json_str(&text).ok())
            .is_some_and(|manifest| manifest.id == plugin_id && manifest.version == version)
    }

    // Old sanitization was not injective and permitted parent components.
    // Migrate only a real directory whose manifest identifies this exact
    // version. Never delete ambiguous entries or attach an unrelated receipt.
    pub(super) fn migrate_legacy_retained(
        &self,
        plugin_id: &str,
        version: &str,
    ) -> Result<(), InstallError> {
        if !self.ensure_versions_dir(plugin_id, false)? {
            return Ok(());
        }
        let destination = self.layout.retained(plugin_id, version);
        let Some(legacy) = self.legacy_retained(plugin_id, version) else {
            return Ok(());
        };
        let old_receipt = self
            .layout
            .versions(plugin_id)
            .join(format!("{}.json", sanitize_version(version)));
        let new_receipt = self.layout.retained_receipt(plugin_id, version);
        let marker = self.layout.retained_migration_proof(plugin_id, version);
        let receipt_bytes = read_observed_file(&old_receipt, MAX_INSTALLED_RECEIPT_BYTES)
            .ok()
            .flatten()
            .filter(|bytes| {
                serde_json::from_slice::<InstallReceipt>(bytes)
                    .ok()
                    .is_some_and(|receipt| receipt.id == plugin_id && receipt.version == version)
            });
        let exists = match fs::symlink_metadata(&destination) {
            Ok(_) if Self::retained_matches(&destination, plugin_id, version) => true,
            Ok(_) => {
                return Err(InstallError::new(
                    InstallErrorCode::InvalidPackage,
                    "retained destination contains unexpected data",
                ))
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => false,
            Err(error) => return Err(error.into()),
        };
        if !exists {
            if !Self::retained_matches(&legacy, plugin_id, version) {
                return Ok(());
            }
            remove_optional_file(&marker)?;
            if let Some(bytes) = &receipt_bytes {
                if let Ok(proof) = Self::migration_receipt_proof(&legacy, bytes, plugin_id) {
                    write_atomic(&marker, proof.as_bytes())?;
                }
            }
            fs::rename(&legacy, &destination)?;
        }
        let matches = receipt_bytes.as_ref().is_some_and(|bytes| {
            read_observed_file(&marker, 64)
                .ok()
                .flatten()
                .is_some_and(|marker| {
                    Self::migration_receipt_proof(&destination, bytes, plugin_id)
                        .is_ok_and(|proof| marker == proof.as_bytes())
                })
        });
        if matches
            && !legacy.exists()
            && matches!(fs::symlink_metadata(&new_receipt), Err(error) if error.kind() == io::ErrorKind::NotFound)
        {
            fs::rename(old_receipt, new_receipt)?;
            remove_optional_file(&marker)?;
        }
        Ok(())
    }

    pub fn versions(&self, plugin_id: &str) -> Result<InstalledVersions, InstallError> {
        if !is_valid_plugin_id(plugin_id) {
            return Err(InstallError::new(
                InstallErrorCode::InvalidPackage,
                "invalid plugin ID",
            ));
        }
        self.ensure_versions_dir(plugin_id, false)?;
        let receipt = self.receipts.load(plugin_id)?;
        let installed = self
            .installed_manifest(plugin_id)
            .map(|manifest| manifest.version);
        let retained = receipt
            .as_ref()
            .map(|receipt| {
                receipt
                    .retained
                    .iter()
                    .filter(|entry| {
                        let path = self.layout.retained(plugin_id, &entry.version);
                        Self::retained_matches(&path, plugin_id, &entry.version)
                            || (!path.exists()
                                && self.legacy_retained(plugin_id, &entry.version).is_some_and(
                                    |legacy| {
                                        Self::retained_matches(&legacy, plugin_id, &entry.version)
                                    },
                                ))
                    })
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        Ok(InstalledVersions {
            id: plugin_id.to_owned(),
            installed,
            receipt,
            retained,
        })
    }

    pub(super) fn finalize(
        &self,
        mut receipt: InstallReceipt,
    ) -> Result<InstallReceipt, InstallError> {
        self.ensure_versions_dir(&receipt.id, false)?;
        self.ensure_store_child(Path::new("receipts"), true)?;
        receipt.retained.sort_by_key(|entry| entry.retained_at);
        while receipt.retained.len() > self.retain {
            let removed = receipt.retained.remove(0);
            let directory = self.layout.retained(&receipt.id, &removed.version);
            if !Self::retained_matches(&directory, &receipt.id, &removed.version) {
                continue;
            }
            if directory.exists() {
                fs::remove_dir_all(&directory)?;
            }
            let _ = fs::remove_file(self.layout.retained_receipt(&receipt.id, &removed.version));
            let _ = fs::remove_file(
                self.layout
                    .retained_local_proof(&receipt.id, &removed.version),
            );
        }
        self.receipts.save(&receipt)?;
        Ok(receipt)
    }

    pub(super) fn checked_package_dir(&self, plugin_id: &str) -> Result<PathBuf, InstallError> {
        if !is_valid_plugin_id(plugin_id) {
            return Err(InstallError::not_installed(plugin_id));
        }
        let target = self.layout.package(plugin_id);
        let metadata =
            fs::symlink_metadata(&target).map_err(|_| InstallError::not_installed(plugin_id))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(InstallError::new(
                InstallErrorCode::InvalidPackage,
                "plugin package is not a regular directory",
            ));
        }
        let root = fs::canonicalize(&self.layout.plugins)?;
        let package = fs::canonicalize(&target)?;
        if package.parent() != Some(root.as_path()) {
            return Err(InstallError::new(
                InstallErrorCode::InvalidPackage,
                "plugin package is outside the user plugin directory",
            ));
        }
        Ok(target)
    }
}
