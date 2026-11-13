//! Common event data, independent of vendor protobuf schemas and currencies.
use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LiveChatAutomationData {
    pub comment: String,
    #[serde(default)]
    pub emotes: Vec<ChatAutomationEmote>,
    pub msg_id: String,
    pub method: String,
    pub is_history: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LiveActivityAutomationData {
    pub msg_id: String,
    pub method: String,
    pub is_history: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LiveGiftAutomationData {
    pub id: String,
    pub name: String,
    pub count: u64,
    pub unit: String,
    /// Decimal string in the stated unit, never assumed to be fiat currency.
    pub amount: String,
    pub msg_id: String,
    pub method: String,
    pub is_history: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LiveSubscriptionAutomationData {
    pub tier: Option<String>,
    pub months: Option<u64>,
    pub gifted: bool,
    pub msg_id: String,
    pub method: String,
    pub is_history: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LiveRoomStatsAutomationData {
    pub viewers: u64,
    pub total_users: Option<u64>,
    pub msg_id: String,
    pub method: String,
    pub is_history: bool,
}
