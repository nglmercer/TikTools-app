//! Writer admission and viewer write-back shared by every points mutation:
//! LIVE commands, approved-effect adjustments and their durable drain.

use super::*;
use crate::db::DatabaseError;

/// Proof that the mutation gate is held and the writer has drained every
/// earlier admission. `remaining` is the budget left for the database call.
pub(super) struct WriterAdmission<'a> {
    pub(super) _gate: MutexGuard<'a, ()>,
    pub(super) database: &'a Arc<DatabaseManager>,
    pub(super) remaining: Duration,
}

impl PointsService {
    /// Takes the mutation gate, then waits (bounded by `budget`) until every
    /// write admitted so far is resolved and committed. The admission keeps
    /// the gate until the caller drops it.
    pub(super) fn admit_writer(
        &self,
        budget: Duration,
    ) -> Result<WriterAdmission<'_>, DatabaseError> {
        let deadline = Instant::now() + budget;
        let gate = loop {
            match self.mutation.try_lock() {
                Ok(guard) => break guard,
                Err(std::sync::TryLockError::Poisoned(_)) => {
                    return Err(DatabaseError::PointsWriterUnavailable)
                }
                Err(std::sync::TryLockError::WouldBlock) => {
                    if Instant::now() >= deadline {
                        return Err(DatabaseError::PointsWriterUnavailable);
                    }
                    std::thread::sleep(Duration::from_millis(1));
                }
            }
        };
        let database = self
            .database
            .as_ref()
            .ok_or(DatabaseError::PointsWriterUnavailable)?;
        let target = self.writer.admitted.load(Ordering::Acquire);
        loop {
            if self.writer.lock_queue().closed
                || self.writer.first_failed_seq.load(Ordering::Acquire) != 0
            {
                return Err(DatabaseError::PointsWriterUnavailable);
            }
            if self.writer.resolved_seq.load(Ordering::Acquire) >= target
                && self.writer.committed_seq.load(Ordering::Acquire) >= target
            {
                break;
            }
            if Instant::now() >= deadline {
                return Err(DatabaseError::PointsWriterUnavailable);
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(DatabaseError::PointsWriterUnavailable);
        }
        Ok(WriterAdmission {
            _gate: gate,
            database,
            remaining,
        })
    }

    /// Stores the viewer snapshot a successful write returned, under `key`.
    /// Callers must not use this for a retained identity that describes an
    /// old commit: it would overwrite a reset or a later LIVE award.
    pub(super) fn remember_viewer(&self, key: &str, viewer: ViewerState) {
        let resolved = self.resolved_seq();
        let mut inner = recover_rwlock_write(&self.inner, "points viewers");
        inner.pending.remove(key);
        inner.viewers.insert(key.into(), viewer);
        inner.evict_if_over(resolved);
    }
}
