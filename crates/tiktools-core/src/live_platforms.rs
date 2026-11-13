//! Managed providers share native event publication, points, and analytics.
//! TikTok retains its mature ingress/effects path; normalized projections never
//! re-enter it. Private execution authority is never accepted from provider JSON.
use crate::*;
use std::collections::{HashMap, VecDeque};
use tiktools_live::{ConnectionManager, LiveEnvelope, LivePayload, ManagedEvent, PlatformId};
#[cfg(feature = "native-tiktok")]
mod native;
mod presentation;
#[cfg(test)]
mod tests;

pub(crate) struct PlatformRuntime {
    pub(crate) manager: ConnectionManager,
    pub(crate) worker: Mutex<Option<tokio::task::JoinHandle<()>>>,
    stop: tokio_util::sync::CancellationToken,
    seen: Mutex<(HashMap<String, Instant>, VecDeque<String>)>,
}
impl PlatformRuntime {
    pub(crate) fn new(live: &LiveService) -> Self {
        let manager = ConnectionManager::default();
        #[cfg(feature = "native-tiktok")]
        manager
            .register(live.adapter())
            .expect("unique built-in TikTok adapter");
        #[cfg(not(feature = "native-tiktok"))]
        let _ = live;
        #[cfg(feature = "http")]
        match tiktools_twitch::TwitchAdapter::new() {
            Ok(adapter) => {
                manager
                    .register(Arc::new(adapter))
                    .expect("unique built-in Twitch adapter");
            }
            Err(error) => tracing::warn!(%error,"Twitch adapter unavailable"),
        }
        Self {
            manager,
            worker: Mutex::new(None),
            stop: tokio_util::sync::CancellationToken::new(),
            seen: Mutex::new((HashMap::new(), VecDeque::new())),
        }
    }
    fn duplicate(&self, event: &LiveEnvelope) -> bool {
        let Some(id) = event.source_event_id.as_deref().filter(|id| !id.is_empty()) else {
            return false;
        };
        // Stable room scope survives transport reconnects; IDs are strings.
        let key = serde_json::to_string(&(
            event.platform,
            &event.channel.id,
            &event.session_id,
            event.payload.event_type(),
            id,
        ))
        .expect("event identity");
        let mut seen = recover_mutex(&self.seen, "platform dedup");
        while let Some(front) = seen.1.front().cloned() {
            if seen.0.len() < 50_000
                && seen
                    .0
                    .get(&front)
                    .is_some_and(|at| at.elapsed() < std::time::Duration::from_secs(600))
            {
                break;
            }
            seen.1.pop_front();
            seen.0.remove(&front);
        }
        if seen.0.contains_key(&key) {
            return true;
        }
        seen.0.insert(key.clone(), Instant::now());
        seen.1.push_back(key);
        false
    }
}

