//! Per-type sample automation events for the `automation.test` harness.
//!
//! The live path builds automation envelopes by serializing the contract
//! structs in [`crate::contracts`]; samples do the same, so a test event can
//! never drift from the shape real events carry (same constructors, same
//! `camelCase` keys). Values mirror the frontend registry
//! (`scripts/generate-contracts.ts`): curated identities plus
//! discriminator-correct sentinels (share action 3, plain join action 0,
//! fresh emit depth 0).

use serde_json::{json, Value};

use crate::contracts::{
    AutomationUser, ChatAutomationData, ConnectionAutomationData, GiftAutomationData,
    LikeAutomationData, MemberAutomationData, PluginEmitAutomationData,
    PointsAwardedAutomationData, RoomStatsAutomationData, SocialAutomationData,
};
use crate::now_millis;

const SAMPLE_METHOD: &str = "WebcastSampleMessage";
const SAMPLE_MSG_ID: &str = "1";

fn sample_data<T: serde::Serialize>(data: T) -> Value {
    serde_json::to_value(data).expect("sample automation data must serialize")
}

/// Curated demo identity shared by every user-carrying sample (and by
/// plugin manifest samples, which have no user of their own).
pub(crate) fn sample_user() -> Value {
    sample_data(AutomationUser {
        user_id: Some("1".to_owned()),
        unique_id: "usuario_demo".to_owned(),
        nickname: "Viewer Demo".to_owned(),
        sec_uid: String::new(),
        avatar_url: Some("sample".to_owned()),
    })
}

/// `(data, user)` for one trigger. Envelope-only events (room stats,
/// connection, points, internal emits) carry no user, matching the live path.
fn sample_data_for(event_type: &str) -> (Value, Option<Value>) {
    let user = || Some(sample_user());
    let method = || SAMPLE_METHOD.to_owned();
    let msg_id = || SAMPLE_MSG_ID.to_owned();
    match event_type {
        "tiktok.chat" => (
            sample_data(ChatAutomationData {
                comment: "Hello there".to_owned(),
                method: method(),
                msg_id: msg_id(),
                is_history: false,
            }),
            user(),
        ),
        "tiktok.gift" => (
            sample_data(GiftAutomationData {
                gift_id: "5655".to_owned(),
                gift_name: "Rosa".to_owned(),
                diamond_count: 1,
                repeat_count: 1,
                combo_count: 1,
                group_id: "sample".to_owned(),
                repeat_end: false,
                streakable: false,
                gift_icon_url: Some("sample".to_owned()),
                method: method(),
                msg_id: msg_id(),
                is_history: false,
            }),
            user(),
        ),
        "tiktok.like" => (
            sample_data(LikeAutomationData {
                count: 1,
                total: 1,
                method: method(),
                msg_id: msg_id(),
                is_history: false,
            }),
            user(),
        ),
        "tiktok.follow" => (
            sample_data(SocialAutomationData {
                action: 1,
                follow_count: 1,
                share_count: 1,
                method: method(),
                msg_id: msg_id(),
                is_history: false,
            }),
            user(),
        ),
        "tiktok.share" => (
            sample_data(SocialAutomationData {
                action: 3,
                follow_count: 1,
                share_count: 1,
                method: method(),
                msg_id: msg_id(),
                is_history: false,
            }),
            user(),
        ),
        "tiktok.social" => (
            sample_data(SocialAutomationData {
                action: 0,
                follow_count: 1,
                share_count: 1,
                method: method(),
                msg_id: msg_id(),
                is_history: false,
            }),
            user(),
        ),
        "tiktok.join" => (
            sample_data(MemberAutomationData {
                member_count: 1,
                action: 0,
                method: method(),
                msg_id: msg_id(),
                is_history: false,
            }),
            user(),
        ),
        "tiktok.room_stats" => (
            sample_data(RoomStatsAutomationData {
                viewers: 1,
                total_users: 1,
                popularity: 1,
                anonymous: 1,
                method: method(),
                msg_id: msg_id(),
                is_history: false,
            }),
            None,
        ),
        "tiktok.connected" | "tiktok.disconnected" => (
            sample_data(ConnectionAutomationData {
                unique_id: "sample".to_owned(),
                room_id: "sample".to_owned(),
            }),
            None,
        ),
        "points.awarded" => (
            sample_data(PointsAwardedAutomationData {
                unique_id: "sample".to_owned(),
                delta: 1.0,
                total_points: 1.0,
                level: 1,
                currency_name: "sample".to_owned(),
                reason: "sample".to_owned(),
            }),
            None,
        ),
        "plugin.emit" => (
            sample_data(PluginEmitAutomationData {
                emit_type: "plugin.sample".to_owned(),
                depth: 0,
                payload: json!({}),
            }),
            None,
        ),
        // Unknown (usually undeclared plugin) triggers: an honest empty
        // payload. Declared plugin types never reach here — the dispatch
        // layer prefers the manifest sample first.
        _ => (json!({}), user()),
    }
}

