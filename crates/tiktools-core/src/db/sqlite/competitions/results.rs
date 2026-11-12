//! Immutable results committed together with lifecycle and winner outbox.
use super::{DatabaseError, DatabaseManager};
use rusqlite::{params, OptionalExtension};
use tiktools_competitions::{Definition, Lifecycle, LifecycleState, ResultRecord, WinnerAction};

pub(super) fn decode_result(
    id: &str,
    generation: i64,
    ranking: &str,
    created_at: i64,
    json: &str,
) -> Result<ResultRecord, DatabaseError> {
    let result: ResultRecord = serde_json::from_str(json)?;
    if result.schema_version != 1
        || id.is_empty()
        || id.len() > 128
        || ranking.is_empty()
        || ranking.len() > 128
        || generation <= 0
        || created_at < 0
        || result.competition_id != id
        || result.ranking_id != ranking
        || result.generation != generation as u64
        || result.finalized_at != created_at as u64
    {
        return Err(DatabaseError::Invalid(
            "stored competition result identity mismatch".into(),
        ));
    }
    result
        .selection
        .validate()
        .map_err(|_| DatabaseError::Invalid("invalid stored competition selection".into()))?;
    Ok(result)
}

impl DatabaseManager {
    /// Inserts at most one result for a competition generation. A retry reads
    /// the committed record, even if the retry proposes a different draw.
    #[cfg(test)]
    pub(crate) fn store_competition_result(
        &self,
        result: &ResultRecord,
        action: Option<&WinnerAction>,
    ) -> Result<ResultRecord, DatabaseError> {
        self.store_competition_result_once(result, action, None)
            .map(|(result, _)| result)
    }

    pub(crate) fn store_competition_result_once(
        &self,
        result: &ResultRecord,
        action: Option<&WinnerAction>,
        owner: Option<&str>,
    ) -> Result<(ResultRecord, bool), DatabaseError> {
        if result.schema_version != 1
            || result.competition_id.is_empty()
            || result.competition_id.len() > 128
            || result.ranking_id.is_empty()
            || result.ranking_id.len() > 128
            || result.generation > i64::MAX as u64
            || result.finalized_at > i64::MAX as u64
            || result.selection.eligible.len() > 100_000
        {
            return Err(DatabaseError::Invalid("invalid competition result".into()));
        }
        result
            .selection
            .validate()
            .map_err(|_| DatabaseError::Invalid("invalid selection".into()))?;
        if let Some(action) = action {
            if action.action_id.is_empty()
                || action.action_id.len() > 128
                || result.selection.winner.is_none()
            {
                return Err(DatabaseError::Invalid("invalid winner action".into()));
            }
        }
        let mut connection = self.open(&self.automation_path())?;
        let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let existing: Option<(String, i64, String)> = tx.query_row(
            "SELECT ranking_id,created_at,result_json FROM competition_results WHERE competition_id=?1 AND generation=?2",
            params![result.competition_id, result.generation as i64], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).optional()?;
        if let Some((ranking, created_at, json)) = existing {
            let stored = decode_result(
                &result.competition_id,
                result.generation as i64,
                &ranking,
                created_at,
                &json,
            )?;
            tx.commit()?;
            return Ok((stored, false));
        }
        if let Some(owner) = owner {
            let current: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM competition_integrity_owners WHERE competition_id=?1 AND generation=?2 AND owner=?3)", params![result.competition_id,result.generation as i64,owner], |row| row.get(0))?;
            if !current {
                return Err(DatabaseError::Invalid("competition owner changed".into()));
            }
        }
        // The lifecycle and winner must commit together. In particular, an
        // orphan, stale generation, or result for a different ranking must
        // never create an outbox entry.
        let row: Option<(String, String)> = tx.query_row(
            "SELECT d.definition_json, s.state_json FROM competition_definitions d JOIN competition_state s ON s.competition_id=d.id WHERE d.id=?1",
            [&result.competition_id], |row| Ok((row.get(0)?, row.get(1)?)),
        ).optional()?;
        let (definition, state) =
            row.ok_or_else(|| DatabaseError::Invalid("competition not found".into()))?;
        let definition: Definition = serde_json::from_str(&definition)?;
        let mut state: LifecycleState = serde_json::from_str(&state)?;
        if state.lifecycle != Lifecycle::Finalizing
            || state.generation != result.generation
            || definition.ranking_id != result.ranking_id
            || definition.selection_mode != result.selection.mode
            || definition.tie_policy != result.selection.tie_policy
        {
            return Err(DatabaseError::Invalid(
                "result does not match finalizing competition".into(),
            ));
        }
        // User winners (draws and challenges) use `on_user_win`; option
        // winners use the selected option's action.
        let expected_action = result
            .selection
            .winner
            .as_deref()
            .and_then(|winner| definition.winner_action(winner));
        if action != expected_action {
            return Err(DatabaseError::Invalid(
                "winner action does not match definition".into(),
            ));
        }
        state.lifecycle = Lifecycle::Finished;
        state.deadline_ms = None;
        state.remaining_ms = None;
        tx.execute(
            "UPDATE competition_state SET state_json=?2 WHERE competition_id=?1",
            params![result.competition_id, serde_json::to_string(&state)?],
        )?;
        tx.execute("INSERT INTO competition_results (competition_id, generation, ranking_id, created_at, result_json) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![result.competition_id, result.generation as i64, result.ranking_id, result.finalized_at as i64, serde_json::to_string(result)?])?;
        if let Some(action) = action {
            // JSON array encoding is collision-free even when ids contain punctuation.
            let key = serde_json::to_string(&(
                "competition-winner",
                &result.competition_id,
                result.generation,
            ))?;
            tx.execute("INSERT INTO competition_action_outbox (competition_id, generation, action_id, idempotency_key, action_json, status, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, 'pending', ?6, ?6)",
                params![result.competition_id, result.generation as i64, action.action_id, key, serde_json::to_string(action)?, result.finalized_at as i64])?;
        }
        tx.commit()?;
        Ok((result.clone(), true))
    }

