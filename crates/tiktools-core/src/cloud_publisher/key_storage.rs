use super::*;

pub(super) fn load_or_create_signing_key(
    path: &std::path::Path,
    id: KeyId,
    publisher: &str,
) -> Result<SigningKey, String> {
    use std::io::Read;
    let parent = path.parent().ok_or("missing key directory")?;
    std::fs::create_dir_all(parent).map_err(|_| "could not create key directory")?;
    let metadata =
        std::fs::symlink_metadata(parent).map_err(|_| "could not inspect key directory")?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err("key directory must be a regular directory".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))
            .map_err(|_| "could not protect key directory")?;
    }
    let load = || -> Result<SigningKey, String> {
        let mut options = std::fs::OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(libc::O_NOFOLLOW);
        }
        let file = options
            .open(path)
            .map_err(|_| "could not safely open existing key")?;
        let metadata = file
            .metadata()
            .map_err(|_| "could not inspect existing key")?;
        if !metadata.is_file() || metadata.len() > 16384 {
            return Err("existing key is not a small regular file".into());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o077 != 0 {
                return Err("existing private key must have mode 0600 or stricter".into());
            }
        }
        let mut bytes = Vec::new();
        file.take(16385)
            .read_to_end(&mut bytes)
            .map_err(|_| "could not read existing key")?;
        let key = SigningKey::from_json(&bytes).map_err(|_| "existing key is invalid")?;
        if key.key_id() != &id || key.publisher() != Some(publisher) {
            return Err(
                "existing key id or publisher does not match; choose a different key id".into(),
            );
        }
        Ok(key)
    };
    match std::fs::symlink_metadata(path) {
        Ok(_) => return load(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err("could not inspect existing key".into()),
    }
    let key =
        SigningKey::generate(id.clone(), Some(publisher.to_owned())).map_err(|e| e.to_string())?;
    match write_owner_only(path, &key.to_json()) {
        Ok(()) => Ok(key),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => load(),
        Err(_) => Err("could not persist private key".into()),
    }
}

/// Exclusive creation of a private key file. Windows has no verified ACL
/// handling here, so key creation fails closed there.
fn write_owner_only(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        crate::private_file::create_exclusive(path, bytes)
    }
    #[cfg(not(unix))]
    {
        let _ = (path, bytes);
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "private key ACL verification is not available on this platform",
        ))
    }
}
