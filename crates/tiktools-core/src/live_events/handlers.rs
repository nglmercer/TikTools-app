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
            self.route_live_competition_command(&effect).await;
            self.route_live_competition_score(&effect).await;
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
        // The replay guard and LiveTotals precede this adapter. Gift streaks
        // contribute only their accepted increments, never repeat totals.
        if let Some(context) = context.as_ref() {
            use tiktools_tiktok::events::CanonicalLiveEvent;
            let (source, user, gift, metrics) = match &event.base {
                CanonicalLiveEvent::Like(like) => {
                    let user = if like.user.id != 0 {
                        like.user.id.to_string()
                    } else {
                        like.user.unique_id.clone()
                    };
                    (
                        "tiktok.like",
                        user,
                        None,
                        vec![("like.count", like.count), ("event.count", 1)],
                    )
                }
                CanonicalLiveEvent::Gift(gift)
                    if !totals.is_null()
                        && ((gift.group_id != 0 && (gift.user.id == 0 || gift.gift_id == 0))
                            || (gift.group_id == 0
                                && (event.gift_streakable()
                                    || gift.repeat_count > 1
                                    || gift.combo_count > 1))) =>
                {
                    self.aggregations
                        .mark_gap(&context.unique_id, &context.room_id, "tiktok.gift");
                    tracing::warn!(msg_id = %event.msg_id(), "ambiguous gift streak identity; ranking contribution rejected");
                    ("", String::new(), None, Vec::new())
                }
                CanonicalLiveEvent::Gift(gift) if !totals.is_null() => {
                    let user = if gift.user.id != 0 {
                        gift.user.id.to_string()
                    } else {
                        gift.user.unique_id.clone()
                    };
                    let increment = totals["increment"].as_u64().unwrap_or(0);
                    let diamonds = totals["diamonds"].as_u64().unwrap_or(0);
                    let price = event.gift_diamond_count().unwrap_or(0);
                    let count = gift.repeat_count.max(gift.combo_count).max(1);
                    let mut metrics = vec![
                        ("gift.count", increment),
                        ("event.count", u64::from(increment > 0)),
                    ];
                    // A newly discovered price can make LiveTotals' cumulative
                    // diamond high-water jump for quantities already observed.
                    // Do not credit that retroactive amount or a zero-delta final.
                    let max_incremental = increment.checked_mul(price);
                    if price > 0
                        && count.checked_mul(price).is_some()
                        && max_incremental.is_some_and(|maximum| diamonds <= maximum)
                        && increment > 0
                    {
                        metrics.push(("gift.diamonds", diamonds));
                    } else if increment > 0 || diamonds > 0 || count.checked_mul(price).is_none() {
                        self.aggregations.metric_gap(
                            &context.unique_id,
                            &context.room_id,
                            "tiktok.gift",
                            "gift.diamonds",
                        );
                    }
                    (
                        "tiktok.gift",
                        user,
                        (gift.gift_id != 0).then(|| gift.gift_id.to_string()),
                        metrics,
                    )
                }
                CanonicalLiveEvent::Social(social) => {
                    use tiktools_tiktok::events::SocialKind;
                    let user = native_user(&event)
                        .map(|u| {
                            if u.id != 0 {
                                u.id.to_string()
                            } else {
                                u.unique_id.clone()
                            }
                        })
                        .unwrap_or_default();
                    match tiktools_tiktok::events::classify_social_action(social.action) {
                        SocialKind::Share => (
                            "tiktok.share",
                            user,
                            None,
                            vec![("share.count", 1), ("event.count", 1)],
                        ),
                        SocialKind::Follow => {
                            ("tiktok.follow", user, None, vec![("event.count", 1)])
                        }
                        _ => ("tiktok.social", user, None, vec![("event.count", 1)]),
                    }
                }
                CanonicalLiveEvent::Chat(_)
                | CanonicalLiveEvent::EmoteChat(_)
                | CanonicalLiveEvent::Member(_) => {
                    let source = if matches!(&event.base, CanonicalLiveEvent::Member(_)) {
                        "tiktok.join"
                    } else {
                        "tiktok.chat"
                    };
                    let user = native_user(&event)
                        .map(|u| {
                            if u.id != 0 {
                                u.id.to_string()
                            } else {
                                u.unique_id.clone()
                            }
                        })
                        .unwrap_or_default();
                    (source, user, None, vec![("event.count", 1)])
                }
                _ => ("", String::new(), None, Vec::new()),
            };
            if !source.is_empty() && (event.msg_id() == 0 || event.metadata.is_history) {
                // Missing identities and history beyond the bounded replay guard
                // cannot be proven new. Reject ranking accounting and expose
                // the gap while leaving existing consumers unchanged.
                self.aggregations
                    .mark_gap(&context.unique_id, &context.room_id, source);
            } else if !source.is_empty() {
                let user_metadata = native_user(&event).map(|user| {
                    let nickname = (!user.nickname.is_empty()).then(|| user.nickname.clone());
                    let unique_id = clean_unique_id(&user.unique_id);
                    let avatar_url = user
                        .avatar_url
                        .as_ref()
                        .filter(|value| {
                            value.len() <= 2048
                                && url::Url::parse(value).is_ok_and(|url| {
                                    matches!(url.scheme(), "http" | "https")
                                        && url.host_str().is_some()
                                        && url.username().is_empty()
                                        && url.password().is_none()
                                })
                        })
                        .cloned();
                    tiktools_aggregation::EntryMetadata {
                        label: nickname.clone().or_else(|| unique_id.clone()),
                        user_id: (user.id != 0).then(|| user.id.to_string()),
                        unique_id,
                        nickname,
                        avatar_url,
                        ..Default::default()
                    }
                });
                let gift_metadata = match &event.base {
                    CanonicalLiveEvent::Gift(item) if item.gift_id != 0 => {
                        let label = (!item.gift_name.is_empty())
                            .then(|| item.gift_name.clone())
                            .or_else(|| {
                                event.gift.as_ref().and_then(|gift| {
                                    (!gift.name.is_empty()).then(|| gift.name.clone())
                                })
                            });
                        Some(tiktools_aggregation::EntryMetadata {
                            label,
                            gift_id: Some(item.gift_id.to_string()),
                            icon_url: event.gift_icon_url().map(str::to_owned),
                            ..Default::default()
                        })
                    }
                    _ => None,
                };
                self.aggregations.observe(
                    tiktools_aggregation::model::Contribution {
                        creator_id: &context.unique_id,
                        room_id: &context.room_id,
                        source,
                        user: (!user.is_empty()).then_some(user.as_str()),
                        gift: gift.as_deref(),
                        option: None,
                        team: None,
                        custom_key: None,
                        metrics: &metrics,
                    },
                    user_metadata.as_ref(),
                    gift_metadata.as_ref(),
                );
            }
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
            if let (tiktools_tiktok::events::CanonicalLiveEvent::Like(like), Some(context)) =
                (&event.base, context.as_ref())
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
        Some(LiveEffect {
            correlation,
            event,
            totals,
            context,
            points,
        })
    }
    #[cfg(feature = "native-tiktok")]
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
    #[cfg(feature = "native-tiktok")]
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
    /// This runs on the ordered ingress owner after canonical deduplication,
    /// independently of the lossy effects and presentation queues. Only
    /// matching commands incur a database write.
    #[cfg(feature = "native-tiktok")]
    pub(crate) async fn route_live_competition_command(self: &Arc<Self>, effect: &LiveEffect) {
        use tiktools_tiktok::events::CanonicalLiveEvent;
        let CanonicalLiveEvent::Chat(chat) = &effect.event.base else {
            return;
        };
        let Some(context) = effect.context.as_ref() else {
            return;
        };
        let routes = recover_mutex(
            &self.competitions.command_index,
            "competition command index",
        )
        .matching(&context.unique_id, &context.room_id, &chat.comment);
        if routes.is_empty() {
            return;
        }
        let user = if chat.user.id != 0 {
            chat.user.id.to_string()
        } else {
            chat.user.unique_id.clone()
        };
        if effect.event.msg_id() == 0 || effect.event.metadata.is_history || user.is_empty() {
            self.mark_invalid_competition_routes(
                routes
                    .iter()
                    .map(|route| route.competition_id.clone())
                    .collect(),
                &context.unique_id,
                &context.room_id,
            )
            .await;
            return;
        }
        #[cfg(feature = "persistence")]
        {
            use tiktools_competitions::{CommandParticipation, ParticipationOutcome};
            if self.aggregations.checkpoint_owner.is_none() {
                self.competitions
                    .participation_failed
                    .fetch_add(routes.len() as u64, Ordering::Relaxed);
                self.record_event_gap();
                self.aggregations.mark_gap(
                    &context.unique_id,
                    &context.room_id,
                    "competition.vote",
                );
                return;
            }
            let core = Arc::clone(self);
            let creator = context.unique_id.clone();
            let room = context.room_id.clone();
            let identity = format!("tiktok.chat:{}", effect.event.msg_id());
            let command = chat.comment.clone();
            let received_at = crate::now_millis();
            let route_count = routes.len() as u64;
            let outcome = tokio::task::spawn_blocking(move || {
                // The same mutex serializes DB admission and projection with
                // host lifecycle transitions and the deadline worker.
                let _mutation =
                    recover_mutex(&core.competitions.mutations, "competition mutations");
                if core.live_ingress_stop.is_cancelled() {
                    core.competitions
                        .participation_failed
                        .fetch_add(routes.len() as u64, Ordering::Relaxed);
                    core.record_event_gap();
                    core.aggregations
                        .mark_gap(&creator, &room, "competition.vote");
                    return;
                }
                for route in routes {
                    let mut previous = None;
                    let outcome = core.db.route_competition_command_with_previous(
                        &route.competition_id,
                        &CommandParticipation {
                            creator_id: &creator,
                            room_id: &room,
                            participant_id: &user,
                            event_identity: &identity,
                            command: &command,
                            received_at,
                        },
                        &mut previous,
                        core.aggregations.checkpoint_owner.as_deref(),
                    );
                    match outcome {
                        Ok(Some(ParticipationOutcome::Accepted {
                            option_id, votes, ..
                        })) => {
                            if let Ok(definition) = core.competition_get(&route.competition_id) {
                                let mut engine =
                                    recover_mutex(&core.aggregations.engine, "aggregations");
                                let prior =
                                    previous.as_ref().map(|(id, votes)| (id.as_str(), *votes));
                                match engine.apply_assignment(
                                    &definition.ranking_id,
                                    prior,
                                    (&option_id, votes),
                                ) {
                                    Ok(()) => {
                                        if let Some(option) = definition
                                            .options
                                            .iter()
                                            .find(|option| option.id == option_id)
                                        {
                                            let metadata = tiktools_aggregation::EntryMetadata {
                                                label: Some(option.label.clone()),
                                                icon_url: option.image_url.clone(),
                                                ..Default::default()
                                            };
                                            if let Err(error) = engine.set_entry_metadata(
                                                &definition.ranking_id,
                                                &option_id,
                                                &metadata,
                                            ) {
                                                engine.mark_ranking_gap(&definition.ranking_id);
                                                tracing::error!(
                                                    ?error,
                                                    "competition option metadata update failed"
                                                );
                                            }
                                        }
                                    }
                                    Err(tiktools_aggregation::Error::IncompatibleDefinition) => {}
                                    Err(error) => {
                                        core.competitions
                                            .participation_failed
                                            .fetch_add(1, Ordering::Relaxed);
                                        core.record_event_gap();
                                        engine.mark_ranking_gap(&definition.ranking_id);
                                        tracing::error!(
                                            ?error,
                                            "competition ranking projection failed"
                                        );
                                    }
                                }
                            }
                            core.competitions
                                .participation_accepted
                                .fetch_add(1, Ordering::Relaxed);
                        }
                        Ok(Some(ParticipationOutcome::Duplicate { .. })) => {
                            core.competitions
                                .participation_duplicate
                                .fetch_add(1, Ordering::Relaxed);
                        }
                        Ok(Some(
                            ParticipationOutcome::Closed
                            | ParticipationOutcome::UnknownCommand
                            | ParticipationOutcome::SwitchForbidden
                            | ParticipationOutcome::VoteLimit,
                        )) => {
                            core.competitions
                                .participation_rejected
                                .fetch_add(1, Ordering::Relaxed);
                        }
                        other => {
                            core.competitions
                                .participation_failed
                                .fetch_add(1, Ordering::Relaxed);
                            core.record_event_gap();
                            core.mark_competition_ranking_failure(
                                &route.competition_id,
                                &creator,
                                &room,
                            );
                            tracing::error!(
                                ?other,
                                "competition command could not be accounted for"
                            );
                        }
                    }
                }
            })
            .await;
            if let Err(error) = outcome {
                self.competitions
                    .participation_failed
                    .fetch_add(route_count, Ordering::Relaxed);
                self.record_event_gap();
                self.aggregations.mark_gap(
                    &context.unique_id,
                    &context.room_id,
                    "competition.vote",
                );
                tracing::error!(%error, "competition command worker failed");
            }
        }
        #[cfg(not(feature = "persistence"))]
        {
            self.competitions
                .participation_failed
                .fetch_add(routes.len() as u64, Ordering::Relaxed);
            self.record_event_gap();
        }
    }
    /// Route accepted canonical deltas on the ordered ingress owner. The
    /// identity and participant assignment are committed before projection.
    #[cfg(feature = "native-tiktok")]
    pub(crate) async fn route_live_competition_score(self: &Arc<Self>, effect: &LiveEffect) {
        use tiktools_tiktok::events::{classify_social_action, CanonicalLiveEvent, SocialKind};
        if matches!(&effect.event.base, CanonicalLiveEvent::Gift(_)) {
            self.route_live_competition_gift(effect).await;
            return;
        }
        let (source, metrics) = match &effect.event.base {
            CanonicalLiveEvent::Like(like) => (
                "tiktok.like",
                vec![("like.count", like.count), ("event.count", 1)],
            ),
            CanonicalLiveEvent::Social(social)
                if matches!(classify_social_action(social.action), SocialKind::Share) =>
            {
                ("tiktok.share", vec![("share.count", 1), ("event.count", 1)])
            }
            _ => return,
        };
        let Some(context) = effect.context.as_ref() else {
            return;
        };
        let routes = recover_mutex(
            &self.competitions.command_index,
            "competition command index",
        )
        .matching_source(&context.unique_id, &context.room_id, source);
        if routes.is_empty() {
            return;
        }
        let user = native_user(&effect.event)
            .map(|user| {
                if user.id != 0 {
                    user.id.to_string()
                } else {
                    user.unique_id.clone()
                }
            })
            .unwrap_or_default();
        if effect.event.msg_id() == 0 || effect.event.metadata.is_history || user.is_empty() {
            self.mark_invalid_competition_routes(
                routes.clone(),
                &context.unique_id,
                &context.room_id,
            )
            .await;
            return;
        }
        #[cfg(feature = "persistence")]
        {
            use crate::db::{ScoredInput, ScoredOutcome};
            let core = Arc::clone(self);
            let creator = context.unique_id.clone();
            let room = context.room_id.clone();
            let identity = effect.event.msg_id().to_string();
            let received_at = crate::now_millis();
            let count = routes.len() as u64;
            let result = tokio::task::spawn_blocking(move || {
                let _mutation = recover_mutex(&core.competitions.mutations, "competition mutations");
                let Some(owner) = core.aggregations.checkpoint_owner.as_deref() else {
                    core.competitions.participation_failed.fetch_add(count, Ordering::Relaxed);
                    core.record_event_gap(); core.aggregations.mark_gap(&creator, &room, "competition.vote"); return;
                };
                if core.live_ingress_stop.is_cancelled() {
                    core.competitions.participation_failed.fetch_add(count, Ordering::Relaxed);
                    core.record_event_gap(); core.aggregations.mark_gap(&creator, &room, "competition.vote"); return;
                }
                for id in routes {
                    let result = (|| {
                        let definition = core.competition_get(&id).map_err(|_| ())?;
                        let contribution = tiktools_aggregation::model::Contribution {
                            creator_id: &creator, room_id: &room, source, user: Some(&user), gift: None, option: None, team: None, custom_key: None, metrics: &metrics,
                        };
                        let score = recover_mutex(&core.aggregations.engine, "aggregations").score_routed(&definition.ranking_id, &contribution).map_err(|_| ())?;
                        if score == 0 { return Ok(()); }
                        match core.db.route_competition_score(&id, &ScoredInput { creator_id: &creator, room_id: &room, participant_id: &user, event_identity: &identity, source, score, received_at }, Some(owner)).map_err(|_| ())? {
                            Some(ScoredOutcome::Accepted { option_id }) => {
                                let mut engine = recover_mutex(&core.aggregations.engine, "aggregations");
                                engine.apply_committed_score(&definition.ranking_id, &option_id, score).map_err(|_| ())?;
                                if let Some(option) = definition.options.iter().find(|o| o.id == option_id) {
                                    engine.set_entry_metadata(&definition.ranking_id, &option_id, &tiktools_aggregation::EntryMetadata { label: Some(option.label.clone()), icon_url: option.image_url.clone(), ..Default::default() }).map_err(|_| ())?;
                                }
                                core.competitions.participation_accepted.fetch_add(1, Ordering::Relaxed);
                            }
                            Some(ScoredOutcome::Duplicate) => { core.competitions.participation_duplicate.fetch_add(1, Ordering::Relaxed); }
                            Some(ScoredOutcome::Closed | ScoredOutcome::Unassigned) => { core.competitions.participation_rejected.fetch_add(1, Ordering::Relaxed); }
                            _ => return Err(()),
                        }
                        Ok(())
                    })();
                    if result.is_err() {
                        core.competitions.participation_failed.fetch_add(1, Ordering::Relaxed);
                        core.record_event_gap(); core.mark_competition_ranking_failure(&id, &creator, &room);
                        tracing::error!(competition_id = %id, "competition scored event could not be accounted for");
                    }
                }
            }).await;
            if let Err(error) = result {
                self.competitions
                    .participation_failed
                    .fetch_add(count, Ordering::Relaxed);
                self.record_event_gap();
                self.aggregations.mark_gap(
                    &context.unique_id,
                    &context.room_id,
                    "competition.vote",
                );
                tracing::error!(%error, "competition scoring worker failed");
            }
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = metrics;
            self.competitions
                .participation_failed
                .fetch_add(routes.len() as u64, Ordering::Relaxed);
            self.record_event_gap();
            self.aggregations
                .mark_gap(&context.unique_id, &context.room_id, "competition.vote");
        }
    }
    #[cfg(feature = "native-tiktok")]
    async fn route_live_competition_gift(self: &Arc<Self>, effect: &LiveEffect) {
        let tiktools_tiktok::events::CanonicalLiveEvent::Gift(gift) = &effect.event.base else {
            return;
        };
        let Some(context) = effect.context.as_ref() else {
            return;
        };
        let routes = recover_mutex(
            &self.competitions.command_index,
            "competition command index",
        )
        .matching_source(&context.unique_id, &context.room_id, "tiktok.gift");
        if routes.is_empty() {
            return;
        }
        let Some(price) = effect.event.gift_diamond_count().filter(|price| *price > 0) else {
            self.mark_invalid_competition_routes(
                routes.clone(),
                &context.unique_id,
                &context.room_id,
            )
            .await;
            return;
        };
        if effect.event.msg_id() == 0
            || effect.event.metadata.is_history
            || gift.user.id == 0
            || gift.gift_id == 0
            || gift.group_id == 0
        {
            self.mark_invalid_competition_routes(
                routes.clone(),
                &context.unique_id,
                &context.room_id,
            )
            .await;
            return;
        }
        #[cfg(feature = "persistence")]
        {
            use crate::db::{GiftInput, GiftOutcome};
            let core = Arc::clone(self);
            let creator = context.unique_id.clone();
            let room = context.room_id.clone();
            let user = gift.user.id.to_string();
            let gift_id = gift.gift_id.to_string();
            let group = gift.group_id.to_string();
            let quantity = gift.repeat_count.max(gift.combo_count).max(1);
            let identity = effect.event.msg_id().to_string();
            let received_at = crate::now_millis();
            let count = routes.len() as u64;
            let result = tokio::task::spawn_blocking(move || {
                let _mutation = recover_mutex(&core.competitions.mutations, "competition mutations");
                let Some(owner) = core.aggregations.checkpoint_owner.as_deref() else { core.record_event_gap(); core.aggregations.mark_gap(&creator,&room,"competition.vote"); return; };
                if core.live_ingress_stop.is_cancelled() { core.record_event_gap(); core.aggregations.mark_gap(&creator,&room,"competition.vote"); return; }
                for id in routes {
                    let result = (|| {
                        let definition = core.competition_get(&id).map_err(|_| ())?;
                        let outcome = core.db.route_competition_gift(&id, &GiftInput { creator_id: &creator, room_id: &room, participant_id: &user, event_identity: &identity, gift_id: &gift_id, group_id: &group, observed_quantity: quantity, price, received_at }, owner, |quantity, diamonds, gift| {
                            let metrics = [("gift.count", quantity), ("gift.diamonds", diamonds), ("event.count", u64::from(quantity > 0))];
                            let contribution = tiktools_aggregation::model::Contribution { creator_id: &creator, room_id: &room, source: "tiktok.gift", user: Some(&user), gift: Some(gift), option: None, team: None, custom_key: None, metrics: &metrics };
                            recover_mutex(&core.aggregations.engine, "aggregations").score_routed(&definition.ranking_id, &contribution).map_err(|_| crate::db::DatabaseError::Invalid("gift scoring failed".into()))
                        }).map_err(|_| ())?;
                        match outcome {
                            Some(GiftOutcome::Accepted { option_id, score }) => {
                                let mut engine = recover_mutex(&core.aggregations.engine, "aggregations");
                                engine.apply_committed_score(&definition.ranking_id, &option_id, score).map_err(|_| ())?;
                                if let Some(option) = definition.options.iter().find(|option| option.id == option_id) {
                                    engine.set_entry_metadata(&definition.ranking_id, &option_id, &tiktools_aggregation::EntryMetadata { label: Some(option.label.clone()), icon_url: option.image_url.clone(), ..Default::default() }).map_err(|_| ())?;
                                }
                                core.competitions.participation_accepted.fetch_add(1, Ordering::Relaxed);
                            }
                            Some(GiftOutcome::Duplicate) => { core.competitions.participation_duplicate.fetch_add(1, Ordering::Relaxed); }
                            Some(GiftOutcome::Closed | GiftOutcome::Unassigned | GiftOutcome::Zero) => { core.competitions.participation_rejected.fetch_add(1, Ordering::Relaxed); }
                            _ => return Err(()),
                        }
                        Ok(())
                    })();
                    if result.is_err() { core.competitions.participation_failed.fetch_add(1, Ordering::Relaxed); core.record_event_gap(); core.mark_competition_ranking_failure(&id,&creator,&room); tracing::error!(competition_id = %id, "competition gift could not be accounted for"); }
                }
            }).await;
            if let Err(error) = result {
                self.competitions
                    .participation_failed
                    .fetch_add(count, Ordering::Relaxed);
                self.record_event_gap();
                self.aggregations.mark_gap(
                    &context.unique_id,
                    &context.room_id,
                    "competition.vote",
                );
                tracing::error!(%error, "competition gift worker failed");
            }
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = price;
            self.competitions
                .participation_failed
                .fetch_add(routes.len() as u64, Ordering::Relaxed);
            self.record_event_gap();
            self.aggregations
                .mark_gap(&context.unique_id, &context.room_id, "competition.vote");
        }
    }
    /// `points.awarded` / `points.delta`: whole points granted by one
    /// committed award, keyed like native user sources so it can share a
    /// ranking with likes or gifts. Fractions are truncated, never rounded up.
    #[cfg(feature = "native-tiktok")]
    pub(crate) fn observe_points_award(
        &self,
        event: &NativeLiveEvent,
        context: &LiveContext,
        delta: f64,
    ) {
        if !(delta.is_finite() && delta >= 1.0) {
            return;
        }
        let Some(user) = crate::helpers::native_user(event) else {
            self.aggregations
                .mark_gap(&context.unique_id, &context.room_id, "points.awarded");
            return;
        };
        let key = if user.id != 0 {
            user.id.to_string()
        } else {
            user.unique_id.clone()
        };
        if key.is_empty() || event.msg_id() == 0 || event.metadata.is_history {
            self.aggregations
                .mark_gap(&context.unique_id, &context.room_id, "points.awarded");
            return;
        }
        let nickname = (!user.nickname.is_empty()).then(|| user.nickname.clone());
        let unique_id = crate::helpers::clean_unique_id(&user.unique_id);
        let metadata = tiktools_aggregation::EntryMetadata {
            label: nickname.clone().or_else(|| unique_id.clone()),
            user_id: (user.id != 0).then(|| user.id.to_string()),
            unique_id,
            nickname,
            ..Default::default()
        };
        let metrics = [("points.delta", delta.min(u64::MAX as f64) as u64)];
        self.aggregations.observe(
            tiktools_aggregation::model::Contribution {
                creator_id: &context.unique_id,
                room_id: &context.room_id,
                source: "points.awarded",
                user: Some(&key),
                gift: None,
                option: None,
                team: None,
                custom_key: None,
                metrics: &metrics,
            },
            Some(&metadata),
            None,
        );
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
#[cfg(feature = "native-tiktok")]
pub(crate) struct LiveEffect {
    pub(crate) correlation: String,
    pub(crate) event: NativeLiveEvent,
    pub(crate) totals: Value,
    pub(crate) context: Option<LiveContext>,
    points: Option<(String, PointAction, AwardOptions, &'static str)>,
}
