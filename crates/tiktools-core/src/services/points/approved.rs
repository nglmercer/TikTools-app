use super::*;
use crate::db::DatabaseError;
use crate::services::effects::ApprovedPointsKey;

// Approved-effect writes share the moderator command budget: they may queue
// behind awards, but must reject promptly when storage lags.
const APPROVED_ADMISSION: Duration = Duration::from_millis(100);

impl PointsService {
    /// Drain earlier optimistic writes while holding the same mutation gate as
    /// LIVE awards/reset/commands, then commit under final trusted admission.
    pub(crate) fn adjust_approved(
        &self,
        key: &ApprovedPointsKey,
        unique_id: &str,
        delta: f64,
        admit: impl FnOnce(
            f64,
            &mut dyn FnMut() -> Result<(), DatabaseError>,
        ) -> Result<(), DatabaseError>,
    ) -> Result<Option<PointAward>, DatabaseError> {
        if !valid_viewer_id(unique_id) || !delta.is_finite() || delta.abs() > 1_000_000_000.0 {
            return Err(DatabaseError::Invalid(
                "invalid approved points adjustment".into(),
            ));
        }
        let admission = self.admit_writer(APPROVED_ADMISSION)?;
        let (award, viewer) = admission.database.adjust_approved_points(
            key,
            unique_id,
            delta,
            &self.config(),
            admission.remaining,
            admit,
        )?;
        // A retained identity describes an old commit. In particular, never
        // overwrite a reset or a later LIVE award with its historical balance.
        if let Some(viewer) = viewer {
            self.remember_viewer(unique_id, viewer);
        }
        Ok(award)
    }
}