/// Fresh sample envelope for a trigger (timestamp set, safe to mutate).
pub(crate) fn sample_automation_event(event_type: &str) -> Value {
    let (data, user) = sample_data_for(event_type);
    let mut event = json!({
        "id": "sample-event",
        "type": event_type,
        "timestamp": now_millis(),
        "data": data,
    });
    if let Some(user) = user {
        event["user"] = user;
    }
    event
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every built-in trigger samples the contract the frontend registry
    /// documents for it — no shared generic blob.
    #[test]
    fn samples_cover_every_builtin_trigger_with_its_own_shape() {
        let cases: &[(&str, &[&str], bool)] = &[
            (
                "tiktok.chat",
                &["comment", "method", "msgId", "isHistory"],
                true,
            ),
            (
                "tiktok.gift",
                &[
                    "giftId",
                    "giftName",
                    "diamondCount",
                    "repeatCount",
                    "comboCount",
                    "groupId",
                    "repeatEnd",
                    "streakable",
                    "method",
                    "msgId",
                    "isHistory",
                ],
                true,
            ),
            (
                "tiktok.like",
                &["count", "total", "method", "msgId", "isHistory"],
                true,
            ),
            (
                "tiktok.follow",
                &[
                    "action",
                    "followCount",
                    "shareCount",
                    "method",
                    "msgId",
                    "isHistory",
                ],
                true,
            ),
            (
                "tiktok.share",
                &[
                    "action",
                    "followCount",
                    "shareCount",
                    "method",
                    "msgId",
                    "isHistory",
                ],
                true,
            ),
            (
                "tiktok.social",
                &[
                    "action",
                    "followCount",
                    "shareCount",
                    "method",
                    "msgId",
                    "isHistory",
                ],
                true,
            ),
            (
                "tiktok.join",
                &["memberCount", "action", "method", "msgId", "isHistory"],
                true,
            ),
            (
                "tiktok.room_stats",
                &[
                    "viewers",
                    "totalUsers",
                    "popularity",
                    "anonymous",
                    "method",
                    "msgId",
                    "isHistory",
                ],
                false,
            ),
            ("tiktok.connected", &["uniqueId", "roomId"], false),
            ("tiktok.disconnected", &["uniqueId", "roomId"], false),
            (
                "points.awarded",
                &[
                    "uniqueId",
                    "delta",
                    "totalPoints",
                    "level",
                    "currencyName",
                    "reason",
                ],
                false,
            ),
            ("plugin.emit", &["emitType", "depth", "payload"], false),
        ];
        for (trigger, keys, has_user) in cases {
            let event = sample_automation_event(trigger);
            assert_eq!(event["type"], *trigger, "type for {trigger}");
            assert_eq!(event["id"], "sample-event", "id for {trigger}");
            let data = event["data"].as_object().expect("data must be an object");
            for key in *keys {
                assert!(data.contains_key(*key), "{trigger} sample lacks `{key}`");
            }
            assert_eq!(
                event.get("user").is_some(),
                *has_user,
                "{trigger} user presence"
            );
        }
    }

    #[test]
    fn gift_sample_keeps_the_values_condition_tests_rely_on() {
        let event = sample_automation_event("tiktok.gift");
        assert_eq!(event["data"]["giftName"], "Rosa");
        assert_eq!(event["data"]["giftId"], "5655");
        assert!(event["data"]["diamondCount"].as_u64().unwrap_or_default() >= 1);
    }

    #[test]
    fn action_discriminators_match_the_live_mapping() {
        assert_eq!(
            sample_automation_event("tiktok.follow")["data"]["action"],
            1
        );
        assert_eq!(sample_automation_event("tiktok.share")["data"]["action"], 3);
        assert_eq!(
            sample_automation_event("tiktok.social")["data"]["action"],
            0
        );
        assert_eq!(sample_automation_event("tiktok.join")["data"]["action"], 0);
        assert_eq!(sample_automation_event("plugin.emit")["data"]["depth"], 0);
    }
}
