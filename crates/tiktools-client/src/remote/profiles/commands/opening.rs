//! Exclusive profile journal ownership, validation and durable version migration.
use super::*;

use super::files::{options, private_file, read};

impl Journal {
    /// Holds an independent process lease for the selected profile journal.
    pub fn open(root: &Path, profile: &RemoteProfile) -> Result<Self, ClientError> {
        profile.validate()?;
        directory::check_directory(root, false)?;
        let directory = root.join("commands");
        directory::check_directory(&directory, true)?;
        let lock_path = directory.join(format!("{}.lock", profile.profile_id));
        if fs::symlink_metadata(&lock_path).is_ok_and(|m| !m.is_file()) {
            return Err(invalid());
        }
        let lock = options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)
            .map_err(|_| invalid())?;
        private_file(&lock)?;
        fs2::FileExt::try_lock_exclusive(&lock).map_err(|_| {
            fail(
                "command_journal_in_use",
                "another controller is using this profile journal",
            )
        })?;
        let path = directory.join(format!("{}.journal", profile.profile_id));
        let binding = Binding::from(profile);
        let mut data = read(&path)?.unwrap_or_else(|| Data {
            version: VERSION,
            binding: binding.clone(),
            pending: None,
            recent: Vec::new(),
        });
        if !matches!(data.version, 1..=VERSION)
            || data.binding != binding
            || data.recent.len() > HISTORY
        {
            return Err(invalid());
        }
        if data
            .pending
            .iter()
            .chain(data.recent.iter().map(|r| &r.intent))
            .any(|i| !i.operation.supported_in(data.version))
        {
            return Err(invalid());
        }
        if let Some(intent) = &data.pending {
            intent.validate()?;
        }
        let mut ids = std::collections::BTreeSet::new();
        if let Some(intent) = &data.pending {
            ids.insert(&intent.command_id);
        }
        for record in &data.recent {
            record.validate()?;
            if !ids.insert(&record.intent.command_id) {
                return Err(invalid());
            }
        }
        let migrate = data.version != VERSION;
        data.version = VERSION;
        let journal = Self {
            directory,
            path,
            _lock: lock,
            data: RwLock::new(data),
            writing: Mutex::new(()),
            failed: AtomicBool::new(false),
        };
        if migrate {
            #[cfg(test)]
            migration_stage("before");
            journal.persist(&*journal.data.read().map_err(|_| unavailable())?)?;
            #[cfg(test)]
            migration_stage("after");
        }
        Ok(journal)
    }
}
