use super::*;

impl WorkspaceState {
    pub(super) fn apply_record(&mut self, record: WorkspaceRecord) -> Result<()> {
        let key = WorkspaceKey::new(record.collection.clone(), record.key.clone())?;
        if !valid_record(&key, &record) {
            return Err(WorkspaceStateError::Invalid);
        }
        let index = match self.index(&key) {
            Ok(i) => i,
            Err(i) => {
                self.entries.insert(
                    i,
                    WorkspaceEntry {
                        target: key,
                        remote: None,
                        pending: None,
                        next: None,
                    },
                );
                i
            }
        };
        let entry = &mut self.entries[index];
        if let Some(old) = &entry.remote {
            if old.revision > record.revision {
                return Ok(());
            }
            if old.revision == record.revision {
                return if old.equivalent(&record) {
                    Ok(())
                } else {
                    Err(WorkspaceStateError::Invalid)
                };
            }
        }
        if let Some(pending) = &mut entry.pending {
            if pending.status == WorkspacePendingStatus::Queued
                && pending.base_revision != Some(record.revision)
            {
                pending.status = WorkspacePendingStatus::Conflict;
            }
        }
        entry.remote = Some(record);
        Ok(())
    }

    pub fn apply_changes(&mut self, page: WorkspaceChanges) -> Result<bool> {
        if self.needs_rebuild {
            return Err(WorkspaceStateError::NeedsRebuild);
        }
        if !page.valid_after(self.cursor) {
            return Err(WorkspaceStateError::Invalid);
        }
        let mut candidate = self.clone();
        for record in page.changes {
            candidate.apply_record(record)?;
        }
        candidate.cursor = page.cursor;
        candidate.validate()?;
        *self = candidate;
        Ok(page.has_more)
    }

    pub fn require_rebuild(&mut self) {
        self.needs_rebuild = true;
    }

    /// Only an explicit read-only snapshot rebuild choice calls this method.
    /// It replaces the remote cache, preserving every local intent; application
    /// of these records remains a separate reviewed application-domain action.
    pub fn install_rebuild(&mut self, snapshot: WorkspaceSnapshot) -> Result<()> {
        if !snapshot.valid() {
            return Err(WorkspaceStateError::Invalid);
        }
        if snapshot.cursor < self.cursor {
            return Err(WorkspaceStateError::StaleReview);
        }
        for old in self
            .entries
            .iter()
            .filter_map(|entry| entry.remote.as_ref())
        {
            let replacement = snapshot
                .records
                .iter()
                .find(|record| record.collection == old.collection && record.key == old.key);
            if old.revision > snapshot.cursor
                || !replacement
                    .is_some_and(|record| record.revision > old.revision || record.equivalent(old))
            {
                return Err(WorkspaceStateError::StaleReview);
            }
        }
        let mut candidate = Self::new(self.origin.clone(), self.account_id.clone())?;
        candidate.cursor = snapshot.cursor;
        candidate.bootstrap_complete = self.bootstrap_complete;
        candidate.next_local_generation = self.next_local_generation;
        candidate.product_receipts = self.product_receipts.clone();
        candidate.product_guarded = self.product_guarded;
        for record in snapshot.records {
            candidate.apply_record(record)?;
        }
        for old in &self.entries {
            if old.pending.is_none() {
                continue;
            }
            let index = match candidate.index(&old.target) {
                Ok(i) => i,
                Err(i) => {
                    candidate.entries.insert(
                        i,
                        WorkspaceEntry {
                            target: old.target.clone(),
                            remote: None,
                            pending: None,
                            next: None,
                        },
                    );
                    i
                }
            };
            let entry = &mut candidate.entries[index];
            let mut pending = old.pending.clone().unwrap();
            if pending.status == WorkspacePendingStatus::Queued
                && pending.base_revision != remote_revision(entry)
            {
                pending.status = WorkspacePendingStatus::Conflict;
            }
            entry.pending = Some(pending);
            entry.next = old.next.clone();
        }
        candidate.validate()?;
        *self = candidate;
        Ok(())
    }
}
