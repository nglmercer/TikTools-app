//! Cooperative cross-process protocol fencing for a private device directory.
//!
//! Older binaries and code that bypasses this coordinator cannot participate.
//! Local application mutations and session eligibility need their own product
//! fence; a protocol lease alone is not user consent to activate cloud data.
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

use cloud_client::{
    Pending, PluginSelection, Record, StateSnapshot, StateStore, StoreError, SyncState,
    WorkspaceState, WorkspaceStore,
};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::CutoverPreview;

const MAX_PROTOCOL_MARKER_BYTES: usize = 4096;
const MAX_UNCERTAIN_MARKER_BYTES: usize = 8192;

fn marker_bytes<T: Serialize>(record: &T, max: usize) -> Result<Vec<u8>, ProtocolFenceError> {
    let bytes = serde_json::to_vec(record).map_err(|_| StoreError::InvalidState)?;
    if bytes.len() > max {
        return Err(StoreError::InvalidState.into());
    }
    Ok(bytes)
}

#[derive(Debug, Error)]
pub enum ProtocolFenceError {
    #[error("another sync or cutover operation is active")]
    Busy,
    #[error("v1 is unavailable while this account uses or is switching to v2")]
    NotV1,
    #[error("v1 has an uncertain prior upload; review its outcome before cutover")]
    UncertainV1,
    #[error("this account requires the product outbox; legacy operations are unavailable")]
    ProductOutboxRequired,
    #[error("product sync is unavailable until the account has a reviewed outbox migration")]
    ProductOutboxNotEnabled,
    #[error("cutover needs the same reviewed inputs and a complete v2 bootstrap")]
    StaleCutover,
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("protocol fence storage failed")]
    Io(#[from] io::Error),
}

#[derive(Clone)]
pub struct ProtocolFence {
    dir: PathBuf,
}

/// Shared leases let normal v1 staging continue while v1 network work runs.
pub(crate) struct V1Lease {
    fence: ProtocolFence,
    _file: File,
}

/// Exclusive across cooperating processes, held through durable cutover.
pub struct CutoverLease {
    fence: ProtocolFence,
    _file: File,
}

/// Held by product v2 operations while accessing this identity. A raw
/// WorkspaceCoordinator alone does not enforce product protocol selection.
pub struct V2Lease {
    _file: File,
}
pub(crate) struct V1NetworkLease {
    _file: File,
}

/// Serializes short product mutation and upload-admission transactions across
/// cooperating processes. It must never be held during an HTTP await.
pub(crate) struct V1AdmissionLease {
    _file: File,
}

// A concurrently forked child can briefly inherit an open lock descriptor
// before close-on-exec takes effect. Explicitly unlock at the end of the
// lease rather than waiting for every inherited descriptor to close. This also
// makes the lifetime of a lease independent of a child's pre-exec delay.
macro_rules! unlock_on_drop {
    ($($lease:ty),+ $(,)?) => {$ (
        impl Drop for $lease {
            fn drop(&mut self) {
                // Closing the file remains a fallback if explicit unlock fails.
                let _ = FileExt::unlock(&self._file);
            }
        }
    )+};
}
unlock_on_drop!(
    V1Lease,
    CutoverLease,
    V2Lease,
    V1NetworkLease,
    V1AdmissionLease
);

pub(crate) struct UncertainUpload {
    path: PathBuf,
}

