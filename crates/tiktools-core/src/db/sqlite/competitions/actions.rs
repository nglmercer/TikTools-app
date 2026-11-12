//! Winner action claims, immutable binding validation and terminal outcomes.
use super::{results::decode_result, ActionClaimOutcome};
use super::{DatabaseError, DatabaseManager};
use rusqlite::{params, OptionalExtension};
use tiktools_competitions::{
    ActionClaim, ActionExecution, ActionResolution, ActionStatus, Definition, Lifecycle,
    LifecycleState, WinnerAction,
};

/// Match the immutable outbox, selected winner and saved definition under the
/// same business transaction used to admit a delegated winner action.
pub(in super::super) fn validate_competition_winner_claim(
    tx: &rusqlite::Connection,
    claim: &ActionClaim,
) -> Result<(), DatabaseError> {
    let denied = || DatabaseError::CommandAuthorization(crate::collaboration::Error::Forbidden);
    let generation = i64::try_from(claim.generation)
        .ok()
        .filter(|g| *g > 0)
        .ok_or_else(denied)?;
    #[allow(clippy::type_complexity)]
    let row: Option<(String, String, String, String, String, i64, String, String)> = tx.query_row(
        "SELECT d.definition_json,s.state_json,o.idempotency_key,o.action_id,o.action_json,r.created_at,r.ranking_id,r.result_json FROM competition_action_outbox o JOIN competition_definitions d ON d.id=o.competition_id JOIN competition_state s ON s.competition_id=o.competition_id JOIN competition_results r ON r.competition_id=o.competition_id AND r.generation=o.generation WHERE o.competition_id=?1 AND o.generation=?2 AND o.status='running'",
        params![claim.competition_id, generation],
        |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?,row.get(6)?,row.get(7)?)),
    ).optional()?;
    let (definition, state, key, action_id, action, created_at, ranking, result) =
        row.ok_or_else(denied)?;
    let definition: Definition = serde_json::from_str(&definition).map_err(|_| denied())?;
    let state: LifecycleState = serde_json::from_str(&state).map_err(|_| denied())?;
    let action: WinnerAction = serde_json::from_str(&action).map_err(|_| denied())?;
    let result = decode_result(
        &claim.competition_id,
        generation,
        &ranking,
        created_at,
        &result,
    )
    .map_err(|_| denied())?;
    let expected_key = serde_json::to_string(&(
        "competition-winner",
        &claim.competition_id,
        claim.generation,
    ))?;
    if definition.id != claim.competition_id
        || definition.ranking_id != ranking
        || definition.selection_mode != result.selection.mode
        || definition.tie_policy != result.selection.tie_policy
        || state.lifecycle != Lifecycle::Finished
        || state.generation != claim.generation
        || key != expected_key
        || claim.idempotency_key != key
        || action_id != action.action_id
        || action != claim.action
        || result
            .selection
            .winner
            .as_deref()
            .and_then(|winner| definition.winner_action(winner))
            != Some(&action)
    {
        return Err(denied());
    }
    Ok(())
}

impl DatabaseManager {
    /// Read the public outbox state without returning action inputs or keys.
    pub(crate) fn competition_action(
        &self,
        id: &str,
        generation: u64,
    ) -> Result<Option<ActionExecution>, DatabaseError> {
        let generation = i64::try_from(generation)
            .map_err(|_| DatabaseError::Invalid("invalid generation".into()))?;
        let connection = self.open(&self.automation_path())?;
        let row: Option<(String, String, i64, i64, Option<String>)> = connection.query_row(
            "SELECT o.action_id,o.status,o.created_at,o.updated_at,d.reason FROM competition_action_outbox o LEFT JOIN competition_action_diagnostics d ON d.competition_id=o.competition_id AND d.generation=o.generation WHERE o.competition_id=?1 AND o.generation=?2",
            params![id, generation], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        ).optional()?;
        row.map(|(action_id, status, created_at, updated_at, reason)| {
            let status = match status.as_str() {
                "pending" => ActionStatus::Pending,
                "running" => ActionStatus::Running,
                "completed" => ActionStatus::Completed,
                "failed" => ActionStatus::Failed,
                "uncertain" => ActionStatus::Uncertain,
                _ => return Err(DatabaseError::Invalid("invalid action status".into())),
            };
            Ok(ActionExecution {
                competition_id: id.to_owned(),
                generation: generation as u64,
                action_id,
                status,
                reason,
                created_at: u64::try_from(created_at)
                    .map_err(|_| DatabaseError::Invalid("invalid action timestamp".into()))?,
                updated_at: u64::try_from(updated_at)
                    .map_err(|_| DatabaseError::Invalid("invalid action timestamp".into()))?,
            })
        })
        .transpose()
    }

