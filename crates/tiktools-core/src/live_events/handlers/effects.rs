//! Deferred analytics and points work after original delivery. Awarded-points automation
//! preserves the captured context and original correlation identity.

use super::LiveEffect;
use crate::{AppCore, AutomationEvent};
use serde_json::json;
use std::sync::Arc;

impl AppCore {
    pub(crate) async fn apply_live_effect(self: &Arc<Self>, effect: LiveEffect) {
        #[cfg(feature = "persistence")]
        {
            if let tiktools_tiktok::events::CanonicalLiveEvent::RoomUser(room) = &effect.event.base
            {
                self.record_analytics_viewers_for(
                    room.total,
                    effect.context.as_ref().map(|c| c.unique_id.clone()),
                )
                .await;
            } else {
                self.record_analytics_event(
                    &effect.event,
                    &effect.totals,
                    effect.context.as_ref().map(|c| c.unique_id.clone()),
                )
                .await;
            }
        }
        let Some((author, action, options, reason)) = effect.points else {
            return;
        };
        match self.live_points_work.award(author, action, options).await {
            Ok(Some(award)) => {
                self.events
                    .publish_domain(crate::events::DomainEvent::PointsChanged {
                        unique_id: award.unique_id.clone(),
                        delta: award.delta,
                        total_points: award.total_points,
                        level: award.level,
                    });
                if let Some(context) = effect.context.as_ref() {
                    self.observe_points_award(&effect.event, context, award.delta);
                }
                if award.delta != 0.0 {
                    let data =
                        serde_json::to_value(crate::contracts::PointsAwardedAutomationData {
                            unique_id: award.unique_id,
                            delta: award.delta,
                            total_points: award.total_points,
                            level: award.level,
                            currency_name: award.currency_name,
                            reason: reason.to_owned(),
                        })
                        .expect("points awarded data");
                    let mut event = match effect.context.as_ref() {
                        Some(context) => self.make_automation_event_with_context(
                            "points.awarded",
                            data,
                            None,
                            context,
                        ),
                        None => {
                            self.make_automation_event_without_context("points.awarded", data, None)
                        }
                    };
                    event["id"] = json!(format!("points-awarded-{}", effect.correlation));
                    self.queue_automation_event(if effect.context.is_some() {
                        AutomationEvent::owner(event)
                    } else {
                        AutomationEvent::owner_without_delegation(event)
                    });
                }
                self.emit_leaderboard_if_due();
            }
            Ok(None) => {}
            Err(error) => {
                tracing::error!(%error, msg_id = %effect.event.msg_id(), correlation_id = %effect.correlation, "LIVE points work failed");
                self.record_event_gap();
                if let Some(context) = effect.context.as_ref() {
                    self.aggregations.mark_gap(
                        &context.unique_id,
                        &context.room_id,
                        "points.awarded",
                    );
                }
            }
        }
    }
}
