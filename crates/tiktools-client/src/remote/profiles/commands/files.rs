//! Private file access, bounded legacy decoding and atomic journal persistence.
use super::*;

pub(super) fn private_file(file: &File) -> Result<(), ClientError> {
    let metadata = file.metadata().map_err(|_| unavailable())?;
    if !metadata.is_file() {
        return Err(invalid());
    }
    directory::private(&metadata).map_err(|_| invalid())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.nlink() != 1 {
            return Err(invalid());
        }
    }
    Ok(())
}

pub(super) fn options() -> OpenOptions {
    let mut options = OpenOptions::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    }
    options
}

pub(super) fn read(path: &Path) -> Result<Option<Data>, ClientError> {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(unavailable()),
        Ok(m) if !m.is_file() => return Err(invalid()),
        _ => {}
    }
    let file = options().read(true).open(path).map_err(|_| invalid())?;
    private_file(&file)?;
    let mut bytes = Vec::new();
    file.take(BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| unavailable())?;
    if bytes.len() as u64 > BYTES {
        return Err(invalid());
    }
    let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|_| invalid())?;
    if value
        .get("version")
        .and_then(serde_json::Value::as_u64)
        .is_some_and(|v| v < 4)
        && (value
            .get("pending")
            .filter(|v| !v.is_null())
            .is_some_and(|i| i.get("proposalRevision").is_some())
            || value
                .get("recent")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|rs| {
                    rs.iter().any(|r| {
                        r.get("intent")
                            .is_some_and(|i| i.get("proposalRevision").is_some())
                    })
                }))
    {
        return Err(invalid());
    }
    if value.get("version").and_then(serde_json::Value::as_u64) == Some(1) {
        let old_intent = |i: &serde_json::Value| {
            i.get("delta").is_none()
                && i.get("currencyName").is_none()
                && i.get("expectedRevision")
                    .and_then(serde_json::Value::as_str)
                    .is_some()
                && matches!(
                    i.get("operation").and_then(serde_json::Value::as_str),
                    Some("disable" | "delete")
                )
        };
        if value
            .get("pending")
            .filter(|v| !v.is_null())
            .is_some_and(|i| !old_intent(i))
            || value
                .get("recent")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|rows| {
                    rows.iter().any(|r| {
                        r.get("pointsResult").is_some()
                            || r.get("intent").is_none_or(|i| !old_intent(i))
                    })
                })
        {
            return Err(invalid());
        }
    }
    serde_json::from_value(value)
        .map(Some)
        .map_err(|_| invalid())
}

impl Journal {
    pub(super) fn persist(&self, data: &Data) -> Result<(), ClientError> {
        directory::check_directory(&self.directory, false).map_err(|_| unavailable())?;
        if fs::symlink_metadata(&self.path).is_ok() {
            let file = options()
                .read(true)
                .open(&self.path)
                .map_err(|_| invalid())?;
            private_file(&file)?;
        }
        let bytes = serde_json::to_vec(data).map_err(|_| invalid())?;
        if bytes.len() as u64 > BYTES {
            return Err(invalid());
        }
        let mut temp =
            tempfile::NamedTempFile::new_in(&self.directory).map_err(|_| unavailable())?;
        private_file(temp.as_file())?;
        temp.write_all(&bytes)
            .and_then(|_| temp.flush())
            .and_then(|_| temp.as_file().sync_all())
            .map_err(|_| unavailable())?;
        temp.persist(&self.path).map_err(|_| unavailable())?;
        #[cfg(test)]
        migration_stage("renamed");
        directory::sync_directory(&self.directory).map_err(|_| unavailable())
    }
}
