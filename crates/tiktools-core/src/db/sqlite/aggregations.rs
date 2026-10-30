use super::{DatabaseError, DatabaseManager};
use rusqlite::{params, OptionalExtension};
use tiktools_aggregation::engine::{Engine, SessionCheckpoint};
use tiktools_aggregation::model::Definition;

impl DatabaseManager {
    pub(crate) fn claim_aggregation_checkpoints(&self, owner: &str) -> Result<bool, DatabaseError> {
        let mut connection = self.open(&self.automation_path())?;
        let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let previous: Option<String> = tx
            .query_row(
                "SELECT status FROM aggregation_checkpoint_run WHERE singleton=1",
                [],
                |row| row.get(0),
            )
            .optional()?;
        tx.execute("INSERT INTO aggregation_checkpoint_owner(singleton,owner) VALUES (1,?1) ON CONFLICT(singleton) DO UPDATE SET owner=excluded.owner", [owner])?;
        tx.execute("INSERT INTO aggregation_checkpoint_run(singleton,status) VALUES (1,'open') ON CONFLICT(singleton) DO UPDATE SET status='open'", [])?;
        tx.commit()?;
        Ok(previous.as_deref() != Some("open"))
    }
    pub(crate) fn finish_aggregation_checkpoints(&self, owner: &str) -> Result<(), DatabaseError> {
        let connection = self.open(&self.automation_path())?;
        if connection.execute("UPDATE aggregation_checkpoint_run SET status='clean' WHERE singleton=1 AND EXISTS (SELECT 1 FROM aggregation_checkpoint_owner WHERE singleton=1 AND owner=?1)", [owner])? != 1 {
            return Err(DatabaseError::Invalid("aggregation checkpoint owner changed".into()));
        }
        Ok(())
    }
    /// Store a complete, validated checkpoint (runtime or clean shutdown) only
    /// for the exact definition currently in storage. The transaction fences edits.
    pub(crate) fn save_aggregation_checkpoint(
        &self,
        checkpoint: &SessionCheckpoint,
        owner: &str,
    ) -> Result<(), DatabaseError> {
        let mut validator = Engine::default();
        validator
            .create(checkpoint.definition.clone())
            .map_err(|_| DatabaseError::Invalid("invalid checkpoint definition".into()))?;
        validator
            .restore_session_checkpoint(checkpoint)
            .map_err(|_| DatabaseError::Invalid("invalid aggregation checkpoint".into()))?;
        let json = serde_json::to_string(checkpoint)?;
        if json.len() > 32 * 1024 * 1024 {
            return Err(DatabaseError::Invalid(
                "aggregation checkpoint too large".into(),
            ));
        }
        let mut connection = self.open(&self.automation_path())?;
        let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let current_owner: Option<String> = tx
            .query_row(
                "SELECT owner FROM aggregation_checkpoint_owner WHERE singleton=1",
                [],
                |row| row.get(0),
            )
            .optional()?;
        if current_owner.as_deref() != Some(owner) {
            return Err(DatabaseError::Invalid(
                "aggregation checkpoint owner changed".into(),
            ));
        }
        let stored: Option<String> = tx
            .query_row(
                "SELECT definition_json FROM aggregation_definitions WHERE id=?1",
                [&checkpoint.definition.id],
                |row| row.get(0),
            )
            .optional()?;
        let definition: Definition = serde_json::from_str(
            &stored.ok_or_else(|| DatabaseError::Invalid("ranking definition missing".into()))?,
        )?;
        if definition != checkpoint.definition {
            return Err(DatabaseError::Invalid("ranking definition changed".into()));
        }
        tx.execute("INSERT INTO aggregation_session_checkpoints(ranking_id,schema_version,checkpoint_json) VALUES (?1,1,?2) ON CONFLICT(ranking_id) DO UPDATE SET schema_version=excluded.schema_version,checkpoint_json=excluded.checkpoint_json", params![checkpoint.definition.id, json])?;
        tx.commit()?;
        Ok(())
    }

