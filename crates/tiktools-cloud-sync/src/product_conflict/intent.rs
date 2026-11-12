//! Frozen keep-local intent validation, digests and recognized recovery phases.
use super::{AbandonPhase, RebasePhase, V1KeepLocalIntent};
use crate::{SyncCoordError, V1ConflictSourceProof, V1LocalEditContext};
use cloud_client::{Edit, Entry, Pending, Record, StateSnapshot, SyncState};
use sha2::{Digest, Sha256};

impl V1KeepLocalIntent {
    pub(super) fn validate(&self, context: &V1LocalEditContext) -> Result<(), SyncCoordError> {
        let pending = self
            .before
            .pending
            .as_ref()
            .filter(|pending| pending.conflict)
            .ok_or(SyncCoordError::StaleReview)?;
        if self.format_version != 1
            || self.origin != context.origin
            || self.account_id != context.account_id
            || self.config_generation != context.config_generation
            || self.source_generation <= 0
            || self.key.is_empty()
            || self.before_generation == 0
            || self.before_generation.checked_add(1).is_none()
            || V1ConflictSourceProof::new(
                self.source_generation,
                self.initial_runtime_epoch.clone(),
            )
            .is_none()
            || !valid_digest(&self.before_digest)
            || !valid_digest(&self.after_digest)
        {
            return Err(SyncCoordError::StaleReview);
        }
        if self
            .cloud
            .as_ref()
            .is_some_and(|record| record.key != self.key)
        {
            return Err(SyncCoordError::StaleReview);
        }
        if let Some(cached) = self.before.remote.as_ref() {
            let Some(cloud) = self.cloud.as_ref() else {
                return Err(SyncCoordError::StaleReview);
            };
            if cloud.revision < cached.revision
                || (cloud.revision == cached.revision && cloud != cached)
            {
                return Err(SyncCoordError::StaleReview);
            }
        }
        if pending.base_revision.is_some_and(|revision| {
            self.cloud
                .as_ref()
                .is_none_or(|record| record.revision < revision)
        }) {
            return Err(SyncCoordError::StaleReview);
        }
        if desired_entry(&self.before, self.cloud.clone(), pending) != self.after {
            return Err(SyncCoordError::StaleReview);
        }
        Ok(())
    }
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(super) fn state_digest(state: &SyncState) -> Result<String, SyncCoordError> {
    let bytes = serde_json::to_vec(state).map_err(|_| SyncCoordError::StaleReview)?;
    Ok(Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

pub(super) fn desired_entry(before: &Entry, cloud: Option<Record>, pending: &Pending) -> Entry {
    let next = if matches!(pending.edit, Edit::Delete) && cloud.is_none() {
        None
    } else {
        Some(Pending {
            base_revision: cloud.as_ref().map(|record| record.revision),
            edit: pending.edit.clone(),
            conflict: false,
        })
    };
    let mut result = before.clone();
    result.remote = cloud;
    result.pending = next;
    result
}

pub(super) fn rebase_phase(
    intent: &V1KeepLocalIntent,
    snapshot: &StateSnapshot,
) -> Result<RebasePhase, SyncCoordError> {
    let entry = snapshot
        .state
        .entries
        .get(&intent.key)
        .ok_or(SyncCoordError::StaleReview)?;
    let digest = state_digest(&snapshot.state)?;
    if snapshot.generation == intent.before_generation
        && entry == &intent.before
        && digest == intent.before_digest
    {
        Ok(RebasePhase::Before)
    } else if snapshot.generation
        == intent
            .before_generation
            .checked_add(1)
            .ok_or(SyncCoordError::StaleReview)?
        && entry == &intent.after
        && digest == intent.after_digest
    {
        Ok(RebasePhase::After)
    } else {
        Err(SyncCoordError::StaleReview)
    }
}

pub(super) fn abandon_phase(
    intent: &V1KeepLocalIntent,
    snapshot: &StateSnapshot,
) -> Result<AbandonPhase, SyncCoordError> {
    match rebase_phase(intent, snapshot) {
        Ok(RebasePhase::Before) => return Ok(AbandonPhase::Before),
        Ok(RebasePhase::After) => return Ok(AbandonPhase::After),
        Err(SyncCoordError::StaleReview) => {}
        Err(error) => return Err(error),
    }
    // A crash can occur after restoring the frozen state but before the
    // journal is deleted. Generation +2 is the sole additional valid phase.
    if intent.before_generation.checked_add(2) == Some(snapshot.generation)
        && snapshot.state.entries.get(&intent.key) == Some(&intent.before)
        && state_digest(&snapshot.state)? == intent.before_digest
    {
        return Ok(AbandonPhase::Restored);
    }
    Err(SyncCoordError::StaleReview)
}