    /// Record an operator's decision about an uncertain outcome. This never
    /// returns it to pending and therefore cannot dispatch the action again.
    pub(crate) fn resolve_uncertain_competition_action(
        &self,
        id: &str,
        generation: u64,
        resolution: ActionResolution,
        now: u64,
    ) -> Result<bool, DatabaseError> {
        let generation = i64::try_from(generation)
            .map_err(|_| DatabaseError::Invalid("invalid generation".into()))?;
        let now =
            i64::try_from(now).map_err(|_| DatabaseError::Invalid("invalid timestamp".into()))?;
        let status = match resolution {
            ActionResolution::Completed => "completed",
            ActionResolution::Failed => "failed",
        };
        let connection = self.open(&self.automation_path())?;
        Ok(connection.execute("UPDATE competition_action_outbox SET status=?3, updated_at=?4 WHERE competition_id=?1 AND generation=?2 AND status='uncertain'", params![id, generation, status, now])? == 1)
    }

    /// Store only an allowlisted reason, in the same transaction as the
    /// state change; errors from actions may contain credentials or payloads.
    pub(crate) fn record_competition_action_outcome(
        &self,
        id: &str,
        generation: u64,
        status: ActionStatus,
        reason: Option<&str>,
        now: i64,
    ) -> Result<bool, DatabaseError> {
        let generation = i64::try_from(generation)
            .map_err(|_| DatabaseError::Invalid("invalid generation".into()))?;
        let status = match status {
            ActionStatus::Completed => "completed",
            ActionStatus::Failed => "failed",
            ActionStatus::Uncertain => "uncertain",
            _ => return Err(DatabaseError::Invalid("invalid action outcome".into())),
        };
        if reason.is_some_and(|reason| {
            !matches!(
                reason,
                "result_unavailable"
                    | "competition_unavailable"
                    | "ranking_unavailable"
                    | "action_unavailable"
                    | "action_disabled"
                    | "invalid_inputs"
                    | "invalid_action"
                    | "winner_unavailable"
                    | "preparation_interrupted"
                    | "execution_failed"
                    | "execution_interrupted"
                    | "operator_marked_uncertain"
                    | "invalid_outbox_record"
                    | "authority_unavailable"
                    | "action_reference_mismatch"
            )
        }) {
            return Err(DatabaseError::Invalid("invalid action reason".into()));
        }
        let mut connection = self.open(&self.automation_path())?;
        let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let changed = tx.execute("UPDATE competition_action_outbox SET status=?3, updated_at=?4 WHERE competition_id=?1 AND generation=?2 AND status='running'", params![id, generation, status, now])? == 1;
        if changed {
            if let Some(reason) = reason {
                tx.execute("INSERT INTO competition_action_diagnostics(competition_id,generation,reason,updated_at) VALUES (?1,?2,?3,?4) ON CONFLICT(competition_id,generation) DO UPDATE SET reason=excluded.reason,updated_at=excluded.updated_at", params![id,generation,reason,now])?;
            }
            tx.commit()?;
        }
        Ok(changed)
    }

    pub(crate) fn pending_competition_actions(&self) -> Result<Vec<(String, u64)>, DatabaseError> {
        let connection = self.open(&self.automation_path())?;
        let mut statement = connection.prepare("SELECT competition_id,generation FROM competition_action_outbox WHERE status='pending' ORDER BY created_at,competition_id LIMIT 32")?;
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        rows.map(|row| {
            let (id, generation) = row?;
            Ok((
                id,
                u64::try_from(generation)
                    .map_err(|_| DatabaseError::Invalid("invalid generation".into()))?,
            ))
        })
        .collect()
    }

