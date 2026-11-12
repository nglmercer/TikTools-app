//! Invalid-route integrity accounting and shared committed-score projection. Ordered ingress
//! remains the only caller of durable routing; projection follows database admission.

mod commands;
mod gifts;
mod scored;

use crate::{recover_mutex, AppCore};
#[cfg(feature = "persistence")]
use std::sync::atomic::Ordering;
use std::sync::Arc;

impl AppCore {
    fn mark_competition_ranking_failure(&self, id: &str, creator: &str, room: &str) {
        match self.competition_get(id) {
            Ok(definition) => recover_mutex(&self.aggregations.engine, "aggregations")
                .mark_ranking_gap(&definition.ranking_id),
            Err(_) => self
                .aggregations
                .mark_gap(creator, room, "competition.vote"),
        }
    }
    /// Resolve invalid routed events against lifecycle under the same lock as
    /// admission. Draft and closed definitions cannot lose contributions.
    async fn mark_invalid_competition_routes(
        self: &Arc<Self>,
        ids: Vec<String>,
        creator: &str,
        room: &str,
    ) {
        #[cfg(feature = "persistence")]
        {
            let core = Arc::clone(self);
            let scoped_creator = creator.to_owned();
            let scoped_room = room.to_owned();
            let outcome = tokio::task::spawn_blocking(move || {
                let _mutation =
                    recover_mutex(&core.competitions.mutations, "competition mutations");
                for id in ids {
                    match (core.competition_get(&id), core.competition_state(&id)) {
                        (Ok(definition), Ok(state))
                            if matches!(
                                state.lifecycle,
                                tiktools_competitions::Lifecycle::Active
                                    | tiktools_competitions::Lifecycle::Paused
                            ) && !state.is_due(crate::now_millis()) =>
                        {
                            core.competitions
                                .participation_failed
                                .fetch_add(1, Ordering::Relaxed);
                            core.record_event_gap();
                            recover_mutex(&core.aggregations.engine, "aggregations")
                                .mark_ranking_gap(&definition.ranking_id);
                        }
                        (Ok(_), Ok(_)) => {
                            core.competitions
                                .participation_rejected
                                .fetch_add(1, Ordering::Relaxed);
                        }
                        _ => {
                            core.record_event_gap();
                            core.aggregations.mark_gap(
                                &scoped_creator,
                                &scoped_room,
                                "competition.vote",
                            );
                        }
                    }
                }
            })
            .await;
            if outcome.is_err() {
                self.record_event_gap();
                self.aggregations
                    .mark_gap(creator, room, "competition.vote");
            }
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = ids;
            self.record_event_gap();
            self.aggregations
                .mark_gap(creator, room, "competition.vote");
        }
    }
}

#[cfg(feature = "persistence")]
fn apply_committed_projection(
    engine: &mut tiktools_aggregation::engine::Engine,
    definition: &tiktools_competitions::Definition,
    option_id: &str,
    score: u64,
) -> Result<(), ()> {
    engine
        .apply_committed_score(&definition.ranking_id, option_id, score)
        .map_err(|_| ())?;
    if let Some(option) = definition
        .options
        .iter()
        .find(|option| option.id == option_id)
    {
        engine
            .set_entry_metadata(
                &definition.ranking_id,
                option_id,
                &tiktools_aggregation::EntryMetadata {
                    label: Some(option.label.clone()),
                    icon_url: option.image_url.clone(),
                    ..Default::default()
                },
            )
            .map_err(|_| ())?;
    }
    Ok(())
}
