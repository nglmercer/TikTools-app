//! Durable command-vote admission and projection on the ordered ingress owner. Lifecycle
//! checks, assignments and their projection share the existing competition mutation guard.

use super::super::LiveEffect;
use crate::{recover_mutex, AppCore};
use std::sync::{atomic::Ordering, Arc};

impl AppCore {
    /// This runs on the ordered ingress owner after canonical deduplication,
    /// independently of the lossy effects and presentation queues. Only
    /// matching commands incur a database write.
    pub(crate) async fn route_live_competition_command(self: &Arc<Self>, effect: &LiveEffect) {
        use tiktools_tiktok::events::CanonicalLiveEvent;
        let CanonicalLiveEvent::Chat(chat) = &effect.event.base else {
            return;
        };
        let Some(context) = effect.context.as_ref() else {
            return;
        };
        let routes = recover_mutex(
            &self.competitions.command_index,
            "competition command index",
        )
        .matching(&context.unique_id, &context.room_id, &chat.comment);
        if routes.is_empty() {
            return;
        }
        let user = if chat.user.id != 0 {
            chat.user.id.to_string()
        } else {
            chat.user.unique_id.clone()
        };
        if effect.event.msg_id() == 0 || effect.event.metadata.is_history || user.is_empty() {
            self.mark_invalid_competition_routes(
                routes
                    .iter()
                    .map(|route| route.competition_id.clone())
                    .collect(),
                &context.unique_id,
                &context.room_id,
            )
            .await;
            return;
        }
        #[cfg(feature = "persistence")]
        {
            use tiktools_competitions::{CommandParticipation, ParticipationOutcome};
            if self.aggregations.checkpoint_owner.is_none() {
                self.competitions
                    .participation_failed
                    .fetch_add(routes.len() as u64, Ordering::Relaxed);
                self.record_event_gap();
                self.aggregations.mark_gap(
                    &context.unique_id,
                    &context.room_id,
                    "competition.vote",
                );
                return;
            }
            let core = Arc::clone(self);
            let creator = context.unique_id.clone();
            let room = context.room_id.clone();
            let identity = format!("tiktok.chat:{}", effect.event.msg_id());
            let command = chat.comment.clone();
            let received_at = crate::now_millis();
            let route_count = routes.len() as u64;
            let outcome = tokio::task::spawn_blocking(move || {
                // The same mutex serializes DB admission and projection with
                // host lifecycle transitions and the deadline worker.
                let _mutation =
                    recover_mutex(&core.competitions.mutations, "competition mutations");
                if core.live_ingress_stop.is_cancelled() {
                    core.competitions
                        .participation_failed
                        .fetch_add(routes.len() as u64, Ordering::Relaxed);
                    core.record_event_gap();
                    core.aggregations
                        .mark_gap(&creator, &room, "competition.vote");
                    return;
                }
                for route in routes {
                    let mut previous = None;
                    let outcome = core.db.route_competition_command_with_previous(
                        &route.competition_id,
                        &CommandParticipation {
                            creator_id: &creator,
                            room_id: &room,
                            participant_id: &user,
                            event_identity: &identity,
                            command: &command,
                            received_at,
                        },
                        &mut previous,
                        core.aggregations.checkpoint_owner.as_deref(),
                    );
                    match outcome {
                        Ok(Some(ParticipationOutcome::Accepted {
                            option_id, votes, ..
                        })) => {
                            if let Ok(definition) = core.competition_get(&route.competition_id) {
                                let mut engine =
                                    recover_mutex(&core.aggregations.engine, "aggregations");
                                let prior =
                                    previous.as_ref().map(|(id, votes)| (id.as_str(), *votes));
                                match engine.apply_assignment(
                                    &definition.ranking_id,
                                    prior,
                                    (&option_id, votes),
                                ) {
                                    Ok(()) => {
                                        if let Some(option) = definition
                                            .options
                                            .iter()
                                            .find(|option| option.id == option_id)
                                        {
                                            let metadata = tiktools_aggregation::EntryMetadata {
                                                label: Some(option.label.clone()),
                                                icon_url: option.image_url.clone(),
                                                ..Default::default()
                                            };
                                            if let Err(error) = engine.set_entry_metadata(
                                                &definition.ranking_id,
                                                &option_id,
                                                &metadata,
                                            ) {
                                                engine.mark_ranking_gap(&definition.ranking_id);
                                                tracing::error!(
                                                    ?error,
                                                    "competition option metadata update failed"
                                                );
                                            }
                                        }
                                    }
                                    Err(tiktools_aggregation::Error::IncompatibleDefinition) => {}
                                    Err(error) => {
                                        core.competitions
                                            .participation_failed
                                            .fetch_add(1, Ordering::Relaxed);
                                        core.record_event_gap();
                                        engine.mark_ranking_gap(&definition.ranking_id);
                                        tracing::error!(
                                            ?error,
                                            "competition ranking projection failed"
                                        );
                                    }
                                }
                            }
                            core.competitions
                                .participation_accepted
                                .fetch_add(1, Ordering::Relaxed);
                        }
                        Ok(Some(ParticipationOutcome::Duplicate { .. })) => {
                            core.competitions
                                .participation_duplicate
                                .fetch_add(1, Ordering::Relaxed);
                        }
                        Ok(Some(
                            ParticipationOutcome::Closed
                            | ParticipationOutcome::UnknownCommand
                            | ParticipationOutcome::SwitchForbidden
                            | ParticipationOutcome::VoteLimit,
                        )) => {
                            core.competitions
                                .participation_rejected
                                .fetch_add(1, Ordering::Relaxed);
                        }
                        other => {
                            core.competitions
                                .participation_failed
                                .fetch_add(1, Ordering::Relaxed);
                            core.record_event_gap();
                            core.mark_competition_ranking_failure(
                                &route.competition_id,
                                &creator,
                                &room,
                            );
                            tracing::error!(
                                ?other,
                                "competition command could not be accounted for"
                            );
                        }
                    }
                }
            })
            .await;
            if let Err(error) = outcome {
                self.competitions
                    .participation_failed
                    .fetch_add(route_count, Ordering::Relaxed);
                self.record_event_gap();
                self.aggregations.mark_gap(
                    &context.unique_id,
                    &context.room_id,
                    "competition.vote",
                );
                tracing::error!(%error, "competition command worker failed");
            }
        }
        #[cfg(not(feature = "persistence"))]
        {
            self.competitions
                .participation_failed
                .fetch_add(routes.len() as u64, Ordering::Relaxed);
            self.record_event_gap();
        }
    }
}