    pub(crate) fn load_competition_result(
        &self,
        competition_id: &str,
        generation: u64,
    ) -> Result<Option<ResultRecord>, DatabaseError> {
        let generation = i64::try_from(generation)
            .map_err(|_| DatabaseError::Invalid("invalid generation".into()))?;
        let connection = self.open(&self.automation_path())?;
        let row: Option<(String, i64, String)> = connection.query_row("SELECT ranking_id,created_at,result_json FROM competition_results WHERE competition_id=?1 AND generation=?2", params![competition_id, generation], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?))).optional()?;
        row.map(|(ranking, created_at, json)| {
            decode_result(competition_id, generation, &ranking, created_at, &json)
        })
        .transpose()
    }

    /// Read only committed results, newest generation first. The cursor is
    /// exclusive, and the caller bounds page size before reaching SQLite.
    pub(crate) fn list_competition_results(
        &self,
        competition_id: &str,
        before_generation: Option<u64>,
        limit: usize,
    ) -> Result<Vec<ResultRecord>, DatabaseError> {
        if limit == 0 || limit > 21 {
            return Err(DatabaseError::Invalid("invalid result page size".into()));
        }
        let cursor = before_generation
            .map(i64::try_from)
            .transpose()
            .map_err(|_| DatabaseError::Invalid("invalid result cursor".into()))?;
        let connection = self.open(&self.automation_path())?;
        let mut statement = connection.prepare(
            "SELECT generation,ranking_id,created_at,result_json FROM competition_results WHERE competition_id=?1 AND (?2 IS NULL OR generation < ?2) ORDER BY generation DESC LIMIT ?3"
        )?;
        let rows = statement.query_map(params![competition_id, cursor, limit as i64], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        rows.map(|row| {
            let (generation, ranking, created_at, json) = row?;
            decode_result(competition_id, generation, &ranking, created_at, &json)
        })
        .collect()
    }

    /// Latest committed results only; drafts and in-progress selections have
    /// no row and cannot leak random state to read-only presentation clients.
    #[cfg(test)]
    pub(crate) fn latest_competition_results(&self) -> Result<Vec<ResultRecord>, DatabaseError> {
        self.latest_competition_results_impl(false)
    }

    /// A corrupt result must not prevent other finished competitions from
    /// refreshing read-only widgets. Invalid rows remain errors in history API.
    pub(crate) fn latest_competition_results_for_presentation(
        &self,
    ) -> Result<Vec<ResultRecord>, DatabaseError> {
        self.latest_competition_results_impl(true)
    }

    fn latest_competition_results_impl(
        &self,
        skip_invalid: bool,
    ) -> Result<Vec<ResultRecord>, DatabaseError> {
        let connection = self.open(&self.automation_path())?;
        let mut statement = connection.prepare("SELECT r.competition_id,r.generation,r.ranking_id,r.created_at,r.result_json FROM competition_results r JOIN competition_state s ON s.competition_id=r.competition_id WHERE json_extract(s.state_json,'$.lifecycle')='finished' AND r.generation=json_extract(s.state_json,'$.generation') ORDER BY r.competition_id LIMIT 129")?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?;
        let mut results = Vec::new();
        for (index, row) in rows.enumerate() {
            if index >= 128 {
                return Err(DatabaseError::Invalid(
                    "competition result capacity exceeded".into(),
                ));
            }
            let (id, generation, ranking, created_at, json) = row?;
            match decode_result(&id, generation, &ranking, created_at, &json) {
                Ok(result) => results.push(result),
                Err(error) if skip_invalid => {
                    tracing::error!(%error, competition_id = %id, generation, "invalid competition result omitted from presentation")
                }
                Err(error) => return Err(error),
            }
        }
        Ok(results)
    }
}
