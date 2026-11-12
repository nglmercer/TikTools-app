use super::*;

impl WorkspaceState {
    pub fn is_product_guarded(&self) -> bool {
        self.product_guarded
    }

    /// Requires a durably saved product-aware workspace envelope before the
    /// caller may rely on this flag. There is deliberately no inverse setter.
    pub fn mark_product_guarded(&mut self) {
        self.product_guarded = true;
    }

    pub fn has_product_receipts(&self) -> bool {
        !self.product_receipts.is_empty()
    }

    /// Capture a receipt only when it identifies the currently queued edit.
    /// Unconfirmed writes and newer edits never inherit an older receipt.
    pub fn product_queued_proof(&self, target: &WorkspaceKey) -> Option<WorkspaceProductProof> {
        let receipt = self
            .product_receipts
            .binary_search_by(|r| r.target.cmp(target))
            .ok()
            .and_then(|index| self.product_receipts.get(index))?;
        let pending = self.entry(target)?.pending.as_ref()?;
        (pending.status == WorkspacePendingStatus::Queued
            && pending.local_generation == receipt.workspace_generation
            && pending.edit == receipt.edit)
            .then(|| WorkspaceProductProof(receipt.clone()))
    }

    /// Recheck an opaque proof against the current state while source is fenced.
    pub fn verify_product_queued(&self, proof: &WorkspaceProductProof) -> Result<()> {
        if self.product_queued_proof(proof.target()).as_ref() != Some(proof) {
            return Err(WorkspaceStateError::StaleReview);
        }
        Ok(())
    }

    /// Capture the exact latest product decision while its prior upload needs
    /// an explicit conflict or uncertain-outcome review. A newer staged edit
    /// supersedes the original intent, and only its own receipt can authorize
    /// a rebase. A receipt on its own is never permission to resend.
    pub fn product_recovery_proof(&self, target: &WorkspaceKey) -> Option<WorkspaceProductProof> {
        let receipt = self
            .product_receipts
            .binary_search_by(|r| r.target.cmp(target))
            .ok()
            .and_then(|index| self.product_receipts.get(index))?;
        let entry = self.entry(target)?;
        let pending = entry.pending.as_ref()?;
        if !matches!(
            pending.status,
            WorkspacePendingStatus::Unconfirmed | WorkspacePendingStatus::Conflict
        ) {
            return None;
        }
        let (edit, generation) = match &entry.next {
            Some(next) => (&next.edit, next.local_generation),
            None => (&pending.edit, pending.local_generation),
        };
        (generation == receipt.workspace_generation && edit == &receipt.edit)
            .then(|| WorkspaceProductProof(receipt.clone()))
    }

    /// Recheck an opaque recovery proof immediately before changing state.
    pub fn verify_product_recovery(&self, proof: &WorkspaceProductProof) -> Result<()> {
        if self.product_recovery_proof(proof.target()).as_ref() != Some(proof) {
            return Err(WorkspaceStateError::StaleReview);
        }
        Ok(())
    }

    /// Persist this transition before the caller sends the approved request.
    pub fn prepare_product_push(&mut self, proof: &WorkspaceProductProof) -> Result<WorkspacePush> {
        if self.product_guarded {
            return Err(WorkspaceStateError::Invalid);
        }
        self.verify_product_queued(proof)?;
        self.prepare_push(proof.target())
    }

    /// Prepare the exact account-bound epoch with the immutable original
    /// request. Persist the returned state atomically before emitting HTTP.
    /// Never attach an epoch retroactively to a legacy unconfirmed request.
    pub fn prepare_product_push_fenced(
        &mut self,
        proof: &WorkspaceProductProof,
        epoch: &WorkspaceWriteEpoch,
    ) -> Result<WorkspacePush> {
        if !self.product_guarded {
            return Err(WorkspaceStateError::Invalid);
        }
        if !epoch.matches_identity(&self.origin, &self.account_id) {
            return Err(WorkspaceStateError::WrongIdentity);
        }
        self.verify_product_queued(proof)?;
        let mut candidate = self.clone();
        let mut ticket = candidate.prepare_push_inner(proof.target())?;
        let index = candidate
            .index(proof.target())
            .map_err(|_| WorkspaceStateError::StaleReview)?;
        candidate.entries[index]
            .pending
            .as_mut()
            .ok_or(WorkspaceStateError::StaleReview)?
            .write_epoch = Some(epoch.clone());
        ticket.pending.write_epoch = Some(epoch.clone());
        candidate.validate()?;
        *self = candidate;
        Ok(ticket)
    }

    /// Stage is transactional in memory; callers durably save before claiming
    /// an edit was staged. A prepared original request is never overwritten.
    /// Atomically record an already durable product decision and its receipt.
    /// The source must fence its identity and generation until this state is
    /// saved. Repeating the exact decision is a no-op even if its pending
    /// write has since been sent or resolved. A stale or ambiguous decision
    /// requires explicit recovery, never automatic replay.
    pub fn stage_product_decision(
        &mut self,
        key: WorkspaceKey,
        edit: WorkspaceEdit,
        source_generation: u64,
        decision_digest: &str,
    ) -> Result<(u64, bool)> {
        if !self.bootstrap_complete
            || !key.valid()
            || !edit.valid(&key)
            || source_generation == 0
            || decision_digest.len() != 64
            || !decision_digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(WorkspaceStateError::Invalid);
        }
        let position = self
            .product_receipts
            .binary_search_by(|receipt| receipt.target.cmp(&key));
        if let Ok(index) = position {
            let receipt = &self.product_receipts[index];
            if source_generation <= receipt.source_generation {
                if source_generation == receipt.source_generation
                    && decision_digest == receipt.decision_digest
                    && edit == receipt.edit
                {
                    return Ok((receipt.workspace_generation, false));
                }
                return Err(WorkspaceStateError::Invalid);
            }
        }
        let mut candidate = self.clone();
        let generation = candidate.stage(key.clone(), edit.clone())?;
        let receipt = WorkspaceProductReceipt {
            target: key,
            source_generation,
            decision_digest: decision_digest.to_owned(),
            edit,
            workspace_generation: generation,
        };
        match position {
            Ok(index) => candidate.product_receipts[index] = receipt,
            Err(index) => candidate.product_receipts.insert(index, receipt),
        }
        candidate.validate()?;
        *self = candidate;
        Ok((generation, true))
    }
}
