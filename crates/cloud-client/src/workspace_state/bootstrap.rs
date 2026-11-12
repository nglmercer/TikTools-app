use super::*;

impl WorkspaceState {
    pub fn new(origin: String, account_id: String) -> Result<Self> {
        if !identity_valid(&origin, &account_id) {
            return Err(WorkspaceStateError::Invalid);
        }
        Ok(Self {
            origin,
            account_id,
            cursor: 0,
            needs_rebuild: false,
            bootstrap_complete: false,
            next_local_generation: 1,
            entries: Vec::new(),
            product_receipts: Vec::new(),
            product_guarded: false,
        })
    }

    pub fn mark_bootstrap_complete(&mut self) {
        self.bootstrap_complete = true;
    }

    /// Explicit cutover only. V1 bases/cursor are never v2 preconditions. A
    /// differing current staged edit is retained as the newer local choice and
    /// requires review together with the imported original pending intent.
    pub fn import_v1_pending(&mut self, legacy: &SyncState) -> Result<usize> {
        self.identity(&legacy.origin, &legacy.account_id)?;
        if self.bootstrap_complete {
            return Err(WorkspaceStateError::StaleReview);
        }
        if self.entries.iter().any(|entry| {
            entry.next.is_some()
                || entry
                    .pending
                    .as_ref()
                    .is_some_and(|pending| pending.status != WorkspacePendingStatus::Queued)
        }) {
            return Err(WorkspaceStateError::Conflict);
        }
        let mut candidate = self.clone();
        let mut count = 0;
        for (key, old) in &legacy.entries {
            let Some(old) = &old.pending else {
                continue;
            };
            let target = WorkspaceKey::new("plugin-selections", key.clone())?;
            let edit = match &old.edit {
                crate::Edit::Put(value) => WorkspaceEdit::Put(
                    serde_json::to_value(value).map_err(|_| WorkspaceStateError::Invalid)?,
                ),
                crate::Edit::Delete => WorkspaceEdit::Delete,
            };
            if !edit.valid(&target) {
                return Err(WorkspaceStateError::Invalid);
            }
            let generation = candidate.generation()?;
            let index = match candidate.index(&target) {
                Ok(i) => i,
                Err(i) => {
                    candidate.entries.insert(
                        i,
                        WorkspaceEntry {
                            target,
                            remote: None,
                            pending: None,
                            next: None,
                        },
                    );
                    i
                }
            };
            let previous = candidate.entries[index]
                .pending
                .as_ref()
                .map(|p| p.edit.clone());
            let differs = previous.as_ref().is_some_and(|value| *value != edit);
            let next = if differs {
                Some(WorkspaceLocalEdit {
                    edit: previous.unwrap(),
                    local_generation: candidate.generation()?,
                })
            } else {
                None
            };
            let entry = &mut candidate.entries[index];
            entry.pending = Some(WorkspacePending {
                base_revision: None,
                base_record: None,
                edit,
                local_generation: generation,
                status: if old.conflict || differs || entry.remote.is_some() {
                    WorkspacePendingStatus::Conflict
                } else {
                    WorkspacePendingStatus::Queued
                },
                write_epoch: None,
            });
            entry.next = next;
            count += 1;
        }
        candidate.validate()?;
        *self = candidate;
        Ok(count)
    }
}