    pub(crate) fn mark_competition_action_uncertain(
        &self,
        id: &str,
        generation: u64,
        now: i64,
    ) -> Result<bool, DatabaseError> {
        self.record_competition_action_outcome(
            id,
            generation,
            ActionStatus::Uncertain,
            Some("operator_marked_uncertain"),
            now,
        )
    }

    /// Atomically claims one pending action. A running or uncertain action
    /// cannot be claimed automatically, even if a caller retries.
    #[cfg(test)]
    pub(crate) fn claim_competition_action(
        &self,
        competition_id: &str,
        generation: u64,
        now: i64,
    ) -> Result<Option<ActionClaim>, DatabaseError> {
        Ok(
            match self.claim_competition_action_detailed(competition_id, generation, now)? {
                ActionClaimOutcome::Claimed(claim) => Some(claim),
                ActionClaimOutcome::Missing | ActionClaimOutcome::Invalid { .. } => None,
            },
        )
    }

    pub(crate) fn claim_competition_action_detailed(
        &self,
        competition_id: &str,
        generation: u64,
        now: i64,
    ) -> Result<ActionClaimOutcome, DatabaseError> {
        let generation = i64::try_from(generation)
            .map_err(|_| DatabaseError::Invalid("invalid generation".into()))?;
        let mut connection = self.open(&self.automation_path())?;
        let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let row: Option<(String, String, String)> = tx.query_row(
            "SELECT idempotency_key,action_id,action_json FROM competition_action_outbox WHERE competition_id=?1 AND generation=?2 AND status='pending'",
            params![competition_id, generation], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).optional()?;
        let Some((idempotency_key, action_id, json)) = row else {
            return Ok(ActionClaimOutcome::Missing);
        };
        let expected_key =
            serde_json::to_string(&("competition-winner", competition_id, generation as u64))?;
        let action: Option<WinnerAction> = serde_json::from_str(&json).ok();
        if action.as_ref().is_none_or(|action| {
            action.action_id != action_id || action_id.is_empty() || action_id.len() > 128
        }) || idempotency_key != expected_key
        {
            tx.execute("UPDATE competition_action_outbox SET status='failed',updated_at=?3 WHERE competition_id=?1 AND generation=?2 AND status='pending'", params![competition_id, generation, now])?;
            tx.execute("INSERT INTO competition_action_diagnostics(competition_id,generation,reason,updated_at) VALUES (?1,?2,'invalid_outbox_record',?3) ON CONFLICT(competition_id,generation) DO UPDATE SET reason=excluded.reason,updated_at=excluded.updated_at", params![competition_id,generation,now])?;
            tx.commit()?;
            tracing::error!(
                competition_id,
                generation,
                "invalid competition action outbox record"
            );
            return Ok(ActionClaimOutcome::Invalid { action_id });
        }
        tx.execute("UPDATE competition_action_outbox SET status='running',updated_at=?3 WHERE competition_id=?1 AND generation=?2 AND status='pending'", params![competition_id,generation,now])?;
        tx.commit()?;
        Ok(ActionClaimOutcome::Claimed(ActionClaim {
            competition_id: competition_id.to_owned(),
            generation: generation as u64,
            idempotency_key,
            action: action.expect("validated action"),
        }))
    }

    /// On restart, running actions may have executed externally. Never retry
    /// them automatically without an explicit, action-specific recovery rule.
    pub(crate) fn mark_interrupted_competition_actions_uncertain(
        &self,
        now: i64,
    ) -> Result<usize, DatabaseError> {
        let connection = self.open(&self.automation_path())?;
        Ok(connection.execute("UPDATE competition_action_outbox SET status='uncertain', updated_at=?1 WHERE status='running'", [now])?)
    }

    #[cfg(test)]
    pub(crate) fn complete_competition_action(
        &self,
        competition_id: &str,
        generation: u64,
        success: bool,
        now: i64,
    ) -> Result<bool, DatabaseError> {
        let generation = i64::try_from(generation)
            .map_err(|_| DatabaseError::Invalid("invalid generation".into()))?;
        let connection = self.open(&self.automation_path())?;
        let status = if success { "completed" } else { "failed" };
        Ok(connection.execute("UPDATE competition_action_outbox SET status=?3, updated_at=?4 WHERE competition_id=?1 AND generation=?2 AND status='running'", params![competition_id, generation, status, now])? == 1)
    }
}
