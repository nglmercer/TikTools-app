//! Durable routing of admitted like and share deltas. The source identity and participant
//! assignment commit before ranking projection.

use super::super::LiveEffect;
use crate::{helpers::native_user, recover_mutex, AppCore};
use std::sync::{atomic::Ordering, Arc};

impl AppCore {
    /// Route accepted canonical deltas on the ordered ingress owner. The
    /// identity and participant assignment are committed before projection.
    pub(crate) async fn route_live_competition_score(self: &Arc<Self>, effect: &LiveEffect) {
        use tiktools_tiktok::events::{classify_social_action, CanonicalLiveEvent, SocialKind};
        if matches!(&effect.event.base, CanonicalLiveEvent::Gift(_)) {
            self.route_live_competition_gift(effect).await;
            return;
        }
        let (source, metrics) = match &effect.event.base {
            CanonicalLiveEvent::Like(like) => (
                "tiktok.like",
                vec![("like.count", like.count), ("event.count", 1)],
            ),
            CanonicalLiveEvent::Social(social)
                if matches!(classify_social_action(social.action), SocialKind::Share) =>
            {
                ("tiktok.share", vec![("share.count", 1), ("event.count", 1)])
            }
            _ => return,
        };
        let Some(context) = effect.context.as_ref() else {
            return;
        };
        let routes = recover_mutex(
            &self.competitions.command_index,
            "competition command index",
        )
        .matching_source(&context.unique_id, &context.room_id, source);
        if routes.is_empty() {
            return;
        }
        let user = native_user(&effect.event)
            .map(|user| {
                if user.id != 0 {
                    user.id.to_string()
                } else {
                    user.unique_id.clone()
                }
            })
            .unwrap_or_default();
        if effect.event.msg_id() == 0 || effect.event.metadata.is_history || user.is_empty() {
            self.mark_invalid_competition_routes(
                routes.clone(),
                &context.unique_id,
                &context.room_id,
            )
            .await;
            return;
        }
        #[cfg(feature = "persistence")]
        {
            use crate::db::{ScoredInput, ScoredOutcome};
            let core = Arc::clone(self);
            let creator = context.unique_id.clone();
            let room = context.room_id.clone();
            let identity = effect.event.msg_id().to_string();
            let received_at = crate::now_millis();
            let count = routes.len() as u64;
            let result = tokio::task::spawn_blocking(move || {
                let _mutation = recover_mutex(&core.competitions.mutations, "competition mutations");
                let Some(owner) = core.aggregations.checkpoint_owner.as_deref() else {
                    core.competitions
                        .participation_failed
                        .fetch_add(count, Ordering::Relaxed);
                    core.record_event_gap();
                    core.aggregations
                        .mark_gap(&creator, &room, "competition.vote");
                    return;
                };
                if core.live_ingress_stop.is_cancelled() {
                    core.competitions
                        .participation_failed
                        .fetch_add(count, Ordering::Relaxed);
                    core.record_event_gap();
                    core.aggregations
                        .mark_gap(&creator, &room, "competition.vote");
                    return;
                }
                for id in routes {
                    let result = (|| {
                        let definition = core.competition_get(&id).map_err(|_| ())?;
                        let contribution = tiktools_aggregation::model::Contribution {
                            creator_id: &creator,
                            room_id: &room,
                            source,
                            user: Some(&user),
                            gift: None,
                            option: None,
                            team: None,
                            custom_key: None,
                            metrics: &metrics,
                        };
                        let score = recover_mutex(&core.aggregations.engine, "aggregations")
                            .score_routed(&definition.ranking_id, &contribution)
                            .map_err(|_| ())?;
                        if score == 0 {
                            return Ok(());
                        }
                        match core
                            .db
                            .route_competition_score(
                                &id,
                                &ScoredInput {
                                    creator_id: &creator,
                                    room_id: &room,
                                    participant_id: &user,
                                    event_identity: &identity,
                                    source,
                                    score,
                                    received_at,
                                },
                                Some(owner),
                            )
                            .map_err(|_| ())?
                        {
                            Some(ScoredOutcome::Accepted { option_id }) => {
                                let mut engine = recover_mutex(&core.aggregations.engine, "aggregations");
                                super::apply_committed_projection(&mut engine, &definition, &option_id, score)?;
                                core.competitions
                                    .participation_accepted
                                    .fetch_add(1, Ordering::Relaxed);
                            }
                            Some(ScoredOutcome::Duplicate) => {
                                core.competitions
                                    .participation_duplicate
                                    .fetch_add(1, Ordering::Relaxed);
                            }
                            Some(ScoredOutcome::Closed | ScoredOutcome::Unassigned) => {
                                core.competitions
                                    .participation_rejected
                                    .fetch_add(1, Ordering::Relaxed);
                            }
                            _ => return Err(()),
                        }
                        Ok(())
                    })();
                    if result.is_err() {
                        core.competitions
                            .participation_failed
                            .fetch_add(1, Ordering::Relaxed);
                        core.record_event_gap();
                        core.mark_competition_ranking_failure(&id, &creator, &room);
                        tracing::error!(competition_id = %id, "competition scored event could not be accounted for");
                    }
                }
            }).await;
            if let Err(error) = result {
                self.competitions
                    .participation_failed
                    .fetch_add(count, Ordering::Relaxed);
                self.record_event_gap();
                self.aggregations.mark_gap(
                    &context.unique_id,
                    &context.room_id,
                    "competition.vote",
                );
                tracing::error!(%error, "competition scoring worker failed");
            }
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = metrics;
            self.competitions
                .participation_failed
                .fetch_add(routes.len() as u64, Ordering::Relaxed);
            self.record_event_gap();
            self.aggregations
                .mark_gap(&context.unique_id, &context.room_id, "competition.vote");
        }
    }
}
