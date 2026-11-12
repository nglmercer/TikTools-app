//! Competition finalization, manual ties and once-only result commitment. The finalizer
//! runs under its caller's competition mutation guard; public selection methods retain
//! the ranking guard as well.

use crate::{control::OperationError, recover_mutex, AppCore};

impl AppCore {
    /// Caller holds the competition mutation lock, fencing vote admission.
    #[cfg(feature = "persistence")]
    pub(super) fn finalize_command_poll(
        self: &std::sync::Arc<Self>,
        id: &str,
        manual_winner: Option<&str>,
    ) -> Result<tiktools_competitions::ResultRecord, OperationError> {
        use tiktools_competitions::{Candidate, CompetitionKind, Lifecycle, ResultRecord};
        let definition = self.competition_get(id)?;
        let state = self.competition_state(id)?;
        if let Some(result) = self
            .db
            .load_competition_result(id, state.generation)
            .map_err(|_| OperationError::unavailable("result read failed"))?
        {
            return Ok(result);
        }
        if state.lifecycle != Lifecycle::Finalizing {
            return Err(OperationError::conflict("competition is not finalizing"));
        }
        if !matches!(
            definition.kind,
            CompetitionKind::Poll
                | CompetitionKind::WeightedPoll
                | CompetitionKind::TeamBattle
                | CompetitionKind::Roulette
                | CompetitionKind::RandomDraw
                | CompetitionKind::WeightedDraw
                | CompetitionKind::LeaderboardChallenge
        ) {
            return Err(OperationError::conflict(
                "competition kind cannot yet be finalized",
            ));
        }
        let challenge = definition.kind == CompetitionKind::LeaderboardChallenge;
        let participant_draw = matches!(
            definition.kind,
            CompetitionKind::RandomDraw | CompetitionKind::WeightedDraw
        );
        if (definition.kind == CompetitionKind::RandomDraw
            && definition.selection_mode != tiktools_competitions::SelectionMode::UniformRandom)
            || (definition.kind == CompetitionKind::WeightedDraw
                && definition.selection_mode
                    != tiktools_competitions::SelectionMode::WeightedRandom)
        {
            return Err(OperationError::conflict(
                "draw selection mode does not match competition kind",
            ));
        }
        let ranking = self.ranking_get(&definition.ranking_id)?;
        // Challenges rank an ordinary (checkpointed) ranking; every other
        // kind requires the durable command-vote projection.
        if challenge
            == ranking
                .sources
                .iter()
                .any(|source| source.source == "competition.vote")
        {
            return Err(OperationError::conflict(if challenge {
                "leaderboard challenges require an ordinary ranking"
            } else {
                "competition ranking is not a durable vote projection"
            }));
        }
        let participants = if challenge {
            recover_mutex(&self.aggregations.engine, "aggregations")
                .selection_snapshot(&definition.ranking_id)
                .map(|snapshot| snapshot.total_participants as u64)
                .unwrap_or(0)
        } else {
            self.db
                .competition_participant_count(id, state.generation)
                .map_err(|_| OperationError::unavailable("participant count unavailable"))?
        };
        if participants < definition.minimum_participants {
            return Err(OperationError::conflict(
                "minimum participation not reached",
            ));
        }
        let snapshot = recover_mutex(&self.aggregations.engine, "aggregations")
            .selection_snapshot(&definition.ranking_id)
            .ok_or_else(|| OperationError::not_found("ranking not found"))?;
        if snapshot.integrity_status == "degraded" {
            return Err(OperationError::conflict("ranking integrity is degraded"));
        }
        let candidates = if definition.kind == CompetitionKind::WeightedDraw {
            let vote_weight = ranking
                .sources
                .iter()
                .filter(|rule| rule.enabled && rule.source == "competition.vote")
                .try_fold(0u64, |sum, rule| sum.checked_add(rule.weight))
                .ok_or_else(|| OperationError::conflict("draw weight overflow"))?;
            self.db
                .competition_weighted_draw_participants(id, state.generation, vote_weight)
                .map_err(|_| OperationError::unavailable("draw weights unavailable"))?
        } else if participant_draw {
            self.db
                .competition_draw_participants(id, state.generation)
                .map_err(|_| OperationError::unavailable("draw participants unavailable"))?
        } else {
            snapshot
                .entries
                .into_iter()
                .filter(|entry| {
                    definition
                        .target_score
                        .is_none_or(|target| entry.score >= target)
                })
                .map(|entry| Candidate {
                    key: entry.key,
                    weight: entry.score,
                })
                .collect()
        };
        let selection = select_candidates(&definition, candidates, manual_winner)?;
        let user_winner = definition.has_user_winner();
        let action = selection
            .winner
            .as_deref()
            .and_then(|winner| definition.winner_action(winner));
        let result = ResultRecord {
            schema_version: 1,
            competition_id: id.to_owned(),
            generation: state.generation,
            ranking_id: definition.ranking_id.clone(),
            ranking_revision: snapshot.revision,
            finalized_at: crate::now_millis(),
            selection,
        };
        let (stored, inserted) = self
            .db
            .store_competition_result_once(
                &result,
                action,
                self.aggregations.checkpoint_owner.as_deref(),
            )
            .map_err(|error| {
                tracing::error!(%error, "competition finalization failed");
                OperationError::unavailable("competition finalization failed")
            })?;
        if inserted {
            if let Some(mut data) = self.competition_context(id, stored.generation) {
                data.selection_mode = Some(stored.selection.mode);
                data.selection_id =
                    serde_json::to_string(&("competition-winner", id, stored.generation)).ok();
                data.winner_id = stored.selection.winner.clone();
                data.option_id = if user_winner {
                    None
                } else {
                    data.winner_id.clone()
                };
                data.winner_score = data
                    .winner_id
                    .as_ref()
                    .and_then(|winner| {
                        stored
                            .selection
                            .eligible
                            .iter()
                            .find(|candidate| &candidate.key == winner)
                    })
                    .map(|candidate| candidate.weight);
                data.action_id = action.map(|action| action.action_id.clone());
                self.publish_competition_event("competition.finished", data.clone());
                if data.winner_id.is_some() {
                    self.publish_competition_event("competition.winner.selected", data);
                }
            }
        }
        Ok(stored)
    }
    pub fn competition_finish(
        self: &std::sync::Arc<Self>,
        id: &str,
    ) -> Result<tiktools_competitions::ResultRecord, OperationError> {
        let _ranking = recover_mutex(&self.aggregations.mutations, "ranking mutations");
        let _mutation = recover_mutex(&self.competitions.mutations, "competition mutations");
        #[cfg(feature = "persistence")]
        {
            let definition = self.competition_get(id)?;
            let ranking = self.ranking_get(&definition.ranking_id)?;
            if !matches!(
                definition.kind,
                tiktools_competitions::CompetitionKind::Poll
                    | tiktools_competitions::CompetitionKind::WeightedPoll
                    | tiktools_competitions::CompetitionKind::TeamBattle
                    | tiktools_competitions::CompetitionKind::Roulette
                    | tiktools_competitions::CompetitionKind::RandomDraw
                    | tiktools_competitions::CompetitionKind::WeightedDraw
                    | tiktools_competitions::CompetitionKind::LeaderboardChallenge
            ) || (definition.kind
                == tiktools_competitions::CompetitionKind::LeaderboardChallenge)
                == ranking
                    .sources
                    .iter()
                    .any(|source| source.source == "competition.vote")
            {
                return Err(OperationError::conflict(
                    "competition kind or ranking cannot yet be finalized",
                ));
            }
            let state = self.competition_state(id)?;
            if matches!(
                state.lifecycle,
                tiktools_competitions::Lifecycle::Active | tiktools_competitions::Lifecycle::Paused
            ) {
                self.db
                    .transition_competition_owned(
                        id,
                        tiktools_competitions::Transition::Finish,
                        crate::now_millis(),
                        self.aggregations.checkpoint_owner.as_deref(),
                    )
                    .map_err(|_| OperationError::unavailable("competition finish failed"))?;
            }
            self.finalize_command_poll(id, None)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = id;
            Err(OperationError::unavailable(
                "competition finalization requires persistence",
            ))
        }
    }
    /// Read only tied leaders after the competition has closed. This uses the
    /// full bounded ranking, not its display limit, and reveals no random state.
    pub fn competition_manual_candidates(
        &self,
        id: &str,
    ) -> Result<(u64, Vec<tiktools_aggregation::Entry>), OperationError> {
        let _ranking = recover_mutex(&self.aggregations.mutations, "ranking mutations");
        let _mutation = recover_mutex(&self.competitions.mutations, "competition mutations");
        let definition = self.competition_get(id)?;
        let state = self.competition_state(id)?;
        if state.lifecycle != tiktools_competitions::Lifecycle::Finalizing
            || definition.selection_mode != tiktools_competitions::SelectionMode::HighestScore
            || definition.tie_policy != tiktools_competitions::TiePolicy::Manual
        {
            return Err(OperationError::conflict(
                "competition is not awaiting a manual tie",
            ));
        }
        let ranking = self.ranking_get(&definition.ranking_id)?;
        if !matches!(
            definition.kind,
            tiktools_competitions::CompetitionKind::Poll
                | tiktools_competitions::CompetitionKind::WeightedPoll
                | tiktools_competitions::CompetitionKind::TeamBattle
                | tiktools_competitions::CompetitionKind::Roulette
                | tiktools_competitions::CompetitionKind::LeaderboardChallenge
        ) || (definition.kind == tiktools_competitions::CompetitionKind::LeaderboardChallenge)
            == ranking
                .sources
                .iter()
                .any(|source| source.source == "competition.vote")
        {
            return Err(OperationError::conflict(
                "competition ranking cannot yet be finalized",
            ));
        }
        let snapshot = recover_mutex(&self.aggregations.engine, "aggregations")
            .selection_snapshot(&definition.ranking_id)
            .ok_or_else(|| OperationError::not_found("ranking not found"))?;
        #[cfg(feature = "persistence")]
        if (if definition.kind == tiktools_competitions::CompetitionKind::LeaderboardChallenge {
            snapshot.total_participants as u64
        } else {
            self.db
                .competition_participant_count(id, state.generation)
                .map_err(|_| OperationError::unavailable("participant count unavailable"))?
        }) < definition.minimum_participants
        {
            return Err(OperationError::conflict(
                "minimum participation not reached",
            ));
        }
        if snapshot.integrity_status == "degraded" {
            return Err(OperationError::conflict("ranking integrity is degraded"));
        }
        let highest = snapshot
            .entries
            .first()
            .map(|entry| entry.score)
            .ok_or_else(|| OperationError::conflict("no eligible entries"))?;
        let leaders: Vec<_> = snapshot
            .entries
            .into_iter()
            .take_while(|entry| entry.score == highest)
            .collect();
        if leaders.len() < 2 {
            return Err(OperationError::conflict("no manual tie"));
        }
        Ok((state.generation, leaders))
    }
    pub fn competition_select_winner(
        self: &std::sync::Arc<Self>,
        id: &str,
        winner: &str,
        generation: u64,
    ) -> Result<tiktools_competitions::ResultRecord, OperationError> {
        if winner.is_empty() || winner.len() > 512 {
            return Err(OperationError::invalid("invalid winner"));
        }
        let _ranking = recover_mutex(&self.aggregations.mutations, "ranking mutations");
        let _mutation = recover_mutex(&self.competitions.mutations, "competition mutations");
        #[cfg(feature = "persistence")]
        {
            if self.competition_state(id)?.generation != generation {
                return Err(OperationError::conflict("competition generation changed"));
            }
            self.finalize_command_poll(id, Some(winner))
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (id, generation);
            Err(OperationError::unavailable(
                "competition finalization requires persistence",
            ))
        }
    }
}