impl AppCore {
    pub(crate) fn start_platform_connections(self: &Arc<Self>) {
        if self.is_shutdown() {
            return;
        }
        let mut task = recover_mutex(&self.live_platforms.worker, "platform worker");
        if task.is_some() {
            return;
        }
        let Some(mut events) = self.live_platforms.manager.take_events() else {
            return;
        };
        let weak = Arc::downgrade(self);
        let stop = self.live_platforms.stop.clone();
        *task = Some(tokio::spawn(async move {
            let mut draining = false;
            loop {
                let event = tokio::select! { biased; () = stop.cancelled(), if !draining => { events.close(); draining=true; continue; }, event = events.recv() => match event { Some(e)=>e,None=>break } };
                let Some(core) = weak.upgrade() else {
                    break;
                };
                match event {
                    ManagedEvent::State(connection) => {
                        if connection.platform != PlatformId::Tiktok {
                            if let (Some(channel), Some(payload)) = (
                                connection.channel.clone(),
                                match connection.state {
                                    tiktools_live::ConnectionState::Connected => {
                                        Some(LivePayload::Connected)
                                    }
                                    tiktools_live::ConnectionState::Disconnected => {
                                        Some(LivePayload::Disconnected)
                                    }
                                    _ => None,
                                },
                            ) {
                                core.handle_platform_event(LiveEnvelope {
                                    schema_version: 1,
                                    platform: connection.platform,
                                    connection_id: connection.connection_id.clone(),
                                    session_id: tiktools_live::identity_key(
                                        connection.platform,
                                        &channel.room_id,
                                    ),
                                    channel,
                                    source_event_id: None,
                                    occurred_at: tiktools_live::now_millis(),
                                    received_at: tiktools_live::now_millis(),
                                    is_history: false,
                                    user: None,
                                    payload,
                                    extensions: Default::default(),
                                })
                                .await;
                            }
                        }
                        core.events
                            .publish_domain(events::DomainEvent::PlatformConnection { connection });
                    }
                    ManagedEvent::Gap {
                        connection_id,
                        lost,
                    } => core
                        .events
                        .publish_domain(events::DomainEvent::PlatformGap {
                            connection_id,
                            lost,
                        }),
                    ManagedEvent::Event(event) => {
                        // TikTok side effects run exactly once in the existing
                        // native ingress worker. Its normalized projection is
                        // published there too, including legacy connections.
                        if event.platform != PlatformId::Tiktok {
                            core.handle_platform_event(event).await;
                        }
                    }
                }
            }
        }));
    }
    pub(crate) async fn stop_platform_connections(&self) {
        self.live_platforms.manager.shutdown().await;
        self.live_platforms.stop.cancel();
        let task = recover_mutex(&self.live_platforms.worker, "platform worker").take();
        if let Some(mut task) = task {
            if tokio::time::timeout(std::time::Duration::from_secs(10), &mut task)
                .await
                .is_err()
            {
                task.abort();
                self.shutdown_update_safe.store(false, Ordering::Release);
            }
        }
    }
    async fn handle_platform_event(self: &Arc<Self>, event: LiveEnvelope) {
        if self.live_platforms.duplicate(&event) {
            return;
        }
        self.events
            .publish_domain(events::DomainEvent::NormalizedLive {
                event: event.clone(),
            });
        let topic = event.payload.event_type();
        let mut data = serde_json::to_value(&event.payload).expect("shared event serialization");
        if let Some(object) = data.as_object_mut() {
            object.remove("kind");
            object.insert(
                "msgId".into(),
                json!(event.source_event_id.as_deref().unwrap_or("")),
            );
            object.insert("method".into(), json!(topic));
            object.insert("isHistory".into(), json!(event.is_history));
        }
        let event_id = format!("platform-{}-{}", event.connection_id, self.next_sequence());
        let user = event.user.as_ref().map(|u| json!({"userId":u.id,"uniqueId":u.key(),"nickname":u.display_name,"avatarUrl":u.avatar_url,"secUid":""}));
        let automation = json!({"id":event_id,"type":topic,"timestamp":event.occurred_at,"connectionId":event.connection_id,"creator":{"uniqueId":tiktools_live::identity_key(event.platform,&event.channel.id),"roomId":event.channel.room_id},"user":user,"data":data,"platform":event.platform,"sourceEventId":event.source_event_id,"extensions":event.extensions});
        // Platform-native events are useful to local Owner rules. Delegated
        // workflows remain denied until platform-aware grants are available;
        // a string topic or channel field cannot grant execution authority.
        let automation = if event.is_history {
            self.events.publish_domain(events::DomainEvent::LiveEvent {
                event_type: topic.into(),
                event: automation,
            });
            None
        } else {
            Some(automation)
        };
        if let Some(ui) = presentation::ui_event(&event, &event_id) {
            self.events.publish_live_event_from_creator(ui, None);
        }
        if event.is_history {
            return;
        }
        #[cfg(feature = "persistence")]
        {
            let db = self.db.clone();
            let record = event.clone();
            if !matches!(
                tokio::task::spawn_blocking(move || db.record_platform_activity(&record)).await,
                Ok(Ok(()))
            ) {
                self.events
                    .publish_domain(events::DomainEvent::PlatformGap {
                        connection_id: event.connection_id.clone(),
                        lost: 1,
                    });
                return;
            }
        }
        // Persist the identity mapping before any automation can award points
        // for this new account. Otherwise a fast points action could create a
        // legacy-looking row before the mapping transaction detects collisions.
        if let Some(automation) = automation {
            self.queue_automation_event(AutomationEvent::owner_without_delegation(automation));
        }
        let Some(user) = event.user else {
            return;
        };
        let action = match event.payload {
            LivePayload::Chat { .. } => PointAction::Chat,
            LivePayload::Follow => PointAction::Follow,
            LivePayload::Like { .. } => PointAction::Like,
            LivePayload::Share { .. } => PointAction::Share,
            LivePayload::Join => PointAction::Join,
            _ => return,
        };
        // Platform currency is intentionally not fed into TikTok's per-coin
        // award rule. Monetary events need an explicit unit-specific rule.
        let key = user.key();
        let options = AwardOptions {
            user_id: Some(user.id.clone()),
            nickname: Some(user.display_name),
            avatar_url: user.avatar_url,
            count: match event.payload {
                LivePayload::Like { count, .. } | LivePayload::Share { count } => {
                    Some(count as f64)
                }
                _ => None,
            },
            ..Default::default()
        };
        let points = self.points.clone();
        match tokio::task::spawn_blocking(move || points.award_points(&key, action, options)).await
        {
            Ok(Some(award)) => {
                if award.delta != 0.0 {
                    let data =
                        serde_json::to_value(crate::contracts::PointsAwardedAutomationData {
                            unique_id: award.unique_id.clone(),
                            delta: award.delta,
                            total_points: award.total_points,
                            level: award.level,
                            currency_name: award.currency_name.clone(),
                            reason: topic.trim_start_matches("live.").into(),
                        })
                        .expect("points awarded data");
                    self.queue_automation_event(AutomationEvent::owner_without_delegation(json!({"id":format!("points-awarded-{event_id}"),"type":"points.awarded","platform":event.platform,"timestamp":tiktools_live::now_millis(),"connectionId":event.connection_id,"creator":{"uniqueId":tiktools_live::identity_key(event.platform,&event.channel.id),"roomId":event.channel.room_id},"data":data})));
                }
                self.events
                    .publish_domain(events::DomainEvent::PointsChanged {
                        unique_id: award.unique_id,
                        delta: award.delta,
                        total_points: award.total_points,
                        level: award.level,
                    });
                self.emit_leaderboard_if_due();
            }
            Ok(None) => {}
            Err(_) => self
                .events
                .publish_domain(events::DomainEvent::PlatformGap {
                    connection_id: event.connection_id,
                    lost: 1,
                }),
        }
    }
}
