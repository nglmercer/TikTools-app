use super::super::sqlite::DatabaseError;
use super::super::DatabaseManager;
use super::ANALYTICS_SCHEMA;

impl DatabaseManager {
    pub(in crate::db) fn ensure_analytics_schema(&self) -> Result<(), DatabaseError> {
        let connection = self.open(&self.analytics_path())?;
        connection.execute_batch(ANALYTICS_SCHEMA)?;
        Ok(())
    }
}
