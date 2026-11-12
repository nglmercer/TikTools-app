//! Discovery snapshots and catalog leases. Scan publication and revision changes remain under
//! the manager locks.

use super::{lock_poisoned, PluginManager};
use crate::{
    discovery::{read_discovered_plugin, MAX_DIRECTORY_ENTRIES},
    DiscoveredPlugin, PluginLoaderError,
};
use std::{collections::BTreeMap, fs};
use tiktools_plugin_api::sync::{recover_rwlock_read, recover_rwlock_write};

impl PluginManager {
    pub fn scan(&self) -> Result<Vec<DiscoveredPlugin>, PluginLoaderError> {
        let _lifecycle = self
            .lifecycle
            .lock()
            .map_err(|_| lock_poisoned("plugin lifecycle"))?;
        self.reap_uncertain_startups();
        {
            let mut registry = self
                .registry
                .write()
                .map_err(|_| lock_poisoned("plugin registry"))?;
            registry.complete_for_cutover = false;
            self.bump_registry_revision();
        }
        let mut discovered = BTreeMap::new();
        let mut complete = true;
        for root in &self.roots {
            match fs::metadata(&root.path) {
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    // An absent configured root cannot prove an empty local
                    // inventory, even in a freshly started host.
                    complete = false;
                    continue;
                }
                Err(error) => {
                    complete = false;
                    tracing::warn!(path = %root.path.display(), %error, "could not inspect plugin root");
                    continue;
                }
            }
            let entries = fs::read_dir(&root.path).map_err(|error| {
                PluginLoaderError::InvalidDirectory(format!("{}: {error}", root.path.display()))
            })?;
            for (index, entry) in entries.enumerate() {
                if index >= MAX_DIRECTORY_ENTRIES {
                    complete = false;
                    tracing::warn!(path = %root.path.display(), "plugin directory entry limit reached");
                    break;
                }
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(error) => {
                        complete = false;
                        tracing::warn!(path = %root.path.display(), %error, "could not inspect plugin entry");
                        continue;
                    }
                };
                let directory = entry.path();
                let hidden = entry.file_name().to_string_lossy().starts_with('.');
                if hidden {
                    continue;
                }
                let kind = match entry.file_type() {
                    Ok(kind) => kind,
                    Err(error) => {
                        complete = false;
                        tracing::warn!(path = %directory.display(), %error, "could not inspect plugin entry type");
                        continue;
                    }
                };
                if kind.is_symlink() {
                    complete = false;
                }
                if !kind.is_dir() {
                    continue;
                }
                match read_discovered_plugin(&directory, root.source) {
                    Ok(plugin) => {
                        if let Some(previous) =
                            discovered.insert(plugin.manifest.id.clone(), plugin)
                        {
                            complete = false;
                            tracing::debug!(
                                id = %previous.manifest.id,
                                "plugin overridden by a later runtime root"
                            );
                        }
                    }
                    Err(error) => {
                        complete = false;
                        tracing::warn!(path = %directory.display(), %error, "plugin skipped")
                    }
                }
            }
        }

        let running_ids = self
            .instances
            .read()
            .map_err(|_| lock_poisoned("plugin instances"))?
            .keys()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        let result: Vec<_> = discovered
            .into_values()
            .map(|mut plugin| {
                // The active instance registry is authoritative. A scan must
                // never report a live runtime as stopped merely because it
                // rebuilt the discovered package record.
                plugin.running = running_ids.contains(&plugin.manifest.id);
                plugin
            })
            .collect();
        let mut registry = self
            .registry
            .write()
            .map_err(|_| lock_poisoned("plugin registry"))?;
        registry.entries = result
            .iter()
            .cloned()
            .map(|plugin| (plugin.manifest.id.clone(), plugin))
            .collect();
        registry.complete_for_cutover = complete;
        self.bump_registry_revision();
        drop(registry);
        Ok(result)
    }

    pub fn list(&self) -> Vec<DiscoveredPlugin> {
        recover_rwlock_read(&self.registry, "plugin registry")
            .entries
            .values()
            .cloned()
            .collect()
    }

    /// Inventory for an informational migration preview. It is not a proof
    /// that files have stayed unchanged since the scan, nor permission to
    /// apply or migrate them. A failed/partial scan invalidates this evidence
    /// while ordinary plugin listing and local operation remain available.
    /// Every configured root, including optional development roots, must exist;
    /// absence cannot distinguish an empty inventory from a missing store.
    pub fn list_complete_for_cutover(
        &self,
    ) -> Result<(u64, Vec<DiscoveredPlugin>), PluginLoaderError> {
        let registry = self
            .registry
            .read()
            .map_err(|_| lock_poisoned("plugin registry"))?;
        if !registry.complete_for_cutover {
            return Err(PluginLoaderError::InvalidDirectory(
                "plugin inventory is incomplete".into(),
            ));
        }
        Ok((
            self.registry_revision(),
            registry.entries.values().cloned().collect(),
        ))
    }

    /// Hold the current complete catalog through a synchronous native
    /// callback. The callback must not reenter the manager or do network I/O.
    /// This protects only the in-process registry, not source files or other
    /// processes. Slow filesystem work must run on a blocking thread.
    pub fn with_complete_cutover_catalog<R>(
        &self,
        operation: impl FnOnce(u64, Vec<DiscoveredPlugin>) -> R,
    ) -> Result<R, PluginLoaderError> {
        let registry = self
            .registry
            .read()
            .map_err(|_| lock_poisoned("plugin registry"))?;
        if !registry.complete_for_cutover {
            return Err(PluginLoaderError::InvalidDirectory(
                "plugin inventory is incomplete".into(),
            ));
        }
        let result = operation(
            self.registry_revision(),
            registry.entries.values().cloned().collect(),
        );
        drop(registry);
        Ok(result)
    }

    pub fn get(&self, id: &str) -> Option<DiscoveredPlugin> {
        recover_rwlock_read(&self.registry, "plugin registry")
            .entries
            .get(id)
            .cloned()
    }

    /// Reads catalog metadata without recovering a poisoned registry. Use
    /// this when the result authorizes a new runtime or sync operation.
    pub fn get_for_admission(
        &self,
        id: &str,
    ) -> Result<Option<DiscoveredPlugin>, PluginLoaderError> {
        let registry = self
            .registry
            .read()
            .map_err(|_| lock_poisoned("plugin registry"))?;
        Ok(registry.entries.get(id).cloned())
    }

    /// Orders trusted final admission against every scan/start/stop/retirement.
    /// The callback must be synchronous and must not reenter lifecycle methods.
    pub fn with_catalog_lease<R>(
        &self,
        apply: impl FnOnce(&Self) -> R,
    ) -> Result<R, PluginLoaderError> {
        let _lifecycle = self
            .lifecycle
            .lock()
            .map_err(|_| lock_poisoned("plugin lifecycle"))?;
        Ok(apply(self))
    }

    pub(super) fn set_running(&self, id: &str, running: bool) {
        let mut registry = recover_rwlock_write(&self.registry, "plugin registry");
        if let Some(plugin) = registry.entries.get_mut(id) {
            plugin.running = running;
        }
        self.bump_registry_revision();
    }
}
