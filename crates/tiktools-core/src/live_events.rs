use super::*;

#[cfg(feature = "native-tiktok")]
use std::time::Duration;

impl AppCore {
    #[cfg(feature = "native-tiktok")]
    pub(super) fn start_live_event_pump(self: &Arc<Self>) {
        if self.live_pump_started.swap(true, Ordering::AcqRel) {
            return;
        }
        let mut receiver = self.live.subscribe();
        let core = Arc::clone(self);
        tracing::info!("native TikTok event pump started");
        tokio::spawn(async move {
            loop {
                match receiver.recv().await {
                    Ok(event) => {
                        tracing::debug!(
                            kind = client_event_kind(&event),
                            "native TikTok event received"
                        );
                        core.handle_native_event(event).await;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                        tracing::warn!(count, "native TikTok event receiver lagged")
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        tracing::warn!("native TikTok event stream closed");
                        break;
                    }
                }
            }
        });
    }

    #[cfg(not(feature = "native-tiktok"))]
    pub(super) fn start_live_event_pump(self: &Arc<Self>) {}

    #[cfg(feature = "native-tiktok")]
    pub(super) async fn connect_native(self: &Arc<Self>, request: ConnectRequest) {
        self.publish_disconnected_event().await;
        self.live.disconnect().await;
        self.emit(HostMessage::Connection {
            status: ipc::messages::ConnectionStatus::Connecting,
            unique_id: clean_unique_id(&request.unique_id),
            title: None,
            room_id: request.room_id.clone(),
            avatar_url: None,
        });

        match self.live.connect(request).await {
            Ok(_) => {}
            Err(error) => {
                self.emit(HostMessage::Error {
                    phase: ipc::messages::ErrorPhase::Connect,
                    message: error.to_string(),
                });
                self.live.disconnect().await;
                self.emit(HostMessage::connection_disconnected());
            }
        }
    }

    #[cfg(not(feature = "native-tiktok"))]
    pub(super) async fn connect_native(self: &Arc<Self>, _request: ConnectRequest) {
        self.emit(HostMessage::Error {
            phase: ipc::messages::ErrorPhase::Connect,
            message: "the native TikTok client is disabled in this build".to_owned(),
        });
        self.emit(HostMessage::connection_disconnected());
    }

    #[cfg(feature = "native-tiktok")]
    pub(super) async fn pick_live(self: &Arc<Self>, session_cookie: &str) {
        match self.live.live_channels(session_cookie).await {
            Ok(mut rooms) => {
                if rooms.is_empty() {
                    self.emit(HostMessage::Error {
                        phase: ipc::messages::ErrorPhase::Connect,
                        message: "TikTok returned no live rooms.".to_owned(),
                    });
                    return;
                }
                // The native discovery client orders rooms by viewers. Picking
                // the first item is deterministic and avoids a random source
                // in the core; callers can request another room explicitly.
                let room = rooms.remove(0);
                self.connect_native(ConnectRequest {
                    unique_id: room.unique_id,
                    session_cookie: session_cookie.to_owned(),
                    room_id: Some(room.room_id),
                })
                .await;
            }
            Err(error) => {
                self.emit(HostMessage::Error {
                    phase: ipc::messages::ErrorPhase::Connect,
                    message: error.to_string(),
                });
                self.emit(HostMessage::connection_disconnected());
            }
        }
    }

    #[cfg(not(feature = "native-tiktok"))]
    pub(super) async fn pick_live(&self, _session_cookie: &str) {
        self.emit(HostMessage::Error {
            phase: ipc::messages::ErrorPhase::Connect,
            message: "the native TikTok client is disabled in this build".to_owned(),
        });
    }

