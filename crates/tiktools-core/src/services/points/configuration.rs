use super::*;

impl PointsService {
    pub fn config(&self) -> PointsConfig {
        recover_rwlock_read(&self.config, "points config").clone()
    }

    pub fn update_config(&self, update: PartialPointsConfig) -> PointsConfig {
        let _mutation = recover_mutex(&self.mutation, "points mutation");
        let updated = {
            let mut config = recover_rwlock_write(&self.config, "points config");
            update.apply(&mut config);
            config.normalize();
            config.clone()
        };
        #[cfg(feature = "persistence")]
        if let Some(database) = &self.database {
            if let Err(error) = database.save_points_config(&updated) {
                tracing::warn!(%error, "could not persist points configuration");
            }
        }
        updated
    }
}
