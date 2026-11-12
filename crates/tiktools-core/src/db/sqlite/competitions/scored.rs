//! Canonical score admission with replay tombstones for ineligible events.
use super::{DatabaseError, DatabaseManager};
use super::{ScoredInput, ScoredOutcome};
use rusqlite::{params, OptionalExtension};
use tiktools_competitions::{Lifecycle, LifecycleState};

impl DatabaseManager {
    /// Admit a canonical scored event and its identity in one transaction.
    /// An unassigned or paused event receives a durable tombstone so it can
    /// never become eligible when replayed after selection or resume.
    pub(crate) fn route_competition_score(
        &self,
        id: &str,
        input: &ScoredInput<'_>,
        owner: Option<&str>,
    ) -> Result<Option<ScoredOutcome>, DatabaseError> {
        if input.creator_id.is_empty()
            || input.creator_id.len() > 256
            || input.room_id.is_empty()
            || input.room_id.len() > 256
            || input.participant_id.is_empty()
            || input.participant_id.len() > 512
            || input.event_identity.is_empty()
            || input.event_identity.len() > 512
            || !matches!(input.source, "tiktok.like" | "tiktok.share")
            || input.score == 0
            || input.received_at > i64::MAX as u64
        {
            return Err(DatabaseError::Invalid(
                "invalid scored participation".into(),
            ));
        }
        let mut connection = self.open(&self.automation_path())?;
        let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let state: Option<String> = tx.query_row("SELECT s.state_json FROM competition_state s JOIN competition_definitions d ON d.id=s.competition_id JOIN aggregation_definitions a ON a.id=d.ranking_id WHERE d.id=?1 AND a.creator_id=?2 AND a.room_id=?3", params![id,input.creator_id,input.room_id], |row| row.get(0)).optional()?;
        let Some(state) = state else { return Ok(None) };
        let state: LifecycleState = serde_json::from_str(&state)?;
        // Closed generations do not need to claim an owner after restart.
        if matches!(
            state.lifecycle,
            Lifecycle::Draft
                | Lifecycle::Finalizing
                | Lifecycle::Finished
                | Lifecycle::Cancelled
                | Lifecycle::Failed
        ) {
            return Ok(Some(ScoredOutcome::Closed));
        }
        let generation = i64::try_from(state.generation)
            .map_err(|_| DatabaseError::Invalid("invalid generation".into()))?;
        if let Some(owner) = owner {
            let current: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM competition_integrity_owners WHERE competition_id=?1 AND generation=?2 AND owner=?3)", params![id,generation,owner], |row| row.get(0))?;
            if !current {
                return Err(DatabaseError::Invalid("competition owner changed".into()));
            }
        }
        let prior: Option<(String,String)> = tx.query_row("SELECT participant_id,score FROM competition_scored_contributions WHERE competition_id=?1 AND generation=?2 AND source=?3 AND event_identity=?4", params![id,generation,input.source,input.event_identity], |row| Ok((row.get(0)?,row.get(1)?))).optional()?;
        if let Some((participant, score)) = prior {
            return Ok(Some(
                if participant == input.participant_id && score == input.score.to_string() {
                    ScoredOutcome::Duplicate
                } else {
                    ScoredOutcome::IdentityConflict
                },
            ));
        }
        if !matches!(state.lifecycle, Lifecycle::Active | Lifecycle::Paused) {
            return Ok(Some(ScoredOutcome::Closed));
        }
        let option: Option<String> = if state.lifecycle == Lifecycle::Active
            && !state.is_due(input.received_at)
        {
            tx.query_row("SELECT option_id FROM competition_participants WHERE competition_id=?1 AND generation=?2 AND participant_id=?3", params![id,generation,input.participant_id], |row| row.get(0)).optional()?
        } else {
            None
        };
        let outcome = if state.lifecycle != Lifecycle::Active || state.is_due(input.received_at) {
            ScoredOutcome::Closed
        } else if let Some(option_id) = &option {
            ScoredOutcome::Accepted {
                option_id: option_id.clone(),
            }
        } else {
            ScoredOutcome::Unassigned
        };
        let count: Option<i64> = tx.query_row("SELECT contributions FROM competition_participation_counts WHERE competition_id=?1 AND generation=?2", params![id,generation], |row| row.get(0)).optional()?;
        let count = count.unwrap_or(0);
        if count < 0 {
            return Err(DatabaseError::Invalid(
                "invalid participation counters".into(),
            ));
        }
        if count >= 500_000 {
            return Ok(Some(ScoredOutcome::Capacity));
        }
        tx.execute("INSERT INTO competition_participation_counts(competition_id,generation,participants,contributions) VALUES (?1,?2,0,1) ON CONFLICT(competition_id,generation) DO UPDATE SET contributions=contributions+1", params![id,generation])?;
        tx.execute("INSERT INTO competition_scored_contributions(competition_id,generation,source,event_identity,participant_id,option_id,score,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)", params![id,generation,input.source,input.event_identity,input.participant_id,option,input.score.to_string(),input.received_at as i64])?;
        tx.commit()?;
        Ok(Some(outcome))
    }
}
