use super::*;
use crate::db::DatabaseError;

impl PointsService {
    /// Drain earlier optimistic writes while holding the same mutation gate as
    /// LIVE awards/reset/commands, then commit under final trusted admission.
    pub(crate) fn adjust_approved(
        &self,
        unique_id: &str,
        delta: f64,
        admit: impl FnOnce(
            f64,
            &mut dyn FnMut() -> Result<(), DatabaseError>,
        ) -> Result<(), DatabaseError>,
    ) -> Result<PointAward, DatabaseError> {
        if !valid_viewer_id(unique_id) || !delta.is_finite() || delta.abs() > 1_000_000_000.0 {
            return Err(DatabaseError::Invalid(
                "invalid approved points adjustment".into(),
            ));
        }
        let deadline = Instant::now() + Duration::from_millis(100);
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
        let (award, viewer) =
            database.adjust_approved_points(unique_id, delta, &self.config(), remaining, admit)?;
        let resolved = self.resolved_seq();
        let mut inner = recover_rwlock_write(&self.inner, "points viewers");
        inner.pending.remove(unique_id);
        inner.viewers.insert(unique_id.into(), viewer);
        inner.evict_if_over(resolved);
        Ok(award)
    }
}
