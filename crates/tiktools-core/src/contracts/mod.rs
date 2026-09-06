//! The application-owned automation contract layer.
//!
//! The native TikTok crate owns the wire-normalized live-event values. These
//! types describe the JSON envelope that TikTools sends to automation and the
//! frontend. The checked-in TypeScript contracts and event registry are
//! generated from this module; consumers should import those generated types
//! instead of recreating them.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AutomationUser {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    pub unique_id: String,
    pub nickname: String,
    pub sec_uid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AutomationCreator {
    pub unique_id: String,
    pub room_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AutomationPoints {
    pub delta: f64,
    pub total: f64,
    pub level: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AutomationEvent {
    pub id: String,
    #[serde(rename = "type")]
    #[schemars(rename = "type")]
    pub event_type: String,
    pub timestamp: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creator: Option<AutomationCreator>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<AutomationUser>,
    pub data: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub points: Option<AutomationPoints>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_event_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChatAutomationData {
    pub comment: String,
    pub method: String,
    pub msg_id: String,
    pub is_history: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GiftAutomationData {
    pub gift_id: String,
    pub gift_name: String,
    pub diamond_count: u64,
    pub repeat_count: u64,
    pub combo_count: u64,
    pub group_id: String,
    pub repeat_end: bool,
    pub streakable: bool,
    pub gift_icon_url: Option<String>,
    pub method: String,
    pub msg_id: String,
    pub is_history: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LikeAutomationData {
    pub count: u64,
    pub total: u64,
    pub method: String,
    pub msg_id: String,
    pub is_history: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SocialAutomationData {
    pub action: i64,
    pub follow_count: u64,
    pub share_count: u64,
    pub method: String,
    pub msg_id: String,
    pub is_history: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MemberAutomationData {
    pub member_count: u64,
    pub action: i32,
    pub method: String,
    pub msg_id: String,
    pub is_history: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RoomStatsAutomationData {
    pub viewers: u64,
    pub total_users: u64,
    pub popularity: u64,
    pub anonymous: u64,
    pub method: String,
    pub msg_id: String,
    pub is_history: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionAutomationData {
    pub unique_id: String,
    pub room_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PointsAwardedAutomationData {
    pub unique_id: String,
    pub delta: f64,
    pub total_points: f64,
    pub level: u32,
    pub currency_name: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginEmitAutomationData {
    pub emit_type: String,
    pub depth: u32,
    pub payload: Value,
}

/// Root used by the schema generator so every public automation contract is
/// emitted into one deterministic `$defs` collection.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AutomationContracts {
    pub automation_user: AutomationUser,
    pub automation_creator: AutomationCreator,
    pub automation_points: AutomationPoints,
    pub automation_event: AutomationEvent,
    pub chat: ChatAutomationData,
    pub gift: GiftAutomationData,
    pub like: LikeAutomationData,
    pub social: SocialAutomationData,
    pub member: MemberAutomationData,
    pub room_stats: RoomStatsAutomationData,
    pub connection: ConnectionAutomationData,
    pub points_awarded: PointsAwardedAutomationData,
    pub plugin_emit: PluginEmitAutomationData,
}

/// Returns the complete schema as JSON for the repository generator.
pub fn automation_contract_schema() -> Value {
    serde_json::to_value(schemars::schema_for!(AutomationContracts))
        .expect("automation contract schema must serialize")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_contracts_serialize_with_the_frontend_names() {
        let event = serde_json::to_value(AutomationEvent {
            id: "event-1".to_owned(),
            event_type: "tiktok.chat".to_owned(),
            timestamp: 1,
            connection_id: None,
            creator: None,
            user: Some(AutomationUser {
                user_id: Some("42".to_owned()),
                unique_id: "alice".to_owned(),
                nickname: "Alice".to_owned(),
                sec_uid: "".to_owned(),
            }),
            data: serde_json::json!({}),
            points: None,
            source_event_id: None,
        })
        .expect("automation event should serialize");
        assert_eq!(event["type"], "tiktok.chat");
        assert_eq!(event["user"]["userId"], "42");
        assert!(event["user"].get("avatarUrl").is_none());
    }

    #[test]
    fn generated_schema_has_the_same_envelope_keys() {
        let schema = automation_contract_schema();
        let properties = &schema["$defs"]["AutomationEvent"]["properties"];
        assert!(properties.get("type").is_some());
        assert!(properties.get("eventType").is_none());
        assert!(schema["$defs"]["AutomationUser"]["properties"]
            .get("avatarUrl")
            .is_none());
    }
}
