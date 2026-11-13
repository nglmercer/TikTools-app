//! Provider-independent contracts. No database, desktop, or provider dependencies.
mod manager;
#[cfg(test)]
mod tests;
pub use manager::{Adapter, AdapterRequest, ConnectionManager, EventSink, ManagedEvent};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{fmt, future::Future, pin::Pin};

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum PlatformId {
    Tiktok,
    Twitch,
    Youtube,
    Facebook,
    Kick,
}

impl PlatformId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Tiktok => "tiktok",
            Self::Twitch => "twitch",
            Self::Youtube => "youtube",
            Self::Facebook => "facebook",
            Self::Kick => "kick",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ViewerIdentity {
    pub platform: PlatformId,
    /// Provider-issued stable ID, never a display name or a JavaScript number.
    pub id: String,
    pub handle: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
}

impl ViewerIdentity {
    /// Collision-free internal key. Legacy TikTok handles stay in the legacy
    /// table; all new identities use a separate, explicit mapping table.
    pub fn key(&self) -> String {
        identity_key(self.platform, &self.id)
    }
}

pub fn identity_key(platform: PlatformId, id: &str) -> String {
    format!("{}:{}:{}", platform.as_str(), id.len(), id)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChannelIdentity {
    pub platform: PlatformId,
    pub id: String,
    pub handle: String,
    pub room_id: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub chat: bool,
    pub follows: bool,
    pub subscriptions: bool,
    pub gifts: bool,
    pub likes: bool,
    pub shares: bool,
    pub joins: bool,
    pub room_stats: bool,
    pub discovery: bool,
    pub send_chat: bool,
    pub moderation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlatformDescriptor {
    pub platform: PlatformId,
    pub name: String,
    pub capabilities: Capabilities,
    pub authentication: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionState {
    Connecting,
    Connected,
    Reconnecting,
    Disconnected,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionSnapshot {
    pub connection_id: String,
    pub platform: PlatformId,
    pub target: String,
    pub state: ConnectionState,
    pub channel: Option<ChannelIdentity>,
    pub capabilities: Capabilities,
    pub reconnect_attempt: u32,
    pub retry_after_ms: Option<u64>,
    pub error: Option<LiveError>,
}

/// Secrets cannot be serialized, and their Debug implementation is redacted.
#[derive(Clone, Default)]
pub struct Credential(String);
impl Credential {
    pub fn new(value: String) -> Self {
        Self(value)
    }
    pub fn expose(&self) -> &str {
        &self.0
    }
}
impl fmt::Debug for Credential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Credential([REDACTED])")
    }
}

#[derive(Debug, Clone, PartialEq, thiserror::Error, Serialize, Deserialize, JsonSchema)]
#[error("{message}")]
#[serde(rename_all = "camelCase")]
pub struct LiveError {
    pub code: String,
    pub message: String,
    pub retry_after_ms: Option<u64>,
}
impl LiveError {
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            retry_after_ms: None,
        }
    }
}

/// Common semantics only. Monetary units and platform-specific attributes are
/// preserved explicitly; unsupported events are never synthesized.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum LivePayload {
    Chat {
        comment: String,
        emotes: Vec<serde_json::Value>,
    },
    Follow,
    Subscription {
        tier: Option<String>,
        months: Option<u64>,
        gifted: bool,
    },
    Gift {
        id: String,
        name: String,
        count: u64,
        unit: String,
        amount: String,
    },
    Like {
        count: u64,
        total: Option<u64>,
    },
    Share {
        count: u64,
    },
    Join,
    RoomStats {
        viewers: u64,
        total_users: Option<u64>,
    },
    Connected,
    Disconnected,
    Extension {
        event_type: String,
        data: serde_json::Value,
    },
}
impl LivePayload {
    pub fn event_type(&self) -> &str {
        match self {
            Self::Chat { .. } => "live.chat",
            Self::Follow => "live.follow",
            Self::Subscription { .. } => "live.subscription",
            Self::Gift { .. } => "live.gift",
            Self::Like { .. } => "live.like",
            Self::Share { .. } => "live.share",
            Self::Join => "live.join",
            Self::RoomStats { .. } => "live.room_stats",
            Self::Connected => "live.connected",
            Self::Disconnected => "live.disconnected",
            Self::Extension { event_type, .. } => event_type,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LiveEnvelope {
    pub schema_version: u32,
    pub platform: PlatformId,
    pub connection_id: String,
    pub channel: ChannelIdentity,
    pub session_id: String,
    pub source_event_id: Option<String>,
    pub occurred_at: u64,
    pub received_at: u64,
    pub is_history: bool,
    pub user: Option<ViewerIdentity>,
    pub payload: LivePayload,
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub extensions: serde_json::Map<String, serde_json::Value>,
}

/// Explicit catalog, not prefix-based trust. Authorization still depends on
/// the host's private execution context, never on any serialized field.
pub fn is_live_trigger(topic: &str) -> bool {
    matches!(
        topic,
        "live.chat"
            | "live.follow"
            | "live.subscription"
            | "live.gift"
            | "live.like"
            | "live.share"
            | "live.join"
            | "live.room_stats"
            | "live.connected"
            | "live.disconnected"
    )
}
pub fn legacy_alias(topic: &str) -> Option<&'static str> {
    match topic {
        "tiktok.chat" => Some("live.chat"),
        "tiktok.follow" => Some("live.follow"),
        "tiktok.gift" => Some("live.gift"),
        "tiktok.like" => Some("live.like"),
        "tiktok.share" => Some("live.share"),
        "tiktok.join" => Some("live.join"),
        "tiktok.room_stats" => Some("live.room_stats"),
        "tiktok.connected" => Some("live.connected"),
        "tiktok.disconnected" => Some("live.disconnected"),
        _ => None,
    }
}
pub fn trigger_matches(trigger: Option<&str>, event: Option<&str>) -> bool {
    trigger.is_some() && (trigger == event || event.and_then(legacy_alias) == trigger)
}
pub fn trigger_matches_event(trigger: Option<&str>, event: &serde_json::Value) -> bool {
    let topic = event.get("type").and_then(serde_json::Value::as_str);
    if trigger == topic {
        return trigger.is_some();
    }
    let alias = if topic == Some("tiktok.join") && event["data"]["action"].as_i64() == Some(3) {
        Some("live.subscription")
    } else {
        topic.and_then(legacy_alias)
    };
    trigger.is_some() && trigger == alias
}
pub fn event_for_trigger<'a>(
    event: &'a serde_json::Value,
    trigger: Option<&str>,
) -> std::borrow::Cow<'a, serde_json::Value> {
    let event_type = event.get("type").and_then(serde_json::Value::as_str);
    if event_type != trigger && trigger_matches_event(trigger, event) {
        let mut view = event.clone();
        view["type"] = serde_json::Value::String(trigger.unwrap().into());
        std::borrow::Cow::Owned(view)
    } else {
        std::borrow::Cow::Borrowed(event)
    }
}
pub fn is_builtin_live_trigger(topic: &str) -> bool {
    is_live_trigger(topic)
        || legacy_alias(topic).is_some()
        || matches!(topic, "tiktok.emote_chat" | "tiktok.social")
}

pub fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u64::MAX as u128) as u64
}
