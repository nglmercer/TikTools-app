//! Restart checkpoints and ownership of the current observation run.
use super::{DatabaseError, DatabaseManager};
use rusqlite::{params, OptionalExtension};

impl DatabaseManager {
    /// Atomically consume a clean shutdown checkpoint and mark this run open.
    /// Missing checkpoints (including older schemas) fail closed.
    pub(crate) fn begin_competition_recovery(
        &self,
        id: &str,
        generation: u64,
        owner: &str,
    ) -> Result<bool, DatabaseError> {
        let generation = i64::try_from(generation)
            .map_err(|_| DatabaseError::Invalid("invalid generation".into()))?;
        let mut connection = self.open(&self.automation_path())?;
        let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let status: Option<String> = tx.query_row("SELECT status FROM competition_integrity_checkpoints WHERE competition_id=?1 AND generation=?2", params![id,generation], |row| row.get(0)).optional()?;
        tx.execute("INSERT INTO competition_integrity_checkpoints(competition_id,generation,status) VALUES (?1,?2,'open') ON CONFLICT(competition_id,generation) DO UPDATE SET status='open'", params![id,generation])?;
        tx.execute("INSERT INTO competition_integrity_owners(competition_id,generation,owner) VALUES (?1,?2,?3) ON CONFLICT(competition_id,generation) DO UPDATE SET owner=excluded.owner", params![id,generation,owner])?;
        tx.commit()?;
        Ok(status.as_deref() == Some("clean"))
    }

    /// Called only after ingress drains; never turn an already degraded
    /// in-memory ranking into a clean restart checkpoint.
    pub(crate) fn save_competition_integrity(
        &self,
        checkpoints: &[(String, u64, bool)],
        owner: &str,
    ) -> Result<(), DatabaseError> {
        let mut connection = self.open(&self.automation_path())?;
        let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        for (id, generation, clean) in checkpoints {
            let generation = i64::try_from(*generation)
                .map_err(|_| DatabaseError::Invalid("invalid generation".into()))?;
            let status = if *clean { "clean" } else { "degraded" };
            let changed = tx.execute("UPDATE competition_integrity_checkpoints SET status=?3 WHERE competition_id=?1 AND generation=?2 AND status='open' AND EXISTS (SELECT 1 FROM competition_integrity_owners o WHERE o.competition_id=?1 AND o.generation=?2 AND o.owner=?4)", params![id,generation,status,owner])?;
            if changed != 1 {
                return Err(DatabaseError::Invalid(
                    "competition integrity checkpoint missing".into(),
                ));
            }
        }
        tx.commit()?;
        Ok(())
    }
}
