//! Ordered LIVE admission and its deferred effects contract.
//!
//! Deduplication, totals and one verified context snapshot precede ranking and
//! original publication. Durable competition routing still precedes deferred
//! analytics/points, using the existing ingress and effects queues.

mod competitions;
mod contributions;
mod effects;
mod lifecycle;
mod publication;

use crate::{
    recover_rwlock_read, AppCore, AwardOptions, LiveContext, NativeLiveEvent, PointAction,
};
use serde_json::Value;
use std::sync::Arc;

type LivePointsAward = (String, PointAction, AwardOptions, &'static str);

impl AppCore {
    pub(crate) async fn handle_live_event(self: &Arc<Self>, event: NativeLiveEvent) {
        if let Some(effect) = self.prepare_live_event(event) {
            self.route_live_competition_command(&effect).await;
            self.route_live_competition_score(&effect).await;
            self.apply_live_effect(effect).await;
        }
    }
    pub(crate) fn prepare_live_event(
        self: &Arc<Self>,
        event: NativeLiveEvent,
    ) -> Option<LiveEffect> {
        self.prepare_live_event_with_generation(event, None)
    }

    pub(crate) fn prepare_live_event_with_generation(
        self: &Arc<Self>,
        event: NativeLiveEvent,
        generation: Option<u64>,
    ) -> Option<LiveEffect> {
        if self.live_dedup.check_duplicate(&event) {
            return None;
        }
        let raw_context = recover_rwlock_read(&self.connection_context, "connection context")
            .clone()
            .filter(|context| {
                generation.is_none_or(|generation| {
                    context.connection_id == format!("connection-{generation}")
                })
            });
        let context = raw_context.clone().filter(|context| {
            context.creator_verified
                && generation.is_none_or(|generation| {
                    self.events
                        .is_verified_live_generation(generation, &context.unique_id)
                })
        });
        // Local high-water marks must not merge unrelated unverified rooms,
        // but their caller-provided creator label is never authority. If the
        // context is missing entirely, isolate by the native generation.
        let totals_context = context
            .clone()
            .or_else(|| {
                raw_context.as_ref().map(|local| LiveContext {
                    unique_id: String::new(),
                    room_id: local.room_id.clone(),
                    connection_id: local.connection_id.clone(),
                    creator_verified: false,
                })
            })
            .or_else(|| {
                generation.map(|generation| LiveContext {
                    unique_id: String::new(),
                    room_id: format!("unattributed-generation-{generation}"),
                    connection_id: format!("connection-{generation}"),
                    creator_verified: false,
                })
            });
        let totals = self.live_totals.observe(&event, totals_context.as_ref());
        if let Some(context) = context.as_ref() {
            self.observe_native_ranking(&event, context, &totals);
        }
        if totals.is_null()
            && matches!(
                &event.base,
                tiktools_tiktok::events::CanonicalLiveEvent::Gift(_)
            )
        {
            self.record_event_gap();
            if let Some(context) = context.as_ref() {
                self.aggregations
                    .mark_gap(&context.unique_id, &context.room_id, "tiktok.gift");
            }
            tracing::warn!(msg_id = %event.msg_id(), "gift aggregate capacity exhausted; original preserved without guessing quantities");
        }
        let (correlation, points) =
            self.publish_prepared_live_event(&event, context.as_ref(), &totals, generation);
        Some(LiveEffect {
            correlation,
            event,
            totals,
            context,
            points,
        })
    }
}
pub(crate) struct LiveEffect {
    pub(crate) correlation: String,
    pub(crate) event: NativeLiveEvent,
    pub(crate) totals: Value,
    pub(crate) context: Option<LiveContext>,
    points: Option<LivePointsAward>,
}
