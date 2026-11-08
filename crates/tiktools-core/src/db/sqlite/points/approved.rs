//! Approved automation uses the ordered writer and a real durable commit.
//! The trusted admission callback holds saved authorization through `commit`.
use super::*;
use crate::services::PointAward;
use rusqlite::TransactionBehavior;

impl DatabaseManager {
    pub(crate) fn adjust_approved_points(
        &self,
        unique_id: &str,
        delta: f64,
        config: &PointsConfig,
        timeout: std::time::Duration,
        admit: impl FnOnce(
            f64,
            &mut dyn FnMut() -> Result<(), DatabaseError>,
        ) -> Result<(), DatabaseError>,
    ) -> Result<(PointAward, ViewerState), DatabaseError> {
        let mut db = self.open(&self.points_path())?;
        db.busy_timeout(timeout)?;
        db.execute_batch("PRAGMA foreign_keys=ON; PRAGMA synchronous=FULL;")?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut viewer =
            commands::bounded_viewer(&tx, unique_id, 16 * 1024)?.ok_or(DatabaseError::NotFound)?;
        // Match the existing core.points adjustment semantics (no subscriber
        // award bonus). Validate the actual rounded/clamped balance change too.
        let total = (((viewer.points + delta).max(0.0)) * 100.0).round() / 100.0;
        let effective = total - viewer.points;
        if !delta.is_finite() || !total.is_finite() || !effective.is_finite() {
            return Err(DatabaseError::Invalid(
                "invalid approved points arithmetic".into(),
            ));
        }
        viewer.points = total;
        viewer.level =
            ((total / config.points_per_level.max(10.0)).floor() as u32).saturating_add(1);
        viewer.last_seen = now();
        let op = PointsWriteOp::Adjust {
            seq: 0,
            unique_id: unique_id.into(),
            points: total,
            level: viewer.level,
        };
        let mut tx = Some(tx);
        admit(effective, &mut || {
            let tx = tx.take().ok_or_else(|| {
                DatabaseError::Invalid("approval admission already consumed".into())
            })?;
            apply_points_op_on(&tx, &op)?;
            versions::Reservations::new(&tx)?.reserve(&tx, &op)?;
            tx.commit()?;
            Ok(())
        })?;
        if tx.is_some() {
            return Err(DatabaseError::Invalid(
                "approval admission did not commit".into(),
            ));
        }
        Ok((
            PointAward {
                unique_id: unique_id.into(),
                delta,
                total_points: total,
                level: viewer.level,
                currency_name: config.currency_name.clone(),
            },
            viewer,
        ))
    }
}
