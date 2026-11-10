//! Cloud plugin-selection sync through the shared control plane.
//!
//! Everything here is nonsecret: the opt-in flag, pending/conflict counts,
//! and error text. V2 downloaded plugin selections can be applied after the
//! native account, server, source, package and runtime checks succeed.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tiktools_cloud_sync::{
    SyncCoordError, SyncStatus, V1Edit as Edit, V1RecoveryChoice, V1RecoveryOutcome,
    V1UncertainReview,
};

use super::OperationError;
use crate::AppCore;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SyncNowResult {
    pub pushed: usize,
    pub pulled: usize,
    pub applied: usize,
    pub skipped: usize,
    pub conflicts: Vec<String>,
    /// At least one remote selection was retained without automatic local
    /// application. This is not a count of local differences or failures.
    #[serde(default)]
    pub application_deferred: bool,
}

/// A v2 network pass persists its cache and attempts supported plugin state
/// changes; other Cloud records are not applied by this operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceV2SyncResult {
    pub pushed: usize,
    pub pulled: usize,
    /// Pending edits in the v2 workspace; additional SQLite intents may be
    /// indicated by `more_queued`.
    pub workspace_pending: usize,
    /// Cached records available for application or review.
    pub available_records: usize,
    pub conflicts: Vec<String>,
    pub more_queued: bool,
}

/// A nonsecret projection of a single selection shown during explicit review.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SyncRecoveryValue {
    pub state: String,
    pub version: Option<String>,
    pub enabled: Option<bool>,
}

