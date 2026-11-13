//! Original automation envelopes and LIVE presentation publication. Every consumer uses the
//! context captured during admission; the returned points request remains deferred.

use super::LivePointsAward;
use crate::{
    helpers::{clean_unique_id, top_viewer_value},
    AppCore, AutomationEvent, LiveContext, NativeLiveEvent,
};
use serde_json::{json, Value};
use std::sync::{atomic::Ordering, Arc};

impl AppCore {
    /// Publish using the captured context, without rereading connection state.
    pub(super) fn publish_prepared_live_event(
        self: &Arc<Self>,
        event: &NativeLiveEvent,
        context: Option<&LiveContext>,
        totals: &Value,
        generation: Option<u64>,
    ) -> (String, Option<LivePointsAward>) {
        // Use the same verified generation snapshot throughout. Rereading
        // the mutable presentation context here could attach another room.
        let mut automation = self.normalize_native_event_with_context(event, context);
        let correlation = automation
            .as_ref()
            .and_then(|event| event["id"].as_str())
            .map(str::to_owned)
            .unwrap_or_else(|| format!("live-{}", self.next_sequence()));
        if let Some(automation) = automation.as_mut() {
            if !totals.is_null() {
                automation["data"]["aggregation"] = totals.clone();
                if matches!(
                    &event.base,
                    tiktools_tiktok::events::CanonicalLiveEvent::Gift(_)
                ) {
                    automation["data"]["count"] = totals["increment"].clone();
                    automation["data"]["amount"] =
                        json!(totals["diamonds"].as_u64().unwrap_or(0).to_string());
                }
            }
        }
        // Originals are delivered before any persistence work. Backfill only
        // reads their journal; it never calls this admission path again.
        if let Some(automation) = automation {
            let envelope = if context.is_some() {
                AutomationEvent::owner(automation)
            } else {
                AutomationEvent::owner_without_delegation(automation)
            };
            #[cfg(feature = "persistence")]
            let envelope = match context.and_then(|context| {
                crate::services::effects::EffectExecution::native(event, context).ok()
            }) {
                Some(execution) => {
                    let (payload, authority) = envelope.into_parts();
                    AutomationEvent::new(payload, authority.with_execution(execution))
                }
                None => envelope,
            };
            self.queue_automation_event(envelope);
        }
        let points = if let Some((mut ui, action, mut options, reason)) =
            self.ui_event_and_points(event, context)
        {
            ui["correlationId"] = json!(correlation);
            // Presentation-only snapshot: automation and points still consume
            // the original LikeEvent.count for every admitted packet.
            if let tiktools_tiktok::events::CanonicalLiveEvent::Like(like) = &event.base {
                if let Some(total) = totals["user"]["total"].as_u64() {
                    ui["likeUserTotal"] = json!(total);
                    let identity = if like.user.id != 0 {
                        Some(like.user.id.to_string())
                    } else {
                        clean_unique_id(&like.user.unique_id)
                    };
                    if let Some(identity) = identity {
                        ui["likeViewerKey"] = json!(identity);
                    }
                }
            }
            let author = ui["author"].as_str().unwrap_or("viewer").to_owned();
            let should_award = match &event.base {
                tiktools_tiktok::events::CanonicalLiveEvent::EmoteChat(_) => false,
                tiktools_tiktok::events::CanonicalLiveEvent::Gift(_) => {
                    options.diamond_count = totals["diamonds"].as_u64().map(|n| n as f64);
                    totals["increment"].as_u64().unwrap_or(0) > 0
                        || totals["diamonds"].as_u64().unwrap_or(0) > 0
                }
                _ => true,
            };
            self.live_ingress
                .ui_published
                .fetch_add(1, Ordering::Relaxed);
            if ui["kind"] == "gift" {
                self.live_ingress.ui_gifts.fetch_add(1, Ordering::Relaxed);
                if !ui["giftDetails"]["imageUrl"].is_string() {
                    self.live_ingress
                        .gifts_without_artwork
                        .fetch_add(1, Ordering::Relaxed);
                }
            }
            tracing::debug!(method = event.method(), msg_id = %event.msg_id(), correlation_id = %correlation, "publishing LIVE UI event");
            // Scoped history uses the captured host context, never the
            // mutable JSON payload or a second connection-context read.
            let creator = generation
                .and_then(|generation| {
                    context.filter(|c| c.connection_id == format!("connection-{generation}"))
                })
                .map(|c| c.unique_id.as_str());
            if creator.is_none() {
                if let Some(generation) = generation {
                    // The global UI still receives the event, but a mismatched or
                    // absent context prevents scoped delivery. Attribute that
                    // loss only through the native generation binding.
                    self.events.record_live_ingress_gap(generation);
                }
            }
            self.events.publish_live_event_from_creator(ui, creator);
            if let (tiktools_tiktok::events::CanonicalLiveEvent::Like(like), Some(context)) =
                (&event.base, context)
            {
                if like.count > 0 {
                    let leaders = self.live_totals.like_leaderboard(context);
                    let user_key = if like.user.id != 0 {
                        like.user.id.to_string()
                    } else {
                        like.user.unique_id.clone()
                    };
                    // When a viewer is outside the top ten, their packet
                    // changes no visible ranking: avoid another broadcast.
                    if leaders
                        .iter()
                        .any(|leader| leader["userId"].as_str() == Some(user_key.as_str()))
                    {
                        self.events
                            .publish_domain(crate::events::DomainEvent::LiveLikesRanking {
                                creator_id: context.unique_id.clone(),
                                room_id: context.room_id.clone(),
                                leaders,
                            });
                    }
                }
            }
            should_award.then_some((author, action, options, reason))
        } else {
            if let tiktools_tiktok::events::CanonicalLiveEvent::RoomUser(room) = &event.base {
                let ranked = if room.top_viewers.is_empty() {
                    &room.ranked_viewers
                } else {
                    &room.top_viewers
                };
                self.events
                    .publish_domain(crate::events::DomainEvent::RoomStats {
                        viewers: room.total,
                        total_users: room.total_user,
                        top_viewers: ranked.iter().take(25).map(top_viewer_value).collect(),
                    });
            }
            None
        };
        (correlation, points)
    }
}
