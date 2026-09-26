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
                gift_name: "Rose".to_owned(),
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

/// Pins a synthetic event to an event record's `eq` filters so firing or
/// testing the record exercises its own configuration (a `giftName = Rose`
/// filter fires a Rose, not whatever the generic sample carries).
///
/// Only exact pins apply: ranges and string operators still evaluate
/// against the data, so they can genuinely fail. Values coerce to the
/// pinned field's JSON type (numbers stay numbers). Returns display-ready
/// `path='value'` entries for honest reporting.
///
/// Callers must only pin synthetic (sample) envelopes: live and custom
/// data stay truthful even when they match nothing.
pub(crate) fn pin_event_to_record(event: &mut Value, record: &Value) -> Vec<String> {
    let Some(filters) = record.get("filters").and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut pinned = Vec::new();
    for filter in filters {
        if filter.get("operator").and_then(Value::as_str) != Some("eq") {
            continue;
        }
        let Some(path) = filter.get("path").and_then(Value::as_str) else {
            continue;
        };
        let Some(value) = filter.get("value").and_then(Value::as_str) else {
            continue;
        };
        if pin_event_path(event, path, value) {
            pinned.push(format!("{path}='{value}'"));
        }
    }
    pinned
}

fn pin_event_path(event: &mut Value, path: &str, value: &str) -> bool {
    let mut parts: Vec<&str> = path
        .trim()
        .trim_start_matches("{{")
        .trim_end_matches("}}")
        .trim()
        .trim_matches('.')
        .split('.')
        .filter(|part| !part.is_empty())
        .collect();
    if parts.first() == Some(&"event") {
        parts.remove(0);
    }
    let Some((leaf, parents)) = parts.split_last() else {
        return false;
    };
    let mut current = event;
    for part in parents {
        if !current.is_object() {
            return false;
        }
        if current.get(part).is_none() {
            current[part] = Value::Object(Default::default());
        }
        current = &mut current[part];
    }
    if !current.is_object() {
        return false;
    }
    current[*leaf] = coerce_pin_value(current.get(*leaf), value);
    true
}

fn coerce_pin_value(existing: Option<&Value>, value: &str) -> Value {
    match existing {
        Some(Value::Number(_)) => {
            value
                .trim()
                .parse::<i64>()
                .map(Value::from)
                .unwrap_or_else(|_| {
                    value
                        .trim()
                        .parse::<f64>()
                        .ok()
                        .filter(|number| number.is_finite())
                        .and_then(serde_json::Number::from_f64)
                        .map(Value::Number)
                        .unwrap_or_else(|| Value::String(value.to_owned()))
                })
        }
        Some(Value::Bool(_)) => match value.trim().to_ascii_lowercase().as_str() {
            "true" | "1" => Value::Bool(true),
            "false" | "0" => Value::Bool(false),
            _ => Value::String(value.to_owned()),
        },
        _ => Value::String(value.to_owned()),
    }
}

/// Keeps a gift event's name/id pair coherent with the gift catalog after
/// pinning: when the record pins `giftName`, the id follows the catalog
/// entry (and vice versa). Explicitly pinned sides always win — only the
/// unpinned side is ever adjusted, and unknown names leave the sample id
/// untouched instead of inventing one.
pub(crate) fn cohere_gift_pair(event: &mut Value, pinned: &[String], catalog: &[Value]) -> bool {
    if event.get("type").and_then(Value::as_str) != Some("tiktok.gift") {
        return false;
    }
    let Some(data) = event.get_mut("data").and_then(Value::as_object_mut) else {
        return false;
    };
    if catalog.is_empty() {
        return false;
    }
    let targets = |field: &str| {
        pinned.iter().any(|pin| {
            pin.starts_with(&format!("event.data.{field}="))
                || pin.starts_with(&format!("data.{field}="))
        })
    };
    let name_pinned = targets("giftName");
    let id_pinned = targets("giftId");
    // Only fill the missing side of a pinned pair: without pins the sample
    // stays exactly as curated, and double pins stay exactly as configured.
    if !name_pinned && !id_pinned {
        return false;
    }
    let name = data
        .get("giftName")
        .and_then(Value::as_str)
        .filter(|name| !name.is_empty());
    let id = data
        .get("giftId")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty());
    if name_pinned && !id_pinned {
        if let Some(name) = name {
            if let Some(entry) = catalog_entry_by_name(catalog, name) {
                if let Some(entry_id) = entry.get("id").and_then(Value::as_str) {
                    data.insert("giftId".to_owned(), Value::String(entry_id.to_owned()));
                    return true;
                }
            }
        }
    } else if id_pinned && !name_pinned {
        if let Some(id) = id {
            if let Some(entry) = catalog
                .iter()
                .find(|entry| entry.get("id").and_then(Value::as_str) == Some(id))
            {
                if let Some(entry_name) = entry.get("name").and_then(Value::as_str) {
                    data.insert("giftName".to_owned(), Value::String(entry_name.to_owned()));
                    return true;
                }
            }
        }
    }
    false
}

