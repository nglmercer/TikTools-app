//! Private storage setup, schema initialization and exclusive host ownership.
use super::*;

impl CollaborationService {
    pub fn open(directory: &Path) -> Result<Self, Error> {
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(directory)?;
        let metadata = fs::symlink_metadata(directory)?;
        if !metadata.is_dir() {
            return Err(Error::Invalid(
                "collaboration directory must not be a symlink",
            ));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o077 != 0 {
                return Err(Error::Invalid("collaboration directory must be private"));
            }
        }
        let lock_path = directory.join("host.lock");
        if let Ok(metadata) = fs::symlink_metadata(&lock_path) {
            if !metadata.is_file() {
                return Err(Error::Invalid(
                    "collaboration ownership lock must be a regular file",
                ));
            }
        }
        let mut options = fs::OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let ownership = options.open(lock_path)?;
        fs2::FileExt::try_lock_exclusive(&ownership).map_err(|error| {
            if error.kind() == std::io::ErrorKind::WouldBlock
                || error.raw_os_error() == fs2::lock_contended_error().raw_os_error()
            {
                Error::HostInUse
            } else {
                Error::Io(error)
            }
        })?;
        let path = directory.join("collaboration.db");
        if let Ok(metadata) = fs::symlink_metadata(&path) {
            if !metadata.is_file() {
                return Err(Error::Invalid(
                    "collaboration database must be a regular file",
                ));
            }
        }
        let db = Connection::open(&path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
        }
        db.busy_timeout(Duration::from_secs(2))?;
        db.execute_batch(
            "PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;",
        )?;
        let version: i64 = db.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version > 1 {
            return Err(Error::Invalid("unsupported collaboration database version"));
        }
        db.execute_batch(include_str!("../schema.sql"))?;
        db.execute_batch("PRAGMA user_version=1;")?;
        prune_audit(&db)?;
        db.execute(
            "INSERT OR IGNORE INTO metadata(key,value) VALUES ('host-id',?1)",
            [Uuid::new_v4().to_string()],
        )?;
        db.execute(
            "INSERT OR IGNORE INTO metadata(key,value) VALUES ('configuration',?1)",
            [serde_json::to_string(&Configuration::default())?],
        )?;
        db.execute(
            "INSERT OR IGNORE INTO metadata(key,value) VALUES ('configuration-revision','1')",
            [],
        )?;
        // An interrupted exchange cannot collect credentials after a new host
        // process starts. Approved long-lived sessions remain independently valid.
        db.execute(
            "UPDATE pairing_requests SET state='expired' WHERE state IN ('pending','approved')",
            [],
        )?;
        let host_id = db.query_row(
            "SELECT value FROM metadata WHERE key='host-id'",
            [],
            |row| row.get(0),
        )?;
        Ok(Self {
            db: Mutex::new(db),
            host_id,
            instance_id: Uuid::new_v4().to_string(),
            authorizations: Mutex::new(BTreeMap::new()),
            gateway: Mutex::new(GatewayStatus::default()),
            _ownership: ownership,
        })
    }
}
