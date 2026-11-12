//! Offline selection staging, explicit conflict resolution and record validation.
use super::*;

impl SyncState {
    pub fn new(origin: String, account_id: String) -> Self {
        Self {
            origin,
            account_id,
            cursor: 0,
            entries: BTreeMap::new(),
        }
    }

    /// Stage an edit locally. Repeated local changes retain the original base revision.
    pub fn stage(&mut self, key: String, edit: Edit) -> Result<(), SyncError> {
        if !valid_key(&key)
            || matches!(&edit, Edit::Put(value) if value.schema_version != 1 || value.plugin_id != key || value.version.len() > 64 || semver::Version::parse(&value.version).is_err())
        {
            return Err(SyncError::Configuration);
        }
        let entry = self.entries.entry(key).or_default();
        if let Some(pending) = &mut entry.pending {
            pending.edit = edit;
        } else {
            entry.pending = Some(Pending {
                base_revision: entry.remote.as_ref().map(|r| r.revision),
                edit,
                conflict: false,
            });
        }
        Ok(())
    }

    /// Explicit user choice to keep local data after reviewing a conflict.
    pub fn rebase(&mut self, key: &str) -> Result<(), SyncError> {
        let entry = self.entries.get_mut(key).ok_or(SyncError::NoPendingEdit)?;
        let pending = entry.pending.as_mut().ok_or(SyncError::NoPendingEdit)?;
        pending.base_revision = entry.remote.as_ref().map(|r| r.revision);
        pending.conflict = false;
        Ok(())
    }

    /// Explicit user choice to discard an unsent local edit.
    pub fn discard(&mut self, key: &str) -> Result<(), SyncError> {
        let entry = self.entries.get_mut(key).ok_or(SyncError::NoPendingEdit)?;
        if entry.pending.take().is_none() {
            return Err(SyncError::NoPendingEdit);
        }
        Ok(())
    }

    pub(super) fn apply(&mut self, record: Record) -> Result<(), SyncError> {
        validate_record(&record)?;
        let entry = self.entries.entry(record.key.clone()).or_default();
        if entry
            .remote
            .as_ref()
            .is_some_and(|old| old.revision >= record.revision)
        {
            return Ok(());
        }
        if let Some(pending) = &mut entry.pending {
            if pending.base_revision != Some(record.revision) {
                pending.conflict = true;
            }
        }
        entry.remote = Some(record);
        Ok(())
    }
}

pub(super) fn valid_key(key: &str) -> bool {
    key.len() <= 128
        && key.split_once('.').is_some()
        && key.split('.').all(|p| {
            !p.is_empty()
                && p.bytes()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-' || c == b'_')
        })
}

pub(super) fn validate_record(record: &Record) -> Result<(), SyncError> {
    if record.collection != COLLECTION
        || !valid_key(&record.key)
        || record.revision <= 0
        || record.deleted != record.value.is_none()
        || record.value.as_ref().is_some_and(|v| {
            v.schema_version != 1
                || v.plugin_id != record.key
                || v.version.len() > 64
                || semver::Version::parse(&v.version).is_err()
        })
    {
        return Err(SyncError::Protocol);
    }
    Ok(())
}
