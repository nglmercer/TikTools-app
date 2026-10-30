//! State owned by AppCore; native ingestion and control operations share it.
use std::sync::Mutex;
use tiktools_aggregation::engine::Engine;
#[cfg(feature = "native-tiktok")]
use tiktools_aggregation::model::Contribution;

#[derive(Default)]
pub(crate) struct AggregationService {
    pub(crate) engine: Mutex<Engine>,
    pub(crate) mutations: Mutex<()>,
    #[cfg(feature = "persistence")]
    pub(crate) checkpoint_owner: Option<String>,
    /// Change token `(instance, revision)` of the last durable write per
    /// ranking. Guarded in practice by `mutations`.
    #[cfg(feature = "persistence")]
    persisted: Mutex<std::collections::HashMap<String, (u64, u64)>>,
    #[cfg(feature = "persistence")]
    pub(crate) checkpoint_started: std::sync::atomic::AtomicBool,
    /// Without persistence, external idempotency keys are kept in memory.
    #[cfg(not(feature = "persistence"))]
    pub(crate) external_seen: Mutex<ExternalSeen>,
}

/// Bounded FIFO set of recently admitted external contribution keys.
#[cfg(not(feature = "persistence"))]
#[derive(Default)]
pub(crate) struct ExternalSeen {
    keys: std::collections::HashSet<String>,
    order: std::collections::VecDeque<String>,
}

#[cfg(not(feature = "persistence"))]
impl ExternalSeen {
    pub(crate) fn admit(&mut self, key: String) -> bool {
        if !self.keys.insert(key.clone()) {
            return false;
        }
        self.order.push_back(key);
        if self.order.len() > 100_000 {
            if let Some(old) = self.order.pop_front() {
                self.keys.remove(&old);
            }
        }
        true
    }
}

