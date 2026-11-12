//! Durable original upload markers and explicit reviewed removal.
use super::ProtocolFenceError;
use super::{
    checked_file, identity_hash, marker_bytes, sync_dir, CutoverLease, ProtocolFence,
    UncertainUpload, V1Lease, V1UncertainUpload, MAX_UNCERTAIN_MARKER_BYTES,
};
use cloud_client::{StateSnapshot, StoreError, SyncState};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};

impl ProtocolFence {
    pub(super) fn uncertain_count(
        &self,
        origin: &str,
        account: &str,
    ) -> Result<usize, ProtocolFenceError> {
        let prefix = format!("v1-uncertain-{}-", identity_hash(origin, account)?);
        let mut count = 0;
        for entry in fs::read_dir(&self.dir)? {
            let entry = entry?;
            if entry.file_name().to_string_lossy().starts_with(&prefix) {
                checked_file(&entry.path())?;
                count += 1;
            }
        }
        Ok(count)
    }

    /// Publish a complete marker only after its bytes have reached the file.
    /// A crash before publication leaves an ignored temporary file; a crash
    /// afterwards leaves a complete marker that prevents an unsafe replay.
    pub(super) fn publish_uncertain(
        &self,
        mut temporary: tempfile::NamedTempFile,
        destination: PathBuf,
        original: &V1UncertainUpload,
    ) -> Result<UncertainUpload, ProtocolFenceError> {
        let bytes = marker_bytes(original, MAX_UNCERTAIN_MARKER_BYTES)?;
        temporary.write_all(&bytes)?;
        temporary.flush()?;
        temporary.as_file().sync_all()?;
        // Never replace any earlier uncertain marker, even in the unlikely
        // event of a random-name collision.
        temporary
            .persist_noclobber(&destination)
            .map_err(|error| error.error)?;
        sync_dir(&self.dir)?;
        Ok(UncertainUpload { path: destination })
    }

    pub(super) fn uncertain_records(
        &self,
        origin: &str,
        account: &str,
    ) -> Result<Vec<(PathBuf, V1UncertainUpload)>, ProtocolFenceError> {
        let prefix = format!("v1-uncertain-{}-", identity_hash(origin, account)?);
        let mut paths = Vec::new();
        for entry in fs::read_dir(&self.dir)? {
            let entry = entry?;
            if entry.file_name().to_string_lossy().starts_with(&prefix) {
                if !checked_file(&entry.path())? {
                    return Err(StoreError::InvalidState.into());
                }
                paths.push(entry.path());
                if paths.len() > 1000 {
                    return Err(StoreError::InvalidState.into());
                }
            }
        }
        paths.sort();
        let mut records = Vec::with_capacity(paths.len());
        for path in paths {
            let mut data = Vec::new();
            File::open(&path)?
                .take((MAX_UNCERTAIN_MARKER_BYTES + 1) as u64)
                .read_to_end(&mut data)?;
            if data.len() > MAX_UNCERTAIN_MARKER_BYTES {
                return Err(StoreError::InvalidState.into());
            }
            let value: V1UncertainUpload =
                serde_json::from_slice(&data).map_err(|_| StoreError::InvalidState)?;
            let mut probe = SyncState::new(origin.to_owned(), account.to_owned());
            if value.format_version != 1
                || value.origin != origin
                || value.account_id != account
                || value.pending.conflict
                || value.pending.base_revision.is_some_and(|v| v < 1)
                || value.pending.base_revision != value.base_record.as_ref().map(|v| v.revision)
                || value.base_record.as_ref().is_some_and(|v| {
                    v.collection != "plugin-selections"
                        || v.key != value.key
                        || v.revision < 1
                        || v.deleted != v.value.is_none()
                        || v.value.as_ref().is_some_and(|selection| {
                            SyncState::new(origin.into(), account.into())
                                .stage(
                                    value.key.clone(),
                                    cloud_client::Edit::Put(selection.clone()),
                                )
                                .is_err()
                        })
                })
                || probe
                    .stage(value.key.clone(), value.pending.edit.clone())
                    .is_err()
            {
                return Err(StoreError::InvalidState.into());
            }
            records.push((path, value));
        }
        Ok(records)
    }
}

