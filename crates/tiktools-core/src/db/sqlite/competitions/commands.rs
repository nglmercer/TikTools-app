//! Transactional command votes and durable receipts for ignored commands.
use super::{DatabaseError, DatabaseManager};
use rusqlite::{params, OptionalExtension};
use tiktools_competitions::{
    CommandParticipation, Definition, Lifecycle, LifecycleState, ParticipationOutcome,
};

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

impl DatabaseManager {
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
}
