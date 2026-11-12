//! Private regular-file checks, bounded serialization and lock-file opens.
use super::ProtocolFence;
use super::ProtocolFenceError;
use cloud_client::StoreError;
use serde::Serialize;
use std::{
    fs::{self, File, OpenOptions},
    io,
    path::Path,
};

pub(super) fn marker_bytes<T: Serialize>(
    record: &T,
    max: usize,
) -> Result<Vec<u8>, ProtocolFenceError> {
    let bytes = serde_json::to_vec(record).map_err(|_| StoreError::InvalidState)?;
    if bytes.len() > max {
        return Err(StoreError::InvalidState.into());
    }
    Ok(bytes)
}

pub(super) fn checked_file(path: &Path) -> Result<bool, ProtocolFenceError> {
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

pub(super) fn sync_dir(dir: &Path) -> Result<(), ProtocolFenceError> {
    #[cfg(unix)]
    File::open(dir)?.sync_all()?;
    #[cfg(not(unix))]
    let _ = dir;
    Ok(())
}

impl ProtocolFence {
    pub(super) fn lock_file_named(&self, name: &str) -> Result<File, ProtocolFenceError> {
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

    pub(super) fn lock_file(&self) -> Result<File, ProtocolFenceError> {
        self.lock_file_named("protocol-operation.lock")
    }
}
