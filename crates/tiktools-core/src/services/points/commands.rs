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
        let admission = self.admit_writer(COMMAND_ADMISSION)?;
        #[cfg(test)]
        let after_admission =
            recover_mutex(&self.after_command_admission, "points command test barrier").take();
        let (result, viewer) = admission.database.execute_points_command(
            operation,
            &self.config(),
            command,
            admission.remaining,
            #[cfg(test)]
            after_admission,
        )?;
        let fresh = viewer.is_some();
        if let Some(viewer) = viewer {
            let key = viewer.unique_id.clone();
            self.remember_viewer(&key, viewer);
        }
        Ok((result, fresh))
    }
}
