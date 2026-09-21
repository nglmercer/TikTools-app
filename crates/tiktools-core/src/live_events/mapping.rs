use crate::*;

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
            ..AwardOptions::default()
        };
        match &event.base {
            tiktools_tiktok::events::CanonicalLiveEvent::Chat(chat) => Some((
                json!({
                    "kind": "chat",
                    "author": unique_id,
                    "nickname": user.nickname,
                    "text": chat.comment,
                    "i18nKey": "chatMessage",
                    "i18nParams": {"comment": chat.comment}
                }),
                PointAction::Chat,
                base_options(),
                "chat",
            )),
            tiktools_tiktok::events::CanonicalLiveEvent::Gift(gift) => {
                let gift_name = event.gift_name().unwrap_or("Gift");
                let diamond_count = event.gift_diamond_count().unwrap_or(gift.diamond_count);
                let count = gift.repeat_count.max(gift.combo_count).max(1);
                let diamonds = diamond_count.max(1);
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
                        "name": gift_name,
                        "count": count,
                        "diamonds": total_diamonds,
                        "imageUrl": event.gift_icon_url()
                    },
                    "i18nKey": "giftSent",
                    "i18nParams": {"count": count, "giftName": gift_name, "diamonds": total_diamonds}
                });
                Some((event, PointAction::Gift, options, "gift"))
            }
            tiktools_tiktok::events::CanonicalLiveEvent::Like(like) => {
                let count = like.count.max(1);
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
                let is_follow = social.action == 1;
                let is_share = social.action == 3;
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
            tiktools_tiktok::events::CanonicalLiveEvent::RoomUser(_)
            | tiktools_tiktok::events::CanonicalLiveEvent::Unknown { .. } => None,
        }
    }
    #[cfg(feature = "native-tiktok")]
    pub(crate) fn normalize_native_event(
        &self,
        event: &NativeLiveEvent,
    ) -> Option<serde_json::Value> {
        let context = read_or_recover(&self.connection_context, "connection context").clone()?;
        let (event_type, data, user) = match &event.base {
            tiktools_tiktok::events::CanonicalLiveEvent::Chat(chat) => (
                "tiktok.chat",
                serde_json::to_value(crate::contracts::ChatAutomationData {
                    comment: chat.comment.clone(),
                    method: event.method().to_owned(),
                    msg_id: event.msg_id().to_string(),
                    is_history: event.is_history(),
                })
                .expect("chat automation data must serialize"),
                Some(user_value(&chat.user)),
            ),
            tiktools_tiktok::events::CanonicalLiveEvent::Gift(gift) => (
                "tiktok.gift",
                serde_json::to_value(crate::contracts::GiftAutomationData {
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
                match social.action {
                    1 => "tiktok.follow",
                    3 => "tiktok.share",
                    _ => "tiktok.social",
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
        let context = read_or_recover(&self.connection_context, "connection context").clone();
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
