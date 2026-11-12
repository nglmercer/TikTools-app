use super::*;

impl WorkspaceState {
    pub(super) fn index(&self, key: &WorkspaceKey) -> std::result::Result<usize, usize> {
        self.entries.binary_search_by(|entry| entry.target.cmp(key))
    }

    pub(super) fn generation(&mut self) -> Result<u64> {
        let value = self.next_local_generation;
        self.next_local_generation = value.checked_add(1).ok_or(WorkspaceStateError::TooLarge)?;
        Ok(value)
    }

    pub fn stage(&mut self, key: WorkspaceKey, edit: WorkspaceEdit) -> Result<u64> {
        if !edit.valid(&key) {
            return Err(WorkspaceStateError::Invalid);
        }
        let mut candidate = self.clone();
        let generation = candidate.generation()?;
        let index = match candidate.index(&key) {
            Ok(i) => i,
            Err(i) => {
                candidate.entries.insert(
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
        let entry = &mut candidate.entries[index];
        if let Some(pending) = &mut entry.pending {
            if pending.status == WorkspacePendingStatus::Queued {
                pending.edit = edit;
                pending.local_generation = generation;
            } else {
                entry.next = Some(WorkspaceLocalEdit {
                    edit,
                    local_generation: generation,
                });
            }
        } else {
            entry.pending = Some(WorkspacePending {
                base_revision: remote_revision(entry),
                base_record: entry.remote.clone(),
                edit,
                local_generation: generation,
                status: WorkspacePendingStatus::Queued,
                write_epoch: None,
            });
        }
        candidate.validate()?;
        *self = candidate;
        Ok(generation)
    }
}