impl V1Lease {
    /// Persist before any v1 write. Cancellation, crash or an unknown reply
    /// leaves this marker intact and blocks migration for this identity.
    pub(crate) fn prepare_upload(
        &self,
        snapshot: &StateSnapshot,
        key: &str,
    ) -> Result<UncertainUpload, ProtocolFenceError> {
        let state = &snapshot.state;
        let origin = &state.origin;
        let account = &state.account_id;
        self.ensure_v1(origin, account)?;
        let entry = state.entries.get(key).ok_or(StoreError::InvalidState)?;
        let pending = entry
            .pending
            .clone()
            .filter(|pending| !pending.conflict)
            .ok_or(StoreError::InvalidState)?;
        let original = V1UncertainUpload {
            format_version: 1,
            origin: origin.clone(),
            account_id: account.clone(),
            source_generation: snapshot.generation,
            key: key.into(),
            pending,
            base_record: entry.remote.clone(),
        };
        let prefix = format!("v1-uncertain-{}-", identity_hash(origin, account)?);
        const TEMP_PREFIX: &str = ".v1-uncertain-tmp-";
        let temp = tempfile::Builder::new()
            .prefix(TEMP_PREFIX)
            .tempfile_in(&self.fence.dir)?;
        let suffix = temp
            .path()
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| name.strip_prefix(TEMP_PREFIX))
            .filter(|suffix| !suffix.is_empty())
            .ok_or(StoreError::InvalidState)?;
        let destination = self.fence.dir.join(format!("{prefix}{suffix}"));
        self.fence.publish_uncertain(temp, destination, &original)
    }

    pub(crate) fn acknowledge_upload(
        &self,
        marker: UncertainUpload,
    ) -> Result<(), ProtocolFenceError> {
        checked_file(&marker.path)?;
        fs::remove_file(marker.path)?;
        sync_dir(&self.fence.dir)
    }
}

impl CutoverLease {
    /// Expose ambiguity as a count, never as an automatic replay decision.
    pub fn uncertain_uploads(
        &self,
        origin: &str,
        account: &str,
    ) -> Result<usize, ProtocolFenceError> {
        self.fence.uncertain_count(origin, account)
    }

    /// Inspection only; resolving an unknown original outcome requires a
    /// future explicit recovery flow and cannot be inferred from equal data.
    pub fn uncertain_details(
        &self,
        origin: &str,
        account: &str,
    ) -> Result<Vec<V1UncertainUpload>, ProtocolFenceError> {
        Ok(self
            .fence
            .uncertain_records(origin, account)?
            .into_iter()
            .map(|(_, value)| value)
            .collect())
    }

    pub(crate) fn uncertain_for_key(
        &self,
        origin: &str,
        account: &str,
        key: &str,
    ) -> Result<Option<(PathBuf, V1UncertainUpload)>, ProtocolFenceError> {
        let mut matches = self
            .fence
            .uncertain_records(origin, account)?
            .into_iter()
            .filter(|(_, item)| item.key == key);
        let first = matches.next();
        if matches.next().is_some() {
            return Err(StoreError::InvalidState.into());
        }
        Ok(first)
    }

    pub(crate) fn remove_reviewed_uncertain(
        &self,
        path: &Path,
        origin: &str,
        account: &str,
        expected: &V1UncertainUpload,
    ) -> Result<(), ProtocolFenceError> {
        let found = self.uncertain_for_key(origin, account, expected.key())?;
        if found
            .as_ref()
            .is_none_or(|(actual_path, actual)| actual_path != path || actual != expected)
        {
            return Err(ProtocolFenceError::StaleCutover);
        }
        fs::remove_file(path)?;
        sync_dir(&self.fence.dir)
    }
}
