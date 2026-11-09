//! Explicit, non-replaying recovery for a potentially committed v1 write.
use std::{path::PathBuf, sync::Arc};

use cloud_client::{Edit, Entry, Pending, Record, Session, StateSnapshot, StoreError, SyncError};

use crate::{SyncCoordError, SyncCoordinator, V1UncertainUpload};

#[derive(
    Clone, Copy, Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema, PartialEq, Eq,
)]
#[serde(rename_all = "camelCase")]
pub enum V1RecoveryChoice {
    KeepLocal,
    TakeCloudIfUnchanged,
    TakeCloudIncludingNewer,
}

#[derive(
    Clone, Copy, Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema, PartialEq, Eq,
)]
#[serde(rename_all = "camelCase")]
pub enum V1RecoveryOutcome {
    Resolved,
    NewerEditNeedsReview,
}

/// Native-only evidence captured by a fresh authenticated read. Product code
/// must retain the object server-side and never reconstruct it from UI fields.
pub struct V1UncertainReview {
    binding: Arc<()>,
    session: Arc<Session>,
    config_generation: u128,
    marker_path: PathBuf,
    original: V1UncertainUpload,
    local: StateSnapshot,
    remote: Option<Record>,
}
impl V1UncertainReview {
    pub fn original(&self) -> &V1UncertainUpload {
        &self.original
    }
    pub fn local(&self) -> Option<&Entry> {
        self.local.state.entries.get(self.original.key())
    }
    pub fn remote(&self) -> Option<&Record> {
        self.remote.as_ref()
    }
}

async fn fresh_record(session: &Session, key: &str) -> Result<Option<Record>, SyncCoordError> {
    let export = session.export_sync().await?;
    Ok(export.records.into_iter().find(|record| record.key == key))
}

fn reject_rewind(local: Option<&Record>, remote: Option<&Record>) -> Result<(), SyncCoordError> {
    if let Some(local) = local {
        let Some(remote) = remote else {
            return Err(SyncError::Protocol.into());
        };
        if remote.revision < local.revision
            || (remote.revision == local.revision && remote != local)
        {
            return Err(SyncError::Protocol.into());
        }
    }
    Ok(())
}

impl SyncCoordinator {
    /// Enumerate only keys for the current live account; values remain native.
    pub async fn uncertain_v1_keys(&self) -> Result<Vec<String>, SyncCoordError> {
        let config = self.read_config();
        if !config.enabled {
            return Err(SyncCoordError::Disabled);
        }
        let session = self
            .auth
            .session()
            .await
            .ok_or(SyncCoordError::NotSignedIn)?;
        let origin = session.instance_origin();
        let account = session.account_id();
        let gate = self.protocol.try_cutover()?;
        self.auth
            .with_current_session(&session, || {
                if self.read_config() != config {
                    return Err(SyncCoordError::StaleReview);
                }
                gate.ensure_v1(&origin, account)?;
                let mut keys: Vec<String> = gate
                    .uncertain_details(&origin, account)?
                    .iter()
                    .map(|v| v.key().to_owned())
                    .collect();
                keys.sort();
                if keys.windows(2).any(|window| window[0] == window[1]) {
                    return Err(StoreError::InvalidState.into());
                }
                Ok(keys)
            })
            .map_err(|_| SyncCoordError::StaleReview)?
    }
    /// A current session and opt-in are required. The original candidate is
    /// retained regardless of whether the current cloud body happens to match.
    pub async fn review_uncertain_v1(
        &self,
        key: &str,
    ) -> Result<V1UncertainReview, SyncCoordError> {
        let config = self.read_config();
        if !config.enabled {
            return Err(SyncCoordError::Disabled);
        }
        let session = self
            .auth
            .session()
            .await
            .ok_or(SyncCoordError::NotSignedIn)?;
        if !self.auth.is_current_session(&session) {
            return Err(SyncCoordError::StaleReview);
        }
        let origin = session.instance_origin();
        let account = session.account_id();
        // Network I/O must not hold the exclusive protocol lease: local
        // staging needs to remain possible while Cloud is slow or offline.
        let remote = fresh_record(&session, key).await?;
        let gate = self.protocol.try_cutover()?;
        self.auth
            .with_current_session(&session, || {
                if self.read_config() != config {
                    return Err(SyncCoordError::StaleReview);
                }
                gate.ensure_v1(&origin, account)?;
                let (marker_path, original) = gate
                    .uncertain_for_key(&origin, account, key)?
                    .ok_or(SyncCoordError::NoUncertainWrite)?;
                let (store, _) = gate.legacy_store(&origin, account)?;
                let local = store.load()?.ok_or(StoreError::InvalidState)?;
                if local.state.origin != origin || local.state.account_id != account {
                    return Err(StoreError::WrongIdentity.into());
                }
                reject_rewind(
                    local
                        .state
                        .entries
                        .get(key)
                        .and_then(|entry| entry.remote.as_ref()),
                    remote.as_ref(),
                )?;
                Ok(V1UncertainReview {
                    binding: self.recovery_binding.clone(),
                    session: session.clone(),
                    config_generation: config.generation,
                    marker_path,
                    original,
                    local,
                    remote,
                })
            })
            .map_err(|_| SyncCoordError::StaleReview)?
    }