    /// Read the newest durable checkpoint without consuming it. A row is
    /// replaced by later runtime or shutdown writes; keeping it means a crash
    /// before the next write restores this older state (an undercount marked
    /// degraded), never an empty ranking. Clean vs unclean comes from the run row.
    pub(crate) fn load_aggregation_checkpoint(
        &self,
        id: &str,
        owner: &str,
    ) -> Result<Option<SessionCheckpoint>, DatabaseError> {
        let connection = self.open(&self.automation_path())?;
        let current_owner: Option<String> = connection
            .query_row(
                "SELECT owner FROM aggregation_checkpoint_owner WHERE singleton=1",
                [],
                |row| row.get(0),
            )
            .optional()?;
        if current_owner.as_deref() != Some(owner) {
            return Err(DatabaseError::Invalid(
                "aggregation checkpoint owner changed".into(),
            ));
        }
        let json: Option<String> = connection.query_row("SELECT checkpoint_json FROM aggregation_session_checkpoints WHERE ranking_id=?1 AND schema_version=1", [id], |row| row.get(0)).optional()?;
        Ok(json.map(|json| serde_json::from_str(&json)).transpose()?)
    }
    /// Drop a ranking's checkpoint before an in-memory reset so a crash
    /// afterwards cannot resurrect the pre-reset accounting.
    pub(crate) fn clear_aggregation_checkpoint(&self, id: &str) -> Result<(), DatabaseError> {
        let connection = self.open(&self.automation_path())?;
        connection.execute(
            "DELETE FROM aggregation_session_checkpoints WHERE ranking_id=?1",
            [id],
        )?;
        Ok(())
    }
    /// Record an external contribution identity. Returns false for a replay.
    /// Keys older than seven days are pruned opportunistically.
    pub(crate) fn admit_external_contribution(
        &self,
        event_id: &str,
        now: i64,
    ) -> Result<bool, DatabaseError> {
        let connection = self.open(&self.automation_path())?;
        let inserted = connection.execute(
            "INSERT OR IGNORE INTO aggregation_external_events(event_id,accepted_at) VALUES (?1,?2)",
            params![event_id, now],
        )? == 1;
        if inserted && now % 64 == 0 {
            connection.execute(
                "DELETE FROM aggregation_external_events WHERE accepted_at < ?1",
                [now.saturating_sub(7 * 24 * 60 * 60 * 1000)],
            )?;
        }
        Ok(inserted)
    }
    pub(crate) fn load_aggregation_definitions(&self) -> Result<Vec<Definition>, DatabaseError> {
        let connection = self.open(&self.automation_path())?;
        let mut statement = connection
            .prepare("SELECT definition_json FROM aggregation_definitions ORDER BY id")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        rows.map(|row| Ok(serde_json::from_str(&row?)?)).collect()
    }
    pub(crate) fn insert_aggregation_definition(
        &self,
        definition: &Definition,
    ) -> Result<(), DatabaseError> {
        let connection = self.open(&self.automation_path())?;
        connection.execute("INSERT INTO aggregation_definitions(id, creator_id, room_id, schema_version, definition_json) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![definition.id, definition.creator_id, definition.room_id, definition.schema_version, serde_json::to_string(definition)?])?;
        Ok(())
    }
    pub(crate) fn update_aggregation_definition(
        &self,
        definition: &Definition,
    ) -> Result<bool, DatabaseError> {
        let mut connection = self.open(&self.automation_path())?;
        let tx = connection.transaction()?;
        let updated = tx.execute("UPDATE aggregation_definitions SET creator_id=?2, room_id=?3, schema_version=?4, definition_json=?5 WHERE id=?1", params![definition.id, definition.creator_id, definition.room_id, definition.schema_version, serde_json::to_string(definition)?])? > 0;
        if updated {
            tx.execute(
                "DELETE FROM aggregation_session_checkpoints WHERE ranking_id=?1",
                [&definition.id],
            )?;
        }
        tx.commit()?;
        Ok(updated)
    }
    pub(crate) fn delete_aggregation_definition(&self, id: &str) -> Result<bool, DatabaseError> {
        let mut connection = self.open(&self.automation_path())?;
        let tx = connection.transaction()?;
        let deleted = tx.execute("DELETE FROM aggregation_definitions WHERE id = ?1", [id])? > 0;
        if deleted {
            tx.execute(
                "DELETE FROM aggregation_session_checkpoints WHERE ranking_id=?1",
                [id],
            )?;
        }
        tx.commit()?;
        Ok(deleted)
    }
}