#[cfg(feature = "persistence")]
fn select_candidates(
    definition: &tiktools_competitions::Definition,
    candidates: Vec<tiktools_competitions::Candidate>,
    manual_winner: Option<&str>,
) -> Result<tiktools_competitions::Selection, OperationError> {
    match manual_winner {
        Some(winner)
            if definition.selection_mode == tiktools_competitions::SelectionMode::HighestScore
                && definition.tie_policy == tiktools_competitions::TiePolicy::Manual =>
        {
            tiktools_competitions::select_manual(candidates, winner)
        }
        Some(_) => {
            return Err(OperationError::conflict(
                "manual selection requires a manual highest-score tie",
            ))
        }
        // A challenge nobody reached closes with an explicit no-winner result.
        None if definition.kind == tiktools_competitions::CompetitionKind::LeaderboardChallenge
            && candidates.is_empty() =>
        {
            Ok(tiktools_competitions::Selection {
                mode: definition.selection_mode,
                tie_policy: definition.tie_policy,
                eligible: Vec::new(),
                winner: None,
            })
        }
        None => tiktools_competitions::select(
            candidates,
            definition.selection_mode,
            definition.tie_policy,
        ),
    }
    .map_err(|error| OperationError::conflict(format!("winner selection unavailable: {error:?}")))
}
