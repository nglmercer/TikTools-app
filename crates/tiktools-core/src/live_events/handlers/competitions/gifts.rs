//! Durable gift streak routing. Database admission owns quantity and price high-water
//! accounting before the committed delta is projected.

use super::super::LiveEffect;
use crate::{recover_mutex, AppCore};
use std::sync::{atomic::Ordering, Arc};

impl AppCore {
    pub(super) async fn route_live_competition_gift(self: &Arc<Self>, effect: &LiveEffect) {
        let tiktools_tiktok::events::CanonicalLiveEvent::Gift(gift) = &effect.event.base else {
            return;
        };
        let Some(context) = effect.context.as_ref() else {
            return;
        };
        let routes = recover_mutex(
            &self.competitions.command_index,
            "competition command index",
        )
        .matching_source(&context.unique_id, &context.room_id, "tiktok.gift");
        if routes.is_empty() {
            return;
        }
        let Some(price) = effect.event.gift_diamond_count().filter(|price| *price > 0) else {
            self.mark_invalid_competition_routes(
                routes.clone(),
                &context.unique_id,
                &context.room_id,
            )
            .await;
            return;
        };
        if effect.event.msg_id() == 0
            || effect.event.metadata.is_history
            || gift.user.id == 0
            || gift.gift_id == 0
            || gift.group_id == 0
        {
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
            use crate::db::{GiftInput, GiftOutcome};
            let core = Arc::clone(self);
            let creator = context.unique_id.clone();
            let room = context.room_id.clone();
            let user = gift.user.id.to_string();
            let gift_id = gift.gift_id.to_string();
            let group = gift.group_id.to_string();
            let quantity = gift.repeat_count.max(gift.combo_count).max(1);
            let identity = effect.event.msg_id().to_string();
            let received_at = crate::now_millis();
            let count = routes.len() as u64;
            let result = tokio::task::spawn_blocking(move || {
                let _mutation = recover_mutex(&core.competitions.mutations, "competition mutations");
                let Some(owner) = core.aggregations.checkpoint_owner.as_deref() else {
                    core.record_event_gap();
                    core.aggregations
                        .mark_gap(&creator, &room, "competition.vote");
                    return;
                };
                if core.live_ingress_stop.is_cancelled() {
                    core.record_event_gap();
                    core.aggregations
                        .mark_gap(&creator, &room, "competition.vote");
                    return;
                }
                for id in routes {
                    let result = (|| {
                        let definition = core.competition_get(&id).map_err(|_| ())?;
                        let outcome = core
                            .db
                            .route_competition_gift(
                                &id,
                                &GiftInput {
                                    creator_id: &creator,
                                    room_id: &room,
                                    participant_id: &user,
                                    event_identity: &identity,
                                    gift_id: &gift_id,
                                    group_id: &group,
                                    observed_quantity: quantity,
                                    price,
                                    received_at,
                                },
                                owner,
                                |quantity, diamonds, gift| {
                                    let metrics = [
                                        ("gift.count", quantity),
                                        ("gift.diamonds", diamonds),
                                        ("event.count", u64::from(quantity > 0)),
                                    ];
                                    let contribution = tiktools_aggregation::model::Contribution {
                                        creator_id: &creator,
                                        room_id: &room,
                                        source: "tiktok.gift",
                                        user: Some(&user),
                                        gift: Some(gift),
                                        option: None,
                                        team: None,
                                        custom_key: None,
                                        metrics: &metrics,
                                    };
                                    recover_mutex(&core.aggregations.engine, "aggregations")
                                        .score_routed(&definition.ranking_id, &contribution)
                                        .map_err(|_| {
                                            crate::db::DatabaseError::Invalid("gift scoring failed".into())
                                        })
                                },
                            )
                            .map_err(|_| ())?;
                        match outcome {
                            Some(GiftOutcome::Accepted { option_id, score }) => {
                                let mut engine = recover_mutex(&core.aggregations.engine, "aggregations");
                                super::apply_committed_projection(&mut engine, &definition, &option_id, score)?;
                                core.competitions
                                    .participation_accepted
                                    .fetch_add(1, Ordering::Relaxed);
                            }
                            Some(GiftOutcome::Duplicate) => {
                                core.competitions
                                    .participation_duplicate
                                    .fetch_add(1, Ordering::Relaxed);
                            }
                            Some(GiftOutcome::Closed | GiftOutcome::Unassigned | GiftOutcome::Zero) => {
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
                        tracing::error!(competition_id = %id, "competition gift could not be accounted for");
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
                tracing::error!(%error, "competition gift worker failed");
            }
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = price;
            self.competitions
                .participation_failed
                .fetch_add(routes.len() as u64, Ordering::Relaxed);
            self.record_event_gap();
            self.aggregations
                .mark_gap(&context.unique_id, &context.room_id, "competition.vote");
        }
    }
}
