use crate::*;
#[cfg(all(feature = "native-tiktok", feature = "persistence"))]
use std::time::Duration;

impl AppCore {
    #[cfg(feature = "native-tiktok")]
    pub(crate) async fn handle_native_event(self: &Arc<Self>, event: ClientEvent) {
        match event {
            ClientEvent::Connected(info) => {
                let generation = self.connection_sequence.load(Ordering::Acquire);
                self.publish_disconnected_event().await;
                if self.live.connection_snapshot().generation == generation {
                    self.handle_connected(info).await;
                }
            }
            ClientEvent::Event(event) => self.handle_live_event(event).await,
            ClientEvent::Reconnecting { attempt, delay_ms } => {
                // Domain topic only: the legacy push was removed once the
                // frontend migrated to `live.reconnecting`.
                self.events
                    .publish_domain(crate::events::DomainEvent::LiveReconnecting {
                        attempt,
                        delay_ms,
                    });
            }
            ClientEvent::Disconnected { reason } => {
                let generation = self.live.connection_snapshot().generation;
                self.publish_disconnected_event().await;
                if self.live.connection_snapshot().generation != generation {
                    return;
                }
                // Native drops publish the same domain events as an explicit
                // RPC disconnect so every origin converges on one topic.
                self.events
                    .publish_domain(crate::events::DomainEvent::LiveDisconnected);
                self.events
                    .publish_domain(crate::events::DomainEvent::CreatorChanged { unique_id: None });
                tracing::info!(%reason, "TikTok live disconnected");
            }
            ClientEvent::Error { phase, message } => {
                let phase_name = match phase {
                    tiktools_tiktok::ErrorPhase::Connect => "connect",
                    tiktools_tiktok::ErrorPhase::Live => "live",
                };
                self.events
                    .publish_domain(crate::events::DomainEvent::LiveError {
                        phase: phase_name.to_owned(),
                        message,
                    });
            }
        }
    }
    #[cfg(feature = "native-tiktok")]
    pub(crate) async fn handle_connected(self: &Arc<Self>, info: tiktools_tiktok::ConnectionInfo) {
        self.live_dedup.set_scope(&info.unique_id, &info.room_id);
        let sequence = self.connection_sequence.load(Ordering::Acquire);
        let context = LiveContext {
            unique_id: info.unique_id.clone(),
            room_id: info.room_id.clone(),
            connection_id: format!("connection-{sequence}"),
        };
        *recover_rwlock_write(&self.connection_context, "connection context") =
            Some(context.clone());

        #[cfg(feature = "persistence")]
        self.write_live_session(&info.unique_id, Some(info.room_id.as_str()), true)
            .await;

        let gifts = info
            .gifts
            .iter()
            .map(|gift| {
                json!({
                    "id": gift.id,
                    "name": gift.name,
                    "diamondCount": gift.diamond_count,
                    "iconUrl": gift.icon_url,
                })
            })
            .collect::<Vec<_>>();

        #[cfg(feature = "persistence")]
        let app_state = {
            let database = Arc::clone(&self.db);
            let info_for_db = info.clone();
            let gifts_for_db = gifts.clone();
            match tokio::time::timeout(
                Duration::from_secs(2),
                tokio::task::spawn_blocking(move || {
                    // Creator/creator-list reads fed only the removed legacy
                    // pushes; the frontend now refreshes via RPC on
                    // `creator.changed`. The writes below are still needed.
                    if let Err(error) = database.save_creator(
                        &info_for_db.unique_id,
                        Some(&info_for_db.room_id),
                        Some(&info_for_db.nickname),
                        info_for_db.avatar_url.as_deref(),
                        Some(&info_for_db.title),
                        Some(&info_for_db.unique_id),
                    ) {
                        tracing::warn!(%error, "could not persist connected creator");
                    }
                    let app_state = database
                        .load_app_state()
                        .ok()
                        .map(|values| {
                            values
                                .into_iter()
                                .filter_map(|(key, value)| {
                                    value.as_str().map(|value| (key, value.to_owned()))
                                })
                                .collect::<std::collections::BTreeMap<_, _>>()
                        })
                        .unwrap_or_default();
                    if let Err(error) = database.save_gift_catalog(&gifts_for_db) {
                        tracing::warn!(%error, "could not persist TikTok gift catalog");
                    }
                    app_state
                }),
            )
            .await
            {
                Ok(Ok(values)) => values,
                Ok(Err(error)) => {
                    tracing::error!(%error, "connection persistence worker failed");
                    std::collections::BTreeMap::new()
                }
                Err(_) => {
                    tracing::warn!(
                        "connection persistence exceeded 2 seconds; continuing live event delivery"
                    );
                    std::collections::BTreeMap::new()
                }
            }
        };

        #[cfg(not(feature = "persistence"))]
        let app_state = std::collections::BTreeMap::new();

        if self.live.connection_snapshot().generation != sequence {
            return;
        }
        self.events
            .publish_domain(crate::events::DomainEvent::LiveConnected {
                unique_id: Some(info.unique_id.clone()),
                room_id: Some(info.room_id.clone()),
            });
        // Re-send a snapshot when attaching to an existing LIVE room so
        // reconnects restore its ranking without replaying every like.
        self.events
            .publish_domain(crate::events::DomainEvent::LiveLikesRanking {
                creator_id: context.unique_id.clone(),
                room_id: context.room_id.clone(),
                leaders: self.live_totals.like_leaderboard(&context),
            });
        self.events
            .publish_domain(crate::events::DomainEvent::CreatorChanged {
                unique_id: Some(info.unique_id.clone()),
            });

        // Connection/creator state travels on `live.connected` plus
        // `creator.changed` now; their legacy pushes were removed with the
        // rest of the migrated duplicates.
        self.emit(HostMessage::AppState { state: app_state });
        self.emit(HostMessage::PointsConfig {
            config: self.points.config(),
        });
        self.emit_leaderboard_if_due();

        self.events
            .publish_domain(crate::events::DomainEvent::GiftsCatalog { gifts });

        self.queue_automation_event(
            self.make_automation_event(
                "tiktok.connected",
                serde_json::to_value(crate::contracts::ConnectionAutomationData {
                    unique_id: info.unique_id,
                    room_id: info.room_id,
                })
                .expect("connection automation data must serialize"),
                None,
            ),
        );
    }
    #[cfg(feature = "native-tiktok")]
    pub(crate) async fn handle_live_event(self: &Arc<Self>, event: NativeLiveEvent) {
        if let Some(effect) = self.prepare_live_event(event) {
            self.apply_live_effect(effect).await;
        }
    }
    #[cfg(feature = "native-tiktok")]
    pub(crate) fn prepare_live_event(
        self: &Arc<Self>,
        event: NativeLiveEvent,
    ) -> Option<LiveEffect> {
        if self.live_dedup.check_duplicate(&event) {
            return None;
        }
        let context = recover_rwlock_read(&self.connection_context, "connection context").clone();
        let totals = self.live_totals.observe(&event, context.as_ref());
        if totals.is_null()
            && matches!(
                &event.base,
                tiktools_tiktok::events::CanonicalLiveEvent::Gift(_)
            )
        {
            self.record_event_gap();
            tracing::warn!(msg_id = %event.msg_id(), "gift aggregate capacity exhausted; original preserved without guessing quantities");
        }
        let mut automation = self.normalize_native_event(&event);
        let correlation = automation
            .as_ref()
            .and_then(|event| event["id"].as_str())
            .map(str::to_owned)
            .unwrap_or_else(|| format!("live-{}", self.next_sequence()));
        if let Some(automation) = automation.as_mut() {
            if !totals.is_null() {
                automation["data"]["aggregation"] = totals.clone();
            }
        }
        // Originals are delivered before any persistence work. Backfill only
        // reads their journal; it never calls this admission path again.
        if let Some(automation) = automation {
            self.queue_automation_event(automation);
        }
        let points = if let Some((mut ui, action, mut options, reason)) =
            self.ui_event_and_points(&event)
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
            self.events
                .publish_domain(crate::events::DomainEvent::LiveUiEvent { event: ui });
            if let tiktools_tiktok::events::CanonicalLiveEvent::Like(like) = &event.base
                && like.count > 0
                && let Some(context) = context.as_ref()
            {
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
        Some(LiveEffect {
            correlation,
            event,
            totals,
            context,
            points,
        })
    }
    #[cfg(feature = "native-tiktok")]
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
                        None => self.make_automation_event("points.awarded", data, None),
                    };
                    event["id"] = json!(format!("points-awarded-{}", effect.correlation));
                    self.queue_automation_event(event);
                }
                self.emit_leaderboard_if_due();
            }
            Ok(None) => {}
            Err(error) => {
                tracing::error!(%error, msg_id = %effect.event.msg_id(), correlation_id = %effect.correlation, "LIVE points work failed");
                self.record_event_gap();
            }
        }
    }
}
#[cfg(feature = "native-tiktok")]
pub(crate) struct LiveEffect {
    pub(crate) correlation: String,
    pub(crate) event: NativeLiveEvent,
    pub(crate) totals: Value,
    pub(crate) context: Option<LiveContext>,
    points: Option<(String, PointAction, AwardOptions, &'static str)>,
}
