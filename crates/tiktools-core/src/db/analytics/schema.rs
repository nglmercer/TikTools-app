use super::super::sqlite::DatabaseError;
use super::super::DatabaseManager;
use super::ANALYTICS_SCHEMA;

impl DatabaseManager {
    pub(in crate::db) fn ensure_analytics_schema(&self) -> Result<(), DatabaseError> {
        let connection = self.open(&self.analytics_path())?;
        if !crate::db::sqlite::schema::schema_unchanged(&connection)? {
            connection.execute_batch(ANALYTICS_SCHEMA)?;
            crate::db::sqlite::schema::stamp_schema_version(&connection)?;
        }
        Ok(())
    }
}