fn catalog_entry_by_name<'a>(catalog: &'a [Value], name: &str) -> Option<&'a Value> {
    catalog
        .iter()
        .find(|entry| entry.get("name").and_then(Value::as_str) == Some(name))
        .or_else(|| {
            catalog.iter().find(|entry| {
                entry
                    .get("name")
                    .and_then(Value::as_str)
                    .is_some_and(|candidate| candidate.eq_ignore_ascii_case(name))
            })
        })
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
        assert_eq!(event["data"]["giftName"], "Rose");
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

    #[test]
    fn pinning_applies_eq_filters_with_type_coercion() {
        let mut event = sample_automation_event("tiktok.gift");
        let record = serde_json::json!({
            "trigger": "tiktok.gift",
            "filters": [
                {"path": "event.data.giftName", "operator": "eq", "value": "Galaxy"},
                {"path": "event.data.comboCount", "operator": "eq", "value": "7"},
                {"path": "event.data.repeatEnd", "operator": "eq", "value": "true"},
                {"path": "event.user.uniqueId", "operator": "eq", "value": "luna_dev"},
                {"path": "event.data.diamondCount", "operator": "gte", "value": "100"},
                {"path": "event.data.giftId", "operator": "contains", "value": "56"},
            ]
        });
        let pinned = pin_event_to_record(&mut event, &record);
        assert_eq!(
            pinned,
            vec![
                "event.data.giftName='Galaxy'",
                "event.data.comboCount='7'",
                "event.data.repeatEnd='true'",
                "event.user.uniqueId='luna_dev'",
            ]
        );
        assert_eq!(event["data"]["giftName"], "Galaxy");
        assert_eq!(event["data"]["comboCount"], 7);
        assert_eq!(event["data"]["repeatEnd"], true);
        assert_eq!(event["user"]["uniqueId"], "luna_dev");
        // Non-eq operators never pin.
        assert_eq!(event["data"]["diamondCount"], 1);
        assert_eq!(event["data"]["giftId"], "5655");
    }

    #[test]
    fn pinning_ignores_malformed_and_mistyped_filters() {
        let mut event = sample_automation_event("tiktok.chat");
        let record = serde_json::json!({
            "trigger": "tiktok.chat",
            "filters": [
                {"operator": "eq", "value": "x"},
                {"path": "event.data.comment", "operator": "eq"},
                {"path": "", "operator": "eq", "value": "x"},
                {"path": "event.data.comment.text", "operator": "eq", "value": "x"},
            ]
        });
        assert!(pin_event_to_record(&mut event, &record).is_empty());
        assert_eq!(event["data"]["comment"], "Hello there");
    }

    fn catalog() -> Vec<Value> {
        vec![
            serde_json::json!({"id": "5655", "name": "Rose", "diamondCount": 1}),
            serde_json::json!({"id": "9999", "name": "Galaxy", "diamondCount": 1000}),
        ]
    }

    #[test]
    fn coherence_follows_the_pinned_side_from_the_catalog() {
        // Pinned name pulls the catalog id.
        let mut event = sample_automation_event("tiktok.gift");
        let pinned = vec!["event.data.giftName='Galaxy'".to_owned()];
        event["data"]["giftName"] = Value::String("Galaxy".to_owned());
        assert!(cohere_gift_pair(&mut event, &pinned, &catalog()));
        assert_eq!(event["data"]["giftId"], "9999");

        // Pinned id pulls the catalog name, replacing the sample name.
        let mut event = sample_automation_event("tiktok.gift");
        let pinned = vec!["event.data.giftId='9999'".to_owned()];
        event["data"]["giftId"] = Value::String("9999".to_owned());
        assert!(cohere_gift_pair(&mut event, &pinned, &catalog()));
        assert_eq!(event["data"]["giftName"], "Galaxy");
    }

    #[test]
    fn coherence_leaves_explicit_and_unknown_pairs_alone() {
        // Double pins win over the catalog.
        let mut event = sample_automation_event("tiktok.gift");
        let pinned = vec![
            "event.data.giftName='Custom'".to_owned(),
            "event.data.giftId='1'".to_owned(),
        ];
        event["data"]["giftName"] = Value::String("Custom".to_owned());
        event["data"]["giftId"] = Value::String("1".to_owned());
        assert!(!cohere_gift_pair(&mut event, &pinned, &catalog()));
        assert_eq!(event["data"]["giftName"], "Custom");
        assert_eq!(event["data"]["giftId"], "1");

        // Unknown names keep the sample id instead of inventing one.
        let mut event = sample_automation_event("tiktok.gift");
        let pinned = vec!["event.data.giftName='Nope'".to_owned()];
        event["data"]["giftName"] = Value::String("Nope".to_owned());
        assert!(!cohere_gift_pair(&mut event, &pinned, &catalog()));
        assert_eq!(event["data"]["giftId"], "5655");

        // No pins, no rewrite — even if the catalog disagrees.
        let mut event = sample_automation_event("tiktok.gift");
        assert!(!cohere_gift_pair(&mut event, &[], &catalog()));
        assert_eq!(event["data"]["giftId"], "5655");

        // Non-gift events are untouched.
        let mut event = sample_automation_event("tiktok.chat");
        assert!(!cohere_gift_pair(
            &mut event,
            &["event.data.comment='x'".to_owned()],
            &catalog()
        ));
    }
}
