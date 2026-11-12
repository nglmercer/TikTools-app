use super::*;

impl WorkspaceState {
    /// The changed state MUST be saved successfully before sending this ticket.
    /// A restart leaves Unconfirmed and refuses automatic repeat dispatch.
    pub fn prepare_push(&mut self, key: &WorkspaceKey) -> Result<WorkspacePush> {
        if self.product_guarded {
            return Err(WorkspaceStateError::Invalid);
        }
        self.prepare_push_inner(key)
    }

    pub(super) fn prepare_push_inner(&mut self, key: &WorkspaceKey) -> Result<WorkspacePush> {
        if self.needs_rebuild {
            return Err(WorkspaceStateError::NeedsRebuild);
        }
        let index = self
            .index(key)
            .map_err(|_| WorkspaceStateError::NoPendingEdit)?;
        let pending = self.entries[index]
            .pending
            .as_mut()
            .ok_or(WorkspaceStateError::NoPendingEdit)?;
        if pending.status != WorkspacePendingStatus::Queued {
            return Err(WorkspaceStateError::Conflict);
        }
        if pending.base_revision.is_none() && matches!(pending.edit, WorkspaceEdit::Delete) {
            return Err(WorkspaceStateError::NeedsRead);
        }
        pending.status = WorkspacePendingStatus::Unconfirmed;
        Ok(WorkspacePush {
            origin: self.origin.clone(),
            account_id: self.account_id.clone(),
            target: key.clone(),
            pending: pending.clone(),
        })
    }

    fn ticket_index(&self, ticket: &WorkspacePush) -> Result<usize> {
        self.identity(&ticket.origin, &ticket.account_id)?;
        let index = self
            .index(&ticket.target)
            .map_err(|_| WorkspaceStateError::StaleReview)?;
        if !self.entries[index].pending.as_ref().is_some_and(|p| {
            p.base_revision == ticket.pending.base_revision
                && p.base_record == ticket.pending.base_record
                && p.edit == ticket.pending.edit
                && p.local_generation == ticket.pending.local_generation
                && p.write_epoch == ticket.pending.write_epoch
                && p.status != WorkspacePendingStatus::Queued
        }) {
            return Err(WorkspaceStateError::StaleReview);
        }
        Ok(index)
    }

    pub fn acknowledge(&mut self, ticket: &WorkspacePush, record: WorkspaceRecord) -> Result<()> {
        let index = self.ticket_index(ticket)?;
        if !valid_record(&ticket.target, &record)
            || !ticket.pending.edit.matches(&record)
            || ticket
                .pending
                .base_revision
                .is_some_and(|base| record.revision < base)
        {
            return Err(WorkspaceStateError::Invalid);
        }
        let mut candidate = self.clone();
        let entry = &mut candidate.entries[index];
        let revision = record.revision;
        let acknowledged = record.clone();
        if entry
            .remote
            .as_ref()
            .is_none_or(|old| old.revision < revision)
        {
            entry.remote = Some(record);
        } else if entry
            .remote
            .as_ref()
            .is_some_and(|old| old.revision == revision && !old.equivalent(&record))
        {
            return Err(WorkspaceStateError::Invalid);
        }
        entry.pending = entry.next.take().map(|next| WorkspacePending {
            base_revision: Some(revision),
            base_record: Some(acknowledged),
            edit: next.edit,
            local_generation: next.local_generation,
            status: if remote_revision(entry) == Some(revision) {
                WorkspacePendingStatus::Queued
            } else {
                WorkspacePendingStatus::Conflict
            },
            write_epoch: None,
        });
        candidate.validate()?;
        *self = candidate;
        Ok(())
    }

    pub fn mark_conflict(&mut self, ticket: &WorkspacePush) -> Result<()> {
        let index = self.ticket_index(ticket)?;
        self.entries[index].pending.as_mut().unwrap().status = WorkspacePendingStatus::Conflict;
        Ok(())
    }
}
