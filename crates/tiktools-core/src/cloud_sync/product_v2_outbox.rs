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

/// A local-only observation, bound to a source revision and host lifetime.
/// It does not prove Cloud identity, consent or server-side quiescence.
pub(crate) struct LocalUntrackedSelectionReview {
    source: crate::db::UntrackedPluginReview,
    version: String,
    runtime_epoch: String,
}

/// Local observation of a durable candidate; reconstructed after restart.
/// It carries no Cloud authorization and does not stage or send a request.
pub(crate) struct ReconciledRuntimeReview {
    entry: crate::db::PluginSyncOutboxEntry,
    runtime_epoch: String,
}

impl<'a> SqliteV2ProductSource<'a> {
    pub(crate) fn new(core: &'a AppCore, origin: &'a str, account: &'a str) -> Self {
        Self {
            core,
            origin,
            account,
        }
    }

    /// Observe an installed local selection without clearing its marker.
    /// Tombstones and mismatched runtime state remain blocked for review.
    #[allow(dead_code)]
    pub(crate) fn review_untracked_selection(
        &self,
        id: &str,
    ) -> Result<Option<LocalUntrackedSelectionReview>, SyncCoordError> {
        self.with_untracked_source(id, None, |source, version, epoch| {
            Ok(LocalUntrackedSelectionReview {
                source: source.clone(),
                version: version.to_owned(),
                runtime_epoch: epoch.to_owned(),
            })
        })
    }

    /// Revalidate the same local observation through a short, non-async
    /// callback. The callback must not perform network I/O, reenter plugin
    /// lifecycle, or treat this local fence as Cloud consent or a barrier.
    #[allow(dead_code)]
    pub(crate) fn with_current_untracked_selection<T>(
        &self,
        review: &LocalUntrackedSelectionReview,
        operation: impl FnOnce(&crate::db::UntrackedPluginReview, &str) -> Result<T, SyncCoordError>,
    ) -> Result<T, SyncCoordError> {
        self.with_untracked_source(&review.source.plugin_id, None, |source, version, epoch| {
            if source != &review.source
                || version != review.version
                || epoch != review.runtime_epoch
            {
                return Err(unavailable());
            }
            operation(source, version)
        })?
        .ok_or_else(unavailable)
    }

    /// Transfer one explicitly approved local review while the coordinator
    /// holds the account/session and source barriers. This is not wired to
    /// product UI; the caller must first obtain the user's explicit choice.
    /// Runtime confirmation and outbox staging remain separate steps.
    #[allow(dead_code)]
    pub(crate) fn reconcile_reviewed_under_barrier(
        &self,
        review: &LocalUntrackedSelectionReview,
        admission: &tiktools_cloud_sync::V2ReconciliationAdmission<'_>,
    ) -> Result<(), WorkspaceCoordError> {
        let context = admission.context();
        if context.origin != self.origin
            || context.account_id != self.account
            || admission.target().collection != "plugin-selections"
            || admission.target().key != review.source.plugin_id
        {
            return Err(WorkspaceCoordError::Ineligible);
        }
        self.with_untracked_source(
            &review.source.plugin_id,
            Some(admission),
            |source, version, epoch| {
                if source != &review.source
                    || version != review.version
                    || epoch != review.runtime_epoch
                {
                    return Err(unavailable());
                }
                self.core
                    .db
                    .reconcile_untracked_plugin(
                        source,
                        &crate::db::LocalPluginSyncIntent {
                            origin: &context.origin,
                            account_id: &context.account_id,
                            config_generation: context.config_generation,
                            version: Some(version),
                        },
                    )
                    .map_err(|_| unavailable())
            },
        )
        .map_err(|_| WorkspaceCoordError::Ineligible)?
        .ok_or(WorkspaceCoordError::Ineligible)
    }