impl AggregationService {
    #[cfg(feature = "persistence")]
    pub(crate) fn restore(db: &crate::db::DatabaseManager) -> Self {
        let mut bytes = [0u8; 16];
        let claim = getrandom::fill(&mut bytes)
            .ok()
            .map(|()| {
                bytes
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            })
            .and_then(|owner| match db.claim_aggregation_checkpoints(&owner) {
                Ok(clean) => Some((owner, clean)),
                Err(error) => {
                    tracing::error!(%error, "cannot claim aggregation checkpoints");
                    None
                }
            });
        let previous_clean = claim.as_ref().is_some_and(|(_, clean)| *clean);
        let checkpoint_owner = claim.map(|(owner, _)| owner);
        let mut engine = Engine::default();
        match db.load_aggregation_definitions() {
            Ok(definitions) => {
                for definition in definitions {
                    if let Err(error) = engine.create(definition) {
                        tracing::error!(?error, "cannot restore ranking definition");
                    }
                }
            }
            Err(error) => tracing::error!(%error, "cannot load ranking definitions"),
        }
        // Restore the newest checkpoint before admission begins. After an
        // unclean exit it lags the lost run, so the ranking is marked degraded. Vote
        // rankings are rebuilt separately from their authoritative ledger.
        {
            for definition in engine.definitions() {
                if definition
                    .sources
                    .iter()
                    .any(|source| source.source == "competition.vote")
                {
                    continue;
                }
                let recovered = match checkpoint_owner
                    .as_deref()
                    .map(|owner| db.load_aggregation_checkpoint(&definition.id, owner))
                {
                    None => false,
                    Some(Ok(Some(checkpoint))) => {
                        if let Err(error) = engine.restore_session_checkpoint(&checkpoint) {
                            tracing::error!(?error, ranking = %definition.id, "invalid aggregation checkpoint");
                            false
                        } else {
                            true
                        }
                    }
                    Some(Ok(None)) => false,
                    Some(Err(error)) => {
                        tracing::error!(%error, ranking = %definition.id, "cannot recover aggregation checkpoint");
                        false
                    }
                };
                if !previous_clean || !recovered {
                    if let Err(error) = engine.mark_recovery_gap(&definition.id) {
                        tracing::error!(?error, ranking = %definition.id, "cannot mark ranking recovery gap");
                    }
                }
            }
        }
        // Command-vote rankings are reconstructed from committed participant assignments.
        match db.load_competition_definitions() {
            Ok(competitions) => {
                let mut used = std::collections::HashSet::new();
                for competition in competitions {
                    let Some(ranking) = engine
                        .definitions()
                        .into_iter()
                        .find(|r| r.id == competition.ranking_id)
                    else {
                        continue;
                    };
                    let vote_ranking = ranking
                        .sources
                        .iter()
                        .any(|source| source.source == "competition.vote");
                    if vote_ranking && !used.insert(ranking.id.clone()) {
                        tracing::error!(ranking = %ranking.id, "multiple competitions share a durable vote ranking");
                        engine.mark_gap(&ranking.creator_id, &ranking.room_id, "competition.vote");
                        continue;
                    }
                    let restored = db
                        .load_competition_state(&competition.id)
                        .and_then(|state| {
                            let state = state.ok_or_else(|| {
                                crate::db::DatabaseError::Invalid(
                                    "competition state missing".into(),
                                )
                            })?;
                            let needs_checkpoint = matches!(
                                state.lifecycle,
                                tiktools_competitions::Lifecycle::Active
                                    | tiktools_competitions::Lifecycle::Paused
                                    | tiktools_competitions::Lifecycle::Finalizing
                            );
                            let clean = if needs_checkpoint {
                                match checkpoint_owner.as_deref() {
                                    Some(owner) => db.begin_competition_recovery(
                                        &competition.id,
                                        state.generation,
                                        owner,
                                    )?,
                                    None => false,
                                }
                            } else {
                                true
                            };
                            let totals = if vote_ranking {
                                db.competition_vote_totals(&competition.id, state.generation)?
                            } else {
                                Vec::new()
                            };
                            let scores = if vote_ranking {
                                db.competition_scored_totals(&competition.id, state.generation)?
                            } else {
                                Vec::new()
                            };
                            Ok((totals, scores, clean))
                        });
                    match restored {
                        Ok((totals, scores, clean)) => {
                            if !vote_ranking {
                                continue;
                            }
                            if !clean {
                                tracing::warn!(ranking = %ranking.id, "unclean competition restart; accounting completeness unknown");
                                engine.mark_gap(
                                    &ranking.creator_id,
                                    &ranking.room_id,
                                    "competition.vote",
                                );
                            }
                            if let Err(error) = engine.restore_assignments_with_scores(
                                &ranking.id,
                                &totals,
                                &scores,
                            ) {
                                tracing::error!(?error, ranking = %ranking.id, "cannot restore competition vote ranking");
                                engine.mark_gap(
                                    &ranking.creator_id,
                                    &ranking.room_id,
                                    "competition.vote",
                                );
                            } else {
                                for option in &competition.options {
                                    let metadata = tiktools_aggregation::EntryMetadata {
                                        label: Some(option.label.clone()),
                                        icon_url: option.image_url.clone(),
                                        ..Default::default()
                                    };
                                    if engine
                                        .set_entry_metadata(&ranking.id, &option.id, &metadata)
                                        .is_err()
                                    {
                                        engine.mark_gap(
                                            &ranking.creator_id,
                                            &ranking.room_id,
                                            "competition.vote",
                                        );
                                    }
                                }
                            }
                        }
                        Err(error) => {
                            tracing::error!(%error, ranking = %ranking.id, "cannot load competition vote ranking");
                            engine.mark_gap(
                                &ranking.creator_id,
                                &ranking.room_id,
                                "competition.vote",
                            );
                        }
                    }
                }
            }
            Err(error) => tracing::error!(%error, "cannot load competitions for ranking recovery"),
        }
        Self {
            engine: Mutex::new(engine),
            mutations: Mutex::new(()),
            checkpoint_owner,
            persisted: Mutex::default(),
            checkpoint_started: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Rankings persisted through session checkpoints. Vote rankings are
    /// rebuilt from the competition ledger instead.
    #[cfg(feature = "persistence")]
    fn checkpointed(engine: &Engine) -> Vec<String> {
        engine
            .definitions()
            .into_iter()
            .filter(|definition| {
                !definition
                    .sources
                    .iter()
                    .any(|source| source.source == "competition.vote")
            })
            .map(|definition| definition.id)
            .collect()
    }

    /// Snapshot one ranking under the engine lock (memory only) and write it
    /// outside that lock, so LIVE ingestion never waits on SQLite. Callers
    /// hold `mutations`, which fences reset, update and delete.
    #[cfg(feature = "persistence")]
    fn persist_one(
        core: &crate::AppCore,
        owner: &str,
        id: &str,
        only_changed: bool,
    ) -> Result<bool, crate::db::DatabaseError> {
        let (token, checkpoint) = {
            let engine = crate::recover_mutex(&core.aggregations.engine, "aggregations");
            let Some(token) = engine
                .change_tokens()
                .into_iter()
                .find(|(active, _, _)| active == id)
                .map(|(_, instance, revision)| (instance, revision))
            else {
                return Ok(false);
            };
            if only_changed
                && crate::recover_mutex(&core.aggregations.persisted, "ranking checkpoints").get(id)
                    == Some(&token)
            {
                return Ok(false);
            }
            (token, engine.session_checkpoint(id))
        };
        let checkpoint = checkpoint.map_err(|error| {
            tracing::error!(?error, ranking = %id, "aggregation snapshot failed");
            crate::db::DatabaseError::Invalid("aggregation snapshot failed".into())
        })?;
        core.db.save_aggregation_checkpoint(&checkpoint, owner)?;
        crate::recover_mutex(&core.aggregations.persisted, "ranking checkpoints")
            .insert(id.to_owned(), token);
        Ok(true)
    }

    /// Runtime durability: write rankings changed since their last checkpoint.
    /// The lock is taken per ranking so control operations are never queued
    /// behind a whole pass. A crash loses at most one interval of accounting
    /// and the next start marks affected rankings degraded.
    #[cfg(feature = "persistence")]
    pub(crate) fn checkpoint_changed(core: &crate::AppCore) -> usize {
        use std::sync::atomic::Ordering;
        let Some(owner) = core.aggregations.checkpoint_owner.as_deref() else {
            return 0;
        };
        let ids = Self::checkpointed(&crate::recover_mutex(
            &core.aggregations.engine,
            "aggregations",
        ));
        crate::recover_mutex(&core.aggregations.persisted, "ranking checkpoints")
            .retain(|id, _| ids.contains(id));
        let mut written = 0;
        for id in ids {
            let _mutation = crate::recover_mutex(&core.aggregations.mutations, "ranking mutations");
            // Checked under the lock: once shutdown begins, only its final
            // checkpoint may write, so a late pass cannot follow the clean mark.
            if core.shutdown_started.load(Ordering::Acquire) {
                break;
            }
            match Self::persist_one(core, owner, &id, true) {
                Ok(true) => written += 1,
                Ok(false) => {}
                Err(error) => {
                    tracing::warn!(%error, ranking = %id, "runtime ranking checkpoint failed")
                }
            }
        }
        written
    }

    /// Starts the runtime checkpoint loop once. Weak ownership keeps it from
    /// extending AppCore's lifetime.
    #[cfg(feature = "persistence")]
    pub(crate) fn start_checkpoints(core: &std::sync::Arc<crate::AppCore>) {
        use std::sync::atomic::Ordering;
        if core.aggregations.checkpoint_owner.is_none()
            || core
                .aggregations
                .checkpoint_started
                .swap(true, Ordering::AcqRel)
        {
            return;
        }
        let interval = std::time::Duration::from_millis(
            std::env::var("TIKTOOLS_RANKING_CHECKPOINT_INTERVAL_MS")
                .ok()
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(10_000)
                .clamp(1_000, 300_000),
        );
        let weak = std::sync::Arc::downgrade(core);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;
                let Some(core) = weak.upgrade() else {
                    break;
                };
                if core.shutdown_started.load(Ordering::Acquire) {
                    break;
                }
                let pass = tokio::task::spawn_blocking(move || Self::checkpoint_changed(&core));
                if let Err(error) = pass.await {
                    tracing::error!(%error, "ranking checkpoint worker failed");
                }
            }
        });
    }

    #[cfg(feature = "persistence")]
    pub(crate) fn checkpoint_shutdown(
        core: &crate::AppCore,
    ) -> Result<(), crate::db::DatabaseError> {
        let _mutation = crate::recover_mutex(&core.aggregations.mutations, "ranking mutations");
        let owner = core
            .aggregations
            .checkpoint_owner
            .as_deref()
            .ok_or_else(|| {
                crate::db::DatabaseError::Invalid("aggregation checkpoint owner unavailable".into())
            })?;
        #[cfg(feature = "native-tiktok")]
        let ingress_complete = !core.live_ingress_stop.is_cancelled()
            && core
                .live_ingress
                .cancelled
                .load(std::sync::atomic::Ordering::Relaxed)
                == 0
            && core
                .live_ingress
                .abandoned
                .load(std::sync::atomic::Ordering::Relaxed)
                == 0
            && core
                .live_ingress
                .admission_cancelled
                .load(std::sync::atomic::Ordering::Relaxed)
                == 0;
        #[cfg(not(feature = "native-tiktok"))]
        let ingress_complete = true;
        let ids = {
            let mut engine = crate::recover_mutex(&core.aggregations.engine, "aggregations");
            if !ingress_complete {
                engine.mark_ingress_loss(1);
            }
            Self::checkpointed(&engine)
        };
        // Materialize one bounded ranking at a time. A large number of
        // high-cardinality definitions must not multiply snapshot memory.
        // Unchanged rankings already have an identical durable row.
        let mut first_error = None;
        for id in ids {
            if let Err(error) = Self::persist_one(core, owner, &id, true) {
                tracing::error!(%error, ranking = %id, "aggregation checkpoint failed");
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
        }
        if let Some(error) = first_error {
            return Err(error);
        }
        core.db.finish_aggregation_checkpoints(owner)?;
        Ok(())
    }

    #[cfg(feature = "native-tiktok")]
    pub(crate) fn ingress_loss(&self, count: u64) {
        crate::recover_mutex(&self.engine, "aggregations").mark_ingress_loss(count);
    }

    #[cfg(feature = "native-tiktok")]
    pub(crate) fn mark_gap(&self, creator: &str, room: &str, source: &str) {
        crate::recover_mutex(&self.engine, "aggregations").mark_gap(creator, room, source);
    }

    #[cfg(feature = "native-tiktok")]
    pub(crate) fn metric_gap(&self, creator: &str, room: &str, source: &str, metric: &str) {
        crate::recover_mutex(&self.engine, "aggregations")
            .mark_metric_gap(creator, room, source, metric);
    }

    #[cfg(feature = "native-tiktok")]
    pub(crate) fn observe(
        &self,
        contribution: Contribution<'_>,
        user: Option<&tiktools_aggregation::EntryMetadata>,
        gift: Option<&tiktools_aggregation::EntryMetadata>,
    ) {
        let mut engine = crate::recover_mutex(&self.engine, "aggregations");
        for (ranking, error) in
            engine.apply_at_with_metadata(&contribution, crate::now_millis(), user, gift)
        {
            tracing::warn!(%ranking, ?error, "ranking contribution rejected");
        }
    }
}