    /// Recheck original marker, all local state and a new authenticated cloud
    /// read before recording a deliberate choice. This never sends a write or
    /// applies a remote selection to a local plugin.
    pub async fn resolve_uncertain_v1(
        &self,
        review: &V1UncertainReview,
        choice: V1RecoveryChoice,
    ) -> Result<V1RecoveryOutcome, SyncCoordError> {
        // Clearing the uncertain marker before the product has durably applied
        // the Cloud value can lose the user's choice, especially when Cloud
        // has no record. Keep the marker and state until the application can
        // commit a source-fenced, crash-recoverable apply intent.
        if choice != V1RecoveryChoice::KeepLocal {
            return Err(SyncCoordError::Local(
                "Cloud recovery is not available yet; the original and local changes were kept"
                    .into(),
            ));
        }
        if !Arc::ptr_eq(&review.binding, &self.recovery_binding) {
            return Err(SyncCoordError::StaleReview);
        }
        let config = self.read_config();
        if !config.enabled
            || config.generation != review.config_generation
            || !self.auth.is_current_session(&review.session)
        {
            return Err(SyncCoordError::StaleReview);
        }
        let origin = review.session.instance_origin();
        let account = review.session.account_id();
        let key = review.original.key();
        // Do not block local staging for the duration of a Cloud request.
        // All durable inputs are rechecked under the lease after it returns.
        let remote = fresh_record(&review.session, key).await?;
        let gate = self.protocol.try_cutover()?;
        if remote != review.remote
            || self.read_config() != config
            || !self.auth.is_current_session(&review.session)
        {
            return Err(SyncCoordError::StaleReview);
        }
        gate.ensure_v1(&origin, account)?;
        gate.ensure_legacy_allowed(&origin, account)?;
        let (path, marker) = gate
            .uncertain_for_key(&origin, account, key)?
            .ok_or(SyncCoordError::NoUncertainWrite)?;
        if path != review.marker_path || marker != review.original {
            return Err(SyncCoordError::StaleReview);
        }
        let (store, _) = gate.legacy_store(&origin, account)?;
        let mut local = store.load()?.ok_or(StoreError::InvalidState)?;
        if local.generation != review.local.generation || local.state != review.local.state {
            return Err(SyncCoordError::StaleReview);
        }
        reject_rewind(
            local
                .state
                .entries
                .get(key)
                .and_then(|entry| entry.remote.as_ref()),
            remote.as_ref(),
        )?;
        let entry = local.state.entries.entry(key.to_owned()).or_default();
        let newer = local.generation != review.original.source_generation()
            || entry.pending.as_ref() != Some(review.original.pending());
        if choice == V1RecoveryChoice::TakeCloudIfUnchanged && newer && entry.pending.is_some() {
            // We cannot distinguish an unrelated-key update from an ABA
            // restage of this same value. Leave marker and state untouched.
            return Ok(V1RecoveryOutcome::NewerEditNeedsReview);
        }
        let candidate = entry
            .pending
            .as_ref()
            .map(|p| p.edit.clone())
            .unwrap_or_else(|| review.original.pending().edit.clone());
        let remote_revision = remote.as_ref().map(|record| record.revision);
        entry.remote = remote;
        if choice == V1RecoveryChoice::KeepLocal {
            // An explicitly reviewed absent delete is already satisfied.
            entry.pending = if matches!(candidate, Edit::Delete) && remote_revision.is_none() {
                None
            } else {
                Some(Pending {
                    base_revision: remote_revision,
                    edit: candidate,
                    conflict: false,
                })
            };
        } else {
            entry.pending = None;
        }
        self.auth
            .with_current_session(&review.session, || {
                if self.read_config() != config {
                    return Err(SyncCoordError::StaleReview);
                }
                store.save(Some(local.generation), &local.state)?;
                gate.remove_reviewed_uncertain(
                    &review.marker_path,
                    &origin,
                    account,
                    &review.original,
                )?;
                Ok(V1RecoveryOutcome::Resolved)
            })
            .map_err(|_| SyncCoordError::StaleReview)?
    }
}
