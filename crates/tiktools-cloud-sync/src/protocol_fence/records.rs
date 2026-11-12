//! Identity-bound protocol and product-outbox marker storage.
use super::ProtocolFenceError;
use super::{
    checked_file, marker_bytes, sync_dir, CutoverPreview, ProductOutboxRecord, ProtocolFence,
    ProtocolRecord, MAX_PROTOCOL_MARKER_BYTES,
};
use cloud_client::{StoreError, WorkspaceState};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{Read, Write},
    path::PathBuf,
};

pub(super) fn identity_hash(origin: &str, account: &str) -> Result<String, ProtocolFenceError> {
    WorkspaceState::new(origin.to_owned(), account.to_owned())
        .map_err(|_| StoreError::InvalidState)?;
    let bytes = serde_json::to_vec(&(origin, account)).map_err(|_| StoreError::InvalidState)?;
    Ok(Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

pub(super) fn candidate_hash(preview: &CutoverPreview) -> Result<String, ProtocolFenceError> {
    let bytes = preview
        .review_material()
        .map_err(|_| StoreError::InvalidState)?;
    Ok(Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

impl ProtocolFence {
    pub(super) fn record_path(
        &self,
        origin: &str,
        account: &str,
    ) -> Result<PathBuf, ProtocolFenceError> {
        Ok(self
            .dir
            .join(format!("protocol-{}.json", identity_hash(origin, account)?)))
    }

    pub(super) fn product_outbox_path(
        &self,
        origin: &str,
        account: &str,
    ) -> Result<PathBuf, ProtocolFenceError> {
        Ok(self.dir.join(format!(
            "v1-product-outbox-{}.json",
            identity_hash(origin, account)?
        )))
    }

    pub(super) fn product_outbox_required(
        &self,
        origin: &str,
        account: &str,
    ) -> Result<bool, ProtocolFenceError> {
        let path = self.product_outbox_path(origin, account)?;
        if !checked_file(&path)? {
            return Ok(false);
        }
        let mut data = Vec::new();
        File::open(path)?
            .take((MAX_PROTOCOL_MARKER_BYTES + 1) as u64)
            .read_to_end(&mut data)?;
        if data.len() > MAX_PROTOCOL_MARKER_BYTES {
            return Err(StoreError::InvalidState.into());
        }
        let record: ProductOutboxRecord =
            serde_json::from_slice(&data).map_err(|_| StoreError::InvalidState)?;
        if record.format_version != 1 || record.origin != origin || record.account_id != account {
            return Err(StoreError::InvalidState.into());
        }
        Ok(true)
    }

    pub(super) fn ensure_legacy_allowed(
        &self,
        origin: &str,
        account: &str,
    ) -> Result<(), ProtocolFenceError> {
        if self.product_outbox_required(origin, account)? {
            return Err(ProtocolFenceError::ProductOutboxRequired);
        }
        Ok(())
    }

    pub(super) fn read_record(
        &self,
        origin: &str,
        account: &str,
    ) -> Result<Option<ProtocolRecord>, ProtocolFenceError> {
        let path = self.record_path(origin, account)?;
        if !checked_file(&path)? {
            return Ok(None);
        }
        let mut data = Vec::new();
        File::open(path)?
            .take((MAX_PROTOCOL_MARKER_BYTES + 1) as u64)
            .read_to_end(&mut data)?;
        if data.len() > MAX_PROTOCOL_MARKER_BYTES {
            return Err(StoreError::InvalidState.into());
        }
        let record: ProtocolRecord =
            serde_json::from_slice(&data).map_err(|_| StoreError::InvalidState)?;
        if !matches!(record.format_version, 1..=3)
            || record.origin != origin
            || record.account_id != account
            || record.candidate_sha256.len() != 64
            || !record
                .candidate_sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(StoreError::InvalidState.into());
        }
        Ok(Some(record))
    }

    pub(super) fn write_record(&self, record: &ProtocolRecord) -> Result<(), ProtocolFenceError> {
        let target = self.record_path(&record.origin, &record.account_id)?;
        checked_file(&target)?;
        let bytes = marker_bytes(record, MAX_PROTOCOL_MARKER_BYTES)?;
        let mut temp = tempfile::NamedTempFile::new_in(&self.dir)?;
        temp.write_all(&bytes)?;
        temp.flush()?;
        temp.as_file().sync_all()?;
        temp.persist(target).map_err(|error| error.error)?;
        sync_dir(&self.dir)
    }

    /// Package source changes affect every account, including an account that
    /// is currently signed out. Conservatively retain a marker if any
    /// account has entered v2 preparation or completed cutover.
    pub(crate) fn has_any_protocol_record(&self) -> Result<bool, ProtocolFenceError> {
        for item in std::fs::read_dir(&self.dir)? {
            let name = item?.file_name();
            if name
                .to_str()
                .is_some_and(|name| name.starts_with("protocol-") && name.ends_with(".json"))
            {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
