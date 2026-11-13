use crate::{Error, SecretKey};
use std::{fs, io::Write, path::Path};

/// Stable identity in an owner-only regular file. Existing corrupt keys fail closed.
pub fn load_or_create_key(path: &Path) -> Result<SecretKey, Error> {
    if let Some(parent) = path.parent() {
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(parent)?;
        if !fs::symlink_metadata(parent)?.is_dir() {
            return Err(Error::Identity);
        }
    }
    if !path.try_exists()? {
        let key = SecretKey::generate();
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(path) {
            Ok(mut file) => {
                file.write_all(&key.to_bytes())?;
                file.sync_all()?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.into()),
        }
    }
    load_key(path)
}

/// Read an approved identity without ever creating a replacement key.
pub fn load_key(path: &Path) -> Result<SecretKey, Error> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.len() != 32 {
        return Err(Error::Identity);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(Error::Identity);
        }
    }
    let bytes: [u8; 32] = fs::read(path)?.try_into().map_err(|_| Error::Identity)?;
    Ok(SecretKey::from_bytes(&bytes))
}
