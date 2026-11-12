//! Cooperative package-store admission and the linear leased API. Mutating methods require
//! the same exclusive lease borrow.

use super::{
    ActivationHooks, CommitOptions, InstallOutcome, InstalledPackageEvidence, Installer,
    LockedPackageStore, PreparedPackage,
};
use crate::{jobs::JobReporter, InstallError, InstallReceipt, UpdatePolicy};
use std::{fs, path::Path};
use tiktools_plugin_api::PluginManifest;

impl LockedPackageStore<'_> {
    /// Bind a journal caller to this lease's actual plugin and store roots.
    /// The configured roots must exist; aliases of the same directories are
    /// accepted. External filesystem changes remain outside this lease.
    pub fn matches_roots(&self, plugins: &Path, store: &Path) -> Result<bool, InstallError> {
        Ok(
            fs::canonicalize(plugins)? == fs::canonicalize(&self.installer.layout.plugins)?
                && fs::canonicalize(store)? == fs::canonicalize(&self.installer.layout.store)?,
        )
    }
    /// Read the receipt while the package-store lease remains held. The
    /// receipt alone does not prove the files or runtime are consistent.
    pub fn receipt(&self, plugin_id: &str) -> Result<Option<InstallReceipt>, InstallError> {
        self.installer.receipt(plugin_id)
    }

    /// Read the installed manifest while the lease remains held. Missing or
    /// malformed manifests both return `None` and must not authorize recovery.
    pub fn installed_manifest(&self, plugin_id: &str) -> Option<PluginManifest> {
        self.installer.installed_manifest(plugin_id)
    }

    /// Apply an already constructed policy while retaining this same lease.
    /// Callers that merge a patch must also read the receipt under the lease.
    pub fn set_policy(
        &mut self,
        plugin_id: &str,
        policy: UpdatePolicy,
    ) -> Result<InstallReceipt, InstallError> {
        self.installer
            .set_policy_under_store_lock(plugin_id, policy)
    }

    /// Recheck the actual installed files and a currently trusted publisher
    /// signature while retaining this store lease. Unsigned, adopted,
    /// inconsistent or unreadable installations return an error rather than
    /// being treated as absent. The observation is not a durable receipt and
    /// must be repeated before a future recovery decision. Cooperative locks
    /// do not fence older binaries or external filesystem writers. Concurrent
    /// replacement of paths by noncooperating writers and Windows junctions or
    /// reparse points have not been verified as a secure boundary.
    pub fn inspect_verified(
        &self,
        plugin_id: &str,
    ) -> Result<InstalledPackageEvidence, InstallError> {
        self.installer.inspect_verified_under_store_lock(plugin_id)
    }

    pub fn commit(
        &mut self,
        prepared: &PreparedPackage,
        options: &CommitOptions,
        hooks: &dyn ActivationHooks,
        reporter: &dyn JobReporter,
    ) -> Result<InstallOutcome, InstallError> {
        self.installer
            .commit_under_store_lock(prepared, options, hooks, reporter)
    }

    pub fn rollback(
        &mut self,
        plugin_id: &str,
        version: Option<&str>,
        hooks: &dyn ActivationHooks,
        reporter: &dyn JobReporter,
    ) -> Result<InstallOutcome, InstallError> {
        self.installer
            .rollback_under_store_lock(plugin_id, version, hooks, reporter, false)
    }

    /// Restore only a previously reviewed local package whose retained tree
    /// still matches its installation-time proof. The provenance check and
    /// swap both happen under this store lease.
    pub fn rollback_local(
        &mut self,
        plugin_id: &str,
        version: Option<&str>,
        hooks: &dyn ActivationHooks,
        reporter: &dyn JobReporter,
    ) -> Result<InstallOutcome, InstallError> {
        self.installer
            .rollback_under_store_lock(plugin_id, version, hooks, reporter, true)
    }

    pub fn uninstall(
        &mut self,
        plugin_id: &str,
        purge_data: Option<&Path>,
        hooks: &dyn ActivationHooks,
    ) -> Result<(), InstallError> {
        self.installer
            .uninstall_under_store_lock(plugin_id, purge_data, hooks)
    }
}

impl Installer {
    /// Acquire the same process-wide store lock used by every cooperative
    /// package mutation. Never hold a sync admission or CloudAuth lock while
    /// waiting for a store lease; acquisition itself is nonblocking.
    pub fn try_lock_store(&self) -> Result<LockedPackageStore<'_>, InstallError> {
        self.try_lock_store_for("package store")
    }

    pub(super) fn try_lock_store_for(
        &self,
        label: &str,
    ) -> Result<LockedPackageStore<'_>, InstallError> {
        // Both acquisitions are nonblocking. On any second-lock failure the
        // first lease drops immediately, so opposite root orders cannot wait
        // on one another. Canonical aliases are locked once.
        let operation = crate::locks::try_package_operation(&self.layout.plugins, label)?;
        fs::create_dir_all(&self.layout.store)?;
        let same = fs::canonicalize(&self.layout.plugins)? == fs::canonicalize(&self.layout.store)?;
        let store_operation = if same {
            None
        } else {
            Some(crate::locks::try_package_operation(
                &self.layout.store,
                label,
            )?)
        };
        Ok(LockedPackageStore {
            installer: self,
            _operation: operation,
            _store_operation: store_operation,
        })
    }
}
