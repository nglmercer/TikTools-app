//! Competition definition operations. The host owns the authoritative state.
use super::OperationError;
use crate::{recover_mutex, AppCore};
use tiktools_competitions::Definition;

/// Presentation transports have a per-message limit. Preserve accounting
/// totals and order while dropping optional artwork/identity first, then
/// reducing the visible prefix if unusually escaped text remains too large.
pub(crate) fn bound_ranking_presentation(
    mut snapshot: tiktools_aggregation::Snapshot,
) -> tiktools_aggregation::Snapshot {
    const BUDGET: usize = 512 * 1024;
    let fits = |value: &tiktools_aggregation::Snapshot| {
        serde_json::to_vec(value).is_ok_and(|bytes| bytes.len() <= BUDGET)
    };
    if fits(&snapshot) {
        return snapshot;
    }
    for entry in &mut snapshot.entries {
        entry.metadata = Default::default();
    }
    if fits(&snapshot) {
        return snapshot;
    }
    while !snapshot.entries.is_empty() && !fits(&snapshot) {
        snapshot.entries.truncate(snapshot.entries.len() / 2);
    }
    snapshot
}

impl AppCore {
    /// Starts one host-owned deadline worker. It uses weak ownership so it
    /// cannot keep an otherwise stopped AppCore alive.
    pub fn start_competition_timer(self: &std::sync::Arc<Self>) {
        use std::sync::atomic::Ordering;
        if tokio::runtime::Handle::try_current().is_err()
            || self.competitions.timer_started.swap(true, Ordering::AcqRel)
        {
            return;
        }
        #[cfg(feature = "persistence")]
        self.start_competition_action_worker();
        #[cfg(feature = "persistence")]
        crate::services::aggregations::AggregationService::start_checkpoints(self);
        let weak = std::sync::Arc::downgrade(self);
        let interval_ms = std::env::var("TIKTOOLS_RANKING_PUBLISH_INTERVAL_MS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(250)
            .clamp(50, 1000);
        tokio::spawn(async move {
            let mut published = std::collections::HashMap::<String, (u64, u64)>::new();
            // Gateway observers are lossy and cannot invoke privileged RPCs.
            // Periodic full refresh allows read-only widgets to recover even
            // when no score changes after they reconnect.
            let mut refreshed = std::time::Instant::now();
            #[cfg(feature = "native-tiktok")]
            let mut last_likes: Option<(String, String, u64)> = None;
            loop {
                let Some(core) = weak.upgrade() else {
                    break;
                };
                if core.shutdown_started.load(Ordering::Acquire) {
                    break;
                }
                let refresh_due = refreshed.elapsed() >= std::time::Duration::from_secs(5);
                let snapshots = {
                    let mut engine =
                        crate::recover_mutex(&core.aggregations.engine, "aggregations");
                    engine.expire_at(crate::now_millis());
                    if refresh_due {
                        published.clear();
                        refreshed = std::time::Instant::now();
                    }
                    if !core.events.has_domain_subscribers() {
                        published.clear();
                        Vec::new()
                    } else {
                        let tokens = engine.change_tokens();
                        published.retain(|id, _| tokens.iter().any(|(active, _, _)| active == id));
                        let mut snapshots = Vec::new();
                        for (id, instance, revision) in tokens {
                            if published.get(&id) != Some(&(instance, revision)) {
                                // Browser widgets render at most 100 rows; keep
                                // presentation traffic bounded independently of
                                // the complete control API and selection state.
                                if let Some(snapshot) = engine.snapshot_unranked_capped(&id, 100) {
                                    snapshots.push(snapshot);
                                    published.insert(id, (instance, revision));
                                }
                            }
                        }
                        snapshots
                    }
                };
                // Sorting and publication never run under the ingestion lock.
                for (snapshot, limit) in snapshots {
                    core.events
                        .publish_domain(crate::events::DomainEvent::RankingUpdated {
                            snapshot: bound_ranking_presentation(snapshot.rank(limit)),
                        });
                }
                #[cfg(feature = "native-tiktok")]
                if refresh_due && core.events.has_domain_subscribers() {
                    // Hold the context read guard through publication so a
                    // room change cannot publish an older room after this one.
                    let context =
                        crate::recover_rwlock_read(&core.connection_context, "connection context");
                    let (creator_id, room_id, leaders) = match context.as_ref() {
                        Some(context) => (
                            context.unique_id.clone(),
                            context.room_id.clone(),
                            core.live_totals.like_leaderboard(context),
                        ),
                        None => (String::new(), String::new(), Vec::new()),
                    };
                    core.events
                        .publish_domain(crate::events::DomainEvent::LiveLikesRanking {
                            creator_id,
                            room_id,
                            leaders,
                        });
                }
                #[cfg(feature = "native-tiktok")]
                if core.events.has_domain_subscribers() {
                    let context =
                        crate::recover_rwlock_read(&core.connection_context, "connection context");
                    let (creator, room) = context
                        .as_ref()
                        .map(|c| (c.unique_id.clone(), c.room_id.clone()))
                        .unwrap_or_default();
                    let revision = core.live_totals.like_revision();
                    let token = (creator.clone(), room.clone(), revision);
                    if refresh_due || last_likes.as_ref() != Some(&token) {
                        let leaders = context
                            .as_ref()
                            .map(|c| core.live_totals.like_leaderboard_with_limit(c, 500).1)
                            .unwrap_or_default();
                        core.events.publish_domain(
                            crate::events::DomainEvent::LiveLikesRankingFull {
                                creator_id: creator,
                                room_id: room,
                                leaders,
                                integrity_status: context
                                    .as_ref()
                                    .map(|context| core.live_totals.like_integrity_status(context))
                                    .unwrap_or("unknown")
                                    .into(),
                            },
                        );
                        last_likes = Some(token);
                    }
                } else {
                    last_likes = None;
                }
                #[cfg(feature = "persistence")]
                let expiry_core = std::sync::Arc::clone(&core);
                drop(core);
                #[cfg(feature = "persistence")]
                match tokio::task::spawn_blocking(move || {
                    expiry_core.competition_timer_tick(refresh_due)
                })
                .await
                {
                    Ok(Ok(_)) => {}
                    Ok(Err(error)) => tracing::error!(%error, "competition expiry failed"),
                    Err(error) => tracing::error!(%error, "competition timer worker failed"),
                }
                tokio::time::sleep(std::time::Duration::from_millis(interval_ms)).await;
            }
        });
    }
    #[cfg(feature = "persistence")]
    pub(crate) fn competition_timer_tick(
        self: &std::sync::Arc<Self>,
        refresh_due: bool,
    ) -> Result<(), crate::db::DatabaseError> {
        let _mutation = crate::recover_mutex(&self.competitions.mutations, "competition mutations");
        let Some(owner) = self.aggregations.checkpoint_owner.as_deref() else {
            return Err(crate::db::DatabaseError::Invalid(
                "competition integrity owner unavailable".into(),
            ));
        };
        self.db
            .expire_due_competitions_owned(crate::now_millis(), Some(owner))?;
        for id in self.db.finalizing_competitions()? {
            // Unsupported or incomplete competitions stay in
            // finalizing for explicit inspection or recovery.
            if let Err(error) = self.finalize_command_poll(&id, None) {
                tracing::debug!(competition_id = %id, message = %error.message, "competition awaiting finalization");
            }
        }
        // Presentation refresh must not block host-owned deadlines
        // or winner finalization when a stored result is corrupt.
        if refresh_due && self.events.has_domain_subscribers() {
            match self.db.latest_competition_results_for_presentation() {
                Ok(results) => {
                    for result in results {
                        match result.reveal_snapshot() {
                            Ok(result) => self.events.publish_domain(
                                crate::events::DomainEvent::CompetitionResult { result },
                            ),
                            Err(error) => {
                                tracing::error!(?error, competition_id = %result.competition_id, "invalid committed reveal")
                            }
                        }
                    }
                }
                Err(error) => tracing::error!(%error, "competition result refresh failed"),
            }
        }
        Ok(())
    }
    /// Caller holds the competition mutation lock, fencing vote admission.
    #[cfg(feature = "persistence")]
    fn finalize_command_poll(
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
        let selection = match manual_winner {
            Some(winner)
                if definition.selection_mode
                    == tiktools_competitions::SelectionMode::HighestScore
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
            None if challenge && candidates.is_empty() => Ok(tiktools_competitions::Selection {
                mode: definition.selection_mode,
                tie_policy: definition.tie_policy,
                eligible: Vec::new(),
                winner: None,
            }),
            None => tiktools_competitions::select(
                candidates,
                definition.selection_mode,
                definition.tie_policy,
            ),
        }
        .map_err(|error| {
            OperationError::conflict(format!("winner selection unavailable: {error:?}"))
        })?;
        let user_winner = participant_draw || challenge;
        let action = if user_winner {
            selection
                .winner
                .as_ref()
                .and(definition.on_user_win.as_ref())
        } else {
            selection
                .winner
                .as_ref()
                .and_then(|winner| {
                    definition
                        .options
                        .iter()
                        .find(|option| &option.id == winner)
                })
                .and_then(|option| option.on_win.as_ref())
        };
        let result = ResultRecord {
            schema_version: 1,
            competition_id: id.to_owned(),
            generation: state.generation,
            ranking_id: definition.ranking_id,
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
    pub fn competition_create(&self, definition: Definition) -> Result<(), OperationError> {
        definition
            .validate()
            .map_err(|_| OperationError::invalid("invalid or ambiguous competition definition"))?;
        // The shared lock order prevents a ranking being deleted while a
        // competition referencing it is being created.
        let _ranking = recover_mutex(&self.aggregations.mutations, "ranking mutations");
        let _mutation = recover_mutex(&self.competitions.mutations, "competition mutations");
        if self.competitions.load_failed {
            return Err(OperationError::unavailable(
                "competition definitions unavailable",
            ));
        }
        let ranking = self
            .ranking_get(&definition.ranking_id)
            .map_err(|_| OperationError::invalid("ranking not found"))?;
        if definition.kind == tiktools_competitions::CompetitionKind::LeaderboardChallenge
            && ranking
                .sources
                .iter()
                .any(|source| source.source == "competition.vote")
        {
            return Err(OperationError::invalid(
                "leaderboard challenges require an ordinary ranking",
            ));
        }
        if ranking
            .sources
            .iter()
            .any(|source| source.source == "competition.vote")
            && self.ranking_snapshot(&ranking.id)?.total_score != 0
        {
            return Err(OperationError::conflict(
                "reset the vote ranking before creating a competition",
            ));
        }
        {
            let definitions =
                recover_mutex(&self.competitions.definitions, "competition definitions");
            if definitions.contains_key(&definition.id) {
                return Err(OperationError::conflict("competition already exists"));
            }
            if definitions.len() >= 128 {
                return Err(OperationError::unavailable(
                    "competition capacity exhausted",
                ));
            }
        }
        let mut candidate =
            recover_mutex(&self.competitions.definitions, "competition definitions").clone();
        candidate.insert(definition.id.clone(), definition.clone());
        let index = crate::services::competitions::CompetitionService::build_index(
            &candidate,
            &self.ranking_list(),
        )
        .map_err(|_| OperationError::invalid("competition command index capacity exceeded"))?;
        #[cfg(feature = "persistence")]
        self.db
            .insert_competition_definition(&definition)
            .map_err(|error| {
                tracing::error!(%error, "competition persistence failed");
                OperationError::unavailable("competition persistence failed")
            })?;
        #[cfg(not(feature = "persistence"))]
        {
            let _ = index;
            return Err(OperationError::unavailable(
                "competition creation requires persistence",
            ));
        }
        #[cfg(feature = "persistence")]
        {
            *recover_mutex(&self.competitions.definitions, "competition definitions") = candidate;
            *recover_mutex(
                &self.competitions.command_index,
                "competition command index",
            ) = index;
            Ok(())
        }
    }
    pub fn competition_update(&self, definition: Definition) -> Result<(), OperationError> {
        definition
            .validate()
            .map_err(|_| OperationError::invalid("invalid or ambiguous competition definition"))?;
        let _ranking = recover_mutex(&self.aggregations.mutations, "ranking mutations");
        let _mutation = recover_mutex(&self.competitions.mutations, "competition mutations");
        if self.competitions.load_failed {
            return Err(OperationError::unavailable(
                "competition definitions unavailable",
            ));
        }
        let ranking = self
            .ranking_get(&definition.ranking_id)
            .map_err(|_| OperationError::invalid("ranking not found"))?;
        if ranking
            .sources
            .iter()
            .any(|source| source.source == "competition.vote")
            && self.ranking_snapshot(&ranking.id)?.total_score != 0
        {
            return Err(OperationError::conflict(
                "reset the vote ranking before updating a competition",
            ));
        }
        if !recover_mutex(&self.competitions.definitions, "competition definitions")
            .contains_key(&definition.id)
        {
            return Err(OperationError::not_found("competition not found"));
        }
        let mut candidate =
            recover_mutex(&self.competitions.definitions, "competition definitions").clone();
        candidate.insert(definition.id.clone(), definition.clone());
        let index = crate::services::competitions::CompetitionService::build_index(
            &candidate,
            &self.ranking_list(),
        )
        .map_err(|_| OperationError::invalid("competition command index capacity exceeded"))?;
        #[cfg(feature = "persistence")]
        match self.db.update_competition_definition(&definition) {
            Ok(true) => {}
            Ok(false) => {
                return Err(OperationError::conflict(
                    "competition has results or storage changed",
                ))
            }
            Err(error) => {
                tracing::error!(%error, "competition update failed");
                return Err(OperationError::unavailable("competition update failed"));
            }
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = index;
            return Err(OperationError::unavailable(
                "competition update requires persistence",
            ));
        }
        #[cfg(feature = "persistence")]
        {
            *recover_mutex(&self.competitions.definitions, "competition definitions") = candidate;
            *recover_mutex(
                &self.competitions.command_index,
                "competition command index",
            ) = index;
            Ok(())
        }
    }
    pub fn competition_delete(&self, id: &str) -> Result<(), OperationError> {
        let _ranking = recover_mutex(&self.aggregations.mutations, "ranking mutations");
        let _mutation = recover_mutex(&self.competitions.mutations, "competition mutations");
        if self.competitions.load_failed {
            return Err(OperationError::unavailable(
                "competition definitions unavailable",
            ));
        }
        if !recover_mutex(&self.competitions.definitions, "competition definitions")
            .contains_key(id)
        {
            return Err(OperationError::not_found("competition not found"));
        }
        #[cfg(feature = "persistence")]
        match self.db.delete_competition_definition(id) {
            Ok(true) => {}
            Ok(false) => {
                return Err(OperationError::conflict(
                    "competition has results or storage changed",
                ))
            }
            Err(error) => {
                tracing::error!(%error, "competition deletion failed");
                return Err(OperationError::unavailable("competition deletion failed"));
            }
        }
        #[cfg(not(feature = "persistence"))]
        return Err(OperationError::unavailable(
            "competition deletion requires persistence",
        ));
        #[cfg(feature = "persistence")]
        {
            recover_mutex(&self.competitions.definitions, "competition definitions").remove(id);
            recover_mutex(
                &self.competitions.command_index,
                "competition command index",
            )
            .remove(id);
            Ok(())
        }
    }
    #[cfg(feature = "persistence")]
    pub fn competition_state(
        &self,
        id: &str,
    ) -> Result<tiktools_competitions::LifecycleState, OperationError> {
        if self.competitions.load_failed {
            return Err(OperationError::unavailable(
                "competition definitions unavailable",
            ));
        }
        self.db
            .load_competition_state(id)
            .map_err(|error| {
                tracing::error!(%error, "competition state read failed");
                OperationError::unavailable("competition state read failed")
            })?
            .ok_or_else(|| OperationError::not_found("competition not found"))
    }
    #[cfg(not(feature = "persistence"))]
    pub fn competition_state(
        &self,
        _id: &str,
    ) -> Result<tiktools_competitions::LifecycleState, OperationError> {
        Err(OperationError::unavailable(
            "competition state requires persistence",
        ))
    }
    pub fn competition_transition(
        self: &std::sync::Arc<Self>,
        id: &str,
        transition: tiktools_competitions::Transition,
    ) -> Result<tiktools_competitions::LifecycleState, OperationError> {
        let _ranking = recover_mutex(&self.aggregations.mutations, "ranking mutations");
        let _mutation = recover_mutex(&self.competitions.mutations, "competition mutations");
        if self.competitions.load_failed {
            return Err(OperationError::unavailable(
                "competition definitions unavailable",
            ));
        }
        if matches!(
            transition,
            tiktools_competitions::Transition::Start | tiktools_competitions::Transition::Resume
        ) {
            let definition = self.competition_get(id)?;
            if self.ranking_snapshot(&definition.ranking_id)?.paused {
                return Err(OperationError::conflict(
                    "resume the competition ranking first",
                ));
            }
        }
        #[cfg(feature = "persistence")]
        {
            if matches!(transition, tiktools_competitions::Transition::Start)
                && self.aggregations.checkpoint_owner.is_none()
            {
                return Err(OperationError::unavailable(
                    "competition integrity owner unavailable",
                ));
            }
            let state = self
                .db
                .transition_competition_owned(
                    id,
                    transition,
                    crate::now_millis(),
                    self.aggregations.checkpoint_owner.as_deref(),
                )
                .map_err(|error| match error {
                    crate::db::DatabaseError::Invalid(_) => {
                        OperationError::conflict("invalid competition transition")
                    }
                    error => {
                        tracing::error!(%error, "competition transition failed");
                        OperationError::unavailable("competition transition failed")
                    }
                })?
                .ok_or_else(|| OperationError::not_found("competition not found"))?;
            let event_type = match transition {
                tiktools_competitions::Transition::Start => Some("competition.started"),
                tiktools_competitions::Transition::Pause => Some("competition.paused"),
                tiktools_competitions::Transition::Resume => Some("competition.resumed"),
                tiktools_competitions::Transition::Cancel => Some("competition.cancelled"),
                _ => None,
            };
            if let (Some(kind), Some(data)) =
                (event_type, self.competition_context(id, state.generation))
            {
                self.publish_competition_event(kind, data);
            }
            Ok(state)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (id, transition);
            Err(OperationError::unavailable(
                "competition state requires persistence",
            ))
        }
    }
    pub fn competition_action(
        &self,
        id: &str,
        generation: u64,
    ) -> Result<Option<tiktools_competitions::ActionExecution>, OperationError> {
        self.competition_get(id)?;
        if generation > i64::MAX as u64 {
            return Err(OperationError::invalid("invalid generation"));
        }
        #[cfg(feature = "persistence")]
        return self.db.competition_action(id, generation).map_err(|error| {
            tracing::error!(%error, "competition action status read failed");
            OperationError::unavailable("competition action status read failed")
        });
        #[cfg(not(feature = "persistence"))]
        Err(OperationError::unavailable(
            "competition actions require persistence",
        ))
    }
    /// Explicitly mark a stranded running action uncertain; never requeue it.
    pub fn competition_mark_action_uncertain(
        &self,
        id: &str,
        generation: u64,
    ) -> Result<(), OperationError> {
        self.competition_get(id)?;
        if generation > i64::MAX as u64 {
            return Err(OperationError::invalid("invalid generation"));
        }
        #[cfg(feature = "persistence")]
        {
            // Hold the same guard used by the worker from before claiming
            // until execution ends, preventing recovery of a live invocation.
            let in_flight = recover_mutex(
                &self.competitions.action_inflight,
                "competition action in flight",
            );
            if in_flight
                .as_ref()
                .is_some_and(|(active_id, active_generation)| {
                    active_id == id && *active_generation == generation
                })
            {
                return Err(OperationError::conflict(
                    "competition action is still executing",
                ));
            }
            let changed = self
                .db
                .mark_competition_action_uncertain(
                    id,
                    generation,
                    crate::now_millis().min(i64::MAX as u64) as i64,
                )
                .map_err(|_| OperationError::unavailable("competition action recovery failed"))?;
            if !changed {
                return Err(OperationError::conflict("action is not running"));
            }
            Ok(())
        }
        #[cfg(not(feature = "persistence"))]
        Err(OperationError::unavailable(
            "competition actions require persistence",
        ))
    }
    pub fn competition_resolve_action(
        &self,
        id: &str,
        generation: u64,
        resolution: tiktools_competitions::ActionResolution,
    ) -> Result<(), OperationError> {
        self.competition_get(id)?;
        if generation > i64::MAX as u64 {
            return Err(OperationError::invalid("invalid generation"));
        }
        #[cfg(feature = "persistence")]
        {
            let updated = self
                .db
                .resolve_uncertain_competition_action(
                    id,
                    generation,
                    resolution,
                    crate::now_millis(),
                )
                .map_err(|error| {
                    tracing::error!(%error, "competition action resolution failed");
                    OperationError::unavailable("competition action resolution failed")
                })?;
            if !updated {
                return Err(OperationError::conflict("action is not uncertain"));
            }
            Ok(())
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = resolution;
            Err(OperationError::unavailable(
                "competition actions require persistence",
            ))
        }
    }
    /// Read committed result history. Pages are bounded and ordered by
    /// generation, so reconnecting clients can resume without rerunning draws.
    pub fn competition_results(
        &self,
        id: &str,
        before_generation: Option<u64>,
        limit: usize,
    ) -> Result<(Vec<tiktools_competitions::ResultRecord>, Option<u64>), OperationError> {
        if !(1..=20).contains(&limit)
            || before_generation.is_some_and(|value| value > i64::MAX as u64)
        {
            return Err(OperationError::invalid("invalid result page"));
        }
        self.competition_get(id)?;
        #[cfg(feature = "persistence")]
        {
            let mut results = self
                .db
                .list_competition_results(id, before_generation, limit + 1)
                .map_err(|error| {
                    tracing::error!(%error, "competition results read failed");
                    OperationError::unavailable("competition results read failed")
                })?;
            let next = if results.len() > limit {
                results.truncate(limit);
                results.last().map(|result| result.generation)
            } else {
                None
            };
            Ok((results, next))
        }
        #[cfg(not(feature = "persistence"))]
        Err(OperationError::unavailable(
            "competition results require persistence",
        ))
    }
    pub fn competition_list(&self) -> Result<Vec<Definition>, OperationError> {
        if self.competitions.load_failed {
            return Err(OperationError::unavailable(
                "competition definitions unavailable",
            ));
        }
        Ok(
            recover_mutex(&self.competitions.definitions, "competition definitions")
                .values()
                .cloned()
                .collect(),
        )
    }
    pub fn competition_get(&self, id: &str) -> Result<Definition, OperationError> {
        if self.competitions.load_failed {
            return Err(OperationError::unavailable(
                "competition definitions unavailable",
            ));
        }
        recover_mutex(&self.competitions.definitions, "competition definitions")
            .get(id)
            .cloned()
            .ok_or_else(|| OperationError::not_found("competition not found"))
    }
}
