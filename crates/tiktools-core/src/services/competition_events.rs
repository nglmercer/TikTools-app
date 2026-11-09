//! Committed competition transitions fan out to existing domain and automation buses.
use crate::{contracts::CompetitionAutomationData, events::DomainEvent, AppCore};
use std::sync::{atomic::Ordering, Arc};

impl AppCore {
    pub(crate) fn competition_context(
        &self,
        id: &str,
        generation: u64,
    ) -> Option<CompetitionAutomationData> {
        let definition = self.competition_get(id).ok()?;
        let ranking = self.ranking_get(&definition.ranking_id).ok()?;
        Some(CompetitionAutomationData {
            competition_id: id.to_owned(),
            ranking_id: ranking.id,
            creator_id: ranking.creator_id,
            room_id: ranking.room_id,
            generation,
            option_id: None,
            winner_id: None,
            winner_score: None,
            selection_mode: None,
            selection_id: None,
            action_id: None,
            action_status: None,
        })
    }
    pub(crate) fn publish_competition_event(
        self: &Arc<Self>,
        kind: &str,
        data: CompetitionAutomationData,
    ) {
        let domain = match kind {
            "competition.started" => DomainEvent::CompetitionStarted(data.clone()),
            "competition.paused" => DomainEvent::CompetitionPaused(data.clone()),
            "competition.resumed" => DomainEvent::CompetitionResumed(data.clone()),
            "competition.cancelled" => DomainEvent::CompetitionCancelled(data.clone()),
            "competition.finished" => DomainEvent::CompetitionFinished(data.clone()),
            "competition.winner.selected" => DomainEvent::CompetitionWinnerSelected(data.clone()),
            "competition.action.completed" => DomainEvent::CompetitionActionCompleted(data.clone()),
            "competition.action.failed" => DomainEvent::CompetitionActionFailed(data.clone()),
            _ => return,
        };
        #[cfg(feature = "persistence")]
        let owner = data.generation == 0
            || self
                .db
                .competition_execution_is_owner(&data.competition_id, data.generation)
                .unwrap_or(false);
        #[cfg(not(feature = "persistence"))]
        let owner = true;
        if owner {
            self.events.publish_domain(domain);
        } else {
            // Plugin/domain observers cannot receive delegated events as Owner.
            self.events.publish_presentation_domain(domain);
        }
        if tokio::runtime::Handle::try_current().is_err() {
            self.automation_state
                .rejected
                .fetch_add(1, Ordering::Relaxed);
            return;
        }
        let event = serde_json::json!({"id": format!("{}-{}", kind.replace('.', "-"), self.next_sequence()),
            "type": kind, "timestamp": crate::now_millis(),
            "creator": {"uniqueId": data.creator_id, "roomId": data.room_id}, "data": data});
        if owner {
            self.queue_automation_event(crate::live_events::AutomationEvent::owner(event));
            return;
        }
        #[cfg(feature = "persistence")]
        {
            let context = (|| {
                let service = self.collaboration().ok()?;
                let fence = self
                    .db
                    .capture_competition_execution_fence(
                        &service,
                        &data.competition_id,
                        data.generation,
                    )
                    .ok()?;
                let subjects = service.execution_authorizations(&fence.subjects()).ok()?;
                crate::automation_runtime::context::ExecutionContext::restricted_competition(
                    subjects, fence,
                )
                .ok()
            })();
            if let Some(context) = context {
                self.queue_automation_event(crate::live_events::AutomationEvent::new(
                    event, context,
                ));
            }
        }
    }
}
