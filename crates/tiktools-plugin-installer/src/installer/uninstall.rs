//! Package removal and runtime restoration on failure. Recovery copies remain preserved when
//! restoration is uncertain.

use super::{ActivationHooks, Installer};
use crate::InstallError;
use std::{fs, path::Path};

impl Installer {
    pub fn uninstall(
        &self,
        plugin_id: &str,
        purge_data: Option<&Path>,
        hooks: &dyn ActivationHooks,
    ) -> Result<(), InstallError> {
        self.try_lock_store_for(plugin_id)?
            .uninstall(plugin_id, purge_data, hooks)
    }

    pub(super) fn uninstall_under_store_lock(
        &self,
        plugin_id: &str,
        purge_data: Option<&Path>,
        hooks: &dyn ActivationHooks,
    ) -> Result<(), InstallError> {
        let _guard = self.locks.try_lock(plugin_id)?;
        self.ensure_versions_dir(plugin_id, false)?;
        self.ensure_store_child(Path::new("local-proofs"), false)?;
        self.ensure_store_child(Path::new("receipts"), false)?;
        let target = self.checked_package_dir(plugin_id)?;
        let was_running = hooks.deactivate(plugin_id).map_err(InstallError::io)?;
        let after_deactivate = self
            .ensure_store_child(Path::new("receipts"), false)
            .and_then(|_| self.ensure_store_child(Path::new("local-proofs"), false))
            .and_then(|_| self.ensure_versions_dir(plugin_id, false));
        if let Err(error) = after_deactivate {
            let resume = hooks.activate(plugin_id, was_running);
            return Err(InstallError::io(format!("package metadata changed while stopping plugin: {error}; runtime recovery: {resume:?}")));
        }
        let trash = match self.layout.new_staging_dir("removed") {
            Ok(trash) => trash,
            Err(error) => {
                return Err(match hooks.activate(plugin_id, was_running) {
                    Ok(()) => InstallError::io(format!("could not prepare plugin removal: {error}")),
                    Err(restore) => InstallError::io(format!(
                        "could not prepare plugin removal and runtime recovery failed: {error}; {restore}"
                    )),
                });
            }
        };
        if let Err(error) = fs::rename(&target, trash.join(plugin_id)) {
            let _ = fs::remove_dir_all(&trash);
            return Err(match hooks.activate(plugin_id, was_running) {
                Ok(()) => InstallError::io(format!("could not remove plugin package: {error}")),
                Err(restore) => InstallError::io(format!(
                    "could not remove plugin package and runtime recovery failed: {error}; {restore}"
                )),
            });
        }
        if let Err(error) = hooks.activate(plugin_id, false) {
            let saved = trash.join(plugin_id);
            return match fs::rename(&saved, &target) {
                Ok(()) => {
                    let resumed = hooks.activate(plugin_id, was_running);
                    let _ = fs::remove_dir_all(&trash);
                    match resumed {
                        Ok(()) => Err(InstallError::io(format!(
                            "could not refresh plugins after uninstall; package restored: {error}"
                        ))),
                        Err(resume) => Err(InstallError::io(format!(
                            "could not refresh plugins after uninstall; package restored but runtime recovery failed: {error}; {resume}"
                        ))),
                    }
                }
                Err(restore) => Err(InstallError::io(format!(
                    "could not refresh plugins after uninstall; package preserved at {} because restoration failed: {error}; {restore}",
                    saved.display()
                ))),
            };
        }
        if let Err(error) = self
            .ensure_store_child(Path::new("receipts"), false)
            .and_then(|_| self.ensure_versions_dir(plugin_id, false))
        {
            return Err(InstallError::io(format!(
                "package metadata changed during removal: {error}; package preserved at {}",
                trash.join(plugin_id).display()
            )));
        }
        let _ = fs::remove_dir_all(&trash);
        self.receipts.remove(plugin_id)?;
        let versions = self.layout.versions(plugin_id);
        if versions.exists() {
            fs::remove_dir_all(versions)?;
        }
        if let Some(data) = purge_data.filter(|path| path.exists()) {
            fs::remove_dir_all(data)?;
        }
        Ok(())
    }
}
