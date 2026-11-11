//! Owner-only file primitives with two distinct guarantees.
//!
//! * [`create_exclusive`] publishes a secret that must never be overwritten
//!   (a device or signing key). It fails with `AlreadyExists` when the
//!   destination is present, so a racing creator can load the winner instead.
//! * [`replace_atomic`] replaces a mutable record. The new bytes are fully
//!   written and synced to a sibling file in the same directory before a
//!   rename swaps them in, so a failure before the swap keeps the old file.
//!
//! Permissions: on Unix new files are `0600` and `O_NOFOLLOW` applies. On
//! Windows files inherit the ACL of the (per-user) data directory; no explicit
//! ACL is set here, so callers that need verified ACLs must refuse to run.

use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

/// Failure of [`replace_atomic`]. `destination_may_have_changed` is `true` only
/// when the rename already happened and a later step (directory sync) failed:
/// the new content is in place but its durability is not confirmed, so the
/// caller must re-read rather than assume the old record is intact.
#[derive(Debug)]
pub(crate) struct ReplaceError {
    pub(crate) error: io::Error,
    // Only the Cloud record writers distinguish the two cases.
    #[cfg_attr(not(all(feature = "cloud-auth", feature = "http")), allow(dead_code))]
    pub(crate) destination_may_have_changed: bool,
}

impl ReplaceError {
    fn before_publish(error: io::Error) -> Self {
        Self {
            error,
            destination_may_have_changed: false,
        }
    }
}

fn random_suffix() -> io::Result<String> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|error| io::Error::other(error.to_string()))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn parent_of(path: &Path) -> io::Result<&Path> {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no parent directory"))
}

fn new_file_options() -> fs::OpenOptions {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    options
}

#[cfg(unix)]
fn sync_directory(directory: &Path) -> io::Result<()> {
    fs::File::open(directory)?.sync_all()
}

#[cfg(not(unix))]
fn sync_directory(_directory: &Path) -> io::Result<()> {
    // Directories cannot be opened for sync with std on Windows.
    Ok(())
}

/// Creates `path` with `bytes` and fails with `AlreadyExists` instead of
/// overwriting. The parent directory is created when missing.
#[cfg(all(feature = "cloud-auth", feature = "http"))]
pub(crate) fn create_exclusive(path: &Path, bytes: &[u8]) -> io::Result<()> {
    fs::create_dir_all(parent_of(path)?)?;
    let mut file = new_file_options().open(path)?;
    let written = file.write_all(bytes).and_then(|_| file.sync_all());
    if written.is_err() {
        // This call created the file, so it is the only one that may remove it.
        drop(file);
        let _ = fs::remove_file(path);
    }
    written
}

/// Atomically replaces (or creates) `path` with `bytes`.
///
/// Callers validate and serialise before calling, so nothing here can fail on
/// content. Only the temporary created by this call is ever removed.
pub(crate) fn replace_atomic(path: &Path, bytes: &[u8]) -> Result<(), ReplaceError> {
    let directory = parent_of(path).map_err(ReplaceError::before_publish)?;
    fs::create_dir_all(directory).map_err(ReplaceError::before_publish)?;
    let name = path
        .file_name()
        .ok_or_else(|| {
            ReplaceError::before_publish(io::Error::new(
                io::ErrorKind::InvalidInput,
                "path has no file name",
            ))
        })?
        .to_string_lossy();
    let temporary: PathBuf = directory.join(format!(
        ".{name}.{}.tmp",
        random_suffix().map_err(ReplaceError::before_publish)?
    ));
    let staged = (|| {
        let mut file = new_file_options().open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()
    })();
    if let Err(error) = staged {
        let _ = fs::remove_file(&temporary);
        return Err(ReplaceError::before_publish(error));
    }
    // `rename` replaces an existing destination on Unix and, in std, on Windows.
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(ReplaceError::before_publish(error));
    }
    sync_directory(directory).map_err(|error| ReplaceError {
        error,
        destination_may_have_changed: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "private-file-{name}-{}-{}",
            std::process::id(),
            random_suffix().unwrap()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn leftovers(dir: &Path) -> Vec<String> {
        fs::read_dir(dir)
            .unwrap()
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.ends_with(".tmp"))
            .collect()
    }

    #[cfg(all(feature = "cloud-auth", feature = "http"))]
    #[test]
    fn exclusive_creation_never_overwrites() {
        let dir = scratch("exclusive");
        let path = dir.join("nested").join("key.bin");
        create_exclusive(&path, b"first").unwrap();
        let error = create_exclusive(&path, b"second").unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read(&path).unwrap(), b"first");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn replacement_swaps_whole_content_and_leaves_no_temporaries() {
        let dir = scratch("replace");
        let path = dir.join("record.json");
        replace_atomic(&path, b"one").unwrap();
        replace_atomic(&path, b"two-longer").unwrap();
        replace_atomic(&path, b"3").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"3");
        assert!(leftovers(&dir).is_empty());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_failure_before_the_swap_keeps_the_previous_record() {
        let dir = scratch("keep");
        let path = dir.join("record.json");
        replace_atomic(&path, b"old").unwrap();
        // A directory in the way makes the rename fail after staging succeeded.
        let blocked = dir.join("blocked");
        fs::create_dir_all(blocked.join("inner")).unwrap();
        let error = replace_atomic(&blocked, b"new").unwrap_err();
        assert!(!error.destination_may_have_changed);
        assert!(blocked.join("inner").is_dir());
        assert!(
            leftovers(&dir).is_empty(),
            "this call's temporary is removed"
        );
        assert_eq!(fs::read(&path).unwrap(), b"old");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_failure_while_staging_leaves_the_destination_alone() {
        let dir = scratch("stage");
        let path = dir.join("record.json");
        replace_atomic(&path, b"old").unwrap();
        // The parent of the destination is a file, so nothing can be staged.
        let impossible = path.join("child.json");
        let error = replace_atomic(&impossible, b"new").unwrap_err();
        assert!(!error.destination_may_have_changed);
        assert_eq!(fs::read(&path).unwrap(), b"old");
        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn unrelated_temporaries_are_not_touched() {
        let dir = scratch("unrelated");
        let path = dir.join("record.json");
        let foreign = dir.join(".record.json.someone-else.tmp");
        fs::write(&foreign, b"theirs").unwrap();
        replace_atomic(&path, b"mine").unwrap();
        assert_eq!(fs::read(&foreign).unwrap(), b"theirs");
        let _ = fs::remove_dir_all(&dir);
    }
}
