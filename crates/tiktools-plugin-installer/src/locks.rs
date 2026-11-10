use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io,
    path::Path,
    sync::{Arc, Mutex},
};

use fs2::FileExt;

use crate::InstallError;

/// A lock shared by every cooperating installer for the same plugin root.
/// Older binaries that do not take it are not fenced. Keep the file outside
/// `.staging`: cleanup must never unlink its own lock.
#[derive(Debug)]
pub(crate) struct PackageOperationLease(File);

impl Drop for PackageOperationLease {
    fn drop(&mut self) {
        // A child forked before close-on-exec can inherit this descriptor.
        // Explicit unlock prevents that child from extending our lease.
        let _ = FileExt::unlock(&self.0);
    }
}

pub(crate) fn try_package_operation(
    root: &Path,
    label: &str,
) -> Result<PackageOperationLease, InstallError> {
    fs::create_dir_all(root)?;
    let path = root.join(".tiktools-package-operations.lock");
    match fs::symlink_metadata(&path) {
        Ok(metadata) if !metadata.file_type().is_file() => {
            return Err(InstallError::io("unsafe plugin package lock file"));
        }
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    }
    let file = options.open(&path)?;
    let opened = file.metadata()?;
    let current = fs::symlink_metadata(&path)?;
    if !opened.is_file() || !current.file_type().is_file() {
        return Err(InstallError::io("unsafe plugin package lock file"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        if opened.dev() != current.dev()
            || opened.ino() != current.ino()
            || opened.permissions().mode() & 0o077 != 0
        {
            return Err(InstallError::io("unsafe plugin package lock file"));
        }
    }
    match FileExt::try_lock_exclusive(&file) {
        Ok(()) => Ok(PackageOperationLease(file)),
        Err(error) if error.kind() == io::ErrorKind::WouldBlock => Err(InstallError::busy(label)),
        Err(error) => Err(error.into()),
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct KeyedLocks {
    held: Arc<Mutex<BTreeSet<String>>>,
}

pub(crate) struct KeyedGuard {
    held: Arc<Mutex<BTreeSet<String>>>,
    key: String,
}

impl KeyedLocks {
    pub(crate) fn try_lock(&self, key: &str) -> Result<KeyedGuard, InstallError> {
        let mut held = self
            .held
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !held.insert(key.to_owned()) {
            return Err(InstallError::busy(key));
        }
        Ok(KeyedGuard {
            held: self.held.clone(),
            key: key.to_owned(),
        })
    }

    pub(crate) fn is_locked(&self, key: &str) -> bool {
        self.held
            .lock()
            .map(|held| held.contains(key))
            .unwrap_or(false)
    }
}

impl Drop for KeyedGuard {
    fn drop(&mut self) {
        if let Ok(mut held) = self.held.lock() {
            held.remove(&self.key);
        }
    }
}
