//! Atomic, generation-checked local state storage. The application owns the directory.
use crate::{valid_key, validate_record, Edit, SyncState};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};
use thiserror::Error;
use url::Url;

const FORMAT_VERSION: u32 = 1;
const MAX_STATE_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("local sync state storage failed")]
    Io(#[from] io::Error),
    #[error("local sync state is invalid or unsupported")]
    InvalidState,
    #[error("local sync state changed in another process; reload before saving")]
    ConcurrentChange,
    #[error("local sync state belongs to a different origin or account")]
    WrongIdentity,
    #[error("the state directory must be private and must not be a symlink")]
    UnsafeDirectory,
}

#[derive(Clone)]
pub struct StateSnapshot {
    pub generation: u64,
    pub state: SyncState,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Envelope {
    format_version: u32,
    generation: u64,
    state: SyncState,
}

/// A private, application-owned directory, not a path supplied by a plugin.
/// Cooperating processes must use this store and must not delete its lock file.
pub struct StateStore {
    directory: PathBuf,
}
impl StateStore {
    pub fn open(directory: impl AsRef<Path>) -> Result<Self, StoreError> {
        let directory = directory.as_ref().to_path_buf();
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(&directory)?;
        let meta = fs::symlink_metadata(&directory)?;
        if !meta.file_type().is_dir() {
            return Err(StoreError::UnsafeDirectory);
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if meta.permissions().mode() & 0o077 != 0 {
                return Err(StoreError::UnsafeDirectory);
            }
        }
        Ok(Self { directory })
    }
    fn lock(&self) -> Result<File, StoreError> {
        let path = self.directory.join("sync-state.lock");
        if let Ok(meta) = fs::symlink_metadata(&path) {
            if !meta.file_type().is_file() {
                return Err(StoreError::UnsafeDirectory);
            }
        }
        let mut opts = OpenOptions::new();
        opts.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        let file = opts.open(path)?;
        FileExt::lock_exclusive(&file)?;
        Ok(file)
    }
    fn read_locked(&self) -> Result<Option<StateSnapshot>, StoreError> {
        let path = self.directory.join("sync-state.json");
        match fs::symlink_metadata(&path) {
            Ok(meta) if !meta.file_type().is_file() => return Err(StoreError::UnsafeDirectory),
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
            _ => (),
        }
        let file = File::open(path)?;
        let mut bytes = Vec::new();
        file.take(MAX_STATE_BYTES + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_STATE_BYTES {
            return Err(StoreError::InvalidState);
        }
        let envelope: Envelope =
            serde_json::from_slice(&bytes).map_err(|_| StoreError::InvalidState)?;
        if envelope.format_version != FORMAT_VERSION
            || envelope.generation == 0
            || !valid_state(&envelope.state)
        {
            return Err(StoreError::InvalidState);
        }
        Ok(Some(StateSnapshot {
            generation: envelope.generation,
            state: envelope.state,
        }))
    }
    pub fn load(&self) -> Result<Option<StateSnapshot>, StoreError> {
        let _guard = self.lock()?;
        self.read_locked()
    }
    /// `expected_generation` is None only when creating an absent file. A save
    /// can be committed even if directory fsync subsequently fails: reload on error.
    pub fn save(
        &self,
        expected_generation: Option<u64>,
        state: &SyncState,
    ) -> Result<StateSnapshot, StoreError> {
        if !valid_state(state) {
            return Err(StoreError::InvalidState);
        }
        let _guard = self.lock()?;
        let current = self.read_locked()?;
        if current.as_ref().map(|v| v.generation) != expected_generation {
            return Err(StoreError::ConcurrentChange);
        }
        if current.as_ref().is_some_and(|v| {
            v.state.origin != state.origin || v.state.account_id != state.account_id
        }) {
            return Err(StoreError::WrongIdentity);
        }
        let generation = expected_generation
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(StoreError::InvalidState)?;
        let data = serde_json::to_vec(&Envelope {
            format_version: FORMAT_VERSION,
            generation,
            state: state.clone(),
        })
        .map_err(|_| StoreError::InvalidState)?;
        if data.len() as u64 > MAX_STATE_BYTES {
            return Err(StoreError::InvalidState);
        }
        let mut temp = tempfile::NamedTempFile::new_in(&self.directory)?;
        temp.write_all(&data)?;
        temp.as_file().sync_all()?;
        temp.persist(self.directory.join("sync-state.json"))
            .map_err(|error| StoreError::Io(error.error))?;
        // On Unix this makes the directory entry durable after the rename.
        #[cfg(unix)]
        File::open(&self.directory)?.sync_all()?;
        Ok(StateSnapshot {
            generation,
            state: state.clone(),
        })
    }
}

fn valid_state(state: &SyncState) -> bool {
    let Ok(origin) = Url::parse(&state.origin) else {
        return false;
    };
    let loopback = matches!(origin.host_str(), Some("127.0.0.1" | "[::1]"));
    if !(origin.scheme() == "https" || origin.scheme() == "http" && loopback)
        || origin.origin().ascii_serialization() != state.origin
        || !origin.username().is_empty()
        || origin.password().is_some()
        || state.account_id.is_empty()
        || state.account_id.len() > 256
        || state.cursor < 0
    {
        return false;
    }
    state.entries.iter().all(|(key, entry)| {
        valid_key(key)
            && entry
                .remote
                .as_ref()
                .is_none_or(|r| r.key == *key && validate_record(r).is_ok())
            && entry.pending.as_ref().is_none_or(|pending| {
                pending.base_revision.is_none_or(|r| r > 0)
                    && (pending.conflict
                        || pending.base_revision == entry.remote.as_ref().map(|r| r.revision))
                    && match &pending.edit {
                        Edit::Delete => true,
                        Edit::Put(v) => {
                            v.schema_version == 1
                                && v.plugin_id == *key
                                && v.version.len() <= 64
                                && semver::Version::parse(&v.version).is_ok()
                        }
                    }
            })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PluginSelection;
    use std::sync::{Arc, Barrier};

    fn selection(version: &str) -> Edit {
        Edit::Put(PluginSelection {
            schema_version: 1,
            plugin_id: "acme.plugin".into(),
            version: version.into(),
            enabled: true,
        })
    }
    fn state() -> SyncState {
        let mut state = SyncState::new("https://cloud.example".into(), "account-1".into());
        state
            .stage("acme.plugin".into(), selection("1.0.0"))
            .unwrap();
        state
    }
    #[test]
    fn state_survives_restart_and_stale_writers_cannot_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("private-state");
        let store = StateStore::open(&state_dir).unwrap();
        assert!(store.load().unwrap().is_none());
        let saved = store.save(None, &state()).unwrap();
        assert_eq!(saved.generation, 1);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(state_dir.join("sync-state.json"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o077,
                0
            );
            assert_eq!(
                fs::metadata(state_dir.join("sync-state.lock"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o077,
                0
            );
        }
        assert_eq!(
            StateStore::open(&state_dir)
                .unwrap()
                .load()
                .unwrap()
                .unwrap()
                .state,
            saved.state
        );
        assert!(matches!(
            store.save(None, &saved.state),
            Err(StoreError::ConcurrentChange)
        ));
        let barrier = Arc::new(Barrier::new(3));
        let handles: Vec<_> = ["2.0.0", "3.0.0"]
            .into_iter()
            .map(|version| {
                let dir = state_dir.clone();
                let barrier = barrier.clone();
                let mut edit = saved.state.clone();
                edit.stage("acme.plugin".into(), selection(version))
                    .unwrap();
                std::thread::spawn(move || {
                    let store = StateStore::open(dir).unwrap();
                    barrier.wait();
                    store.save(Some(1), &edit)
                })
            })
            .collect();
        barrier.wait();
        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
        assert_eq!(
            results
                .iter()
                .filter(|r| matches!(r, Err(StoreError::ConcurrentChange)))
                .count(),
            1
        );
        let latest = store.load().unwrap().unwrap();
        assert_eq!(latest.generation, 2);
        assert!(latest.state.entries["acme.plugin"].pending.is_some());
        let mut wrong = latest.state.clone();
        wrong.account_id = "different".into();
        assert!(matches!(
            store.save(Some(2), &wrong),
            Err(StoreError::WrongIdentity)
        ));
        assert_eq!(store.load().unwrap().unwrap().generation, 2);
    }
    #[test]
    fn corrupt_or_future_state_is_not_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("private-state");
        let store = StateStore::open(&state_dir).unwrap();
        let snapshot = store.save(None, &state()).unwrap();
        let path = state_dir.join("sync-state.json");
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        value["state"]["token"] = serde_json::json!("do-not-persist");
        fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(matches!(store.load(), Err(StoreError::InvalidState)));
        assert!(matches!(
            store.save(Some(snapshot.generation), &snapshot.state),
            Err(StoreError::InvalidState)
        ));
        value.as_object_mut().unwrap().remove("state");
        value["formatVersion"] = serde_json::json!(99);
        fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(matches!(store.load(), Err(StoreError::InvalidState)));
    }
    #[cfg(unix)]
    #[test]
    fn rejects_public_directory_and_symlinked_state() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let dir = tempfile::tempdir().unwrap();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o755)).unwrap();
        assert!(matches!(
            StateStore::open(dir.path()),
            Err(StoreError::UnsafeDirectory)
        ));
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let store = StateStore::open(dir.path()).unwrap();
        symlink("elsewhere", dir.path().join("sync-state.json")).unwrap();
        assert!(matches!(store.load(), Err(StoreError::UnsafeDirectory)));
    }
}
