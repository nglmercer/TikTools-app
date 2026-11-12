//! Local lifecycle transitions, coordination revisions and deadline expiry.
use super::{DatabaseError, DatabaseManager};
use rusqlite::{params, OptionalExtension};
use tiktools_competitions::{Definition, Lifecycle, LifecycleState, Transition};

impl DatabaseManager {
    /// Read state and its monotonic coordination revision in one SQLite
    /// snapshot. A missing or deleted revision is an integrity failure.
    pub(crate) fn competition_state_revision(
        &self,
        id: &str,
    ) -> Result<Option<(LifecycleState, i64)>, DatabaseError> {
        let connection = self.open(&self.automation_path())?;
        let row: Option<(String, Option<i64>, Option<i64>)> = connection.query_row(
            "SELECT s.state_json,r.revision,r.deleted FROM competition_state s LEFT JOIN competition_control_revisions r ON r.competition_id=s.competition_id WHERE s.competition_id=?1",
            [id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).optional()?;
        row.map(|(json, revision, deleted)| {
            let revision = revision
                .filter(|n| *n > 0)
                .filter(|_| deleted == Some(0))
                .ok_or_else(|| DatabaseError::Invalid("competition revision missing".into()))?;
            Ok((serde_json::from_str(&json)?, revision))
        })
        .transpose()
    }

    pub(crate) fn load_competition_state(
        &self,
        id: &str,
    ) -> Result<Option<LifecycleState>, DatabaseError> {
        let connection = self.open(&self.automation_path())?;
        let state: Option<String> = connection
            .query_row(
                "SELECT state_json FROM competition_state WHERE competition_id=?1",
                [id],
                |row| row.get(0),
            )
            .optional()?;
        state
            .map(|json| serde_json::from_str(&json).map_err(DatabaseError::from))
            .transpose()
    }

    #[cfg(test)]
    pub(crate) fn transition_competition(
        &self,
        id: &str,
        transition: Transition,
        now: u64,
    ) -> Result<Option<LifecycleState>, DatabaseError> {
        self.transition_competition_owned(id, transition, now, None)
    }

    pub(crate) fn transition_competition_owned(
        &self,
        id: &str,
        transition: Transition,
        now: u64,
        owner: Option<&str>,
    ) -> Result<Option<LifecycleState>, DatabaseError> {
        let mut connection = self.open(&self.automation_path())?;
        let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let row: Option<(String, String)> = tx.query_row("SELECT d.definition_json, s.state_json FROM competition_definitions d JOIN competition_state s ON s.competition_id=d.id WHERE d.id=?1", [id], |row| Ok((row.get(0)?, row.get(1)?))).optional()?;
        let Some((definition, state)) = row else {
            return Ok(None);
        };
        let definition: Definition = serde_json::from_str(&definition)?;
        let mut state: LifecycleState = serde_json::from_str(&state)?;
        if !matches!(transition, Transition::Start) && state.lifecycle != Lifecycle::Draft {
            if let Some(owner) = owner {
                let current: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM competition_integrity_owners WHERE competition_id=?1 AND generation=?2 AND owner=?3)", params![id, state.generation as i64, owner], |row| row.get(0))?;
                if !current {
                    return Err(DatabaseError::Invalid("competition owner changed".into()));
                }
            }
        }
        state
            .transition(transition, now, definition.duration_ms)
            .map_err(|error| {
                DatabaseError::Invalid(format!("invalid lifecycle transition: {error:?}"))
            })?;
        tx.execute(
            "UPDATE competition_state SET state_json=?2 WHERE competition_id=?1",
            params![id, serde_json::to_string(&state)?],
        )?;
        if matches!(transition, Transition::Start) {
            let generation = i64::try_from(state.generation)
                .map_err(|_| DatabaseError::Invalid("invalid generation".into()))?;
            tx.execute("INSERT INTO competition_execution_generations(competition_id,generation,origin) VALUES (?1,?2,'owner')", params![id,generation])?;
            tx.execute("INSERT INTO competition_integrity_checkpoints(competition_id,generation,status) VALUES (?1,?2,'open')", params![id, generation])?;
            if let Some(owner) = owner {
                tx.execute("INSERT INTO competition_integrity_owners(competition_id,generation,owner) VALUES (?1,?2,?3)", params![id,generation,owner])?;
            }
        }
        tx.commit()?;
        Ok(Some(state))
    }

    pub(crate) fn finalizing_competitions(&self) -> Result<Vec<String>, DatabaseError> {
        let connection = self.open(&self.automation_path())?;
        let mut statement = connection.prepare("SELECT competition_id FROM competition_state WHERE json_extract(state_json,'$.lifecycle')='finalizing' ORDER BY competition_id LIMIT 129")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        let ids = rows.collect::<Result<Vec<_>, _>>()?;
        if ids.len() > 128 {
            return Err(DatabaseError::Invalid(
                "competition capacity exceeded".into(),
            ));
        }
        Ok(ids)
    }

    /// Atomically advance expired active competitions. A bounded number of
    /// definitions is allowed by the host, so this scan stays bounded.
    #[cfg(test)]
    pub(crate) fn expire_due_competitions(&self, now: u64) -> Result<usize, DatabaseError> {
        self.expire_due_competitions_owned(now, None)
    }

    pub(crate) fn expire_due_competitions_owned(
        &self,
        now: u64,
        owner: Option<&str>,
    ) -> Result<usize, DatabaseError> {
        let mut connection = self.open(&self.automation_path())?;
        let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let rows = {
            let mut statement =
                tx.prepare("SELECT competition_id, state_json FROM competition_state")?;
            let rows = statement.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };
        let mut expired = 0;
        for (id, json) in rows {
            let mut state: LifecycleState = serde_json::from_str(&json)?;
            if state.is_due(now) {
                if let Some(owner) = owner {
                    let current: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM competition_integrity_owners WHERE competition_id=?1 AND generation=?2 AND owner=?3)", params![id,state.generation as i64,owner], |row| row.get(0))?;
                    if !current {
                        continue;
                    }
                }
                state
                    .transition(Transition::Expire, now, None)
                    .map_err(|error| {
                        DatabaseError::Invalid(format!("invalid expiry: {error:?}"))
                    })?;
                tx.execute(
                    "UPDATE competition_state SET state_json=?2 WHERE competition_id=?1",
                    params![id, serde_json::to_string(&state)?],
                )?;
                expired += 1;
            }
        }
        tx.commit()?;
        Ok(expired)
    }
}
