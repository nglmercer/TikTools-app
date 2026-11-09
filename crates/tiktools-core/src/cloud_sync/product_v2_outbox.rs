//! Native v2 staging adapter. Not wired to product lifecycle or networking.
//! Product activation requires complete mutation admission, package removal
//! reconciliation, and source-proof validation immediately before dispatch.
use crate::AppCore;
use tiktools_cloud_sync::{PluginSelection, SyncCoordError, V2ProductDecision, V2ProductSource};

fn unavailable() -> SyncCoordError {
    SyncCoordError::Local("local sync source needs review".into())
}

pub(crate) struct SqliteV2ProductSource<'a> {
    core: &'a AppCore,
    origin: &'a str,
    account: &'a str,
}
impl<'a> SqliteV2ProductSource<'a> {
    pub(crate) fn new(core: &'a AppCore, origin: &'a str, account: &'a str) -> Self {
        Self {
            core,
            origin,
            account,
        }
    }
}
impl V2ProductSource for SqliteV2ProductSource<'_> {
    fn with_fenced_decision<T>(
        &self,
        stage: impl FnOnce(&V2ProductDecision) -> Result<T, SyncCoordError>,
    ) -> Result<T, SyncCoordError> {
        let runtime = self
            .core
            .try_plugin_sync_runtime_guard()
            .map_err(|_| unavailable())?;
        let epoch = self.core.sync.runtime_epoch().map_err(|_| unavailable())?;
        let entry = self
            .core
            .db
            .next_plugin_sync_intent(self.origin, self.account)
            .map_err(|_| unavailable())?
            .ok_or_else(unavailable)?;
        if entry.origin != self.origin
            || entry.account_id != self.account
            || entry.generation <= 0
            || entry.source_generation != entry.generation
            || entry.runtime_epoch.as_deref() != Some(epoch)
        {
            return Err(unavailable());
        }
        let (version, enabled) = match (&entry.version, entry.enabled) {
            (Some(version), Some(enabled))
                if runtime.matches_selection(&entry.plugin_id, version, enabled) =>
            {
                (version.clone(), enabled)
            }
            // A tombstone requires separate package/filesystem reconciliation.
            _ => return Err(unavailable()),
        };
        let decision = V2ProductDecision {
            origin: entry.origin.clone(),
            account_id: entry.account_id.clone(),
            config_generation: entry.config_generation,
            source_generation: entry.generation as u64,
            plugin_id: entry.plugin_id.clone(),
            selection: Some(PluginSelection {
                schema_version: 1,
                plugin_id: entry.plugin_id.clone(),
                version,
                enabled,
            }),
        };
        let result = stage(&decision)?;
        if !self
            .core
            .db
            .acknowledge_plugin_sync_intent(&entry, epoch)
            .map_err(|_| unavailable())?
        {
            return Err(unavailable());
        }
        Ok(result)
    }
}