/// Frozen original candidate and base, without credentials or a replay action.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct V1UncertainUpload {
    format_version: u32,
    origin: String,
    account_id: String,
    source_generation: u64,
    key: String,
    pending: Pending,
    base_record: Option<Record>,
}
impl V1UncertainUpload {
    pub fn key(&self) -> &str {
        &self.key
    }
    pub fn pending(&self) -> &Pending {
        &self.pending
    }
    pub fn base_record(&self) -> Option<&Record> {
        self.base_record.as_ref()
    }
    pub fn source_generation(&self) -> u64 {
        self.source_generation
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProtocolRecord {
    format_version: u32,
    origin: String,
    account_id: String,
    mode: ProtocolMode,
    candidate_sha256: String,
}

/// Opaque exact marker used only to revalidate a native consent review.
#[allow(dead_code)] // Used by the not-yet-exposed native v2 consent review.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ResumeProtocolRecord(ProtocolRecord);

#[allow(dead_code)]
impl ResumeProtocolRecord {
    pub(crate) fn is_preparing(&self) -> bool {
        self.0.mode == ProtocolMode::Preparing
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProductOutboxRecord {
    format_version: u32,
    origin: String,
    account_id: String,
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum ProtocolMode {
    Preparing,
    V2,
}

fn identity_hash(origin: &str, account: &str) -> Result<String, ProtocolFenceError> {
    WorkspaceState::new(origin.to_owned(), account.to_owned())
        .map_err(|_| StoreError::InvalidState)?;
    let bytes = serde_json::to_vec(&(origin, account)).map_err(|_| StoreError::InvalidState)?;
    Ok(Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn candidate_hash(preview: &CutoverPreview) -> Result<String, ProtocolFenceError> {
    let bytes = preview
        .review_material()
        .map_err(|_| StoreError::InvalidState)?;
    Ok(Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn checked_file(path: &Path) -> Result<bool, ProtocolFenceError> {
    match fs::symlink_metadata(path) {
        Ok(meta) => {
            if !meta.file_type().is_file() {
                return Err(StoreError::UnsafeDirectory.into());
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if meta.permissions().mode() & 0o077 != 0 {
                    return Err(StoreError::UnsafeDirectory.into());
                }
            }
            Ok(true)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

fn sync_dir(dir: &Path) -> Result<(), ProtocolFenceError> {
    #[cfg(unix)]
    File::open(dir)?.sync_all()?;
    #[cfg(not(unix))]
    let _ = dir;
    Ok(())
}

impl ProtocolFence {
    /// The path is application-owned, private and shared by both protocols.
    pub fn open(dir: impl AsRef<Path>) -> Result<Self, ProtocolFenceError> {
        let dir = dir.as_ref().to_path_buf();
        StateStore::open(&dir)?;
        Ok(Self { dir })
    }

    fn lock_file_named(&self, name: &str) -> Result<File, ProtocolFenceError> {
        let path = self.dir.join(name);
        checked_file(&path)?;
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        Ok(options.open(path)?)
    }

    fn lock_file(&self) -> Result<File, ProtocolFenceError> {
        self.lock_file_named("protocol-operation.lock")
    }

    pub(crate) fn try_v1(&self) -> Result<V1Lease, ProtocolFenceError> {
        let file = self.lock_file()?;
        match FileExt::try_lock_shared(&file) {
            Ok(()) => Ok(V1Lease {
                fence: self.clone(),
                _file: file,
            }),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                Err(ProtocolFenceError::Busy)
            }
            Err(error) => Err(error.into()),
        }
    }

    /// Lifecycle operations exclude local staging as well as network runs.
    pub(crate) fn try_exclusive_v1(&self) -> Result<V1Lease, ProtocolFenceError> {
        let file = self.lock_file()?;
        match FileExt::try_lock_exclusive(&file) {
            Ok(()) => Ok(V1Lease {
                fence: self.clone(),
                _file: file,
            }),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                Err(ProtocolFenceError::Busy)
            }
            Err(error) => Err(error.into()),
        }
    }

    /// Existing legacy bytes remain at their original path. A different
    /// identity gets a separate private partition rather than replacing them.
    fn legacy_store(
        &self,
        origin: &str,
        account: &str,
    ) -> Result<(StateStore, PathBuf), ProtocolFenceError> {
        let hash = identity_hash(origin, account)?;
        let root = StateStore::open(&self.dir)?;
        let namespace = self.dir.join("legacy-v1");
        let partition = namespace.join(hash);
        let namespace_exists = match fs::symlink_metadata(&namespace) {
            Ok(meta) if meta.file_type().is_dir() => {
                StateStore::open(&namespace)?;
                true
            }
            Ok(_) => return Err(StoreError::UnsafeDirectory.into()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => false,
            Err(error) => return Err(error.into()),
        };
        if namespace_exists {
            match fs::symlink_metadata(&partition) {
                Ok(meta) if meta.file_type().is_dir() => {
                    return Ok((StateStore::open(&partition)?, partition))
                }
                Ok(_) => return Err(StoreError::UnsafeDirectory.into()),
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        if root.load()?.is_some_and(|snapshot| {
            snapshot.state.origin != origin || snapshot.state.account_id != account
        }) {
            StateStore::open(&namespace)?;
            let store = StateStore::open(&partition)?;
            sync_dir(&namespace)?;
            sync_dir(&self.dir)?;
            return Ok((store, partition));
        }
        Ok((root, self.dir.clone()))
    }

    /// Captures the legacy state for a preview without exposing another
    /// account's partition. The product must still recheck under cutover.
    pub fn legacy_snapshot(
        &self,
        origin: &str,
        account: &str,
    ) -> Result<Option<StateSnapshot>, ProtocolFenceError> {
        let _lease = self.try_v1()?;
        let (store, _) = self.legacy_store(origin, account)?;
        Ok(store.load()?)
    }

    pub fn try_cutover(&self) -> Result<CutoverLease, ProtocolFenceError> {
        let file = self.lock_file()?;
        match FileExt::try_lock_exclusive(&file) {
            Ok(()) => Ok(CutoverLease {
                fence: self.clone(),
                _file: file,
            }),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                Err(ProtocolFenceError::Busy)
            }
            Err(error) => Err(error.into()),
        }
    }

    pub fn try_v2(&self, origin: &str, account: &str) -> Result<V2Lease, ProtocolFenceError> {
        let file = self.lock_file()?;
        match FileExt::try_lock_shared(&file) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                return Err(ProtocolFenceError::Busy)
            }
            Err(error) => return Err(error.into()),
        }
        let lease = V2Lease { _file: file };
        if !self
            .read_record(origin, account)?
            .is_some_and(|record| record.mode == ProtocolMode::V2)
        {
            return Err(ProtocolFenceError::StaleCutover);
        }
        Ok(lease)
    }

    fn record_path(&self, origin: &str, account: &str) -> Result<PathBuf, ProtocolFenceError> {
        Ok(self
            .dir
            .join(format!("protocol-{}.json", identity_hash(origin, account)?)))
    }

    fn product_outbox_path(
        &self,
        origin: &str,
        account: &str,
    ) -> Result<PathBuf, ProtocolFenceError> {
        Ok(self.dir.join(format!(
            "v1-product-outbox-{}.json",
            identity_hash(origin, account)?
        )))
    }

    fn product_outbox_required(
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

    fn ensure_legacy_allowed(&self, origin: &str, account: &str) -> Result<(), ProtocolFenceError> {
        if self.product_outbox_required(origin, account)? {
            return Err(ProtocolFenceError::ProductOutboxRequired);
        }
        Ok(())
    }

    fn read_record(
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
        if !matches!(record.format_version, 1 | 2)
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

    fn write_record(&self, record: &ProtocolRecord) -> Result<(), ProtocolFenceError> {
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

    fn uncertain_count(&self, origin: &str, account: &str) -> Result<usize, ProtocolFenceError> {
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
    fn publish_uncertain(
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

    fn uncertain_records(
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
    /// A product drain may run only after explicit migration installed the
    /// per-account marker that fences cooperating legacy writers. Call this
    /// while holding the shared admission lease, not as a preflight alone.
    pub(crate) fn ensure_product_outbox(
        &self,
        origin: &str,
        account: &str,
    ) -> Result<(), ProtocolFenceError> {
        if self.fence.product_outbox_required(origin, account)? {
            Ok(())
        } else {
            Err(ProtocolFenceError::ProductOutboxNotEnabled)
        }
    }
    pub(crate) fn ensure_legacy_allowed(
        &self,
        origin: &str,
        account: &str,
    ) -> Result<(), ProtocolFenceError> {
        self.fence.ensure_legacy_allowed(origin, account)
    }
    pub(crate) fn ensure_outbox_mode(
        &self,
        origin: &str,
        account: &str,
        marked: bool,
    ) -> Result<(), ProtocolFenceError> {
        if marked {
            self.ensure_product_outbox(origin, account)
        } else {
            self.ensure_legacy_allowed(origin, account)
        }
    }
    /// Reserve a local mutation or admission of an upload. Callers must take
    /// the protocol lease first and keep it while this lease is live.
    pub(crate) fn try_admission(&self) -> Result<V1AdmissionLease, ProtocolFenceError> {
        let file = self.fence.lock_file_named("v1-admission.lock")?;
        match FileExt::try_lock_exclusive(&file) {
            Ok(()) => Ok(V1AdmissionLease { _file: file }),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                Err(ProtocolFenceError::Busy)
            }
            Err(error) => Err(error.into()),
        }
    }
    pub(crate) fn legacy_store(
        &self,
        origin: &str,
        account: &str,
    ) -> Result<(StateStore, PathBuf), ProtocolFenceError> {
        self.fence.legacy_store(origin, account)
    }
    /// At most one cooperating v1 network run can dispatch at a time. Local
    /// staging retains its independent short state lock and remains available.
    pub(crate) fn try_network(&self) -> Result<V1NetworkLease, ProtocolFenceError> {
        let file = self.fence.lock_file_named("v1-network.lock")?;
        match FileExt::try_lock_exclusive(&file) {
            Ok(()) => Ok(V1NetworkLease { _file: file }),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                Err(ProtocolFenceError::Busy)
            }
            Err(error) => Err(error.into()),
        }
    }

    pub(crate) fn ensure_no_uncertain(
        &self,
        origin: &str,
        account: &str,
    ) -> Result<(), ProtocolFenceError> {
        if self.fence.uncertain_count(origin, account)? != 0 {
            return Err(ProtocolFenceError::UncertainV1);
        }
        Ok(())
    }
    pub(crate) fn ensure_v1(&self, origin: &str, account: &str) -> Result<(), ProtocolFenceError> {
        if self.fence.read_record(origin, account)?.is_some() {
            return Err(ProtocolFenceError::NotV1);
        }
        Ok(())
    }

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
    #[allow(dead_code)] // Used by the not-yet-exposed native v2 consent review.
    pub(crate) fn resume_record(
        &self,
        origin: &str,
        account: &str,
    ) -> Result<ResumeProtocolRecord, ProtocolFenceError> {
        let record = self
            .fence
            .read_record(origin, account)?
            .ok_or(ProtocolFenceError::StaleCutover)?;
        // A historical preparation does not bind all reviewed inputs.
        if record.mode == ProtocolMode::Preparing && record.format_version != 2 {
            return Err(ProtocolFenceError::StaleCutover);
        }
        Ok(ResumeProtocolRecord(record))
    }
    /// Reconstruct an interrupted preparation for a new explicit review.
    /// The v1 inputs, installed selections and resulting candidate must hash
    /// to the original prepared marker; no migration state is changed.
    pub(crate) fn preview_preparing(
        &self,
        origin: &str,
        account: &str,
        selections: &[PluginSelection],
    ) -> Result<CutoverPreview, ProtocolFenceError> {
        self.ensure_legacy_allowed(origin, account)?;
        if self.uncertain_uploads(origin, account)? != 0 {
            return Err(ProtocolFenceError::UncertainV1);
        }
        let record = self
            .fence
            .read_record(origin, account)?
            .ok_or(ProtocolFenceError::StaleCutover)?;
        if record.format_version != 2 || record.mode != ProtocolMode::Preparing {
            return Err(ProtocolFenceError::StaleCutover);
        }
        let legacy = self.fence.legacy_store(origin, account)?.0.load()?;
        let preview =
            CutoverPreview::new(origin.into(), account.into(), selections, legacy.as_ref())
                .map_err(|_| ProtocolFenceError::StaleCutover)?;
        if record.candidate_sha256 != candidate_hash(&preview)? {
            return Err(ProtocolFenceError::StaleCutover);
        }
        Ok(preview)
    }

    /// Detect a repeated acknowledgement of the same completed cutover.
    /// Workspace content can legitimately evolve after completion, so the
    /// immutable protocol record is the evidence, not the present snapshot.
    pub(crate) fn completed_matching(
        &self,
        preview: &CutoverPreview,
        origin: &str,
        account: &str,
    ) -> Result<bool, ProtocolFenceError> {
        let Some(record) = self.fence.read_record(origin, account)? else {
            return Ok(false);
        };
        if record.mode != ProtocolMode::V2 {
            return Ok(false);
        }
        if record.format_version != 2 || record.candidate_sha256 != candidate_hash(preview)? {
            return Err(ProtocolFenceError::StaleCutover);
        }
        Ok(true)
    }

    pub(crate) fn ensure_legacy_allowed(
        &self,
        origin: &str,
        account: &str,
    ) -> Result<(), ProtocolFenceError> {
        self.fence.ensure_legacy_allowed(origin, account)
    }

    /// Persist the requirement before product mutations start writing an
    /// outbox. Future activation must perform a reviewed source migration
    /// before invoking this; older binaries cannot honor this cooperative fence.
    #[allow(dead_code)]
    pub(crate) fn require_product_outbox(
        &self,
        origin: &str,
        account: &str,
    ) -> Result<(), ProtocolFenceError> {
        self.ensure_v1(origin, account)?;
        if self.fence.uncertain_count(origin, account)? != 0 {
            return Err(ProtocolFenceError::UncertainV1);
        }
        if self.fence.product_outbox_required(origin, account)? {
            return Ok(());
        }
        let target = self.fence.product_outbox_path(origin, account)?;
        let record = ProductOutboxRecord {
            format_version: 1,
            origin: origin.to_owned(),
            account_id: account.to_owned(),
        };
        let bytes = marker_bytes(&record, MAX_PROTOCOL_MARKER_BYTES)?;
        let mut temp = tempfile::NamedTempFile::new_in(&self.fence.dir)?;
        temp.write_all(&bytes)?;
        temp.flush()?;
        temp.as_file().sync_all()?;
        temp.persist_noclobber(target)
            .map_err(|error| error.error)?;
        sync_dir(&self.fence.dir)
    }
    pub(crate) fn ensure_v1(&self, origin: &str, account: &str) -> Result<(), ProtocolFenceError> {
        if self.fence.read_record(origin, account)?.is_some() {
            return Err(ProtocolFenceError::NotV1);
        }
        Ok(())
    }

    pub(crate) fn legacy_store(
        &self,
        origin: &str,
        account: &str,
    ) -> Result<(StateStore, PathBuf), ProtocolFenceError> {
        self.fence.legacy_store(origin, account)
    }
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

    /// Stop v1 for this identity before bootstrap. Repeating after a crash is
    /// allowed only for the same reviewed candidate and unchanged v1 inputs.
    /// The product must independently hold the local source/review fence.
    pub fn prepare(
        &self,
        preview: &CutoverPreview,
        origin: &str,
        account: &str,
        selections: &[PluginSelection],
    ) -> Result<(), ProtocolFenceError> {
        self.ensure_legacy_allowed(origin, account)?;
        if self.uncertain_uploads(origin, account)? != 0 {
            return Err(ProtocolFenceError::UncertainV1);
        }
        let legacy = self.fence.legacy_store(origin, account)?.0.load()?;
        preview
            .verify_inputs(origin, account, selections, legacy.as_ref())
            .map_err(|_| ProtocolFenceError::StaleCutover)?;
        let candidate_sha256 = candidate_hash(preview)?;
        if let Some(existing) = self.fence.read_record(origin, account)? {
            if existing.format_version != 2
                || existing.mode != ProtocolMode::Preparing
                || existing.candidate_sha256 != candidate_sha256
            {
                return Err(ProtocolFenceError::StaleCutover);
            }
            return Ok(());
        }
        self.fence.write_record(&ProtocolRecord {
            format_version: 2,
            origin: origin.into(),
            account_id: account.into(),
            mode: ProtocolMode::Preparing,
            candidate_sha256,
        })
    }

    /// Commit protocol selection only after the exact reviewed candidate is
    /// durably bootstrapped. A crash leaves either Preparing or V2, never v1.
    pub fn finish(
        &self,
        preview: &CutoverPreview,
        origin: &str,
        account: &str,
    ) -> Result<(), ProtocolFenceError> {
        self.finish_with_hook(preview, origin, account, || {})
    }

    fn finish_with_hook(
        &self,
        preview: &CutoverPreview,
        origin: &str,
        account: &str,
        before_commit: impl FnOnce(),
    ) -> Result<(), ProtocolFenceError> {
        let mut record = self
            .fence
            .read_record(origin, account)?
            .ok_or(ProtocolFenceError::StaleCutover)?;
        if record.format_version != 2
            || record.mode != ProtocolMode::Preparing
            || record.candidate_sha256 != candidate_hash(preview)?
        {
            return Err(ProtocolFenceError::StaleCutover);
        }
        let legacy = self.fence.legacy_store(origin, account)?.0.load()?;
        preview
            .verify_inputs(origin, account, &preview.selections(), legacy.as_ref())
            .map_err(|_| ProtocolFenceError::StaleCutover)?;
        let store = WorkspaceStore::open(&self.fence.dir, origin, account)?;
        store
            .try_with_locked_snapshot(|snapshot| {
                let state = snapshot.ok_or(ProtocolFenceError::StaleCutover)?.state;
                if !preview.matches_bootstrap(&state) {
                    return Err(ProtocolFenceError::StaleCutover);
                }
                before_commit();
                record.mode = ProtocolMode::V2;
                self.fence.write_record(&record)
            })?
            .ok_or(ProtocolFenceError::Busy)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cloud_client::SyncState;
    use std::time::{Duration, Instant};

    const ORIGIN: &str = "https://cloud.example";
    const ACCOUNT: &str = "account-a";

    #[test]
    fn product_outbox_marker_is_identity_bound_and_corruption_fails_closed() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().join("private");
        let fence = ProtocolFence::open(&dir).unwrap();
        let gate = fence.try_cutover().unwrap();
        gate.require_product_outbox(ORIGIN, ACCOUNT).unwrap();
        assert!(matches!(
            gate.ensure_legacy_allowed(ORIGIN, ACCOUNT),
            Err(ProtocolFenceError::ProductOutboxRequired)
        ));
        let preview = CutoverPreview::new(ORIGIN.into(), ACCOUNT.into(), &[], None).unwrap();
        assert!(matches!(
            gate.prepare(&preview, ORIGIN, ACCOUNT, &[]),
            Err(ProtocolFenceError::ProductOutboxRequired)
        ));
        assert!(gate.ensure_legacy_allowed(ORIGIN, "account-b").is_ok());
        drop(gate);
        let path = fence.product_outbox_path(ORIGIN, ACCOUNT).unwrap();
        fs::write(&path, b"{invalid").unwrap();
        let reopened = ProtocolFence::open(&dir).unwrap();
        let lease = reopened.try_v1().unwrap();
        assert!(matches!(
            lease.ensure_legacy_allowed(ORIGIN, ACCOUNT),
            Err(ProtocolFenceError::Store(StoreError::InvalidState))
        ));
    }

    #[test]
    fn interrupted_marker_preparation_leaves_only_an_ignored_partial_file() {
        let dir = tempfile::tempdir().unwrap();
        let private = dir.path().join("private");
        let fence = ProtocolFence::open(&private).unwrap();
        let mut temp = tempfile::Builder::new()
            .prefix(".v1-uncertain-tmp-")
            .tempfile_in(&private)
            .unwrap();
        temp.write_all(b"{\"unfinished\":").unwrap();
        temp.flush().unwrap();
        let (_file, path) = temp.keep().unwrap();
        assert!(path.is_file());
        assert_eq!(fence.uncertain_count(ORIGIN, ACCOUNT).unwrap(), 0);
        assert!(fence.uncertain_records(ORIGIN, ACCOUNT).unwrap().is_empty());
    }

    #[test]
    fn marker_publication_never_overwrites_an_existing_destination() {
        let dir = tempfile::tempdir().unwrap();
        let private = dir.path().join("private");
        let fence = ProtocolFence::open(&private).unwrap();
        let prefix = format!("v1-uncertain-{}-", identity_hash(ORIGIN, ACCOUNT).unwrap());
        let destination = private.join(format!("{prefix}collision"));
        let mut existing = OpenOptions::new();
        existing.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            existing.mode(0o600);
        }
        existing
            .open(&destination)
            .unwrap()
            .write_all(b"original marker")
            .unwrap();
        let temp = tempfile::Builder::new()
            .prefix(".v1-uncertain-tmp-")
            .tempfile_in(&private)
            .unwrap();
        let temporary_path = temp.path().to_owned();
        let original = V1UncertainUpload {
            format_version: 1,
            origin: ORIGIN.into(),
            account_id: ACCOUNT.into(),
            source_generation: 1,
            key: "acme.plugin".into(),
            pending: Pending {
                base_revision: None,
                edit: cloud_client::Edit::Delete,
                conflict: false,
            },
            base_record: None,
        };
        assert!(matches!(
            fence.publish_uncertain(temp, destination.clone(), &original),
            Err(ProtocolFenceError::Io(ref error)) if error.kind() == io::ErrorKind::AlreadyExists
        ));
        assert_eq!(fs::read(&destination).unwrap(), b"original marker");
        assert!(!temporary_path.exists());
        assert_eq!(fence.uncertain_count(ORIGIN, ACCOUNT).unwrap(), 1);
    }

    #[test]
    fn cutover_fences_all_v1_leases_and_recovers_preparing_after_restart() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().join("private");
        let fence = ProtocolFence::open(&dir).unwrap();
        let legacy = StateStore::open(&dir)
            .unwrap()
            .save(None, &SyncState::new(ORIGIN.into(), ACCOUNT.into()))
            .unwrap();
        let old_bytes = fs::read(dir.join("sync-state.json")).unwrap();
        let preview =
            CutoverPreview::new(ORIGIN.into(), ACCOUNT.into(), &[], Some(&legacy)).unwrap();
        let v1 = fence.try_v1().unwrap();
        let another_v1 = ProtocolFence::open(&dir).unwrap().try_v1().unwrap();
        assert!(matches!(fence.try_cutover(), Err(ProtocolFenceError::Busy)));
        drop(v1);
        drop(another_v1);
        let cutover = fence.try_cutover().unwrap();
        cutover.prepare(&preview, ORIGIN, ACCOUNT, &[]).unwrap();
        assert!(matches!(fence.try_v1(), Err(ProtocolFenceError::Busy)));
        drop(cutover);
        let v1 = fence.try_v1().unwrap();
        assert!(matches!(
            v1.ensure_v1(ORIGIN, ACCOUNT),
            Err(ProtocolFenceError::NotV1)
        ));
        assert!(v1.ensure_v1(ORIGIN, "account-b").is_ok());
        drop(v1);
        assert!(matches!(
            fence.try_v2(ORIGIN, ACCOUNT),
            Err(ProtocolFenceError::StaleCutover)
        ));
        let cutover = ProtocolFence::open(&dir).unwrap().try_cutover().unwrap();
        cutover.prepare(&preview, ORIGIN, ACCOUNT, &[]).unwrap();
        assert!(matches!(
            cutover.finish(&preview, ORIGIN, ACCOUNT),
            Err(ProtocolFenceError::StaleCutover)
        ));
        let store = WorkspaceStore::open(&dir, ORIGIN, ACCOUNT).unwrap();
        assert!(store
            .try_with_locked_snapshot::<_, StoreError>(|_| Ok(()))
            .unwrap()
            .is_some());
        let workspace =
            crate::WorkspaceCoordinator::open(&dir, ORIGIN.into(), ACCOUNT.into()).unwrap();
        workspace
            .bootstrap(&preview.selections(), preview.legacy())
            .unwrap();
        cutover.finish(&preview, ORIGIN, ACCOUNT).unwrap();
        drop(cutover);
        let v2 = fence.try_v2(ORIGIN, ACCOUNT).unwrap();
        assert!(matches!(fence.try_cutover(), Err(ProtocolFenceError::Busy)));
        drop(v2);
        assert!(matches!(
            fence.try_v1().unwrap().ensure_v1(ORIGIN, ACCOUNT),
            Err(ProtocolFenceError::NotV1)
        ));
        assert_eq!(fs::read(dir.join("sync-state.json")).unwrap(), old_bytes);
    }

    #[test]
    fn finish_holds_workspace_state_lock_until_protocol_marker_is_written() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().join("private");
        let fence = ProtocolFence::open(&dir).unwrap();
        let preview = CutoverPreview::new(ORIGIN.into(), ACCOUNT.into(), &[], None).unwrap();
        let cutover = fence.try_cutover().unwrap();
        cutover.prepare(&preview, ORIGIN, ACCOUNT, &[]).unwrap();
        let coordinator =
            crate::WorkspaceCoordinator::open(&dir, ORIGIN.into(), ACCOUNT.into()).unwrap();
        coordinator.bootstrap(&[], None).unwrap();
        let store = WorkspaceStore::open(&dir, ORIGIN, ACCOUNT).unwrap();
        let snapshot = store.load().unwrap().unwrap();
        let mut newer = snapshot.state.clone();
        let key = cloud_client::WorkspaceKey::new("plugin-selections", "acme.plugin").unwrap();
        let selection = PluginSelection {
            schema_version: 1,
            plugin_id: "acme.plugin".into(),
            version: "1.0.0".into(),
            enabled: true,
        };
        newer
            .stage(
                key,
                cloud_client::WorkspaceEdit::Put(serde_json::to_value(selection).unwrap()),
            )
            .unwrap();
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        let writer_dir = dir.clone();
        let mut writer = None;
        cutover
            .finish_with_hook(&preview, ORIGIN, ACCOUNT, || {
                assert!(store
                    .try_with_locked_snapshot::<_, StoreError>(|_| Ok(()))
                    .unwrap()
                    .is_none());
                writer = Some(std::thread::spawn(move || {
                    let store = WorkspaceStore::open(&writer_dir, ORIGIN, ACCOUNT).unwrap();
                    started_tx.send(()).unwrap();
                    let result = store.save(Some(snapshot.generation), &newer);
                    done_tx.send(result.is_ok()).unwrap();
                }));
                started_rx.recv_timeout(Duration::from_secs(3)).unwrap();
                assert!(done_rx.recv_timeout(Duration::from_millis(100)).is_err());
            })
            .unwrap();
        assert!(done_rx.recv_timeout(Duration::from_secs(3)).unwrap());
        writer.unwrap().join().unwrap();
        assert_eq!(store.load().unwrap().unwrap().state.entries.len(), 1);
        drop(cutover);
        assert!(fence.try_v2(ORIGIN, ACCOUNT).is_ok());
    }

    #[test]
    fn prepared_cutover_binds_full_legacy_snapshot_across_restart() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().join("private");
        let fence = ProtocolFence::open(&dir).unwrap();
        let store = StateStore::open(&dir).unwrap();
        let legacy = store
            .save(None, &SyncState::new(ORIGIN.into(), ACCOUNT.into()))
            .unwrap();
        let reviewed =
            CutoverPreview::new(ORIGIN.into(), ACCOUNT.into(), &[], Some(&legacy)).unwrap();
        let cutover = fence.try_cutover().unwrap();
        cutover.prepare(&reviewed, ORIGIN, ACCOUNT, &[]).unwrap();
        drop(cutover);

        // A writer outside the protocol changes only the legacy cursor; there
        // are still no staged entries, so a digest of entries alone collides.
        let mut changed = legacy.state.clone();
        changed.cursor = 1;
        let changed = store.save(Some(legacy.generation), &changed).unwrap();
        let fresh =
            CutoverPreview::new(ORIGIN.into(), ACCOUNT.into(), &[], Some(&changed)).unwrap();
        assert_eq!(reviewed.entries(), fresh.entries());
        let cutover = ProtocolFence::open(&dir).unwrap().try_cutover().unwrap();
        assert!(matches!(
            cutover.prepare(&fresh, ORIGIN, ACCOUNT, &[]),
            Err(ProtocolFenceError::StaleCutover)
        ));
        assert!(matches!(
            cutover.prepare(&reviewed, ORIGIN, ACCOUNT, &[]),
            Err(ProtocolFenceError::StaleCutover)
        ));
        drop(cutover);

        // Returning the bytes to their old values still has a new source
        // generation and cannot revive the original review.
        let restored = store.save(Some(changed.generation), &legacy.state).unwrap();
        let restored_preview =
            CutoverPreview::new(ORIGIN.into(), ACCOUNT.into(), &[], Some(&restored)).unwrap();
        let cutover = ProtocolFence::open(&dir).unwrap().try_cutover().unwrap();
        assert!(matches!(
            cutover.prepare(&restored_preview, ORIGIN, ACCOUNT, &[]),
            Err(ProtocolFenceError::StaleCutover)
        ));
        assert!(matches!(
            cutover.finish(&restored_preview, ORIGIN, ACCOUNT),
            Err(ProtocolFenceError::StaleCutover)
        ));
    }

    #[test]
    fn legacy_preparation_is_refused_but_completed_v2_remains_readable() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().join("private");
        let fence = ProtocolFence::open(&dir).unwrap();
        let preview = CutoverPreview::new(ORIGIN.into(), ACCOUNT.into(), &[], None).unwrap();
        let legacy_digest = Sha256::digest(serde_json::to_vec(preview.entries()).unwrap())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let mut record = ProtocolRecord {
            format_version: 1,
            origin: ORIGIN.into(),
            account_id: ACCOUNT.into(),
            mode: ProtocolMode::Preparing,
            candidate_sha256: legacy_digest,
        };
        fence.write_record(&record).unwrap();
        let cutover = fence.try_cutover().unwrap();
        assert!(matches!(
            cutover.prepare(&preview, ORIGIN, ACCOUNT, &[]),
            Err(ProtocolFenceError::StaleCutover)
        ));
        assert!(matches!(
            cutover.finish(&preview, ORIGIN, ACCOUNT),
            Err(ProtocolFenceError::StaleCutover)
        ));
        drop(cutover);

        // Existing completed protocol selection must not silently revert to v1.
        record.mode = ProtocolMode::V2;
        fence.write_record(&record).unwrap();
        assert!(fence.try_v2(ORIGIN, ACCOUNT).is_ok());
        let v1 = fence.try_v1().unwrap();
        assert!(matches!(
            v1.ensure_v1(ORIGIN, ACCOUNT),
            Err(ProtocolFenceError::NotV1)
        ));
    }

    #[test]
    fn oversized_cutover_and_outbox_markers_do_not_disable_v1() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().join("private");
        let fence = ProtocolFence::open(&dir).unwrap();
        let origin = format!("https://{}.example", "a".repeat(5000));
        // A syntactically valid origin may still exceed a marker's byte cap.
        let preview = CutoverPreview::new(origin.clone(), ACCOUNT.into(), &[], None).unwrap();
        let cutover = fence.try_cutover().unwrap();
        assert!(matches!(
            cutover.prepare(&preview, &origin, ACCOUNT, &[]),
            Err(ProtocolFenceError::Store(StoreError::InvalidState))
        ));
        assert!(!fence.record_path(&origin, ACCOUNT).unwrap().exists());
        assert!(matches!(
            cutover.require_product_outbox(&origin, ACCOUNT),
            Err(ProtocolFenceError::Store(StoreError::InvalidState))
        ));
        assert!(!fence
            .product_outbox_path(&origin, ACCOUNT)
            .unwrap()
            .exists());
        drop(cutover);
        let v1 = fence.try_v1().unwrap();
        assert!(v1.ensure_v1(&origin, ACCOUNT).is_ok());
        assert!(v1.ensure_legacy_allowed(&origin, ACCOUNT).is_ok());
    }

    #[test]
    fn oversized_uncertain_marker_fails_before_publication() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().join("private");
        let fence = ProtocolFence::open(&dir).unwrap();
        let origin = format!("https://{}.example", "a".repeat(9000));
        let mut state = SyncState::new(origin.clone(), ACCOUNT.into());
        state
            .stage("acme.plugin".into(), cloud_client::Edit::Delete)
            .unwrap();
        let snapshot = StateSnapshot {
            generation: 1,
            state,
        };
        let v1 = fence.try_v1().unwrap();
        assert!(matches!(
            v1.prepare_upload(&snapshot, "acme.plugin"),
            Err(ProtocolFenceError::Store(StoreError::InvalidState))
        ));
        assert_eq!(fence.uncertain_count(&origin, ACCOUNT).unwrap(), 0);
        assert!(fence
            .uncertain_records(&origin, ACCOUNT)
            .unwrap()
            .is_empty());
        assert!(v1.ensure_v1(&origin, ACCOUNT).is_ok());
    }

    #[test]
    fn ambiguous_uploads_are_per_identity_and_survive_restart() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().join("private");
        let fence = ProtocolFence::open(&dir).unwrap();
        let first = fence.try_v1().unwrap();
        let second = fence.try_v1().unwrap();
        let network = first.try_network().unwrap();
        assert!(matches!(
            second.try_network(),
            Err(ProtocolFenceError::Busy)
        ));
        drop(network);
        let mut source = SyncState::new(ORIGIN.into(), ACCOUNT.into());
        source
            .stage(
                "acme.plugin".into(),
                cloud_client::Edit::Put(PluginSelection {
                    schema_version: 1,
                    plugin_id: "acme.plugin".into(),
                    version: "1.0.0".into(),
                    enabled: false,
                }),
            )
            .unwrap();
        let uncertain = first
            .prepare_upload(
                &StateSnapshot {
                    generation: 1,
                    state: source.clone(),
                },
                "acme.plugin",
            )
            .unwrap();
        source
            .stage(
                "acme.plugin".into(),
                cloud_client::Edit::Put(PluginSelection {
                    schema_version: 1,
                    plugin_id: "acme.plugin".into(),
                    version: "2.0.0".into(),
                    enabled: true,
                }),
            )
            .unwrap();
        let known = second
            .prepare_upload(
                &StateSnapshot {
                    generation: 2,
                    state: source.clone(),
                },
                "acme.plugin",
            )
            .unwrap();
        second.acknowledge_upload(known).unwrap();
        drop(uncertain); // Simulates cancellation after dispatch.
        drop(first);
        drop(second);
        let cutover = ProtocolFence::open(&dir).unwrap().try_cutover().unwrap();
        assert_eq!(cutover.uncertain_uploads(ORIGIN, ACCOUNT).unwrap(), 1);
        let frozen = cutover.uncertain_details(ORIGIN, ACCOUNT).unwrap();
        assert_eq!(frozen.len(), 1);
        assert_eq!(frozen[0].key(), "acme.plugin");
        assert!(
            matches!(&frozen[0].pending().edit, cloud_client::Edit::Put(value) if value.version == "1.0.0" && !value.enabled)
        );
        assert!(frozen[0].base_record().is_none());
        assert_eq!(cutover.uncertain_uploads(ORIGIN, "account-b").unwrap(), 0);
        let preview = CutoverPreview::new(ORIGIN.into(), ACCOUNT.into(), &[], None).unwrap();
        assert!(matches!(
            cutover.prepare(&preview, ORIGIN, ACCOUNT, &[]),
            Err(ProtocolFenceError::UncertainV1)
        ));
    }

    #[test]
    fn corrupt_protocol_record_is_never_treated_as_v1() {
        let temp = tempfile::tempdir().unwrap();
        let fence = ProtocolFence::open(temp.path().join("private")).unwrap();
        let path = fence.record_path(ORIGIN, ACCOUNT).unwrap();
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        options.open(path).unwrap().write_all(b"bad").unwrap();
        assert!(matches!(
            fence.try_v1().unwrap().ensure_v1(ORIGIN, ACCOUNT),
            Err(ProtocolFenceError::Store(StoreError::InvalidState))
        ));
    }

    #[test]
    fn child_shared_lease() {
        let Some(dir) = std::env::var_os("TIKTOOLS_FENCE_CHILD_DIR") else {
            return;
        };
        let dir = PathBuf::from(dir);
        let fence = ProtocolFence::open(&dir).unwrap();
        let _lease = fence.try_v1().unwrap();
        fs::write(dir.join("child-ready"), b"ready").unwrap();
        let until = Instant::now() + Duration::from_secs(20);
        while !dir.join("child-exit").exists() && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(dir.join("child-exit").exists());
    }

    #[cfg(unix)]
    #[test]
    fn dropped_lease_unlocks_before_forked_child_executes() {
        use std::os::unix::{net::UnixStream, process::CommandExt};

        let temp = tempfile::tempdir().unwrap();
        let fence = ProtocolFence::open(temp.path().join("private")).unwrap();
        // A raw lock serves as a control: closing it in the parent leaves it
        // held until the inherited descriptor is closed by the child.
        let raw = fence.lock_file_named("fork-control.lock").unwrap();
        FileExt::try_lock_exclusive(&raw).unwrap();
        let gate = fence.try_cutover().unwrap();
        let (mut parent, mut child) = UnixStream::pair().unwrap();
        parent
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        child
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .args(["--exact", "protocol_fence::tests::child_shared_lease"])
            .env_remove("TIKTOOLS_FENCE_CHILD_DIR")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        // The closure performs only socket reads/writes, and reports both
        // failures to spawn without panicking or allocating in the child.
        unsafe {
            command.pre_exec(move || {
                child.write_all(&[1])?;
                let mut resume = [0u8; 1];
                child.read_exact(&mut resume)?;
                Ok(())
            });
        }
        // Command::spawn waits for exec, so perform it on another thread while
        // the main test observes the deliberately paused pre-exec child.
        let handle = std::thread::spawn(move || command.spawn().and_then(|mut child| child.wait()));
        let mut ready = [0u8; 1];
        parent.read_exact(&mut ready).unwrap();
        drop(raw);
        drop(gate);
        let control = fence.lock_file_named("fork-control.lock").unwrap();
        let control_held = matches!(FileExt::try_lock_shared(&control), Err(ref error) if error.kind() == io::ErrorKind::WouldBlock);
        let lease = fence.try_v1();
        parent.write_all(&[1]).unwrap();
        assert!(handle.join().unwrap().unwrap().success());
        assert!(control_held, "control flock must remain held across fork");
        assert!(
            lease.is_ok(),
            "the dropped lease must release its flock before exec"
        );
    }

    #[test]
    fn cutover_waits_for_v1_in_a_separate_process() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().join("private");
        let fence = ProtocolFence::open(&dir).unwrap();
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "protocol_fence::tests::child_shared_lease"])
            .env("TIKTOOLS_FENCE_CHILD_DIR", &dir)
            .spawn()
            .unwrap();
        let until = Instant::now() + Duration::from_secs(10);
        while !dir.join("child-ready").exists() && Instant::now() < until {
            assert!(child.try_wait().unwrap().is_none());
            std::thread::sleep(Duration::from_millis(10));
        }
        if !dir.join("child-ready").exists() {
            let _ = child.kill();
            let _ = child.wait();
            panic!("child did not acquire v1 lease");
        }
        assert!(matches!(fence.try_cutover(), Err(ProtocolFenceError::Busy)));
        fs::write(dir.join("child-exit"), b"done").unwrap();
        assert!(child.wait().unwrap().success());
        assert!(fence.try_cutover().is_ok());
    }
}
