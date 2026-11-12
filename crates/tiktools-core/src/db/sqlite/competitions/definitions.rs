//! Definition storage and atomic removal of generation-scoped participation.
use super::{DatabaseError, DatabaseManager};
use rusqlite::params;
use tiktools_competitions::{Definition, LifecycleState};

impl DatabaseManager {
    pub(crate) fn insert_competition_definition(
        &self,
        definition: &Definition,
    ) -> Result<(), DatabaseError> {
        definition
            .validate()
            .map_err(|_| DatabaseError::Invalid("invalid competition definition".into()))?;
        let mut connection = self.open(&self.automation_path())?;
        let tx = connection.transaction()?;
        tx.execute("INSERT INTO competition_definitions(id, ranking_id, schema_version, definition_json) VALUES (?1, ?2, ?3, ?4)", params![definition.id, definition.ranking_id, definition.schema_version, serde_json::to_string(definition)?])?;
        tx.execute(
            "INSERT INTO competition_state(competition_id, state_json) VALUES (?1, ?2)",
            params![
                definition.id,
                serde_json::to_string(&LifecycleState::default())?
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub(crate) fn load_competition_definitions(&self) -> Result<Vec<Definition>, DatabaseError> {
        let connection = self.open(&self.automation_path())?;
        let mut statement = connection
            .prepare("SELECT definition_json FROM competition_definitions ORDER BY id")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        rows.map(|row| Ok(serde_json::from_str(&row?)?)).collect()
    }

    pub(crate) fn update_competition_definition(
        &self,
        definition: &Definition,
    ) -> Result<bool, DatabaseError> {
        definition
            .validate()
            .map_err(|_| DatabaseError::Invalid("invalid competition definition".into()))?;
        let connection = self.open(&self.automation_path())?;
        Ok(connection.execute("UPDATE competition_definitions SET ranking_id=?2, schema_version=?3, definition_json=?4 WHERE id=?1 AND EXISTS (SELECT 1 FROM competition_state WHERE competition_id=?1 AND json_extract(state_json, '$.lifecycle')='draft') AND NOT EXISTS (SELECT 1 FROM competition_results WHERE competition_id=?1)", params![definition.id, definition.ranking_id, definition.schema_version, serde_json::to_string(definition)?])? == 1)
    }

    pub(crate) fn delete_competition_definition(&self, id: &str) -> Result<bool, DatabaseError> {
        let mut connection = self.open(&self.automation_path())?;
        let tx = connection.transaction()?;
        let deleted = tx.execute("DELETE FROM competition_definitions WHERE id=?1 AND EXISTS (SELECT 1 FROM competition_state WHERE competition_id=?1 AND json_extract(state_json, '$.lifecycle') IN ('draft', 'cancelled')) AND NOT EXISTS (SELECT 1 FROM competition_results WHERE competition_id=?1)", [id])? == 1;
        if deleted {
            // Cancelled drafts may contain participation. Remove all scoped
            // state atomically so recreating an ID cannot inherit old votes.
            for table in [
                "competition_contributions",
                "competition_scored_contributions",
                "competition_gift_receipts",
                "competition_gift_participants",
                "competition_gift_combo_prices",
                "competition_gift_combos",
                "competition_gift_combo_counts",
                "competition_participants",
                "competition_participation_counts",
                "competition_integrity_checkpoints",
                "competition_integrity_owners",
                "competition_execution_subjects",
                "competition_execution_generations",
                "competition_state",
            ] {
                tx.execute(
                    &format!("DELETE FROM {table} WHERE competition_id=?1"),
                    [id],
                )?;
            }
        }
        tx.commit()?;
        Ok(deleted)
    }
}
