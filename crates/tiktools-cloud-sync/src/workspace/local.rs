//! Offline staging and replay receipts for native product decisions.
use super::MAX_LOCAL_RETRIES;
use super::{WorkspaceCoordError, WorkspaceCoordinator};
use cloud_client::{StoreError, WorkspaceEdit, WorkspaceKey, WorkspaceRecord, WorkspaceStateError};

impl WorkspaceCoordinator {
    /// An offline local decision is queued only after its private atomic save.
    pub fn stage(
        &self,
        key: WorkspaceKey,
        edit: WorkspaceEdit,
    ) -> Result<u64, WorkspaceCoordError> {
        let _lease = self.generic_lease()?;
        self.update(|state| {
            if !state.bootstrap_complete {
                return Err(WorkspaceCoordError::BootstrapRequired);
            }
            Ok(state.stage(key.clone(), edit.clone())?)
        })
    }

    /// Save a fenced durable source decision and its replay receipt in the
    /// same workspace transaction. The caller owns identity, consent and
    /// source fencing and must not interpret this as permission to dispatch.
    #[cfg(test)]
    pub(crate) fn stage_product_decision(
        &self,
        key: WorkspaceKey,
        edit: WorkspaceEdit,
        source_generation: u64,
        decision_digest: &str,
    ) -> Result<(u64, bool), WorkspaceCoordError> {
        self.stage_product_decision_with_base(key, edit, source_generation, decision_digest, None)
    }

    pub(crate) fn stage_product_decision_with_base(
        &self,
        key: WorkspaceKey,
        edit: WorkspaceEdit,
        source_generation: u64,
        decision_digest: &str,
        reviewed_base: Option<Option<&WorkspaceRecord>>,
    ) -> Result<(u64, bool), WorkspaceCoordError> {
        let _operation = self
            .store
            .try_operation_lock()?
            .ok_or(WorkspaceCoordError::Busy)?;
        for _ in 0..MAX_LOCAL_RETRIES {
            let mut snapshot = self.snapshot()?;
            if !snapshot.state.bootstrap_complete {
                return Err(WorkspaceCoordError::BootstrapRequired);
            }
            if !snapshot.state.is_product_guarded() {
                return Err(crate::ProtocolFenceError::ProductOutboxNotEnabled.into());
            }
            if let Some(base) = reviewed_base {
                if snapshot.state.needs_rebuild {
                    return Err(WorkspaceStateError::NeedsRebuild.into());
                }
                let entry = snapshot.state.entry(&key);
                if entry.and_then(|e| e.remote.as_ref()) != base {
                    return Err(WorkspaceStateError::StaleReview.into());
                }
                if let Some(entry) = entry {
                    if entry.next.is_some() {
                        return Err(WorkspaceStateError::StaleReview.into());
                    }
                    if let Some(pending) = &entry.pending {
                        let same_receipt =
                            snapshot.state.product_queued_proof(&key).is_some_and(|p| {
                                p.source_generation() == source_generation
                                    && p.decision_digest() == decision_digest
                                    && p.edit() == &edit
                            });
                        if !same_receipt || pending.base_record.as_ref() != base {
                            return Err(WorkspaceStateError::StaleReview.into());
                        }
                    }
                }
            }
            let result = snapshot.state.stage_product_decision(
                key.clone(),
                edit.clone(),
                source_generation,
                decision_digest,
            )?;
            if !result.1 {
                return Ok(result);
            }
            let expected = (snapshot.generation != 0).then_some(snapshot.generation);
            match self.store.save(expected, &snapshot.state) {
                Ok(_) => return Ok(result),
                Err(StoreError::ConcurrentChange) => continue,
                Err(error) => return Err(error.into()),
            }
        }
        Err(StoreError::ConcurrentChange.into())
    }
}