    #[cfg(feature = "native-tiktok")]
    pub(super) async fn handle_native_event(self: &Arc<Self>, event: ClientEvent) {
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
    pub(super) async fn handle_connected(self: &Arc<Self>, info: tiktools_tiktok::ConnectionInfo) {
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

        self.queue_automation_event(self.make_automation_event(
            "tiktok.connected",
            json!({"uniqueId": info.unique_id, "roomId": info.room_id}),
            None,
        ));
    }

    #[cfg(feature = "native-tiktok")]
    pub(super) async fn handle_live_event(self: &Arc<Self>, event: NativeLiveEvent) {
        let automation_event = self.normalize_native_event(&event);
        let Some((mut ui_event, action, options, reason)) = self.ui_event_and_points(&event) else {
            if let Some(event) = automation_event {
                self.queue_automation_event(event);
            }
            if let NativeLiveEvent::RoomUser {
                total, total_user, ..
            } = event
            {
                self.emit(HostMessage::RoomStats {
                    viewers: total,
                    total_users: total_user,
                    top_viewers: Vec::new(),
                });
            }
            return;
        };

        let should_award = !matches!(
            event,
            NativeLiveEvent::Gift {
                streakable: true,
                repeat_end: false,
                ..
            }
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
                        json!({
                            "delta": award.delta,
                            "total": award.total_points,
                            "level": award.level
                        }),
                    );
                }
            }
            self.queue_automation_event(event);
            if let Some(award) = point_award.as_ref().filter(|award| award.delta != 0.0) {
                self.queue_automation_event(self.make_automation_event(
                    "points.awarded",
                    json!({
                        "uniqueId": award.unique_id,
                        "delta": award.delta,
                        "totalPoints": award.total_points,
                        "level": award.level,
                        "currencyName": award.currency_name,
                        "reason": reason
                    }),
                    None,
                ));
            }
        }
        self.emit(HostMessage::LiveEvent { event: ui_event });
        self.emit_leaderboard_if_due();
    }

    #[cfg(feature = "native-tiktok")]
    pub(super) fn ui_event_and_points(
        &self,
        event: &NativeLiveEvent,
    ) -> Option<(serde_json::Value, PointAction, AwardOptions, &'static str)> {
        let user = native_user(event)?;
        let unique_id = clean_unique_id(&user.unique_id).unwrap_or_else(|| "viewer".to_owned());
        let base_options = || AwardOptions {
            user_id: user.user_id.clone(),
            nickname: (!user.nickname.is_empty()).then(|| user.nickname.clone()),
            ..AwardOptions::default()
        };
        match event {
            NativeLiveEvent::Chat { comment, .. } => Some((
                json!({
                    "kind": "chat",
                    "author": unique_id,
                    "nickname": user.nickname,
                    "text": comment,
                    "i18nKey": "chatMessage",
                    "i18nParams": {"comment": comment}
                }),
                PointAction::Chat,
                base_options(),
                "chat",
            )),
            NativeLiveEvent::Gift {
                gift_name,
                diamond_count,
                repeat_count,
                combo_count,
                repeat_end,
                streakable,
                gift_icon_url,
                ..
            } => {
                let count = (*repeat_count).max(*combo_count).max(1);
                let diamonds = (*diamond_count).max(1);
                let total_diamonds = diamonds.saturating_mul(count);
                let mut options = base_options();
                if !*streakable || *repeat_end {
                    options.diamond_count = Some(total_diamonds as f64);
                }
                let event = json!({
                    "kind": "gift",
                    "author": unique_id,
                    "nickname": user.nickname,
                    "text": format!("sent {count}× {gift_name} ({total_diamonds})"),
                    "giftDetails": {
                        "name": gift_name,
                        "count": count,
                        "diamonds": total_diamonds,
                        "imageUrl": gift_icon_url
                    },
                    "i18nKey": "giftSent",
                    "i18nParams": {"count": count, "giftName": gift_name, "diamonds": total_diamonds}
                });
                Some((event, PointAction::Gift, options, "gift"))
            }
            NativeLiveEvent::Like { count, .. } => {
                let count = (*count).max(1);
                let mut options = base_options();
                options.count = Some(count as f64);
                Some((
                    json!({
                        "kind": "like",
                        "author": unique_id,
                        "nickname": user.nickname,
                        "text": format!("sent {} {}", count, if count == 1 { "like" } else { "likes" }),
                        "likeCount": count,
                        "i18nKey": "likeSent",
                        "i18nParams": {"count": count}
                    }),
                    PointAction::Like,
                    options,
                    "like",
                ))
            }
            NativeLiveEvent::Member { .. } => Some((
                json!({
                    "kind": "member",
                    "author": unique_id,
                    "nickname": user.nickname,
                    "text": "joined the LIVE",
                    "i18nKey": "joinedLive",
                    "i18nParams": {}
                }),
                PointAction::Join,
                base_options(),
                "join",
            )),
            NativeLiveEvent::Social { action, .. } => {
                let is_follow = *action == 1;
                let is_share = *action == 3;
                let (text, i18n_key, point_action, reason) = if is_follow {
                    (
                        "followed the creator",
                        "followedCreator",
                        PointAction::Follow,
                        "follow",
                    )
                } else if is_share {
                    ("shared the LIVE", "sharedLive", PointAction::Share, "share")
                } else {
                    (
                        "performed a social action",
                        "socialAction",
                        PointAction::Manual,
                        "social",
                    )
                };
                Some((
                    json!({
                        "kind": "social",
                        "author": unique_id,
                        "nickname": user.nickname,
                        "text": text,
                        "i18nKey": i18n_key,
                        "i18nParams": {}
                    }),
                    point_action,
                    base_options(),
                    reason,
                ))
            }
            NativeLiveEvent::RoomUser { .. } | NativeLiveEvent::Unknown { .. } => None,
        }
    }

    #[cfg(feature = "native-tiktok")]
    pub(super) fn normalize_native_event(
        &self,
        event: &NativeLiveEvent,
    ) -> Option<serde_json::Value> {
        let context = self
            .connection_context
            .read()
            .expect("connection context lock poisoned")
            .clone()?;
        let (event_type, data, user) = match event {
            NativeLiveEvent::Chat {
                user,
                comment,
                method,
                msg_id,
                is_history,
            } => (
                "tiktok.chat",
                json!({"comment": comment, "method": method, "msgId": msg_id.to_string(), "isHistory": is_history}),
                Some(user_value(user)),
            ),
            NativeLiveEvent::Gift {
                user,
                gift_id,
                gift_name,
                diamond_count,
                repeat_count,
                combo_count,
                group_id,
                repeat_end,
                streakable,
                gift_icon_url,
                method,
                msg_id,
                is_history,
            } => (
                "tiktok.gift",
                json!({
                    "giftId": gift_id.to_string(),
                    "giftName": gift_name,
                    "diamondCount": diamond_count,
                    "repeatCount": repeat_count,
                    "comboCount": combo_count,
                    "groupId": group_id.to_string(),
                    "repeatEnd": repeat_end,
                    "streakable": streakable,
                    "giftIconUrl": gift_icon_url,
                    "method": method,
                    "msgId": msg_id.to_string(),
                    "isHistory": is_history
                }),
                Some(user_value(user)),
            ),
            NativeLiveEvent::Like {
                user,
                count,
                total,
                method,
                msg_id,
                is_history,
            } => (
                "tiktok.like",
                json!({"count": count, "total": total, "method": method, "msgId": msg_id.to_string(), "isHistory": is_history}),
                Some(user_value(user)),
            ),
            NativeLiveEvent::Member {
                user,
                member_count,
                action,
                method,
                msg_id,
                is_history,
            } => (
                "tiktok.join",
                json!({"memberCount": member_count, "action": action, "method": method, "msgId": msg_id.to_string(), "isHistory": is_history}),
                Some(user_value(user)),
            ),
            NativeLiveEvent::Social {
                user,
                action,
                follow_count,
                share_count,
                method,
                msg_id,
                is_history,
            } => (
                match action {
                    1 => "tiktok.follow",
                    3 => "tiktok.share",
                    _ => "tiktok.social",
                },
                json!({"action": action, "followCount": follow_count, "shareCount": share_count, "method": method, "msgId": msg_id.to_string(), "isHistory": is_history}),
                Some(user_value(user)),
            ),
            NativeLiveEvent::RoomUser {
                total,
                popularity,
                total_user,
                anonymous,
                method,
                msg_id,
                is_history,
            } => (
                "tiktok.room_stats",
                json!({"viewers": total, "totalUsers": total_user, "popularity": popularity, "anonymous": anonymous, "topViewers": [], "method": method, "msgId": msg_id.to_string(), "isHistory": is_history}),
                None,
            ),
            NativeLiveEvent::Unknown { .. } => return None,
        };
        Some(self.make_automation_event_with_context(event_type, data, user, &context))
    }

    #[cfg(feature = "native-tiktok")]
    pub(super) fn make_automation_event(
        &self,
        event_type: &str,
        data: serde_json::Value,
        user: Option<serde_json::Value>,
    ) -> serde_json::Value {
        let context = self
            .connection_context
            .read()
            .expect("connection context lock poisoned")
            .clone();
        match context {
            Some(context) => {
                self.make_automation_event_with_context(event_type, data, user, &context)
            }
            None => json!({
                "id": format!("{}-{}", event_type.replace('.', "-"), self.next_sequence()),
                "type": event_type,
                "timestamp": now_millis(),
                "data": data,
                "user": user
            }),
        }
    }

    pub(super) fn make_automation_event_with_context(
        &self,
        event_type: &str,
        data: serde_json::Value,
        user: Option<serde_json::Value>,
        context: &LiveContext,
    ) -> serde_json::Value {
        let mut event = json!({
            "id": format!("{}-{}", event_type.replace('.', "-"), self.next_sequence()),
            "type": event_type,
            "timestamp": now_millis(),
            "connectionId": context.connection_id,
            "creator": {"uniqueId": context.unique_id, "roomId": context.room_id},
            "data": data
        });
        if let Some(user) = user {
            event["user"] = user;
        }
        event
    }

    pub(super) async fn publish_automation_event(self: &Arc<Self>, event: serde_json::Value) {
        self.remember_automation_event(&event);
        Box::pin(self.run_automation_event(event)).await;
    }

    #[cfg(feature = "native-tiktok")]
    pub(super) fn queue_automation_event(self: &Arc<Self>, event: serde_json::Value) {
        self.remember_automation_event(&event);
        let event_type = event
            .get("type")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown");
        let Ok(permit) = Arc::clone(&self.automation_slots).try_acquire_owned() else {
            tracing::warn!(
                event_type,
                "automation concurrency limit reached; dropping live automation event"
            );
            return;
        };
        let core = Arc::clone(self);
        tokio::spawn(async move {
            Box::pin(core.run_automation_event(event)).await;
            drop(permit);
        });
    }

    pub(super) fn remember_automation_event(&self, event: &serde_json::Value) {
        *self
            .last_automation_event
            .write()
            .expect("automation event lock poisoned") = Some(event.clone());
        *self
            .last_automation_event_at
            .write()
            .expect("automation timestamp lock poisoned") = Some(now_millis());
        if event.get("type").and_then(Value::as_str) == Some("hotkey.status") {
            self.emit(HostMessage::HotkeyStatus {
                status: event.get("data").cloned().unwrap_or_else(|| json!({})),
            });
        }
        self.events.publish(AppEvent::TikTok(event.clone()));
        let now = now_millis();
        let last = self
            .last_automation_context_emit_at
            .load(std::sync::atomic::Ordering::Acquire);
        if (last == 0 || now.saturating_sub(last) >= 100)
            && self
                .last_automation_context_emit_at
                .compare_exchange(
                    last,
                    now,
                    std::sync::atomic::Ordering::AcqRel,
                    std::sync::atomic::Ordering::Acquire,
                )
                .is_ok()
        {
            self.emit(HostMessage::AutomationContext {
                event: Some(event.clone()),
                captured_at: *self
                    .last_automation_event_at
                    .read()
                    .expect("automation timestamp lock poisoned"),
            });
        }
    }

    pub(super) async fn publish_disconnected_event(self: &Arc<Self>) {
        let context = self
            .connection_context
            .write()
            .expect("connection context lock poisoned")
            .take();
        let Some(context) = context else { return };
        self.publish_automation_event(self.make_automation_event_with_context(
            "tiktok.disconnected",
            json!({"uniqueId": context.unique_id, "roomId": context.room_id}),
            None,
            &context,
        ))
        .await;
    }
}
