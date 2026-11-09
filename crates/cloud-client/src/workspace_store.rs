//! Separate, private V2 state partitions. V1 files and generations are never
//! rewritten, and changing accounts cannot delete another account's pending work.
use crate::{workspace_state::MAX_WORKSPACE_STATE_BYTES, StoreError, WorkspaceState};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

#[derive(Clone)]
pub struct WorkspaceStateSnapshot {
    pub generation: u64,
    pub state: WorkspaceState,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Envelope {
    format_version: u32,
    generation: u64,
    state: WorkspaceState,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EnvelopeRef<'a> {
    format_version: u32,
    generation: u64,
    state: &'a WorkspaceState,
}

pub struct WorkspaceStore {
    directory: PathBuf,
    origin: String,
    account_id: String,
}
/// Held only by operations that may contact the server. Local stages use the
/// independent short state lock, so they remain available while HTTP awaits.
pub struct WorkspaceOperationGuard {
    _file: File,
}

struct WorkspaceStateGuard(File);
impl Drop for WorkspaceStateGuard {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.0);
    }
}

fn private_directory(path: &Path) -> Result<(), StoreError> {
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)?;
    let meta = fs::symlink_metadata(path)?;
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
    Ok(())
}
fn check_file(path: &Path) -> Result<bool, StoreError> {
    match fs::symlink_metadata(path) {
        Ok(meta) => {
            if !meta.file_type().is_file() {
                return Err(StoreError::UnsafeDirectory);
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if meta.permissions().mode() & 0o077 != 0 {
                    return Err(StoreError::UnsafeDirectory);
                }
            }
            Ok(true)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}
impl WorkspaceStore {
    /// `root` is a trusted application-owned private path, never plugin input.
    pub fn open(
        root: impl AsRef<Path>,
        origin: impl Into<String>,
        account_id: impl Into<String>,
    ) -> Result<Self, StoreError> {
        let origin = origin.into();
        let account_id = account_id.into();
        WorkspaceState::new(origin.clone(), account_id.clone())
            .map_err(|_| StoreError::InvalidState)?;
        let root = root.as_ref();
        private_directory(root)?;
        let identity = serde_json::to_vec(&("workspace-v2", &origin, &account_id))
            .map_err(|_| StoreError::InvalidState)?;
        let digest = Sha256::digest(identity);
        let partition: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
        let namespace = root.join("workspace-v2");
        private_directory(&namespace)?;
        let directory = namespace.join(partition);
        private_directory(&directory)?;
        #[cfg(unix)]
        {
            File::open(&namespace)?.sync_all()?;
            File::open(root)?.sync_all()?;
        }
        Ok(Self {
            directory,
            origin,
            account_id,
        })
    }
    fn lock_file(&self, name: &str) -> Result<File, StoreError> {
        let path = self.directory.join(name);
        check_file(&path)?;
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        Ok(options.open(path)?)
    }
    fn lock(&self) -> Result<File, StoreError> {
        let file = self.lock_file("workspace-state.lock")?;
        FileExt::lock_exclusive(&file)?;
        Ok(file)
    }
    pub fn try_operation_lock(&self) -> Result<Option<WorkspaceOperationGuard>, StoreError> {
        let file = self.lock_file("workspace-operation.lock")?;
        match FileExt::try_lock_exclusive(&file) {
            Ok(()) => Ok(Some(WorkspaceOperationGuard { _file: file })),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => Ok(None),
            Err(error) => Err(error.into()),
        }
    }
    fn read_locked(&self) -> Result<Option<WorkspaceStateSnapshot>, StoreError> {
        let path = self.directory.join("workspace-state.json");
        if !check_file(&path)? {
            return Ok(None);
        }
        let mut bytes = Vec::new();
        File::open(path)?
            .take(MAX_WORKSPACE_STATE_BYTES as u64 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > MAX_WORKSPACE_STATE_BYTES || !bounded_depth(&bytes) {
            return Err(StoreError::InvalidState);
        }
        let envelope: Envelope =
            serde_json::from_slice(&bytes).map_err(|_| StoreError::InvalidState)?;
        if envelope.format_version != 2
            || envelope.generation == 0
            || envelope.state.validate().is_err()
        {
            return Err(StoreError::InvalidState);
        }
        if envelope.state.origin != self.origin || envelope.state.account_id != self.account_id {
            return Err(StoreError::WrongIdentity);
        }
        Ok(Some(WorkspaceStateSnapshot {
            generation: envelope.generation,
            state: envelope.state,
        }))
    }
    pub fn load(&self) -> Result<Option<WorkspaceStateSnapshot>, StoreError> {
        let _guard = self.lock()?;
        self.read_locked()
    }
    /// Invoke a synchronous callback while holding the same private state
    /// lock used by `save`. Returns `None` instead of waiting for a competing
    /// writer. The callback must not reenter this store, await or do network
    /// I/O; callers performing filesystem work must use a blocking thread.
    /// This coordinates only writers that honor the workspace state lock.
    pub fn try_with_locked_snapshot<T, E: From<StoreError>>(
        &self,
        operation: impl FnOnce(Option<WorkspaceStateSnapshot>) -> Result<T, E>,
    ) -> Result<Option<T>, E> {
        let file = self.lock_file("workspace-state.lock").map_err(E::from)?;
        match FileExt::try_lock_exclusive(&file) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(None),
            Err(error) => return Err(E::from(StoreError::Io(error))),
        }
        let _guard = WorkspaceStateGuard(file);
        let snapshot = self.read_locked().map_err(E::from)?;
        Ok(Some(operation(snapshot)?))
    }
    /// A rename may commit before a directory-fsync failure is returned. Reload
    /// on storage error; neither retry a prepared request nor erase uncertainty.
    pub fn save(
        &self,
        expected_generation: Option<u64>,
        state: &WorkspaceState,
    ) -> Result<WorkspaceStateSnapshot, StoreError> {
        self.save_with_publication_guard(expected_generation, state, |publish| publish())
    }

    /// Prepare and fsync the complete candidate under the state CAS lock,
    /// then invoke `guard` immediately before its atomic publication. The
    /// guard must invoke `publish` exactly once on success. This permits a
    /// caller to fence a short rename against a session transition without
    /// holding that session fence while serializing or fsyncing the file.
    /// A blocked native rename may still delay the holder of that fence.
    /// The parent directory sync occurs after the callback and can report an
    /// error after the rename has already committed.
    pub fn save_with_publication_guard<E: From<StoreError>>(
        &self,
        expected_generation: Option<u64>,
        state: &WorkspaceState,
        guard: impl FnOnce(&mut dyn FnMut() -> Result<(), E>) -> Result<(), E>,
    ) -> Result<WorkspaceStateSnapshot, E> {
        let (_state_lock, generation, temp) = self
            .prepare_save(expected_generation, state)
            .map_err(E::from)?;
        let mut temp = Some(temp);
        let mut published = false;
        let mut publish_failed = false;
        {
            let mut publish = || {
                let result = match temp.take() {
                    Some(file) => file
                        .persist(self.directory.join("workspace-state.json"))
                        .map(|_| ())
                        .map_err(|error| E::from(StoreError::Io(error.error))),
                    None => Err(E::from(StoreError::InvalidState)),
                };
                match result {
                    Ok(()) => {
                        published = true;
                        Ok(())
                    }
                    Err(error) => {
                        publish_failed = true;
                        Err(error)
                    }
                }
            };
            guard(&mut publish)?;
        }
        if !published || publish_failed {
            return Err(E::from(StoreError::InvalidState));
        }
        #[cfg(unix)]
        File::open(&self.directory)
            .and_then(|directory| directory.sync_all())
            .map_err(StoreError::Io)
            .map_err(E::from)?;
        Ok(WorkspaceStateSnapshot {
            generation,
            state: state.clone(),
        })
    }

    fn prepare_save(
        &self,
        expected_generation: Option<u64>,
        state: &WorkspaceState,
    ) -> Result<(File, u64, tempfile::NamedTempFile), StoreError> {
        state.validate().map_err(|_| StoreError::InvalidState)?;
        if state.origin != self.origin || state.account_id != self.account_id {
            return Err(StoreError::WrongIdentity);
        }
        let state_lock = self.lock()?;
        let current = self.read_locked()?;
        if current.as_ref().map(|snapshot| snapshot.generation) != expected_generation {
            return Err(StoreError::ConcurrentChange);
        }
        let generation = expected_generation
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(StoreError::InvalidState)?;
        let mut temp = tempfile::NamedTempFile::new_in(&self.directory)?;
        let mut writer = CappedWriter {
            file: temp.as_file_mut(),
            bytes: 0,
        };
        serde_json::to_writer(
            &mut writer,
            &EnvelopeRef {
                format_version: 2,
                generation,
                state,
            },
        )
        .map_err(|error| {
            if writer.bytes > MAX_WORKSPACE_STATE_BYTES {
                StoreError::InvalidState
            } else {
                StoreError::Io(io::Error::other(error))
            }
        })?;
        temp.as_file().sync_all()?;
        Ok((state_lock, generation, temp))
    }
}
struct CappedWriter<'a> {
    file: &'a mut File,
    bytes: usize,
}
impl Write for CappedWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.bytes = self.bytes.saturating_add(bytes.len());
        if self.bytes > MAX_WORKSPACE_STATE_BYTES {
            return Err(io::Error::other("workspace state byte budget"));
        }
        self.file.write_all(bytes)?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}
/// Reject deeply nested corrupt input before serde constructs a Value/tree.
fn bounded_depth(bytes: &[u8]) -> bool {
    let mut depth = 0usize;
    let mut string = false;
    let mut escaped = false;
    for byte in bytes {
        if string {
            if escaped {
                escaped = false;
            } else if *byte == b'\\' {
                escaped = true;
            } else if *byte == b'"' {
                string = false;
            }
            continue;
        }
        match byte {
            b'"' => string = true,
            b'{' | b'[' => {
                depth += 1;
                if depth > 64 {
                    return false;
                }
            }
            b'}' | b']' => {
                let Some(next) = depth.checked_sub(1) else {
                    return false;
                };
                depth = next;
            }
            _ => {}
        }
    }
    depth == 0 && !string
}

#[cfg(test)]
mod tests;
