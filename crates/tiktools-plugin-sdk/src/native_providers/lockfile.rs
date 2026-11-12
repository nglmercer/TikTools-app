use super::*;

/// Read and validate a lockfile. Unknown schema versions fail closed:
/// an unreadable lockfile never degrades into an unpinned download.
pub fn read_lockfile(path: &Path) -> Result<NativeLibsLockfile, NativeProviderError> {
    let bytes = std::fs::read(path).map_err(|error| io_error(path, error))?;
    let lockfile: NativeLibsLockfile =
        serde_json::from_slice(&bytes).map_err(|error| NativeProviderError::Io {
            path: path.display().to_string(),
            message: format!("lockfile is not valid JSON: {error}"),
        })?;
    if lockfile.version != LOCKFILE_VERSION {
        return Err(NativeProviderError::LockVersion {
            path: path.display().to_string(),
            version: lockfile.version,
        });
    }
    Ok(lockfile)
}

/// Write a lockfile: pretty JSON with a trailing newline, deterministic
/// key order (`BTreeMap`), so diffs stay reviewable.
pub fn write_lockfile(
    path: &Path,
    lockfile: &NativeLibsLockfile,
) -> Result<(), NativeProviderError> {
    let mut text =
        serde_json::to_string_pretty(lockfile).map_err(|error| NativeProviderError::Io {
            path: path.display().to_string(),
            message: format!("lockfile is not serializable: {error}"),
        })?;
    text.push('\n');
    std::fs::write(path, text).map_err(|error| io_error(path, error))
}

pub(super) fn check_lock_matches(
    package: &str,
    declaration: &NativeLibDeclaration,
    locked: &LockedNativeLib,
) -> Result<(), NativeProviderError> {
    let mismatch = |message: &str| NativeProviderError::LockMismatch {
        package: package.to_owned(),
        message: message.to_owned(),
    };
    if locked.provider != declaration.provider {
        return Err(mismatch("provider changed; re-pin"));
    }
    if locked.version != declaration.version {
        return Err(mismatch("version changed; re-pin"));
    }
    if declaration.provider == NativeLibProvider::Github
        && (locked.repo.as_deref() != declaration.repo.as_deref()
            || locked.tag.as_deref() != declaration.tag.as_deref()
            || locked.binary.as_deref() != declaration.binary.as_deref())
    {
        return Err(mismatch("repo, tag, or binary changed; re-pin"));
    }
    Ok(())
}
