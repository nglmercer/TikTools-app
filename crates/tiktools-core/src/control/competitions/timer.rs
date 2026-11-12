//! The single host deadline worker and bounded ranking publication. Weak ownership, refresh
//! tokens and finalization use the existing host state.

use super::bound_ranking_presentation;
use crate::AppCore;

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
}
