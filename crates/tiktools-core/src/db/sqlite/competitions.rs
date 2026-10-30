//! Durable immutable winner records. No side effects are dispatched here.
use super::{DatabaseError, DatabaseManager};
use rusqlite::{params, OptionalExtension};
use tiktools_competitions::{
    ActionClaim, ActionExecution, ActionResolution, ActionStatus, CommandParticipation, Definition,
    Lifecycle, LifecycleState, ParticipationOutcome, ResultRecord, Transition, WinnerAction,
};

pub(crate) enum ActionClaimOutcome {
    Missing,
    Invalid { action_id: String },
    Claimed(ActionClaim),
}

fn decode_result(
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

/// Retain the identity of a recognized but ineligible command while its
/// generation can become active again. Otherwise replay after resume or a
/// later option switch could turn an ignored event into a vote. Receipts
/// share the generation's existing bounded contribution budget.
#[allow(clippy::too_many_arguments)]
fn record_ignored_command(
    tx: rusqlite::Transaction<'_>,
    competition: &str,
    generation: i64,
    identity: &str,
    participant: &str,
    option: &str,
    now: u64,
    outcome: ParticipationOutcome,
) -> Result<Option<ParticipationOutcome>, DatabaseError> {
    let count: Option<i64> = tx.query_row(
        "SELECT contributions FROM competition_participation_counts WHERE competition_id=?1 AND generation=?2",
        params![competition, generation], |row| row.get(0),
    ).optional()?;
    let count = count.unwrap_or(0);
    if count < 0 {
        return Err(DatabaseError::Invalid(
            "invalid participation counters".into(),
        ));
    }
    if count >= 500_000 {
        return Ok(Some(ParticipationOutcome::Capacity));
    }
    tx.execute("INSERT INTO competition_participation_counts(competition_id,generation,participants,contributions) VALUES (?1,?2,0,1) ON CONFLICT(competition_id,generation) DO UPDATE SET contributions=contributions+1", params![competition,generation])?;
    tx.execute("INSERT INTO competition_contributions(competition_id,generation,event_identity,participant_id,option_id,created_at) VALUES (?1,?2,?3,?4,?5,?6)", params![competition,generation,identity,participant,option,now as i64])?;
    tx.commit()?;
    Ok(Some(outcome))
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ScoredOutcome {
    Accepted { option_id: String },
    Duplicate,
    IdentityConflict,
    Unassigned,
    Closed,
    Capacity,
}

pub(crate) struct ScoredInput<'a> {
    pub creator_id: &'a str,
    pub room_id: &'a str,
    pub participant_id: &'a str,
    pub event_identity: &'a str,
    pub source: &'a str,
    pub score: u64,
    pub received_at: u64,
}

#[derive(Debug)]
pub(crate) struct GiftInput<'a> {
    pub creator_id: &'a str,
    pub room_id: &'a str,
    pub participant_id: &'a str,
    pub event_identity: &'a str,
    pub gift_id: &'a str,
    pub group_id: &'a str,
    pub observed_quantity: u64,
    pub price: u64,
    pub received_at: u64,
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum GiftOutcome {
    Accepted { option_id: String, score: u64 },
    Duplicate,
    IdentityConflict,
    Closed,
    Unassigned,
    Zero,
    Capacity,
}

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
            tx.execute("INSERT INTO competition_integrity_checkpoints(competition_id,generation,status) VALUES (?1,?2,'open')", params![id, generation])?;
            if let Some(owner) = owner {
                tx.execute("INSERT INTO competition_integrity_owners(competition_id,generation,owner) VALUES (?1,?2,?3)", params![id,generation,owner])?;
            }
        }
        tx.commit()?;
        Ok(Some(state))
    }
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
    /// Persist a command vote and its replay identity in one transaction.
    /// The store fails closed at its fixed generation capacity: it never evicts
    /// identities while the generation can still accept contributions.
    #[cfg(test)]
    pub(crate) fn route_competition_command(
        &self,
        id: &str,
        input: &CommandParticipation<'_>,
    ) -> Result<Option<ParticipationOutcome>, DatabaseError> {
        self.route_competition_command_with_previous(id, input, &mut None, None)
    }
    pub(crate) fn route_competition_command_with_previous(
        &self,
        id: &str,
        input: &CommandParticipation<'_>,
        previous: &mut Option<(String, u64)>,
        owner: Option<&str>,
    ) -> Result<Option<ParticipationOutcome>, DatabaseError> {
        *previous = None;
        let CommandParticipation {
            creator_id,
            room_id,
            participant_id: participant,
            event_identity: identity,
            command,
            received_at: now,
        } = *input;
        const MAX_PARTICIPANTS: i64 = 100_000;
        const MAX_CONTRIBUTIONS: i64 = 500_000;
        if creator_id.is_empty()
            || creator_id.len() > 256
            || room_id.is_empty()
            || room_id.len() > 256
            || participant.is_empty()
            || participant.len() > 512
            || identity.is_empty()
            || identity.len() > 512
            || now > i64::MAX as u64
        {
            return Err(DatabaseError::Invalid(
                "invalid participation identity".into(),
            ));
        }
        let mut connection = self.open(&self.automation_path())?;
        let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let row: Option<(String, String)> = tx.query_row("SELECT d.definition_json, s.state_json FROM competition_definitions d JOIN competition_state s ON s.competition_id=d.id JOIN aggregation_definitions a ON a.id=d.ranking_id WHERE d.id=?1 AND a.creator_id=?2 AND a.room_id=?3", params![id, creator_id, room_id], |row| Ok((row.get(0)?, row.get(1)?))).optional()?;
        let Some((definition, state)) = row else {
            return Ok(None);
        };
        let definition: Definition = serde_json::from_str(&definition)?;
        let state: LifecycleState = serde_json::from_str(&state)?;
        // Draft and terminal states cannot admit participation and may not
        // have a current owner after restart.
        if matches!(
            state.lifecycle,
            Lifecycle::Draft
                | Lifecycle::Finalizing
                | Lifecycle::Finished
                | Lifecycle::Cancelled
                | Lifecycle::Failed
        ) {
            return Ok(Some(ParticipationOutcome::Closed));
        }
        let generation = i64::try_from(state.generation)
            .map_err(|_| DatabaseError::Invalid("invalid generation".into()))?;
        if let Some(owner) = owner {
            let current: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM competition_integrity_owners WHERE competition_id=?1 AND generation=?2 AND owner=?3)", params![id,generation,owner], |row| row.get(0))?;
            if !current {
                return Err(DatabaseError::Invalid("competition owner changed".into()));
            }
        }
        let option = definition.option_for_command(command);
        let existing: Option<(String, String)> = tx.query_row("SELECT participant_id, option_id FROM competition_contributions WHERE competition_id=?1 AND generation=?2 AND event_identity=?3", params![id, generation, identity], |row| Ok((row.get(0)?, row.get(1)?))).optional()?;
        if let Some((prior_participant, prior_option)) = existing {
            return Ok(Some(
                if prior_participant == participant && option == Some(prior_option.as_str()) {
                    ParticipationOutcome::Duplicate {
                        option_id: prior_option,
                    }
                } else {
                    ParticipationOutcome::IdentityConflict
                },
            ));
        }
        if state.lifecycle != Lifecycle::Active || state.is_due(now) {
            if matches!(state.lifecycle, Lifecycle::Active | Lifecycle::Paused) {
                if let Some(option) = option {
                    return record_ignored_command(
                        tx,
                        id,
                        generation,
                        identity,
                        participant,
                        option,
                        now,
                        ParticipationOutcome::Closed,
                    );
                }
            }
            return Ok(Some(ParticipationOutcome::Closed));
        }
        let Some(option) = option else {
            return Ok(Some(ParticipationOutcome::UnknownCommand));
        };
        let existing: Option<(String, i64)> = tx.query_row("SELECT option_id, votes FROM competition_participants WHERE competition_id=?1 AND generation=?2 AND participant_id=?3", params![id, generation, participant], |row| Ok((row.get(0)?, row.get(1)?))).optional()?;
        let counts: Option<(i64, i64)> = tx.query_row("SELECT participants, contributions FROM competition_participation_counts WHERE competition_id=?1 AND generation=?2", params![id, generation], |row| Ok((row.get(0)?, row.get(1)?))).optional()?;
        let (participants, contributions) = counts.unwrap_or((0, 0));
        if participants < 0 || contributions < 0 {
            return Err(DatabaseError::Invalid(
                "invalid participation counters".into(),
            ));
        }
        if contributions >= MAX_CONTRIBUTIONS {
            return Ok(Some(ParticipationOutcome::Capacity));
        }
        let prior = existing
            .as_ref()
            .map(|(option, votes)| (option.clone(), *votes as u64));
        let gift_participant: bool = if existing.is_none() {
            tx.query_row("SELECT EXISTS(SELECT 1 FROM competition_gift_participants WHERE competition_id=?1 AND generation=?2 AND participant_id=?3)",params![id,generation,participant],|row| row.get(0))?
        } else {
            false
        };
        let (votes, switched, new_participant) = match existing {
            Some((previous, votes)) => {
                if previous != option && !definition.allow_switch {
                    return record_ignored_command(
                        tx,
                        id,
                        generation,
                        identity,
                        participant,
                        option,
                        now,
                        ParticipationOutcome::SwitchForbidden,
                    );
                }
                if votes <= 0 {
                    return Err(DatabaseError::Invalid(
                        "invalid participant vote count".into(),
                    ));
                }
                let switched = previous != option;
                let next_votes = if votes as u64 >= definition.vote_limit {
                    if !switched {
                        return record_ignored_command(
                            tx,
                            id,
                            generation,
                            identity,
                            participant,
                            option,
                            now,
                            ParticipationOutcome::VoteLimit,
                        );
                    }
                    votes
                } else {
                    votes
                        .checked_add(1)
                        .ok_or_else(|| DatabaseError::Invalid("vote count overflow".into()))?
                };
                (next_votes, switched, false)
            }
            None => {
                if participants >= MAX_PARTICIPANTS && !gift_participant {
                    return Ok(Some(ParticipationOutcome::Capacity));
                }
                (1, false, true)
            }
        };
        tx.execute("INSERT INTO competition_participation_counts(competition_id,generation,participants,contributions) VALUES (?1,?2,?3,1) ON CONFLICT(competition_id,generation) DO UPDATE SET participants=participants+excluded.participants,contributions=contributions+1", params![id, generation, i64::from(new_participant && !gift_participant)])?;
        tx.execute("INSERT INTO competition_participants(competition_id,generation,participant_id,option_id,votes,updated_at) VALUES (?1,?2,?3,?4,?5,?6) ON CONFLICT(competition_id,generation,participant_id) DO UPDATE SET option_id=excluded.option_id,votes=excluded.votes,updated_at=excluded.updated_at", params![id,generation,participant,option,votes,now as i64])?;
        tx.execute("INSERT INTO competition_contributions(competition_id,generation,event_identity,participant_id,option_id,created_at) VALUES (?1,?2,?3,?4,?5,?6)", params![id,generation,identity,participant,option,now as i64])?;
        tx.commit()?;
        *previous = prior;
        Ok(Some(ParticipationOutcome::Accepted {
            option_id: option.to_owned(),
            votes: votes as u64,
            switched,
        }))
    }
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

    /// Atomically admit a combo packet, its scoped high-water mark and the
    /// resulting shared-engine score. No contribution is assigned implicitly.
    pub(crate) fn route_competition_gift<F>(
        &self,
        id: &str,
        input: &GiftInput<'_>,
        owner: &str,
        score: F,
    ) -> Result<Option<GiftOutcome>, DatabaseError>
    where
        F: FnOnce(u64, u64, &str) -> Result<u64, DatabaseError>,
    {
        if input.creator_id.is_empty()
            || input.creator_id.len() > 256
            || input.room_id.is_empty()
            || input.room_id.len() > 256
            || input.participant_id.is_empty()
            || input.participant_id.len() > 512
            || input.event_identity.is_empty()
            || input.event_identity.len() > 512
            || input.gift_id.is_empty()
            || input.gift_id.len() > 128
            || input.group_id.is_empty()
            || input.group_id.len() > 128
            || input.observed_quantity == 0
            || input.price == 0
            || input.received_at > i64::MAX as u64
            || owner.is_empty()
        {
            return Err(DatabaseError::Invalid("invalid gift contribution".into()));
        }
        let mut connection = self.open(&self.automation_path())?;
        let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let row: Option<(String,String)> = tx.query_row("SELECT d.definition_json,s.state_json FROM competition_state s JOIN competition_definitions d ON d.id=s.competition_id JOIN aggregation_definitions a ON a.id=d.ranking_id WHERE d.id=?1 AND a.creator_id=?2 AND a.room_id=?3", params![id,input.creator_id,input.room_id], |row| Ok((row.get(0)?,row.get(1)?))).optional()?;
        let Some((definition, state)) = row else {
            return Ok(None);
        };
        let state: LifecycleState = serde_json::from_str(&state)?;
        if !matches!(state.lifecycle, Lifecycle::Active | Lifecycle::Paused) {
            return Ok(Some(GiftOutcome::Closed));
        }
        let generation = i64::try_from(state.generation)
            .map_err(|_| DatabaseError::Invalid("invalid generation".into()))?;
        let current: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM competition_integrity_owners WHERE competition_id=?1 AND generation=?2 AND owner=?3)", params![id,generation,owner], |row| row.get(0))?;
        if !current {
            return Err(DatabaseError::Invalid("competition owner changed".into()));
        }
        let prior: Option<(String,String,String,String,String)> = tx.query_row("SELECT participant_id,gift_id,group_id,observed_quantity,price FROM competition_gift_receipts WHERE competition_id=?1 AND generation=?2 AND event_identity=?3",params![id,generation,input.event_identity],|row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?))).optional()?;
        if let Some((participant, gift, group, quantity, price)) = prior {
            return Ok(Some(
                if participant == input.participant_id
                    && gift == input.gift_id
                    && group == input.group_id
                    && quantity == input.observed_quantity.to_string()
                    && price == input.price.to_string()
                {
                    GiftOutcome::Duplicate
                } else {
                    GiftOutcome::IdentityConflict
                },
            ));
        }
        let count: Option<i64> = tx.query_row("SELECT contributions FROM competition_participation_counts WHERE competition_id=?1 AND generation=?2", params![id,generation], |row| row.get(0)).optional()?;
        if count.unwrap_or(0) < 0 {
            return Err(DatabaseError::Invalid(
                "invalid participation counters".into(),
            ));
        }
        if count.unwrap_or(0) >= 500_000 {
            return Ok(Some(GiftOutcome::Capacity));
        }
        let previous_price: Option<String> = tx.query_row("SELECT price FROM competition_gift_combo_prices WHERE competition_id=?1 AND generation=?2 AND participant_id=?3 AND gift_id=?4 AND group_id=?5",params![id,generation,input.participant_id,input.gift_id,input.group_id],|row|row.get(0)).optional()?;
        if previous_price
            .as_ref()
            .is_some_and(|price| price != &input.price.to_string())
        {
            return Err(DatabaseError::Invalid("gift combo price changed".into()));
        }
        if previous_price.is_none() && input.observed_quantity > 1 {
            return Err(DatabaseError::Invalid(
                "gift combo began before observation".into(),
            ));
        }
        if previous_price.is_none() {
            tx.execute("INSERT INTO competition_gift_combo_prices(competition_id,generation,participant_id,gift_id,group_id,price) VALUES (?1,?2,?3,?4,?5,?6)",params![id,generation,input.participant_id,input.gift_id,input.group_id,input.price.to_string()])?;
        }
        let delta = super::gift_accounting::advance_combo(
            &tx,
            id,
            state.generation,
            input.participant_id,
            input.gift_id,
            input.group_id,
            input.observed_quantity,
            Some(input.price),
        )?;
        let score = if delta.quantity == 0 {
            0
        } else {
            score(
                delta.quantity,
                delta
                    .diamonds
                    .ok_or_else(|| DatabaseError::Invalid("gift price missing".into()))?,
                input.gift_id,
            )?
        };
        let definition: Definition = serde_json::from_str(&definition)?;
        let option = if state.lifecycle == Lifecycle::Active && !state.is_due(input.received_at) {
            definition.options.iter().find(|option| option.gift_ids.iter().any(|gift| gift == input.gift_id)).map(|option| option.id.clone()).or(tx.query_row("SELECT option_id FROM competition_participants WHERE competition_id=?1 AND generation=?2 AND participant_id=?3",params![id,generation,input.participant_id],|row| row.get(0)).optional()?)
        } else {
            None
        };
        let outcome = if state.lifecycle != Lifecycle::Active || state.is_due(input.received_at) {
            GiftOutcome::Closed
        } else if score == 0 {
            GiftOutcome::Zero
        } else if let Some(option_id) = &option {
            GiftOutcome::Accepted {
                option_id: option_id.clone(),
                score,
            }
        } else {
            GiftOutcome::Unassigned
        };
        let credited_option = if matches!(outcome, GiftOutcome::Accepted { .. }) {
            option
        } else {
            None
        };
        let new_gift_participant = if credited_option.is_some() {
            let existing: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM competition_participants WHERE competition_id=?1 AND generation=?2 AND participant_id=?3 UNION ALL SELECT 1 FROM competition_gift_participants WHERE competition_id=?1 AND generation=?2 AND participant_id=?3)",params![id,generation,input.participant_id],|row| row.get(0))?;
            if !existing {
                let participants: Option<i64> = tx.query_row("SELECT participants FROM competition_participation_counts WHERE competition_id=?1 AND generation=?2",params![id,generation],|row| row.get(0)).optional()?;
                if participants.unwrap_or(0) >= 100_000 {
                    return Ok(Some(GiftOutcome::Capacity));
                }
                tx.execute("INSERT INTO competition_gift_participants(competition_id,generation,participant_id) VALUES (?1,?2,?3)",params![id,generation,input.participant_id])?;
            }
            !existing
        } else {
            false
        };
        tx.execute("INSERT INTO competition_participation_counts(competition_id,generation,participants,contributions) VALUES (?1,?2,?3,1) ON CONFLICT(competition_id,generation) DO UPDATE SET participants=participants+excluded.participants,contributions=contributions+1", params![id,generation,i64::from(new_gift_participant)])?;
        tx.execute("INSERT INTO competition_gift_receipts(competition_id,generation,event_identity,participant_id,gift_id,group_id,observed_quantity,price) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",params![id,generation,input.event_identity,input.participant_id,input.gift_id,input.group_id,input.observed_quantity.to_string(),input.price.to_string()])?;
        if let Some(option) = credited_option {
            tx.execute("INSERT INTO competition_scored_contributions(competition_id,generation,source,event_identity,participant_id,option_id,score,created_at) VALUES (?1,?2,'tiktok.gift',?3,?4,?5,?6,?7)",params![id,generation,input.event_identity,input.participant_id,option,score.to_string(),input.received_at as i64])?;
        }
        tx.commit()?;
        Ok(Some(outcome))
    }

    /// Recovery-only bounded scan with checked arithmetic; ignored receipts do
    /// not contribute to the projection.
    pub(crate) fn competition_scored_totals(
        &self,
        id: &str,
        generation: u64,
    ) -> Result<Vec<(String, u64)>, DatabaseError> {
        let generation = i64::try_from(generation)
            .map_err(|_| DatabaseError::Invalid("invalid generation".into()))?;
        let connection = self.open(&self.automation_path())?;
        let mut statement = connection.prepare("SELECT option_id,score FROM competition_scored_contributions WHERE competition_id=?1 AND generation=?2 AND option_id IS NOT NULL LIMIT 500001")?;
        let rows = statement.query_map(params![id, generation], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut totals = std::collections::BTreeMap::<String, u64>::new();
        for (index, row) in rows.enumerate() {
            if index >= 500_000 {
                return Err(DatabaseError::Invalid(
                    "scored contribution capacity exceeded".into(),
                ));
            }
            let (option, score) = row?;
            let score: u64 = score
                .parse()
                .map_err(|_| DatabaseError::Invalid("invalid score".into()))?;
            let total = totals.entry(option).or_default();
            *total = total
                .checked_add(score)
                .ok_or_else(|| DatabaseError::Invalid("score overflow".into()))?;
            if totals.len() > 256 {
                return Err(DatabaseError::Invalid("option capacity exceeded".into()));
            }
        }
        Ok(totals.into_iter().collect())
    }
    /// Bounded restart projection: the option index supports one grouped scan
    /// per generation, never one scan per incoming event.
    pub(crate) fn competition_vote_totals(
        &self,
        id: &str,
        generation: u64,
    ) -> Result<Vec<(String, u64)>, DatabaseError> {
        let generation = i64::try_from(generation)
            .map_err(|_| DatabaseError::Invalid("invalid generation".into()))?;
        let connection = self.open(&self.automation_path())?;
        let mut statement = connection.prepare("SELECT option_id, SUM(votes) FROM competition_participants WHERE competition_id=?1 AND generation=?2 GROUP BY option_id ORDER BY option_id LIMIT 257")?;
        let rows = statement.query_map(params![id, generation], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        let mut totals = Vec::new();
        for row in rows {
            let (option, votes) = row?;
            if totals.len() == 256 || votes <= 0 {
                return Err(DatabaseError::Invalid("invalid vote projection".into()));
            }
            totals.push((option, votes as u64));
        }
        Ok(totals)
    }
    /// Freeze a bounded distinct participant set for a uniform draw. Gift-only
    /// entrants and command voters share one identity space.
    pub(crate) fn competition_draw_participants(
        &self,
        id: &str,
        generation: u64,
    ) -> Result<Vec<tiktools_competitions::Candidate>, DatabaseError> {
        let generation = i64::try_from(generation)
            .map_err(|_| DatabaseError::Invalid("invalid generation".into()))?;
        let connection = self.open(&self.automation_path())?;
        let mut statement = connection.prepare("SELECT participant_id FROM competition_participants WHERE competition_id=?1 AND generation=?2 UNION SELECT participant_id FROM competition_gift_participants WHERE competition_id=?1 AND generation=?2 ORDER BY participant_id LIMIT 100001")?;
        let rows = statement.query_map(params![id, generation], |row| row.get::<_, String>(0))?;
        let mut candidates = Vec::new();
        for row in rows {
            if candidates.len() >= 100_000 {
                return Err(DatabaseError::Invalid("draw capacity exceeded".into()));
            }
            let key = row?;
            if key.is_empty() || key.len() > 512 {
                return Err(DatabaseError::Invalid("invalid participant".into()));
            }
            candidates.push(tiktools_competitions::Candidate { key, weight: 1 });
        }
        Ok(candidates)
    }
    /// Build ticket weights from committed vote quantities and scored
    /// contributions. This bounded scan happens only at finalization.
    pub(crate) fn competition_weighted_draw_participants(
        &self,
        id: &str,
        generation: u64,
        vote_weight: u64,
    ) -> Result<Vec<tiktools_competitions::Candidate>, DatabaseError> {
        let generation = i64::try_from(generation)
            .map_err(|_| DatabaseError::Invalid("invalid generation".into()))?;
        let mut connection = self.open(&self.automation_path())?;
        let tx = connection.transaction()?;
        let mut weights = std::collections::BTreeMap::<String, u64>::new();
        {
            let mut statement = tx.prepare("SELECT participant_id,votes FROM competition_participants WHERE competition_id=?1 AND generation=?2 LIMIT 100001")?;
            let rows = statement.query_map(params![id, generation], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?;
            for row in rows {
                let (participant, votes) = row?;
                if participant.is_empty()
                    || participant.len() > 512
                    || votes <= 0
                    || weights.len() >= 100_000
                {
                    return Err(DatabaseError::Invalid("invalid draw participants".into()));
                }
                let weight = (votes as u64)
                    .checked_mul(vote_weight)
                    .ok_or_else(|| DatabaseError::Invalid("draw weight overflow".into()))?;
                weights.insert(participant, weight);
            }
        }
        {
            let mut statement = tx.prepare("SELECT participant_id FROM competition_gift_participants WHERE competition_id=?1 AND generation=?2 LIMIT 100001")?;
            let rows =
                statement.query_map(params![id, generation], |row| row.get::<_, String>(0))?;
            for row in rows {
                let participant = row?;
                if participant.is_empty()
                    || participant.len() > 512
                    || (!weights.contains_key(&participant) && weights.len() >= 100_000)
                {
                    return Err(DatabaseError::Invalid("invalid draw participants".into()));
                }
                weights.entry(participant).or_insert(0);
            }
        }
        {
            let mut statement = tx.prepare("SELECT participant_id,score FROM competition_scored_contributions WHERE competition_id=?1 AND generation=?2 AND option_id IS NOT NULL LIMIT 500001")?;
            let rows = statement.query_map(params![id, generation], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            for (index, row) in rows.enumerate() {
                if index >= 500_000 {
                    return Err(DatabaseError::Invalid(
                        "draw contribution capacity exceeded".into(),
                    ));
                }
                let (participant, score) = row?;
                let score = score
                    .parse::<u64>()
                    .map_err(|_| DatabaseError::Invalid("invalid contribution score".into()))?;
                let weight = weights
                    .get_mut(&participant)
                    .ok_or_else(|| DatabaseError::Invalid("draw participant missing".into()))?;
                *weight = weight
                    .checked_add(score)
                    .ok_or_else(|| DatabaseError::Invalid("draw weight overflow".into()))?;
            }
        }
        tx.commit()?;
        Ok(weights
            .into_iter()
            .map(|(key, weight)| tiktools_competitions::Candidate { key, weight })
            .collect())
    }
    pub(crate) fn competition_participant_count(
        &self,
        id: &str,
        generation: u64,
    ) -> Result<u64, DatabaseError> {
        let generation = i64::try_from(generation)
            .map_err(|_| DatabaseError::Invalid("invalid generation".into()))?;
        let connection = self.open(&self.automation_path())?;
        let count: Option<i64> = connection.query_row("SELECT participants FROM competition_participation_counts WHERE competition_id=?1 AND generation=?2", params![id, generation], |row| row.get(0)).optional()?;
        u64::try_from(count.unwrap_or(0))
            .map_err(|_| DatabaseError::Invalid("invalid participant count".into()))
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
        let expected_action = (!matches!(
            definition.kind,
            tiktools_competitions::CompetitionKind::RandomDraw
                | tiktools_competitions::CompetitionKind::WeightedDraw
        ))
        .then_some(result.selection.winner.as_ref())
        .flatten()
        .and_then(|winner| {
            definition
                .options
                .iter()
                .find(|option| &option.id == winner)
                .and_then(|option| option.on_win.as_ref())
        });
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