    /// Reconstruct a local runtime observation from a durable candidate.
    /// The source must still match and no local marker may remain for it.
    #[allow(dead_code)]
    pub(crate) fn review_reconciled_runtime(
        &self,
        id: &str,
    ) -> Result<Option<ReconciledRuntimeReview>, SyncCoordError> {
        if self
            .core
            .shutdown_started
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return Err(unavailable());
        }
        let runtime = self
            .core
            .try_plugin_sync_runtime_guard()
            .map_err(|_| unavailable())?;
        let fence = tiktools_cloud_sync::ProductEmissionFence::open(
            self.core.db.paths().data.join("plugin-sync-emission"),
        )
        .map_err(|_| unavailable())?;
        let _lease = fence.try_untracked_edit().map_err(|_| unavailable())?;
        if self
            .core
            .db
            .review_untracked_plugin(id)
            .map_err(|_| unavailable())?
            .is_some()
        {
            return Err(unavailable());
        }
        let Some(entry) = self
            .core
            .db
            .plugin_sync_intent_for(self.origin, self.account, id)
            .map_err(|_| unavailable())?
        else {
            return Ok(None);
        };
        let (Some(version), Some(enabled)) = (entry.version.as_deref(), entry.enabled) else {
            return Err(unavailable());
        };
        if entry.generation <= 0
            || entry.generation != entry.source_generation
            || !runtime.matches_selection(id, version, enabled)
        {
            return Err(unavailable());
        }
        let epoch = self.core.sync.runtime_epoch().map_err(|_| unavailable())?;
        if self
            .core
            .shutdown_started
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return Err(unavailable());
        }
        Ok(Some(ReconciledRuntimeReview {
            entry,
            runtime_epoch: epoch.to_owned(),
        }))
    }

    /// Confirm the exact reconciled outbox candidate against this host's
    /// current runtime. This performs no Cloud operation and grants no
    /// consent; the later staging path must independently recheck identity
    /// and consent. A failed confirmation leaves the durable candidate for
    /// explicit retry rather than rebuilding or replaying the local edit.
    #[allow(dead_code)]
    pub(crate) fn confirm_reconciled_runtime(
        &self,
        review: &ReconciledRuntimeReview,
    ) -> Result<(), SyncCoordError> {
        if self
            .core
            .shutdown_started
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return Err(unavailable());
        }
        let runtime = self
            .core
            .try_plugin_sync_runtime_guard()
            .map_err(|_| unavailable())?;
        let fence = tiktools_cloud_sync::ProductEmissionFence::open(
            self.core.db.paths().data.join("plugin-sync-emission"),
        )
        .map_err(|_| unavailable())?;
        let _lease = fence.try_untracked_edit().map_err(|_| unavailable())?;
        let epoch = self.core.sync.runtime_epoch().map_err(|_| unavailable())?;
        let (Some(version), Some(enabled)) =
            (review.entry.version.as_deref(), review.entry.enabled)
        else {
            return Err(unavailable());
        };
        if epoch != review.runtime_epoch
            || review.entry.origin != self.origin
            || review.entry.account_id != self.account
            || !runtime.matches_selection(&review.entry.plugin_id, version, enabled)
        {
            return Err(unavailable());
        }
        let entry = self
            .core
            .db
            .plugin_sync_intent_for(self.origin, self.account, &review.entry.plugin_id)
            .map_err(|_| unavailable())?
            .ok_or_else(unavailable)?;
        if entry != review.entry
            || entry.generation != entry.source_generation
            || self
                .core
                .db
                .review_untracked_plugin(&entry.plugin_id)
                .map_err(|_| unavailable())?
                .is_some()
            || self
                .core
                .shutdown_started
                .load(std::sync::atomic::Ordering::Acquire)
        {
            return Err(unavailable());
        }
        if !self
            .core
            .db
            .confirm_plugin_sync_runtime(&entry, epoch)
            .map_err(|_| unavailable())?
        {
            return Err(unavailable());
        }
        if self
            .core
            .shutdown_started
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return Err(unavailable());
        }
        Ok(())
    }

    fn with_untracked_source<T>(
        &self,
        id: &str,
        admission: Option<&tiktools_cloud_sync::V2ReconciliationAdmission<'_>>,
        operation: impl FnOnce(
            &crate::db::UntrackedPluginReview,
            &str,
            &str,
        ) -> Result<T, SyncCoordError>,
    ) -> Result<Option<T>, SyncCoordError> {
        if self
            .core
            .shutdown_started
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return Err(unavailable());
        }
        let runtime = self
            .core
            .try_plugin_sync_runtime_guard()
            .map_err(|_| unavailable())?;
        let source_dir = self.core.db.paths().data.join("plugin-sync-emission");
        let _lease = if let Some(admission) = admission {
            if !admission.matches_source_dir(&source_dir) {
                return Err(unavailable());
            }
            None
        } else {
            let fence = tiktools_cloud_sync::ProductEmissionFence::open(source_dir)
                .map_err(|_| unavailable())?;
            Some(fence.try_untracked_edit().map_err(|_| unavailable())?)
        };
        let Some(source) = self
            .core
            .db
            .review_untracked_plugin(id)
            .map_err(|_| unavailable())?
        else {
            return Ok(None);
        };
        if !source.installed {
            return Err(unavailable());
        }
        let plugin = self.core.plugins.get(id).ok_or_else(unavailable)?;
        if !runtime.matches_selection(id, &plugin.manifest.version, source.enabled) {
            return Err(unavailable());
        }
        let epoch = self.core.sync.runtime_epoch().map_err(|_| unavailable())?;
        if self
            .core
            .shutdown_started
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return Err(unavailable());
        }
        operation(&source, &plugin.manifest.version, epoch).map(Some)
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
    fn acquire_emission_guard(&self) -> Result<Box<dyn Send>, WorkspaceCoordError> {
        if self
            .core
            .shutdown_started
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return Err(WorkspaceCoordError::Ineligible);
        }
        let fence = tiktools_cloud_sync::ProductEmissionFence::open(
            self.core.db.paths().data.join("plugin-sync-emission"),
        )
        .map_err(|_| WorkspaceCoordError::Ineligible)?;
        let lease = fence
            .try_emission()
            .map_err(|_| WorkspaceCoordError::Ineligible)?;
        if self
            .core
            .db
            .has_untracked_plugin_sync()
            .map_err(|_| WorkspaceCoordError::Ineligible)?
        {
            return Err(WorkspaceCoordError::Ineligible);
        }
        if self
            .core
            .shutdown_started
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return Err(WorkspaceCoordError::Ineligible);
        }
        Ok(Box::new(lease))
    }

    fn with_fenced_upload(
        &self,
        decision: &V2ProductDecision,
        admit: &mut dyn FnMut() -> Result<(), WorkspaceCoordError>,
    ) -> Result<(), WorkspaceCoordError> {
        if self
            .core
            .shutdown_started
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return Err(WorkspaceCoordError::Ineligible);
        }
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
                || {
                    if self
                        .core
                        .shutdown_started
                        .load(std::sync::atomic::Ordering::Acquire)
                    {
                        Ok(Err(WorkspaceCoordError::Ineligible))
                    } else {
                        Ok(admit())
                    }
                },
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
    fn acquire_emission_guard(&self) -> Result<Box<dyn Send>, WorkspaceCoordError> {
        SqliteV2ProductSource::new(&self.core, &self.origin, &self.account).acquire_emission_guard()
    }

    fn with_fenced_upload(
        &self,
        decision: &V2ProductDecision,
        admit: &mut dyn FnMut() -> Result<(), WorkspaceCoordError>,
    ) -> Result<(), WorkspaceCoordError> {
        SqliteV2ProductSource::new(&self.core, &self.origin, &self.account)
            .with_fenced_upload(decision, admit)
    }
}
