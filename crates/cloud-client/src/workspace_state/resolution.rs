use super::*;

impl WorkspaceState {
    pub fn observe_read(&mut self, read: &WorkspaceRead) -> Result<()> {
        self.identity(&read.origin, &read.account_id)?;
        let mut candidate = self.clone();
        match &read.record {
            Some(record) => candidate.apply_record(record.clone())?,
            None => {
                if let Some(entry) = candidate.entry(&read.target) {
                    if entry.remote.is_some() {
                        return Err(WorkspaceStateError::StaleReview);
                    }
                }
            }
        }
        candidate.validate()?;
        *self = candidate;
        Ok(())
    }

    /// Complete only the exact queued delete captured before the target GET.
    /// A newer equal-looking deletion is a different local intent and cannot
    /// be cleared with the earlier read's absence proof.
    pub fn complete_absent_delete(
        &mut self,
        read: &WorkspaceRead,
        expected_local_generation: u64,
    ) -> Result<()> {
        self.identity(&read.origin, &read.account_id)?;
        let index = self
            .index(&read.target)
            .map_err(|_| WorkspaceStateError::StaleReview)?;
        let entry = &self.entries[index];
        if read.record.is_some()
            || entry.remote.is_some()
            || entry.next.is_some()
            || !entry.pending.as_ref().is_some_and(|pending| {
                pending.status == WorkspacePendingStatus::Queued
                    && pending.base_revision.is_none()
                    && pending.base_record.is_none()
                    && matches!(pending.edit, WorkspaceEdit::Delete)
                    && pending.local_generation == expected_local_generation
            })
        {
            return Err(WorkspaceStateError::StaleReview);
        }
        let mut candidate = self.clone();
        candidate.entries.remove(index);
        candidate.validate()?;
        *self = candidate;
        Ok(())
    }

    /// A new live read must still match the revision displayed for explicit
    /// choice, and the local generation must still be the reviewed candidate.
    pub fn resolve(
        &mut self,
        read: &WorkspaceRead,
        expected_remote_revision: Option<i64>,
        expected_local_generation: u64,
        keep_local: bool,
    ) -> Result<()> {
        self.identity(&read.origin, &read.account_id)?;
        if read.revision() != expected_remote_revision
            || self.local_generation(&read.target) != Some(expected_local_generation)
        {
            return Err(WorkspaceStateError::StaleReview);
        }
        if self.entry(&read.target).and_then(remote_revision) > read.revision() {
            return Err(WorkspaceStateError::StaleReview);
        }
        let mut candidate = self.clone();
        let index = candidate
            .index(&read.target)
            .map_err(|_| WorkspaceStateError::NoPendingEdit)?;
        let entry = &mut candidate.entries[index];
        if let Some(record) = &read.record {
            if !valid_record(&read.target, record) {
                return Err(WorkspaceStateError::Invalid);
            }
        }
        if entry
            .remote
            .as_ref()
            .zip(read.record.as_ref())
            .is_some_and(|(old, new)| old.revision == new.revision && !old.equivalent(new))
        {
            return Err(WorkspaceStateError::Invalid);
        }
        entry.remote = read.record.clone();
        let desired = entry
            .next
            .take()
            .map(|next| next.edit)
            .or_else(|| entry.pending.as_ref().map(|p| p.edit.clone()))
            .ok_or(WorkspaceStateError::NoPendingEdit)?;
        if keep_local && !(read.record.is_none() && matches!(desired, WorkspaceEdit::Delete)) {
            entry.pending = Some(WorkspacePending {
                base_revision: read.revision(),
                base_record: read.record.clone(),
                edit: desired,
                local_generation: expected_local_generation,
                status: WorkspacePendingStatus::Queued,
                write_epoch: None,
            });
        } else {
            entry.pending = None;
        }
        if entry.remote.is_none() && entry.pending.is_none() && entry.next.is_none() {
            candidate.entries.remove(index);
        }
        candidate.validate()?;
        *self = candidate;
        Ok(())
    }
}

impl Session {
    pub async fn workspace_read_for_resolution(
        &self,
        key: &WorkspaceKey,
    ) -> std::result::Result<WorkspaceRead, SyncError> {
        if !key.valid() {
            return Err(SyncError::Configuration);
        }
        let record = match self.workspace_record(&key.collection, &key.key).await {
            Ok(record) => Some(record),
            Err(SyncError::Http(404)) => None,
            Err(error) => return Err(error),
        };
        Ok(WorkspaceRead {
            origin: self.instance_origin(),
            account_id: self.account_id().into(),
            target: key.clone(),
            record,
        })
    }
}
