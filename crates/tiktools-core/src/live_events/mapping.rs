use crate::*;

#[cfg(feature = "native-tiktok")]
fn automation_emotes(
    emotes: &[tiktools_tiktok::events::LiveEmote],
) -> Vec<crate::contracts::ChatAutomationEmote> {
    emotes
        .iter()
        .map(|emote| crate::contracts::ChatAutomationEmote {
            id: emote.id.clone(),
            shortcode: emote.shortcode.clone(),
            image_url: emote.image_url.clone(),
            index: emote.index,
            emote_type: emote.emote_type,
            package_id: emote.package_id.clone(),
        })
        .collect()
}

impl AppCore {
    #[cfg(feature = "native-tiktok")]
    pub(crate) fn ui_event_and_points(
        &self,
        event: &NativeLiveEvent,
    ) -> Option<(serde_json::Value, PointAction, AwardOptions, &'static str)> {
        let user = native_user(event)?;
        let unique_id = clean_unique_id(&user.unique_id).unwrap_or_else(|| "viewer".to_owned());
        let base_options = || AwardOptions {
            user_id: (user.id != 0).then(|| user.id.to_string()),
            nickname: (!user.nickname.is_empty()).then(|| user.nickname.clone()),
            avatar_url: user.avatar_url.clone(),
            ..AwardOptions::default()
        };
        let (mut ui_event, action, options, reason) = match &event.base {
            tiktools_tiktok::events::CanonicalLiveEvent::Chat(chat) => Some((
                json!({
                    "kind": "chat",
                    "author": unique_id,
                    "nickname": user.nickname,
                    "text": chat.comment,
                    "emotes": serde_json::to_value(&chat.emotes)
                        .expect("chat emotes must serialize"),
                    "i18nKey": "chatMessage",
                    "i18nParams": {"comment": chat.comment}
                }),
                PointAction::Chat,
                base_options(),
                "chat",
            )),
            tiktools_tiktok::events::CanonicalLiveEvent::EmoteChat(chat) => Some((
                json!({
                    "kind": "chat",
                    "author": unique_id,
                    "nickname": user.nickname,
                    "text": "",
                    "emotes": serde_json::to_value(&chat.emotes)
                        .expect("emote chat metadata must serialize"),
                    "i18nKey": "chatMessage",
                    "i18nParams": {"comment": ""}
                }),
                // Emote-only messages are visible chat activity but do not
                // qualify for comment points.
                PointAction::Manual,
                base_options(),
                "emote_chat",
            )),
            tiktools_tiktok::events::CanonicalLiveEvent::Gift(gift) => {
                let gift_name = event.gift_name().unwrap_or("Gift");
                let diamond_count = event.gift_diamond_count().unwrap_or(gift.diamond_count);
                let count = gift.repeat_count.max(gift.combo_count).max(1);
                let diamonds = diamond_count;
                let total_diamonds = diamonds.saturating_mul(count);
                let mut options = base_options();
                if !event.gift_streakable() || gift.repeat_end {
                    options.diamond_count = Some(total_diamonds as f64);
                }
                let event = json!({
                    "kind": "gift",
                    "author": unique_id,
                    "nickname": user.nickname,
                    "text": format!("sent {count}× {gift_name} ({total_diamonds})"),
                    "giftDetails": {
                        "giftId": gift.gift_id.to_string(),
                        "name": gift_name,
                        "count": count,
                        "diamonds": total_diamonds,
                        "imageUrl": event.gift_icon_url(),
                        "groupId": gift.group_id.to_string(),
                        "repeatEnd": gift.repeat_end,
                        "streakable": event.gift_streakable()
                    },
                    "i18nKey": "giftSent",
                    "i18nParams": {"count": count, "giftName": gift_name, "diamonds": total_diamonds}
                });
                Some((event, PointAction::Gift, options, "gift"))
            }
            tiktools_tiktok::events::CanonicalLiveEvent::Like(like) => {
                let count = like.count;
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
            tiktools_tiktok::events::CanonicalLiveEvent::Member(_) => Some((
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
            tiktools_tiktok::events::CanonicalLiveEvent::Social(social) => {
                use tiktools_tiktok::events::SocialKind;
                let (text, i18n_key, point_action, reason) =
                    match tiktools_tiktok::events::classify_social_action(social.action) {
                        SocialKind::Follow => (
                            "followed the creator",
                            "followedCreator",
                            PointAction::Follow,
                            "follow",
                        ),
                        SocialKind::Share => {
                            ("shared the LIVE", "sharedLive", PointAction::Share, "share")
                        }
                        // An unconfirmed social action is shown as a generic
                        // social event and awards nothing: it is never counted
                        // as a follow or a share.
                        SocialKind::Other | SocialKind::Unknown => (
                            "performed a social action",
                            "socialAction",
                            PointAction::Manual,
                            "social",
                        ),
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
            tiktools_tiktok::events::CanonicalLiveEvent::RoomUser(_)
            | tiktools_tiktok::events::CanonicalLiveEvent::Unknown { .. } => None,
        }?;
        // The feed card renders the avatar only when present; omitting the
        // key (instead of a null) keeps the UI shape identical to before
        // for events TikTok sends without a user image.
        if let Some(avatar_url) = user.avatar_url.as_ref() {
            ui_event["avatarUrl"] = json!(avatar_url);
        }
        // Stable message identity: TikTok replays recent messages as
        // history when the connection drops and rejoins, so the same
        // event can arrive twice. The frontend keys its dedup cache on
        // `msgId` to drop the replay instead of re-triggering the
        // event. String form keeps large snowflake ids JS-safe.
        if let Some(object) = ui_event.as_object_mut() {
            if let Some(context) =
                recover_rwlock_read(&self.connection_context, "connection context").as_ref()
            {
                object.insert("roomId".to_owned(), json!(context.room_id));
                object.insert("creatorId".to_owned(), json!(context.unique_id));
            }
            object.insert("sourceType".to_owned(), json!(format!("tiktok.{reason}")));
            object.insert("msgId".to_owned(), json!(event.msg_id().to_string()));
            object.insert("isHistory".to_owned(), json!(event.is_history()));
        }
        Some((ui_event, action, options, reason))
    }
    #[cfg(feature = "native-tiktok")]
    pub(crate) fn normalize_native_event(
        &self,
        event: &NativeLiveEvent,
    ) -> Option<serde_json::Value> {
        let context =
            recover_rwlock_read(&self.connection_context, "connection context").clone()?;
        let (event_type, data, user) = match &event.base {
            tiktools_tiktok::events::CanonicalLiveEvent::Chat(chat) => (
                "tiktok.chat",
                serde_json::to_value(crate::contracts::ChatAutomationData {
                    comment: chat.comment.clone(),
                    emotes: automation_emotes(&chat.emotes),
                    method: event.method().to_owned(),
                    msg_id: event.msg_id().to_string(),
                    is_history: event.is_history(),
                })
                .expect("chat automation data must serialize"),
                Some(user_value(&chat.user)),
            ),
            tiktools_tiktok::events::CanonicalLiveEvent::EmoteChat(chat) => (
                "tiktok.emote_chat",
                serde_json::to_value(crate::contracts::EmoteChatAutomationData {
                    emotes: automation_emotes(&chat.emotes),
                    method: event.method().to_owned(),
                    msg_id: event.msg_id().to_string(),
                    is_history: event.is_history(),
                })
                .expect("emote chat automation data must serialize"),
                Some(user_value(&chat.user)),
            ),
            tiktools_tiktok::events::CanonicalLiveEvent::Gift(gift) => (
                "tiktok.gift",
                serde_json::to_value(crate::contracts::GiftAutomationData {
                    aggregation: None,
                    gift_id: gift.gift_id.to_string(),
                    gift_name: event.gift_name().unwrap_or("Gift").to_owned(),
                    diamond_count: event.gift_diamond_count().unwrap_or(gift.diamond_count),
                    repeat_count: gift.repeat_count,
                    combo_count: gift.combo_count,
                    group_id: gift.group_id.to_string(),
                    repeat_end: gift.repeat_end,
                    streakable: event.gift_streakable(),
                    gift_icon_url: event.gift_icon_url().map(str::to_owned),
                    method: event.method().to_owned(),
                    msg_id: event.msg_id().to_string(),
                    is_history: event.is_history(),
                })
                .expect("gift automation data must serialize"),
                Some(user_value(&gift.user)),
            ),
            tiktools_tiktok::events::CanonicalLiveEvent::Like(like) => (
                "tiktok.like",
                serde_json::to_value(crate::contracts::LikeAutomationData {
                    aggregation: None,
                    count: like.count,
                    total: like.total,
                    method: event.method().to_owned(),
                    msg_id: event.msg_id().to_string(),
                    is_history: event.is_history(),
                })
                .expect("like automation data must serialize"),
                Some(user_value(&like.user)),
            ),
            tiktools_tiktok::events::CanonicalLiveEvent::Member(member) => (
                "tiktok.join",
                serde_json::to_value(crate::contracts::MemberAutomationData {
                    member_count: member.member_count,
                    action: member.action,
                    method: event.method().to_owned(),
                    msg_id: event.msg_id().to_string(),
                    is_history: event.is_history(),
                })
                .expect("member automation data must serialize"),
                Some(user_value(&member.user)),
            ),
            tiktools_tiktok::events::CanonicalLiveEvent::Social(social) => (
                match tiktools_tiktok::events::classify_social_action(social.action) {
                    tiktools_tiktok::events::SocialKind::Follow => "tiktok.follow",
                    tiktools_tiktok::events::SocialKind::Share => "tiktok.share",
                    tiktools_tiktok::events::SocialKind::Other
                    | tiktools_tiktok::events::SocialKind::Unknown => "tiktok.social",
                },
                serde_json::to_value(crate::contracts::SocialAutomationData {
                    action: social.action,
                    follow_count: social.follow_count,
                    share_count: social.share_count,
                    method: event.method().to_owned(),
                    msg_id: event.msg_id().to_string(),
                    is_history: event.is_history(),
                })
                .expect("social automation data must serialize"),
                Some(user_value(&social.user)),
            ),
            tiktools_tiktok::events::CanonicalLiveEvent::RoomUser(room) => (
                "tiktok.room_stats",
                serde_json::to_value(crate::contracts::RoomStatsAutomationData {
                    viewers: room.total,
                    total_users: room.total_user,
                    popularity: room.popularity,
                    anonymous: room.anonymous,
                    method: event.method().to_owned(),
                    msg_id: event.msg_id().to_string(),
                    is_history: event.is_history(),
                })
                .expect("room stats automation data must serialize"),
                None,
            ),
            tiktools_tiktok::events::CanonicalLiveEvent::Unknown { .. } => return None,
        };
        Some(self.make_automation_event_with_context(event_type, data, user, &context))
    }
    #[cfg(feature = "native-tiktok")]
    pub(crate) fn make_automation_event(
        &self,
        event_type: &str,
        data: serde_json::Value,
        user: Option<serde_json::Value>,
    ) -> serde_json::Value {
        let context = recover_rwlock_read(&self.connection_context, "connection context").clone();
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
    pub(crate) fn make_automation_event_with_context(
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
}

#[cfg(all(test, feature = "native-tiktok"))]
mod regression_tests {
    use super::*;
    struct Emitter;
    impl HostEmitter for Emitter {
        fn emit(&self, _: HostMessage) {}
    }

    /// Full native path: TikTok LikeEvent -> dedup -> packet delta totals ->
    /// normalized tiktok.like -> filter/cooldown -> REAL core.log action.
    /// No browser mock and no LIVE network connection required.
    #[tokio::test]
    async fn e2e_batched_likes_trigger_saved_150_goals_without_breaking_old_templates() {
        use tiktools_tiktok::events::{CanonicalLiveEvent, EventMetadata, EventUser, LikeEvent};

        let core = Arc::new(AppCore::new(Arc::new(Emitter)));
        *recover_rwlock_write(&core.connection_context, "test context") = Some(LiveContext {
            unique_id: "batch-test-creator".into(),
            room_id: "batch-test-room".into(),
            connection_id: "batch-test-connection".into(),
        });
        core.automation.replace_snapshot(&json!({
            "actions": [
                {"id": "log-recurring", "name": "Recurring", "enabled": true,
                 "typeId": "core.log", "config": {"message": "every 150 likes"}},
                {"id": "log-once", "name": "Once", "enabled": true,
                 "typeId": "core.log", "config": {"message": "first 150 likes"}},
                {"id": "log-old", "name": "Old packet", "enabled": true,
                 "typeId": "core.log", "config": {"message": "large like batch"}}
            ],
            "events": [
                {"id": "recurring", "name": "Every 150", "enabled": true,
                 "trigger": "tiktok.like", "cooldownMs": 0, "actionIds": ["log-recurring"],
                 "filters": [{"path": "event.data.aggregation.global",
                              "operator": "crosses-multiple", "value": "150"}]},
                {"id": "legacy-once", "name": "First 150", "enabled": true,
                 "trigger": "tiktok.like", "cooldownMs": 0, "actionIds": ["log-once"],
                 "filters": [
                    {"path": "event.data.aggregation.global.previous", "operator": "lt", "value": "150"},
                    {"path": "event.data.aggregation.global.total", "operator": "gte", "value": "150"}
                 ]},
                {"id": "legacy-count", "name": "Packet count", "enabled": true,
                 "trigger": "tiktok.like", "cooldownMs": 0, "actionIds": ["log-old"],
                 "filters": [{"path": "event.data.count", "operator": "gte", "value": "150"}]}
            ]
        }));

        // 45+60+45=150: exactly one milestone. The next batch of 300
        // crosses 300 and 450 at once and must execute TWO actions.
        // Room-wide total is intentionally unrelated and must not be added.
        let mut last_event = None;
        for (index, count) in [45_u64, 60, 45, 300].into_iter().enumerate() {
            let event = NativeLiveEvent {
                base: CanonicalLiveEvent::Like(LikeEvent {
                    user: EventUser {
                        id: 8877123,
                        ..Default::default()
                    },
                    count,
                    total: 50_000 + count,
                }),
                metadata: EventMetadata {
                    method: "WebcastLikeMessage".into(),
                    msg_id: 900_001 + index as u64,
                    is_history: false,
                },
                gift: None,
            };
            last_event = Some(event.clone());
            core.handle_live_event(event).await;
        }

        let runs = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let runs = core.automation.recent_runs();
                if runs.len() >= 5 {
                    break runs;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("native like batches must execute all five configured actions");

        let count_runs = |id: &str| {
            runs.iter()
                .filter(|run| run["eventId"] == id && run["status"] == "ok" && run["test"] == false)
                .count()
        };
        assert_eq!(
            count_runs("recurring"),
            3,
            "150, 300 and 450 are three milestones"
        );
        assert_eq!(
            count_runs("legacy-once"),
            1,
            "old goal fires only at first 150"
        );
        assert_eq!(
            count_runs("legacy-count"),
            1,
            "old packet filter still sees count=300"
        );

        // Exact duplicate packets must not trigger already reached milestones.
        core.handle_live_event(last_event.expect("simulated batch"))
            .await;
        let stable = core.automation.recent_runs();
        assert_eq!(stable.len(), 5, "replayed message id must not double-count");
        core.shutdown().await;
    }

    #[tokio::test]
    async fn deeply_nested_live_gift_reaches_ui_and_original_automation() {
        let core = Arc::new(AppCore::new(Arc::new(Emitter)));
        *recover_rwlock_write(&core.connection_context, "test context") = Some(LiveContext {
            unique_id: "fixture-creator".into(),
            room_id: "fixture-room".into(),
            connection_id: "fixture-connection".into(),
        });
        let mut receiver = core.events.subscribe_projection();
        let events = tiktools_tiktok::events::decode_batch(
            include_bytes!("../../../tiktools-tiktok/tests/fixtures/deep-gift.batch.pb"),
            &mut std::collections::HashMap::new(),
        )
        .unwrap();
        assert_eq!(events.len(), 1);
        core.handle_live_event(events.into_iter().next().unwrap())
            .await;
        let mut gift_ui = false;
        let mut original = false;
        while let Ok(event) = receiver.try_recv() {
            match event {
                crate::events::DomainEvent::LiveUiEvent { event } => {
                    assert_eq!(event["kind"], "gift");
                    assert_eq!(event["msgId"], "9007199254740994");
                    gift_ui = true;
                }
                crate::events::DomainEvent::LiveEvent { event_type, .. }
                    if event_type == "tiktok.gift" =>
                {
                    original = true
                }
                _ => {}
            }
        }
        assert!(
            gift_ui && original,
            "gift reaches both original automation and live.ui-event"
        );
        core.shutdown().await;
    }

    #[tokio::test]
    async fn protobuf_gift_and_emotes_publish_both_ui_and_automation_contracts() {
        let core = Arc::new(AppCore::new(Arc::new(Emitter)));
        *recover_rwlock_write(&core.connection_context, "test context") = Some(LiveContext {
            unique_id: "creator".into(),
            room_id: "room".into(),
            connection_id: "connection".into(),
        });
        let fixtures: [(&str, &[u8], &str); 3] = [
            (
                "WebcastGiftMessage",
                include_bytes!("../../tests/fixtures/live/gift.pb"),
                "tiktok.gift",
            ),
            (
                "WebcastChatMessage",
                include_bytes!("../../tests/fixtures/live/chat-inline-emote.pb"),
                "tiktok.chat",
            ),
            (
                "WebcastEmoteChatMessage",
                include_bytes!("../../tests/fixtures/live/emote-chat.pb"),
                "tiktok.emote_chat",
            ),
        ];
        for (method, bytes, expected_type) in fixtures {
            let mut receiver = core.events.subscribe_projection();
            let base = ttl_live_events::decode_event(method, bytes);
            let event = NativeLiveEvent {
                base,
                metadata: tiktools_tiktok::events::EventMetadata {
                    method: method.into(),
                    msg_id: 9007199254740993,
                    is_history: false,
                },
                gift: None,
            };
            core.handle_live_event(event).await;
            let mut ui = None;
            let mut automation = None;
            while let Ok(event) = receiver.try_recv() {
                match event {
                    crate::events::DomainEvent::LiveUiEvent { event } => ui = Some(event),
                    crate::events::DomainEvent::LiveEvent { event, event_type }
                        if event_type == expected_type =>
                    {
                        automation = Some(event)
                    }
                    _ => {}
                }
            }
            let ui = ui.expect("UI delivery independent of automation execution");
            let automation = automation.expect("existing automation contract");
            assert_eq!(ui["msgId"], "9007199254740993");
            assert_eq!(ui["sourceType"], expected_type);
            assert_eq!(ui["roomId"], "room");
            assert_eq!(automation["data"]["msgId"], ui["msgId"]);
            if expected_type == "tiktok.gift" {
                assert!(ui["giftDetails"]["giftId"].is_string());
                assert_eq!(ui["giftDetails"]["giftId"], automation["data"]["giftId"]);
                assert!(ui["giftDetails"]["name"].is_string());
            } else {
                let emote = &ui["emotes"][0];
                assert!(emote["imageUrl"].is_string(), "camelCase imageUrl");
                assert!(emote.get("image_url").is_none());
                assert_eq!(ui["emotes"], automation["data"]["emotes"]);
            }
        }
        core.points.shutdown();
    }
}
