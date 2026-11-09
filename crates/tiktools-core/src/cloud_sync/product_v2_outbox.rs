//! Native v2 staging adapter. Not wired to product lifecycle or networking.
//! Product activation requires complete mutation admission, package removal
//! reconciliation, and source-proof validation immediately before dispatch.
use crate::AppCore;
use tiktools_cloud_sync::{
    PluginSelection, SyncCoordError, V2ProductDecision, V2ProductSource, V2ProductUploadSource,
    WorkspaceCoordError,
};

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

impl V2ProductUploadSource for SqliteV2ProductSource<'_> {
    fn with_fenced_upload(
        &self,
        decision: &V2ProductDecision,
        admit: &mut dyn FnMut() -> Result<(), WorkspaceCoordError>,
    ) -> Result<(), WorkspaceCoordError> {
        let stale = || WorkspaceCoordError::State(cloud_client::WorkspaceStateError::StaleReview);
        if decision.origin != self.origin || decision.account_id != self.account {
            return Err(stale());
        }
        let selection = decision.selection.as_ref().ok_or_else(stale)?;
        if selection.plugin_id != decision.plugin_id {
            return Err(stale());
        }
        let runtime = self
            .core
            .try_plugin_sync_runtime_guard()
            .map_err(|_| stale())?;
        if !runtime.matches_selection(&decision.plugin_id, &selection.version, selection.enabled) {
            return Err(stale());
        }
        self.core
            .db
            .with_current_v2_upload_source::<_, crate::db::DatabaseError>(
                &decision.plugin_id,
                decision.source_generation,
                selection.enabled,
                || Ok(admit()),
            )
            .map_err(|_| stale())?
    }
}

/// Owned native source for the background dispatcher. The synchronous source
/// guard is still scoped to one admission; no SQLite or runtime guard escapes
/// into the HTTP await.
pub(crate) struct OwnedSqliteV2UploadSource {
    core: std::sync::Arc<AppCore>,
    origin: String,
    account: String,
}

impl OwnedSqliteV2UploadSource {
    pub(crate) fn new(core: std::sync::Arc<AppCore>, origin: String, account: String) -> Self {
        Self {
            core,
            origin,
            account,
        }
    }
}

impl V2ProductUploadSource for OwnedSqliteV2UploadSource {
    fn with_fenced_upload(
        &self,
        decision: &V2ProductDecision,
        admit: &mut dyn FnMut() -> Result<(), WorkspaceCoordError>,
    ) -> Result<(), WorkspaceCoordError> {
        SqliteV2ProductSource::new(&self.core, &self.origin, &self.account)
            .with_fenced_upload(decision, admit)
    }
}