impl SyncRecoveryValue {
    fn absent() -> Self {
        Self {
            state: "absent".into(),
            version: None,
            enabled: None,
        }
    }
    fn deleted() -> Self {
        Self {
            state: "deleted".into(),
            version: None,
            enabled: None,
        }
    }
    fn selection(value: &tiktools_cloud_sync::PluginSelection) -> Self {
        Self {
            state: "selected".into(),
            version: Some(value.version.clone()),
            enabled: Some(value.enabled),
        }
    }
    fn edit(value: &Edit) -> Self {
        match value {
            Edit::Put(value) => Self::selection(value),
            Edit::Delete => Self::deleted(),
        }
    }
    fn record(value: Option<&tiktools_cloud_sync::V1Record>) -> Self {
        match value {
            None => Self::absent(),
            Some(record) if record.deleted => Self::deleted(),
            Some(record) => record
                .value
                .as_ref()
                .map(Self::selection)
                .unwrap_or_else(Self::absent),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SyncRecoveryReview {
    pub handle: String,
    pub key: String,
    pub base: SyncRecoveryValue,
    pub original: SyncRecoveryValue,
    pub local: SyncRecoveryValue,
    pub cloud: SyncRecoveryValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SyncRecoveryChoice {
    KeepLocal,
    TakeCloud,
    TakeCloudIncludingNewer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SyncRecoveryOutcome {
    Resolved,
    NewerEditNeedsReview,
}

pub(crate) struct RecoveryHandle {
    token: String,
    review: V1UncertainReview,
    newer_confirmation_required: bool,
}

/// One v2 conflict key for the current account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct V2ConflictTarget {
    pub collection: String,
    pub key: String,
}

/// A generic nonsecret projection for UI comparison. `revision` is present
/// for Cloud-backed values; `value` carries the portable JSON when present.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct V2ConflictValue {
    pub state: String,
    pub revision: Option<i64>,
    pub value: Option<serde_json::Value>,
    pub version: Option<String>,
    pub enabled: Option<bool>,
}

/// Reviewed v2 conflict comparison bound to a one-time handle. Resolution
/// re-reads Cloud and rejects the handle unless the fresh comparison matches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct V2ConflictReview {
    pub handle: String,
    pub collection: String,
    pub key: String,
    pub base: V2ConflictValue,
    pub local: V2ConflictValue,
    pub cloud: V2ConflictValue,
    pub has_newer: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum V2ConflictChoice {
    KeepLocal,
    TakeCloud,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum V2ConflictOutcome {
    Resolved,
}

pub(crate) struct V2ConflictHandle {
    token: String,
    session: std::sync::Arc<cloud_client::Session>,
    snapshot: tiktools_cloud_sync::V2ProductConflictSnapshot,
}

fn v2_value_from_edit(edit: &cloud_client::WorkspaceEdit) -> V2ConflictValue {
    match edit {
        cloud_client::WorkspaceEdit::Delete => V2ConflictValue {
            state: "deleted".into(),
            revision: None,
            value: None,
            version: None,
            enabled: None,
        },
        cloud_client::WorkspaceEdit::Put(value) => selection_projection(value),
    }
}

fn v2_value_from_record(record: Option<&cloud_client::WorkspaceRecord>) -> V2ConflictValue {
    match record {
        None => V2ConflictValue {
            state: "absent".into(),
            revision: None,
            value: None,
            version: None,
            enabled: None,
        },
        Some(record) if record.deleted => V2ConflictValue {
            state: "deleted".into(),
            revision: Some(record.revision),
            value: None,
            version: None,
            enabled: None,
        },
        Some(record) => {
            let mut projected =
                record
                    .value
                    .as_ref()
                    .map(selection_projection)
                    .unwrap_or(V2ConflictValue {
                        state: "present".into(),
                        revision: Some(record.revision),
                        value: None,
                        version: None,
                        enabled: None,
                    });
            projected.revision = Some(record.revision);
            projected
        }
    }
}

fn selection_projection(value: &serde_json::Value) -> V2ConflictValue {
    let version = value
        .get("version")
        .and_then(|item| item.as_str())
        .map(str::to_owned);
    let enabled = value.get("enabled").and_then(|item| item.as_bool());
    V2ConflictValue {
        state: "present".into(),
        revision: None,
        value: Some(value.clone()),
        version,
        enabled,
    }
}

fn sync_error(error: SyncCoordError) -> OperationError {
    let message = error.to_string();
    let reason = error.code();
    let mut operation = match reason {
        "disabled" | "not-signed-in" | "network" | "unauthorized" => {
            OperationError::unavailable(message)
        }
        "already-running" | "stale-review" => OperationError::conflict(message),
        "no-pending-edit" | "no-uncertain-write" => OperationError::not_found(message),
        _ => OperationError::internal(message),
    };
    operation.data = Some(serde_json::json!({ "reason": reason }));
    operation
}

impl AppCore {
    fn sync_coordinator(&self) -> Result<&tiktools_cloud_sync::SyncCoordinator, OperationError> {
        self.sync.coordinator().map_err(|reason| {
            let mut operation =
                OperationError::unavailable(format!("cloud sync is unavailable: {reason}"));
            operation.data = Some(serde_json::json!({ "reason": "storage" }));
            operation
        })
    }

    pub async fn sync_status(&self) -> Result<SyncStatus, OperationError> {
        Ok(self.sync_coordinator()?.status().await)
    }

    pub fn sync_set_enabled(&self, enabled: bool) -> Result<SyncStatus, OperationError> {
        let _invalidate = super::cloud_account::CloudContextInvalidation::new(&self.events);
        self.sync_coordinator()?
            .set_enabled(enabled)
            .map_err(sync_error)
    }

    /// Pull and push selections. Remote values remain in the durable account
    /// snapshot; applying them needs a session-, consent- and source-fenced
    /// review and a recoverable local-runtime outcome.
    pub async fn sync_now(&self) -> Result<SyncNowResult, OperationError> {
        let outcome = self
            .sync_coordinator()?
            .sync_now()
            .await
            .map_err(sync_error)?;
        let skipped = outcome.remote.len();
        Ok(SyncNowResult {
            pushed: outcome.pushed,
            pulled: outcome.pulled,
            applied: 0,
            skipped,
            conflicts: outcome.conflicts,
            application_deferred: skipped != 0,
        })
    }

    /// Explicit v2 pass for an already selected, account-scoped protocol.
    /// Supported downloaded plugin state changes are applied when eligible.
    pub async fn sync_v2_now(
        self: &std::sync::Arc<Self>,
    ) -> Result<WorkspaceV2SyncResult, OperationError> {
        self.sync_v2_with_restore(true).await
    }

    pub(crate) async fn sync_v2_with_restore(
        self: &std::sync::Arc<Self>,
        restore: bool,
    ) -> Result<WorkspaceV2SyncResult, OperationError> {
        #[cfg(feature = "persistence")]
        {
            use tiktools_cloud_sync::ProductLocalEditContext;
            let core = self.clone();
            let selected = tokio::task::spawn_blocking(move || {
                core.sync
                    .coordinator()
                    .map_err(|_| OperationError::unavailable("Cloud sync storage is unavailable"))?
                    .with_product_local_edit(|context| context)
                    .map_err(sync_error)
            })
            .await
            .map_err(|_| OperationError::unavailable("Cloud sync worker is unavailable"))??;
            let ProductLocalEditContext::V2(Some(identity)) = selected else {
                return Err(OperationError::unavailable(
                    "Cloud v2 is not enabled for the current account",
                ));
            };
            if restore && self.cloud.auth().session().await.is_none() {
                let _ = self.cloud.auth().restore().await;
            }
            let session = self
                .cloud
                .auth()
                .session()
                .await
                .ok_or_else(|| OperationError::unavailable("Cloud sign-in is required"))?;
            if session.instance_origin() != identity.origin
                || session.account_id() != identity.account_id
            {
                return Err(OperationError::conflict("Cloud account changed; try again"));
            }
            let coordinator = self.sync_coordinator()?.clone();
            let epoch = coordinator
                .capture_v2_config_epoch(&session)
                .ok_or_else(|| OperationError::conflict("Cloud sync setting changed; try again"))?;
            let core = self.clone();
            let origin = identity.origin.clone();
            let account = identity.account_id.clone();
            let outbox_more =
                tokio::task::spawn_blocking(move || -> Result<bool, OperationError> {
                    let coordinator = core.sync.coordinator().map_err(|_| {
                        OperationError::unavailable("Cloud sync storage is unavailable")
                    })?;
                    let source = crate::cloud_sync::product_v2_outbox::SqliteV2ProductSource::new(
                        &core, &origin, &account,
                    );
                    for _ in 0..50 {
                        if core
                            .db
                            .next_plugin_sync_intent(&origin, &account)
                            .map_err(|_| {
                                OperationError::unavailable("Local sync queue is unavailable")
                            })?
                            .is_none()
                        {
                            return Ok(false);
                        }
                        coordinator
                            .stage_v2_product_decision(&source)
                            .map_err(sync_error)?;
                    }
                    Ok(core
                        .db
                        .next_plugin_sync_intent(&origin, &account)
                        .map_err(|_| {
                            OperationError::unavailable("Local sync queue is unavailable")
                        })?
                        .is_some())
                })
                .await
                .map_err(|_| OperationError::unavailable("Cloud sync worker is unavailable"))??;
            let source = std::sync::Arc::new(
                crate::cloud_sync::product_v2_outbox::OwnedSqliteV2UploadSource::new(
                    self.clone(),
                    identity.origin.clone(),
                    identity.account_id.clone(),
                ),
            );
            let outcome = coordinator
                .sync_v2_product(session.clone(), source)
                .await
                .map_err(|_| {
                    OperationError::unavailable(
                        "Cloud v2 sync could not complete; pending changes were retained",
                    )
                })?;
            self.cloud
                .auth()
                .with_current_session(&session, || {
                    coordinator.is_v2_config_epoch_current(&epoch, &session)
                })
                .map_err(|_| OperationError::conflict("Cloud account changed; try again"))?
                .then_some(())
                .ok_or_else(|| OperationError::conflict("Cloud account changed; try again"))?;
            #[cfg(feature = "plugin-install")]
            self.apply_cached_v2_selections(session.clone(), identity, outcome.remote.clone())
                .await;
            self.cloud
                .auth()
                .with_current_session(&session, || {
                    coordinator.is_v2_config_epoch_current(&epoch, &session)
                })
                .map_err(|_| OperationError::conflict("Cloud account changed; try again"))?
                .then_some(())
                .ok_or_else(|| OperationError::conflict("Cloud account changed; try again"))?;
            Ok(WorkspaceV2SyncResult {
                pushed: outcome.pushed,
                pulled: outcome.pulled,
                workspace_pending: outcome.pending,
                available_records: outcome.remote.len(),
                conflicts: outcome
                    .conflicts
                    .into_iter()
                    .map(|key| format!("{}/{}", key.collection, key.key))
                    .collect(),
                more_queued: outbox_more || outcome.more_queued,
            })
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = restore;
            Err(OperationError::unavailable(
                "Cloud v2 sync requires persistence",
            ))
        }
    }

    /// Explicit rebuild from a consistent export after cursor expiry. Never
    /// adopts contents into the running host.
    pub async fn sync_v2_rebuild(self: &std::sync::Arc<Self>) -> Result<(), OperationError> {
        #[cfg(feature = "persistence")]
        {
            let (session, _) = self.v2_identity().await?;
            let coordinator = self.sync_coordinator()?.clone();
            let epoch = self.v2_epoch(&coordinator, &session)?;
            coordinator
                .rebuild_v2_product(session.clone())
                .await
                .map_err(|_| OperationError::unavailable("Cloud v2 rebuild could not complete"))?;
            self.cloud
                .auth()
                .with_current_session(&session, || {
                    coordinator.is_v2_config_epoch_current(&epoch, &session)
                })
                .map_err(|_| OperationError::conflict("Cloud account changed; try again"))?
                .then_some(())
                .ok_or_else(|| OperationError::conflict("Cloud account changed; try again"))
        }
        #[cfg(not(feature = "persistence"))]
        {
            Err(OperationError::unavailable(
                "Cloud v2 rebuild requires persistence",
            ))
        }
    }

    /// Stage owner-owned desktop preferences under v2 fences. The native
    /// object is adapted to the portable subset; unknown fields stay local.
    pub async fn sync_v2_stage_preferences(
        self: &std::sync::Arc<Self>,
        native: serde_json::Value,
    ) -> Result<(), OperationError> {
        #[cfg(feature = "persistence")]
        {
            let portable = tiktools_cloud_sync::preference_from_native(&native)
                .ok_or_else(|| OperationError::invalid("preferences must be an object"))?;
            let (session, _) = self.v2_identity().await?;
            let coordinator = self.sync_coordinator()?.clone();
            let epoch = self.v2_epoch(&coordinator, &session)?;
            let core_session = session.clone();
            let target = cloud_client::WorkspaceKey::new("preferences", "desktop")
                .map_err(|_| OperationError::invalid("invalid preferences key"))?;
            let edit = cloud_client::WorkspaceEdit::Put(portable);
            let core = self.clone();
            tokio::task::spawn_blocking(move || {
                core.sync
                    .coordinator()
                    .map_err(|_| OperationError::unavailable("Cloud sync storage is unavailable"))?
                    .stage_v2_product_edit(core_session, target, edit)
                    .map_err(|_| OperationError::conflict("Cloud v2 staging was rejected"))
            })
            .await
            .map_err(|_| OperationError::unavailable("Cloud sync worker is unavailable"))??;
            self.cloud
                .auth()
                .with_current_session(&session, || {
                    coordinator.is_v2_config_epoch_current(&epoch, &session)
                })
                .map_err(|_| OperationError::conflict("Cloud account changed; try again"))?
                .then_some(())
                .ok_or_else(|| OperationError::conflict("Cloud account changed; try again"))
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = native;
            Err(OperationError::unavailable(
                "Cloud v2 preferences require persistence",
            ))
        }
    }

    /// Stage owner-owned plugin settings under v2 fences. The native object
    /// is adapted to the portable envelope; unknown fields stay local.
    pub async fn sync_v2_stage_plugin_settings(
        self: &std::sync::Arc<Self>,
        plugin_id: &str,
        native: serde_json::Value,
    ) -> Result<(), OperationError> {
        #[cfg(feature = "persistence")]
        {
            if cloud_client::WorkspaceKey::new("plugin-settings", plugin_id).is_err() {
                return Err(OperationError::invalid("invalid plugin id"));
            }
            let portable = tiktools_cloud_sync::settings_from_native(plugin_id, &native)
                .ok_or_else(|| OperationError::invalid("settings must be an object"))?;
            let (session, _) = self.v2_identity().await?;
            let coordinator = self.sync_coordinator()?.clone();
            let epoch = self.v2_epoch(&coordinator, &session)?;
            let core_session = session.clone();
            let target = cloud_client::WorkspaceKey::new("plugin-settings", plugin_id)
                .map_err(|_| OperationError::invalid("invalid plugin id"))?;
            let edit = cloud_client::WorkspaceEdit::Put(portable);
            let core = self.clone();
            tokio::task::spawn_blocking(move || {
                core.sync
                    .coordinator()
                    .map_err(|_| OperationError::unavailable("Cloud sync storage is unavailable"))?
                    .stage_v2_product_edit(core_session, target, edit)
                    .map_err(|_| OperationError::conflict("Cloud v2 staging was rejected"))
            })
            .await
            .map_err(|_| OperationError::unavailable("Cloud sync worker is unavailable"))??;
            self.cloud
                .auth()
                .with_current_session(&session, || {
                    coordinator.is_v2_config_epoch_current(&epoch, &session)
                })
                .map_err(|_| OperationError::conflict("Cloud account changed; try again"))?
                .then_some(())
                .ok_or_else(|| OperationError::conflict("Cloud account changed; try again"))
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (plugin_id, native);
            Err(OperationError::unavailable(
                "Cloud v2 plugin settings require persistence",
            ))
        }
    }

    async fn v2_identity(
        self: &std::sync::Arc<Self>,
    ) -> Result<
        (
            std::sync::Arc<cloud_client::Session>,
            tiktools_cloud_sync::V2LocalEditContext,
        ),
        OperationError,
    > {
        use tiktools_cloud_sync::ProductLocalEditContext;
        let core = self.clone();
        let selected = tokio::task::spawn_blocking(move || {
            core.sync
                .coordinator()
                .map_err(|_| OperationError::unavailable("Cloud sync storage is unavailable"))?
                .with_product_local_edit(|context| context)
                .map_err(sync_error)
        })
        .await
        .map_err(|_| OperationError::unavailable("Cloud sync worker is unavailable"))??;
        let ProductLocalEditContext::V2(Some(identity)) = selected else {
            return Err(OperationError::unavailable(
                "Cloud v2 is not enabled for the current account",
            ));
        };
        if self.cloud.auth().session().await.is_none() {
            let _ = self.cloud.auth().restore().await;
        }
        let session = self
            .cloud
            .auth()
            .session()
            .await
            .ok_or_else(|| OperationError::unavailable("Cloud sign-in is required"))?;
        if session.instance_origin() != identity.origin
            || session.account_id() != identity.account_id
        {
            return Err(OperationError::conflict("Cloud account changed; try again"));
        }
        Ok((session, identity))
    }

    fn v2_epoch(
        &self,
        coordinator: &tiktools_cloud_sync::SyncCoordinator,
        session: &std::sync::Arc<cloud_client::Session>,
    ) -> Result<tiktools_cloud_sync::SyncConfigEpoch, OperationError> {
        coordinator
            .capture_v2_config_epoch(session)
            .ok_or_else(|| OperationError::conflict("Cloud sync setting changed; try again"))
    }

    /// Local-only list of v2 conflicts for the current account.
    pub async fn sync_v2_conflict_list(
        self: &std::sync::Arc<Self>,
    ) -> Result<Vec<V2ConflictTarget>, OperationError> {
        #[cfg(feature = "persistence")]
        {
            let (session, _) = self.v2_identity().await?;
            let coordinator = self.sync_coordinator()?.clone();
            let epoch = self.v2_epoch(&coordinator, &session)?;
            let core_session = session.clone();
            let keys = tokio::task::spawn_blocking({
                let list_coordinator = coordinator.clone();
                move || list_coordinator.v2_product_conflict_keys(&core_session)
            })
            .await
            .map_err(|_| OperationError::unavailable("Cloud sync worker is unavailable"))?
            .map_err(|_| OperationError::unavailable("Cloud v2 conflict list is unavailable"))?;
            self.cloud
                .auth()
                .with_current_session(&session, || {
                    coordinator.is_v2_config_epoch_current(&epoch, &session)
                })
                .map_err(|_| OperationError::conflict("Cloud account changed; try again"))?
                .then_some(())
                .ok_or_else(|| OperationError::conflict("Cloud account changed; try again"))?;
            Ok(keys
                .into_iter()
                .map(|target| V2ConflictTarget {
                    collection: target.collection,
                    key: target.key,
                })
                .collect())
        }
        #[cfg(not(feature = "persistence"))]
        {
            Err(OperationError::unavailable(
                "Cloud v2 conflicts require persistence",
            ))
        }
    }

    /// Capture one v2 conflict comparison. The handle is one-time; resolution
    /// re-reads Cloud and requires the same comparison.
    pub async fn sync_v2_review_conflict(
        self: &std::sync::Arc<Self>,
        collection: &str,
        key: &str,
    ) -> Result<V2ConflictReview, OperationError> {
        #[cfg(feature = "persistence")]
        {
            let (session, _) = self.v2_identity().await?;
            let coordinator = self.sync_coordinator()?.clone();
            let epoch = self.v2_epoch(&coordinator, &session)?;
            let target = cloud_client::WorkspaceKey::new(collection.to_owned(), key.to_owned())
                .map_err(|_| OperationError::not_found("unknown Cloud conflict"))?;
            let snapshot = coordinator
                .review_v2_product(session.clone(), target)
                .await
                .map_err(|_| OperationError::conflict("Cloud conflict changed; review again"))?;
            self.cloud
                .auth()
                .with_current_session(&session, || {
                    coordinator.is_v2_config_epoch_current(&epoch, &session)
                })
                .map_err(|_| OperationError::conflict("Cloud account changed; try again"))?
                .then_some(())
                .ok_or_else(|| OperationError::conflict("Cloud account changed; try again"))?;
            let entry = &snapshot.entry;
            let base = match entry.pending.as_ref() {
                Some(pending) => v2_value_from_record(pending.base_record.as_ref()),
                None => v2_value_from_record(entry.remote.as_ref()),
            };
            let local = match entry.pending.as_ref() {
                Some(pending) => v2_value_from_edit(&pending.edit),
                None => v2_value_from_record(entry.remote.as_ref()),
            };
            let cloud = v2_value_from_record(snapshot.cloud_record.as_ref());
            let has_newer = entry.next.is_some();
            let mut bytes = [0u8; 32];
            getrandom::fill(&mut bytes)
                .map_err(|_| OperationError::internal("could not create review handle"))?;
            let token = bytes
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>();
            let review = V2ConflictReview {
                handle: token.clone(),
                collection: snapshot.target.collection.clone(),
                key: snapshot.target.key.clone(),
                base,
                local,
                cloud,
                has_newer,
            };
            {
                let mut slot = self.sync.v2_conflict.lock().await;
                *slot = Some(V2ConflictHandle {
                    token,
                    session,
                    snapshot,
                });
            }
            Ok(review)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (collection, key);
            Err(OperationError::unavailable(
                "Cloud v2 conflicts require persistence",
            ))
        }
    }

    /// Resolve one reviewed v2 conflict. Keep-local rebases onto the fresh
    /// Cloud base (still pending); take-cloud discards the pending edit.
    /// Neither choice applies Cloud state locally.
    pub async fn sync_v2_resolve_conflict(
        self: &std::sync::Arc<Self>,
        handle: &str,
        choice: V2ConflictChoice,
    ) -> Result<V2ConflictOutcome, OperationError> {
        #[cfg(feature = "persistence")]
        {
            let mut slot = self.sync.v2_conflict.lock().await;
            let item = slot
                .take()
                .ok_or_else(|| OperationError::conflict("review expired; review again"))?;
            if item.token != handle {
                *slot = Some(item);
                return Err(OperationError::conflict("review expired; review again"));
            }
            let session = item.session.clone();
            let target = item.snapshot.target.clone();
            let expected = item.snapshot.clone();
            let keep_local = choice == V2ConflictChoice::KeepLocal;
            let coordinator = self.sync_coordinator()?.clone();
            let epoch = self.v2_epoch(&coordinator, &session)?;
            let source = std::sync::Arc::new(
                crate::cloud_sync::product_v2_outbox::OwnedSqliteV2UploadSource::new(
                    self.clone(),
                    session.instance_origin(),
                    session.account_id().to_owned(),
                ),
            );
            let result = coordinator
                .resolve_v2_product(session.clone(), source, target, expected, keep_local)
                .await;
            match result {
                Ok(()) => {}
                Err(_) => {
                    *slot = Some(item);
                    return Err(OperationError::conflict(
                        "Cloud conflict changed; review again",
                    ));
                }
            }
            self.cloud
                .auth()
                .with_current_session(&session, || {
                    coordinator.is_v2_config_epoch_current(&epoch, &session)
                })
                .map_err(|_| OperationError::conflict("Cloud account changed; try again"))?
                .then_some(())
                .ok_or_else(|| OperationError::conflict("Cloud account changed; try again"))?;
            Ok(V2ConflictOutcome::Resolved)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (handle, choice);
            Err(OperationError::unavailable(
                "Cloud v2 conflicts require persistence",
            ))
        }
    }

    /// Keep both sides of a conflict until source-fenced recovery can bind the
    /// reviewed Cloud and local states to a durable outcome. Neither choice
    /// may discard or rebase a newer local edit through the legacy bypass.
    pub async fn sync_resolve_selection(
        &self,
        _key: &str,
        _keep_local: bool,
    ) -> Result<SyncStatus, OperationError> {
        let mut error = OperationError::unavailable(
            "Cloud conflict resolution is unavailable until local application and recovery are protected",
        );
        error.data = Some(serde_json::json!({ "reason": "conflict-review-unavailable" }));
        Err(error)
    }

    /// List only keys; marker details and credentials stay in the native process.
    pub async fn sync_uncertain_keys(&self) -> Result<Vec<String>, OperationError> {
        self.sync_coordinator()?
            .uncertain_v1_keys()
            .await
            .map_err(sync_error)
    }

    pub async fn sync_review_uncertain(
        &self,
        key: &str,
    ) -> Result<SyncRecoveryReview, OperationError> {
        let mut slot = self.sync.recovery.lock().await;
        *slot = None;
        let review = self
            .sync_coordinator()?
            .review_uncertain_v1(key)
            .await
            .map_err(sync_error)?;
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes)
            .map_err(|_| OperationError::internal("could not create review handle"))?;
        let token = bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let projection = SyncRecoveryReview {
            handle: token.clone(),
            key: review.original().key().to_owned(),
            base: SyncRecoveryValue::record(review.original().base_record()),
            original: SyncRecoveryValue::edit(&review.original().pending().edit),
            local: review
                .local()
                .and_then(|entry| entry.pending.as_ref())
                .map(|p| SyncRecoveryValue::edit(&p.edit))
                .unwrap_or_else(SyncRecoveryValue::absent),
            cloud: SyncRecoveryValue::record(review.remote()),
        };
        *slot = Some(RecoveryHandle {
            token,
            review,
            newer_confirmation_required: false,
        });
        Ok(projection)
    }

    /// Resolution remains unavailable until local mutations and sync staging
    /// share a crash-safe boundary. Otherwise even keeping the local value can
    /// upload an older staged edit after a newer change failed to stage.
    pub async fn sync_resolve_uncertain(
        &self,
        _handle: &str,
        _choice: SyncRecoveryChoice,
    ) -> Result<SyncRecoveryOutcome, OperationError> {
        Err(OperationError::unavailable(
            "recovery resolution is unavailable while local changes are being protected",
        ))
    }

    /// The native review remains private and session-bound. This must only be
    /// wired after durable source/application fencing is in place.
    #[allow(dead_code)]
    async fn sync_resolve_uncertain_when_fenced(
        &self,
        handle: &str,
        choice: SyncRecoveryChoice,
    ) -> Result<SyncRecoveryOutcome, OperationError> {
        let mut slot = self.sync.recovery.lock().await;
        let mut item = slot
            .take()
            .ok_or_else(|| OperationError::conflict("review expired; review again"))?;
        if item.token != handle {
            *slot = Some(item);
            return Err(OperationError::conflict("review expired; review again"));
        }
        let native_choice = match choice {
            SyncRecoveryChoice::KeepLocal => V1RecoveryChoice::KeepLocal,
            SyncRecoveryChoice::TakeCloud => V1RecoveryChoice::TakeCloudIfUnchanged,
            SyncRecoveryChoice::TakeCloudIncludingNewer if item.newer_confirmation_required => {
                V1RecoveryChoice::TakeCloudIncludingNewer
            }
            SyncRecoveryChoice::TakeCloudIncludingNewer => {
                *slot = Some(item);
                return Err(OperationError::conflict("review newer local changes first"));
            }
        };
        match self
            .sync_coordinator()?
            .resolve_uncertain_v1(&item.review, native_choice)
            .await
            .map_err(sync_error)?
        {
            V1RecoveryOutcome::Resolved => Ok(SyncRecoveryOutcome::Resolved),
            V1RecoveryOutcome::NewerEditNeedsReview => {
                item.newer_confirmation_required = true;
                *slot = Some(item);
                Ok(SyncRecoveryOutcome::NewerEditNeedsReview)
            }
        }
    }
}
