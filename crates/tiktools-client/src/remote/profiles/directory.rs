use super::*;
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::Path,
    sync::Arc,
};
pub(super) struct Lease {
    directory: PathBuf,
    path: PathBuf,
    _lock: Arc<File>,
    pub profile: Option<RemoteProfile>,
}
impl Lease {
    pub fn open(directory: PathBuf, name: &str, create: bool) -> Result<Self, ClientError> {
        if !valid_name(name) {
            return Err(fail(
                "invalid_params",
                "profile name must use 1..64 ASCII letters, digits, underscores or hyphens",
            ));
        }
        check_directory(&directory, create)?;
        let lock = directory.join(format!("{name}.lock"));
        if fs::symlink_metadata(&lock).is_ok() {
            check_file(&lock)?;
        }
        let mut options = OpenOptions::new();
        options.create(true).truncate(false).read(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options.open(&lock).map_err(|_| profile_error())?;
        check_file(&lock)?;
        fs2::FileExt::try_lock_exclusive(&file).map_err(|_| {
            fail(
                "profile_in_use",
                "another process is opening or changing this remote profile",
            )
        })?;
        let path = directory.join(format!("{name}.json"));
        let profile = read(&path)?;
        if !create && profile.is_none() {
            return Err(fail("profile_not_found", "remote profile does not exist"));
        }
        if profile.as_ref().is_some_and(|p| p.name != name) {
            return Err(fail(
                "profile_invalid",
                "profile filename binding is invalid",
            ));
        }
        Ok(Self {
            directory,
            path,
            _lock: Arc::new(file),
            profile,
        })
    }
    pub fn clone_for_write(&self) -> Self {
        Self {
            directory: self.directory.clone(),
            path: self.path.clone(),
            _lock: self._lock.clone(),
            profile: self.profile.clone(),
        }
    }
    pub fn write(&self, profile: &RemoteProfile) -> Result<(), ClientError> {
        profile.validate()?;
        check_directory(&self.directory, false)?;
        if fs::symlink_metadata(&self.path).is_ok() {
            check_file(&self.path)?;
        }
        let catalog = self.directory.join(".catalog.lock");
        if fs::symlink_metadata(&catalog).is_ok() {
            check_file(&catalog)?;
        }
        let mut options = OpenOptions::new();
        options.create(true).truncate(false).read(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let catalog = options.open(&catalog).map_err(|_| profile_error())?;
        fs2::FileExt::try_lock_exclusive(&catalog).map_err(|_| {
            fail(
                "profile_in_use",
                "another process is changing the profile catalog",
            )
        })?;
        if self.profile.is_none() {
            let existing = list(&self.directory)?;
            if existing.iter().any(|p| {
                p.profile_id == profile.profile_id
                    || (p.origin == profile.origin
                        && p.host_id == profile.host_id
                        && p.session_id == profile.session_id
                        && p.principal_id == profile.principal_id)
            }) {
                return Err(fail("profile_session_exists","this approved session already belongs to a stored profile; pair a new device session explicitly"));
            }
            if existing.len() >= 32 {
                return Err(fail("profile_limit", "remote profile limit reached"));
            }
        }
        if profile.name != self.path.file_stem().and_then(|v| v.to_str()).unwrap_or("") {
            return Err(fail(
                "profile_invalid",
                "profile filename binding is invalid",
            ));
        }
        let mut temporary =
            tempfile::NamedTempFile::new_in(&self.directory).map_err(|_| profile_error())?;
        serde_json::to_writer(&mut temporary, profile).map_err(|_| profile_error())?;
        temporary.flush().map_err(|_| profile_error())?;
        temporary
            .as_file()
            .sync_all()
            .map_err(|_| profile_error())?;
        temporary.persist(&self.path).map_err(|_| profile_error())?;
        sync_directory(&self.directory)?;
        Ok(())
    }
    pub fn remove(&self) -> Result<(), ClientError> {
        check_file(&self.path)?;
        fs::remove_file(&self.path).map_err(|_| profile_error())?;
        sync_directory(&self.directory)
    }
}
pub(super) fn sync_directory(path: &Path) -> Result<(), ClientError> {
    #[cfg(unix)]
    {
        File::open(path)
            .and_then(|file| file.sync_all())
            .map_err(|_| profile_error())?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}
pub(super) fn private(metadata: &fs::Metadata) -> Result<(), ClientError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        // SAFETY: geteuid has no arguments, does not mutate memory and cannot fail.
        if metadata.permissions().mode() & 0o077 != 0
            || metadata.uid() != unsafe { libc::geteuid() }
        {
            return Err(fail(
                "profile_invalid",
                "remote profile paths must be private to the current user",
            ));
        }
    }
    #[cfg(not(unix))]
    let _ = metadata;
    Ok(())
}
pub(super) fn check_directory(path: &Path, create: bool) -> Result<(), ClientError> {
    if !path.exists() && create {
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(path).map_err(|_| profile_error())?;
    }
    let metadata = fs::symlink_metadata(path).map_err(|_| {
        fail(
            "profile_not_found",
            "remote profile directory does not exist",
        )
    })?;
    if !metadata.is_dir() {
        return Err(fail(
            "profile_invalid",
            "profile directory must not be a symlink",
        ));
    }
    private(&metadata)
}
fn check_file(path: &Path) -> Result<(), ClientError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| profile_error())?;
    if !metadata.is_file() {
        return Err(fail(
            "profile_invalid",
            "profile files must be regular files",
        ));
    }
    private(&metadata)
}
fn read(path: &Path) -> Result<Option<RemoteProfile>, ClientError> {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(profile_error()),
        Ok(_) => {}
    }
    check_file(path)?;
    let file = File::open(path).map_err(|_| profile_error())?;
    let mut bytes = Vec::new();
    file.take(64 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| profile_error())?;
    if bytes.len() > 64 * 1024 {
        return Err(fail(
            "profile_invalid",
            "remote profile exceeds its size limit",
        ));
    }
    let profile: RemoteProfile = serde_json::from_slice(&bytes)
        .map_err(|_| fail("profile_invalid", "remote profile is invalid"))?;
    profile.validate()?;
    Ok(Some(profile))
}
pub(super) fn list(directory: &Path) -> Result<Vec<RemoteProfile>, ClientError> {
    if !directory.exists() {
        return Ok(Vec::new());
    }
    check_directory(directory, false)?;
    let mut profiles = Vec::new();
    for entry in fs::read_dir(directory).map_err(|_| profile_error())? {
        let entry = entry.map_err(|_| profile_error())?;
        if entry.path().extension().and_then(|v| v.to_str()) != Some("json") {
            continue;
        }
        if profiles.len() >= 32 {
            return Err(fail(
                "profile_invalid",
                "remote profile inventory exceeds its limit",
            ));
        }
        if let Some(profile) = read(&entry.path())? {
            if entry.path().file_stem().and_then(|v| v.to_str()) != Some(profile.name.as_str()) {
                return Err(fail(
                    "profile_invalid",
                    "profile filename binding is invalid",
                ));
            }
            profiles.push(profile);
        }
    }
    profiles.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(profiles)
}
