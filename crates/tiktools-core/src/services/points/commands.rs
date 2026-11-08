use super::*;
use crate::{
    collaboration::{AdditiveCommandRequest, Versioned},
    control::PointsAdjustment,
    db::{CommandTransaction, DatabaseError},
};

// The LIVE points worker has a bounded ingress queue. A moderator command may
// briefly serialize with awards, but must reject promptly when storage lags.
const COMMAND_ADMISSION: Duration = Duration::from_millis(100);

impl PointsService {
    pub(crate) fn execute_command(
        &self,
        operation: &PointsAdjustment,
        command: &CommandTransaction<AdditiveCommandRequest>,
    ) -> Result<(Versioned<Value>, bool), DatabaseError> {
        let deadline = Instant::now() + COMMAND_ADMISSION;
        let _mutation = loop {
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
        #[cfg(test)]
        let after_admission =
            recover_mutex(&self.after_command_admission, "points command test barrier").take();
        let (result, viewer) = database.execute_points_command(
            operation,
            &self.config(),
            command,
            remaining,
            #[cfg(test)]
            after_admission,
        )?;
        let fresh = viewer.is_some();
        if let Some(viewer) = viewer {
            let resolved = self.resolved_seq();
            let mut inner = recover_rwlock_write(&self.inner, "points viewers");
            inner.pending.remove(&viewer.unique_id);
            inner.viewers.insert(viewer.unique_id.clone(), viewer);
            inner.evict_if_over(resolved);
        }
        Ok((result, fresh))
    }
}
