//! Bounded consent-file reads and durable marker/preference replacement.
use crate::SyncConfig;
use crate::{SyncCoordError, SyncCoordinator};
use std::path::{Path, PathBuf};

const CONSENT_MARKER_FILE: &str = "cloud-sync-consent-initialized";
const CONSENT_MARKER: &[u8] = b"tiktools-sync-consent-initialized-v1\n";

fn read_regular_file(
    path: &Path,
    max: usize,
) -> Result<(Vec<u8>, std::fs::Metadata), SyncCoordError> {
    use std::io::Read;
    let invalid = || SyncCoordError::Io("sync preference file is unreadable".into());
    let before = std::fs::symlink_metadata(path).map_err(|_| invalid())?;
    if !before.file_type().is_file() {
        return Err(invalid());
    }
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options.open(path).map_err(|_| invalid())?;
    let opened = file.metadata().map_err(|_| invalid())?;
    if !opened.file_type().is_file() || opened.len() > max as u64 {
        return Err(invalid());
    }
    let mut bytes = Vec::new();
    file.take(max as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| invalid())?;
    if bytes.len() > max {
        return Err(invalid());
    }
    Ok((bytes, opened))
}

impl SyncCoordinator {
    pub(crate) fn consent_marker_path(&self) -> PathBuf {
        self.config_path.with_file_name(CONSENT_MARKER_FILE)
    }

    pub(crate) fn consent_marker_present(&self) -> Result<bool, SyncCoordError> {
        let path = self.consent_marker_path();
        let metadata = match std::fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(SyncCoordError::Io(error.to_string())),
        };
        if !metadata.file_type().is_file() || metadata.len() != CONSENT_MARKER.len() as u64 {
            return Err(SyncCoordError::Io("invalid sync consent marker".into()));
        }
        let (bytes, opened) = read_regular_file(&path, CONSENT_MARKER.len())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if opened.permissions().mode() & 0o077 != 0 {
                return Err(SyncCoordError::Io("invalid sync consent marker".into()));
            }
        }
        #[cfg(not(unix))]
        let _ = opened;
        if bytes != CONSENT_MARKER {
            return Err(SyncCoordError::Io("invalid sync consent marker".into()));
        }
        Ok(true)
    }

    /// Caller holds the exclusive protocol lease, or shared protocol plus
    /// admission. The marker is installed and directory-synced before a
    /// preference is written or a legacy preference is used for mutation.
    pub(crate) fn ensure_consent_marker(&self) -> Result<(), SyncCoordError> {
        let dir = self
            .config_path
            .parent()
            .ok_or_else(|| SyncCoordError::Io("invalid config directory".into()))?;
        if !self.consent_marker_present()? {
            let mut temp = tempfile::NamedTempFile::new_in(dir)
                .map_err(|error| SyncCoordError::Io(error.to_string()))?;
            use std::io::Write;
            temp.write_all(CONSENT_MARKER)
                .map_err(|error| SyncCoordError::Io(error.to_string()))?;
            temp.as_file()
                .sync_all()
                .map_err(|error| SyncCoordError::Io(error.to_string()))?;
            if let Err(error) = temp.persist_noclobber(self.consent_marker_path()) {
                if error.error.kind() != std::io::ErrorKind::AlreadyExists
                    || !self.consent_marker_present()?
                {
                    return Err(SyncCoordError::Io(error.error.to_string()));
                }
            }
        }
        #[cfg(unix)]
        std::fs::File::open(dir)
            .and_then(|dir| dir.sync_all())
            .map_err(|error| SyncCoordError::Io(error.to_string()))?;
        Ok(())
    }

    /// Read only a bounded regular file. On Unix the open itself rejects
    /// symlinks and is nonblocking, including if a FIFO replaces the file.
    pub(crate) fn read_config_bytes(&self) -> Result<Vec<u8>, SyncCoordError> {
        Ok(read_regular_file(&self.config_path, 4096)?.0)
    }

    pub(crate) fn read_config(&self) -> SyncConfig {
        let mut config = self
            .read_config_bytes()
            .ok()
            .and_then(|bytes| serde_json::from_slice::<SyncConfig>(&bytes).ok())
            .filter(SyncConfig::valid)
            .unwrap_or(SyncConfig {
                version: 1,
                enabled: false,
                generation: 0,
                scope: None,
            });
        // Files predating consent generations do not establish an identity.
        // Keep their state intact, but require explicit opt-in before sync.
        if config.generation == 0 {
            config.enabled = false;
        }
        config
    }

    pub(crate) fn write_config(&self, config: &SyncConfig) -> Result<(), SyncCoordError> {
        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| SyncCoordError::Io(e.to_string()))?;
        }
        let bytes = serde_json::to_vec(config).map_err(|e| SyncCoordError::Io(e.to_string()))?;
        let dir = self
            .config_path
            .parent()
            .ok_or_else(|| SyncCoordError::Io("invalid config directory".into()))?;
        let mut temp =
            tempfile::NamedTempFile::new_in(dir).map_err(|e| SyncCoordError::Io(e.to_string()))?;
        use std::io::Write;
        temp.write_all(&bytes)
            .map_err(|e| SyncCoordError::Io(e.to_string()))?;
        temp.as_file()
            .sync_all()
            .map_err(|e| SyncCoordError::Io(e.to_string()))?;
        temp.persist(&self.config_path)
            .map_err(|e| SyncCoordError::Io(e.error.to_string()))?;
        #[cfg(unix)]
        std::fs::File::open(dir)
            .and_then(|dir| dir.sync_all())
            .map_err(|e| SyncCoordError::Io(e.to_string()))?;
        Ok(())
    }

    /// The caller must hold exclusive protocol admission and any required
    /// current-session fence. Every explicit decision gets a new identity.
    pub(crate) fn write_new_consent(
        &self,
        enabled: bool,
        previous: u128,
        scope: Option<[u8; 32]>,
    ) -> Result<(), SyncCoordError> {
        self.ensure_consent_marker()?;
        // The high bit keeps new identities outside legacy u64 generations.
        let mut bytes = [0_u8; 16];
        getrandom::fill(&mut bytes)
            .map_err(|_| SyncCoordError::Io("cannot create sync consent identity".into()))?;
        bytes[0] |= 0x80;
        let generation = u128::from_be_bytes(bytes);
        if generation == previous {
            return Err(SyncCoordError::Io("sync consent identity collision".into()));
        }
        self.write_config(&SyncConfig {
            version: if scope.is_some() { 2 } else { 1 },
            enabled,
            generation,
            scope,
        })
    }
}
