//! Serialized pending intent and result transitions without command replay.
use super::*;

impl Journal {
    pub fn begin(&self, intent: Intent) -> Result<(), ClientError> {
        intent.validate()?;
        let _writing = self.writing.lock().map_err(|_| unavailable())?;
        if self.failed.load(Ordering::Acquire) {
            return Err(unavailable());
        }
        let mut next = self.data.read().map_err(|_| unavailable())?.clone();
        if next.pending.is_some() {
            return Err(fail(
                "command_pending",
                "resolve or explicitly review the existing command first",
            ));
        }
        if next
            .recent
            .iter()
            .any(|r| r.intent.command_id == intent.command_id)
        {
            return Err(invalid());
        }
        next.pending = Some(intent);
        if let Err(error) = self.persist(&next) {
            // A rename may have succeeded before directory sync failed. Never
            // dispatch after a partial persistence failure, or silently replace it.
            *self.data.write().map_err(|_| unavailable())? = next;
            self.failed.store(true, Ordering::Release);
            return Err(error);
        }
        *self.data.write().map_err(|_| unavailable())? = next;
        Ok(())
    }

    pub fn finish(&self, record: Record) -> Result<(), ClientError> {
        record.validate()?;
        let _writing = self.writing.lock().map_err(|_| unavailable())?;
        if self.failed.load(Ordering::Acquire) {
            return Err(unavailable());
        }
        let mut next = self.data.read().map_err(|_| unavailable())?.clone();
        if next.pending.as_ref() != Some(&record.intent) {
            return Err(fail("stale_view", "pending command changed"));
        }
        next.pending = None;
        next.recent.insert(0, record);
        next.recent.truncate(HISTORY);
        if let Err(error) = self.persist(&next) {
            self.failed.store(true, Ordering::Release);
            return Err(error);
        }
        *self.data.write().map_err(|_| unavailable())? = next;
        Ok(())
    }

    pub fn remove_resolved(&self) -> Result<(), ClientError> {
        let _writing = self.writing.lock().map_err(|_| unavailable())?;
        if self.view().pending.is_some() || self.failed.load(Ordering::Acquire) {
            return Err(fail(
                "command_pending",
                "review the pending command before forgetting this profile",
            ));
        }
        crate::remote::profiles::drafts::remove(&self.path.with_extension("drafts"))?;
        crate::remote::profiles::proposals::remove(&self.path.with_extension("proposals"))?;
        match fs::remove_file(&self.path) {
            Ok(()) => directory::sync_directory(&self.directory).map_err(|_| unavailable()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err(unavailable()),
        }
    }
}
