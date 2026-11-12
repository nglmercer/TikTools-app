use super::*;

impl WorkspaceState {
    pub fn validate(&self) -> Result<()> {
        if !identity_valid(&self.origin, &self.account_id)
            || self.cursor < 0
            || self.next_local_generation == 0
        {
            return Err(WorkspaceStateError::Invalid);
        }
        if self.entries.len() > MAX_ENTRIES {
            return Err(WorkspaceStateError::TooLarge);
        }
        if self.product_receipts.len() > MAX_ENTRIES {
            return Err(WorkspaceStateError::TooLarge);
        }
        let mut receipt_previous = None;
        let mut receipt_bytes = 0usize;
        for receipt in &self.product_receipts {
            if !receipt.target.valid()
                || receipt_previous.is_some_and(|key| key >= &receipt.target)
                || receipt.source_generation == 0
                || receipt.workspace_generation == 0
                || receipt.workspace_generation >= self.next_local_generation
                || receipt.decision_digest.len() != 64
                || !receipt
                    .decision_digest
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                || !receipt.edit.valid(&receipt.target)
            {
                return Err(WorkspaceStateError::Invalid);
            }
            receipt_previous = Some(&receipt.target);
            receipt_bytes = receipt_bytes
                .checked_add(receipt.edit.bytes()?)
                .ok_or(WorkspaceStateError::TooLarge)?;
            if receipt_bytes > VALUE_BYTES {
                return Err(WorkspaceStateError::TooLarge);
            }
        }
        let mut previous = None;
        let mut totals = [0usize; 4];
        for entry in &self.entries {
            if !entry.target.valid() || previous.is_some_and(|key| key >= &entry.target) {
                return Err(WorkspaceStateError::Invalid);
            }
            if entry.remote.is_none() && entry.pending.is_none() && entry.next.is_none() {
                return Err(WorkspaceStateError::Invalid);
            }
            previous = Some(&entry.target);
            if let Some(remote) = &entry.remote {
                if !valid_record(&entry.target, remote) {
                    return Err(WorkspaceStateError::Invalid);
                }
                if let Some(value) = &remote.value {
                    totals[0] = totals[0]
                        .checked_add(value_bytes(value)?)
                        .ok_or(WorkspaceStateError::TooLarge)?;
                }
            }
            if let Some(pending) = &entry.pending {
                if !pending.edit.valid(&entry.target)
                    || pending.local_generation == 0
                    || pending.local_generation >= self.next_local_generation
                    || pending.base_revision.is_some_and(|n| n < 1)
                    || pending.base_revision
                        != pending.base_record.as_ref().map(|record| record.revision)
                    || pending
                        .base_record
                        .as_ref()
                        .is_some_and(|record| !valid_record(&entry.target, record))
                    || (pending.status == WorkspacePendingStatus::Queued
                        && pending.base_revision != remote_revision(entry))
                    || pending.write_epoch.as_ref().is_some_and(|epoch| {
                        !self.product_guarded
                            || pending.status == WorkspacePendingStatus::Queued
                            || !epoch.matches_identity(&self.origin, &self.account_id)
                    })
                {
                    return Err(WorkspaceStateError::Invalid);
                }
                totals[1] = totals[1]
                    .checked_add(pending.edit.bytes()?)
                    .ok_or(WorkspaceStateError::TooLarge)?;
                if let Some(value) = pending
                    .base_record
                    .as_ref()
                    .and_then(|record| record.value.as_ref())
                {
                    totals[3] = totals[3]
                        .checked_add(value_bytes(value)?)
                        .ok_or(WorkspaceStateError::TooLarge)?;
                }
            }
            if let Some(next) = &entry.next {
                if !next.edit.valid(&entry.target)
                    || next.local_generation >= self.next_local_generation
                    || !entry.pending.as_ref().is_some_and(|p| {
                        p.status != WorkspacePendingStatus::Queued
                            && next.local_generation > p.local_generation
                    })
                {
                    return Err(WorkspaceStateError::Invalid);
                }
                totals[2] = totals[2]
                    .checked_add(next.edit.bytes()?)
                    .ok_or(WorkspaceStateError::TooLarge)?;
            }
            if totals.iter().any(|total| *total > VALUE_BYTES) {
                return Err(WorkspaceStateError::TooLarge);
            }
        }
        Ok(())
    }

    pub(super) fn identity(&self, origin: &str, account: &str) -> Result<()> {
        if self.origin != origin || self.account_id != account {
            return Err(WorkspaceStateError::WrongIdentity);
        }
        Ok(())
    }
}

pub(super) fn value_bytes(value: &Value) -> Result<usize> {
    serde_json::to_vec(value)
        .map(|v| v.len())
        .map_err(|_| WorkspaceStateError::Invalid)
}
pub(super) fn identity_valid(origin: &str, account: &str) -> bool {
    crate::parse_origin(origin).is_ok_and(|url| url.origin().ascii_serialization() == origin)
        && !account.is_empty()
        && account.len() <= 256
        && account.trim() == account
        && !account.chars().any(char::is_control)
}
pub(super) fn valid_record(target: &WorkspaceKey, record: &WorkspaceRecord) -> bool {
    record.collection == target.collection
        && record.key == target.key
        && record.valid(record.revision)
}
pub(super) fn remote_revision(entry: &WorkspaceEntry) -> Option<i64> {
    entry.remote.as_ref().map(|r| r.revision)
}
