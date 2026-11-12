//! Native connection lifecycle and its bounded persistence work. Generation checks and
//! verified creator context remain tied to the original transport snapshot.

use crate::{
    recover_rwlock_write, AppCore, AutomationEvent, ClientEvent, HostMessage, LiveContext,
};
use serde_json::json;
use std::sync::{atomic::Ordering, Arc};
#[cfg(feature = "persistence")]
use std::time::Duration;

impl AppCore {
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
    pub(crate) async fn handle_connected(self: &Arc<Self>, info: tiktools_tiktok::ConnectionInfo) {
        self.live_dedup.set_scope(&info.unique_id, &info.room_id);
        let sequence = self.connection_sequence.load(Ordering::Acquire);
        let context = LiveContext {
            unique_id: info.unique_id.clone(),
            room_id: info.room_id.clone(),
            connection_id: format!("connection-{sequence}"),
            creator_verified: info.verified_creator.as_deref() == Some(info.unique_id.as_str())
                && self
                    .events
                    .is_verified_live_generation(sequence, &info.unique_id),
        };
        *recover_rwlock_write(&self.connection_context, "connection context") =
            Some(context.clone());

        #[cfg(feature = "persistence")]
        if context.creator_verified {
            self.write_live_session(&info.unique_id, Some(info.room_id.as_str()), true)
                .await;
        }

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
            let creator_verified = context.creator_verified;
            match tokio::time::timeout(
                Duration::from_secs(2),
                tokio::task::spawn_blocking(move || {
                    // Creator/creator-list reads fed only the removed legacy
                    // pushes; the frontend now refreshes via RPC on
                    // `creator.changed`. The writes below are still needed.
                    if creator_verified {
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
        let creator_verified = context.creator_verified
            && self
                .events
                .is_verified_live_generation(sequence, &context.unique_id);
        self.events
            .publish_domain(crate::events::DomainEvent::LiveConnected {
                unique_id: Some(info.unique_id.clone()),
                room_id: Some(info.room_id.clone()),
            });
        // Re-send a snapshot when attaching to an existing LIVE room so
        // reconnects restore its ranking without replaying every like.
        if creator_verified {
            self.events
                .publish_domain(crate::events::DomainEvent::LiveLikesRanking {
                    creator_id: context.unique_id.clone(),
                    room_id: context.room_id.clone(),
                    leaders: self.live_totals.like_leaderboard(&context),
                });
        }
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

        let connected_data = serde_json::to_value(crate::contracts::ConnectionAutomationData {
            unique_id: info.unique_id,
            room_id: info.room_id,
        })
        .expect("connection automation data must serialize");
        let connected = if creator_verified {
            self.make_automation_event_with_context(
                "tiktok.connected",
                connected_data,
                None,
                &context,
            )
        } else {
            self.make_automation_event_without_context("tiktok.connected", connected_data, None)
        };
        self.queue_automation_event(if creator_verified {
            AutomationEvent::owner(connected)
        } else {
            AutomationEvent::owner_without_delegation(connected)
        });
    }
}
