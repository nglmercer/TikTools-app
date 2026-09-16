use crate::*;
#[cfg(feature = "native-tiktok")]
use std::time::Duration;

impl AppCore {
    #[cfg(feature = "native-tiktok")]
    pub(crate) async fn handle_native_event(self: &Arc<Self>, event: ClientEvent) {
        match event {
            ClientEvent::Connected(info) => self.handle_connected(info).await,
            ClientEvent::Event(event) => self.handle_live_event(event).await,
            ClientEvent::Reconnecting { attempt, delay_ms } => {
                self.emit(HostMessage::Reconnecting { attempt, delay_ms });
            }
            ClientEvent::Disconnected { reason } => {
                self.live.disconnect().await;
                self.publish_disconnected_event().await;
                tracing::info!(%reason, "TikTok live disconnected");
                self.emit(HostMessage::connection_disconnected());
            }
            ClientEvent::Error { phase, message } => {
                self.emit(HostMessage::Error {
                    phase: match phase {
                        tiktools_tiktok::ErrorPhase::Connect => ipc::messages::ErrorPhase::Connect,
                        tiktools_tiktok::ErrorPhase::Live => ipc::messages::ErrorPhase::Live,
                    },
                    message,
                });
            }
        }
    }
    #[cfg(feature = "native-tiktok")]
    pub(crate) async fn handle_connected(self: &Arc<Self>, info: tiktools_tiktok::ConnectionInfo) {
        let sequence = self.connection_sequence.fetch_add(1, Ordering::AcqRel) + 1;
        let context = LiveContext {
            unique_id: info.unique_id.clone(),
            room_id: info.room_id.clone(),
            connection_id: format!("connection-{sequence}"),
        };
        *self
            .connection_context
            .write()
            .expect("connection context lock poisoned") = Some(context.clone());

        #[cfg(feature = "persistence")]
        self.write_live_session(&info.unique_id, Some(info.room_id.as_str()), true);

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
        let (creator, recent_creators, app_state) = {
            let database = Arc::clone(&self.db);
            let info_for_db = info.clone();
            let gifts_for_db = gifts.clone();
            match tokio::time::timeout(
                Duration::from_secs(2),
                tokio::task::spawn_blocking(move || {
                    let creator = match database.save_creator(
                        &info_for_db.unique_id,
                        Some(&info_for_db.room_id),
                        Some(&info_for_db.nickname),
                        info_for_db.avatar_url.as_deref(),
                        Some(&info_for_db.title),
                        Some(&info_for_db.unique_id),
                    ) {
                        Ok(creator) => creator,
                        Err(error) => {
                            tracing::warn!(%error, "could not persist connected creator");
                            creator_value(&info_for_db)
                        }
                    };
                    let recent_creators =
                        database.load_recent_creators(10).unwrap_or_else(|error| {
                            tracing::warn!(%error, "could not load recent creators");
                            Vec::new()
                        });
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
                    (creator, recent_creators, app_state)
                }),
            )
            .await
            {
                Ok(Ok(values)) => values,
                Ok(Err(error)) => {
                    tracing::error!(%error, "connection persistence worker failed");
                    (
                        creator_value(&info),
                        Vec::new(),
                        std::collections::BTreeMap::new(),
                    )
                }
                Err(_) => {
                    tracing::warn!(
                        "connection persistence exceeded 2 seconds; continuing live event delivery"
                    );
                    (
                        creator_value(&info),
                        Vec::new(),
                        std::collections::BTreeMap::new(),
                    )
                }
            }
        };

        #[cfg(not(feature = "persistence"))]
        let (creator, recent_creators, app_state) = (
            creator_value(&info),
            Vec::new(),
            std::collections::BTreeMap::new(),
        );

        self.emit(HostMessage::Connection {
            status: ipc::messages::ConnectionStatus::Connected,
            unique_id: Some(info.unique_id.clone()),
            title: Some(info.title.clone()).filter(|value| !value.is_empty()),
            room_id: Some(info.room_id.clone()),
            avatar_url: info.avatar_url.clone(),
        });
        self.emit(HostMessage::CreatorState {
            creator: Some(creator),
        });
        self.emit(HostMessage::RecentCreators {
            creators: recent_creators,
        });
        self.emit(HostMessage::AppState { state: app_state });
        self.emit(HostMessage::PointsConfig {
            config: self.points.config(),
        });
        self.emit_leaderboard_if_due();

        self.emit(HostMessage::GiftCatalog { gifts });

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
        let automation_event = self.normalize_native_event(&event);
        let Some((mut ui_event, action, options, reason)) = self.ui_event_and_points(&event) else {
            if let Some(event) = automation_event {
                self.queue_automation_event(event);
            }
            if let tiktools_tiktok::events::CanonicalLiveEvent::RoomUser(room) = &event.base {
                self.emit(HostMessage::RoomStats {
                    viewers: room.total,
                    total_users: room.total_user,
                    top_viewers: Vec::new(),
                });
                #[cfg(feature = "persistence")]
                self.record_analytics_viewers(room.total);
            }
            return;
        };

        #[cfg(feature = "persistence")]
        self.record_analytics_event(&event);

        let should_award = !matches!(
            &event.base,
            tiktools_tiktok::events::CanonicalLiveEvent::Gift(gift)
                if event.gift_streakable() && !gift.repeat_end
        );
        let point_award = if should_award {
            let unique_id = ui_event["author"].as_str().unwrap_or("viewer").to_owned();
            let points = Arc::clone(&self.points);
            match tokio::time::timeout(
                Duration::from_secs(2),
                tokio::task::spawn_blocking(move || {
                    points.award_points(&unique_id, action, options)
                }),
            )
            .await
            {
                Ok(Ok(award)) => award,
                Ok(Err(error)) => {
                    tracing::error!(%error, "points worker failed while handling TikTok event");
                    None
                }
                Err(_) => {
                    tracing::warn!(
                        "points persistence exceeded 2 seconds; continuing live event delivery"
                    );
                    None
                }
            }
        } else {
            None
        };
        if let Some(award) = point_award.as_ref() {
            if let Some(object) = ui_event.as_object_mut() {
                object.insert("points".to_owned(), json!(award.total_points));
                object.insert("level".to_owned(), json!(award.level));
                object.insert("pointsDelta".to_owned(), json!(award.delta));
            }
            self.emit(HostMessage::PointsAwarded {
                unique_id: award.unique_id.clone(),
                delta: award.delta,
                total_points: award.total_points,
                level: award.level,
            });
        }
        if let Some(event) = automation_event {
            let mut event = event;
            if let Some(award) = point_award.as_ref() {
                if let Some(object) = event.as_object_mut() {
                    object.insert(
                        "points".to_owned(),
                        serde_json::to_value(crate::contracts::AutomationPoints {
                            delta: award.delta,
                            total: award.total_points,
                            level: award.level,
                        })
                        .expect("automation points must serialize"),
                    );
                }
            }
            self.queue_automation_event(event);
            if let Some(award) = point_award.as_ref().filter(|award| award.delta != 0.0) {
                self.queue_automation_event(
                    self.make_automation_event(
                        "points.awarded",
                        serde_json::to_value(crate::contracts::PointsAwardedAutomationData {
                            unique_id: award.unique_id.clone(),
                            delta: award.delta,
                            total_points: award.total_points,
                            level: award.level,
                            currency_name: award.currency_name.clone(),
                            reason: reason.to_owned(),
                        })
                        .expect("points awarded automation data must serialize"),
                        None,
                    ),
                );
            }
        }
        self.emit(HostMessage::LiveEvent { event: ui_event });
        self.emit_leaderboard_if_due();
    }
}
