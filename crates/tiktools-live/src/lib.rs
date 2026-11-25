//! Provider-neutral contracts for live platform plugins.
//!
//! Provider plugins own transport, authentication, reconnects and parsing.
//! They never write points, feed rows or automation authority directly: they
//! emit these bounded ingestion messages and the host validates, stamps and
//! distributes the resulting canonical events.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::Digest;

pub const SCHEMA_VERSION: u32 = 1;
/// Plugin-owned transport event consumed by the host ingestion boundary.
/// It is deliberately outside the reserved `live.` namespace; only plugins
/// with the `live.ingest` capability have it interpreted as ingestion.
pub const INGEST_EVENT_TYPE: &str = "platform.live.ingest";
pub const MAX_BATCH_MESSAGES: usize = 96;
pub const MAX_STREAM_CHARS: usize = 64;
pub const MAX_EVENT_TYPE_CHARS: usize = 32;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Capabilities {
    #[serde(default)]
    pub chat: bool,
    #[serde(default)]
    pub follows: bool,
    #[serde(default)]
    pub subscriptions: bool,
    #[serde(default)]
    pub gifts: bool,
    #[serde(default)]
    pub likes: bool,
    #[serde(default)]
    pub shares: bool,
    #[serde(default)]
    pub joins: bool,
    #[serde(default)]
    pub room_stats: bool,
    #[serde(default)]
    pub discovery: bool,
    #[serde(default)]
    pub send_chat: bool,
    #[serde(default)]
    pub moderation: bool,
}

/// Provider channel identity supplied by a plugin. The host adds the trusted
/// platform owner before storing or publishing it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChannelInput {
    pub id: String,
    pub handle: String,
    #[serde(default)]
    pub room_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChannelIdentity {
    pub platform: String,
    pub id: String,
    pub handle: String,
    pub room_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ViewerInput {
    pub id: String,
    #[serde(default)]
    pub handle: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ViewerIdentity {
    pub platform: String,
    pub id: String,
    pub handle: String,
    pub display_name: String,
    #[serde(default)]
    pub avatar_url: Option<String>,
}

impl ViewerIdentity {
    pub fn key(&self) -> String {
        identity_key(&self.platform, &self.id)
    }
}

/// Collision-free storage/automation identity. The readable length-prefixed
/// form is used when it fits the existing 128-character viewer-key limit;
/// longer provider/plugin identities use stable SHA-256 fingerprints so they
/// can never collide by truncation.
pub fn identity_key(platform: &str, id: &str) -> String {
    let readable = format!("{}:{}:{}", platform, id.len(), id);
    if readable.len() <= 128 {
        return readable;
    }
    let platform_digest = sha256_hex(platform.as_bytes());
    let id_digest = sha256_hex(id.as_bytes());
    format!("p:{}:{}", &platform_digest[..16], &id_digest[..32])
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = sha2::Sha256::digest(bytes);
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SessionState {
    Connecting,
    Connected,
    Reconnecting,
    Disconnected,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlatformError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum LivePayload {
    Chat {
        comment: String,
    },
    Follow,
    Subscription {
        #[serde(default)]
        tier: Option<String>,
        #[serde(default)]
        months: Option<u64>,
        #[serde(default)]
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
        #[serde(default)]
        total: Option<u64>,
    },
    Share {
        #[serde(default = "one")]
        count: u64,
    },
    Join,
    RoomStats {
        viewers: u64,
        #[serde(default)]
        total_users: Option<u64>,
    },
    Connected,
    Disconnected,
}

fn one() -> u64 {
    1
}

impl LivePayload {
    pub fn event_type(&self) -> &'static str {
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
        }
    }
}

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

/// One-way legacy projection: generic rules can consume native TikTok events,
/// while TikTok-specific rules never consume other platforms.
pub fn canonical_trigger(topic: &str) -> Option<&'static str> {
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LiveEventInput {
    #[serde(default)]
    pub source_event_id: Option<String>,
    #[serde(default)]
    pub occurred_at: Option<u64>,
    #[serde(default)]
    pub is_history: bool,
    #[serde(default)]
    pub user: Option<ViewerInput>,
    pub payload: LivePayload,
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub extensions: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LiveEnvelope {
    pub schema_version: u32,
    pub platform: String,
    pub connection_id: String,
    pub session_id: String,
    pub channel: ChannelIdentity,
    #[serde(default)]
    pub source_event_id: Option<String>,
    pub occurred_at: u64,
    pub received_at: u64,
    pub is_history: bool,
    #[serde(default)]
    pub user: Option<ViewerIdentity>,
    pub payload: LivePayload,
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub extensions: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlatformSessionSnapshot {
    pub session_id: String,
    pub connection_id: String,
    pub platform: String,
    pub stream: String,
    pub state: SessionState,
    #[serde(default)]
    pub channel: Option<ChannelIdentity>,
    pub capabilities: Capabilities,
    pub reconnect_attempt: u32,
    #[serde(default)]
    pub retry_after_ms: Option<u64>,
    #[serde(default)]
    pub error: Option<PlatformError>,
    pub started_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum PlatformIngestMessage {
    SessionStart {
        stream: String,
        channel: ChannelInput,
        #[serde(default)]
        capabilities: Capabilities,
    },
    SessionState {
        stream: String,
        state: SessionState,
        #[serde(default)]
        reconnect_attempt: u32,
        #[serde(default)]
        retry_after_ms: Option<u64>,
        #[serde(default)]
        error: Option<PlatformError>,
    },
    SessionEnd {
        stream: String,
        #[serde(default)]
        error: Option<PlatformError>,
    },
    Gap {
        stream: String,
        lost: u64,
    },
    Event {
        stream: String,
        event: LiveEventInput,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LiveIngestBatch {
    pub schema_version: u32,
    pub messages: Vec<PlatformIngestMessage>,
}

impl LiveIngestBatch {
    pub fn new(messages: Vec<PlatformIngestMessage>) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            messages,
        }
    }
}

pub fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_keys_are_collision_free_and_bounded() {
        assert_eq!(identity_key("twitch", "42"), "twitch:2:42");
        let long = identity_key(&"platform".repeat(30), &"user".repeat(80));
        assert!(long.len() <= 128);
        assert!(long.starts_with("p:"));
        assert_ne!(identity_key("a:1:x", "bc"), identity_key("a", "1:xbc"));
    }

    #[test]
    fn batch_uses_versioned_wire_shape() {
        let batch = LiveIngestBatch::new(vec![PlatformIngestMessage::Gap {
            stream: "main".into(),
            lost: 2,
        }]);
        let value = serde_json::to_value(&batch).unwrap();
        assert_eq!(value["schemaVersion"], 1);
        assert_eq!(value["messages"][0]["kind"], "gap");
        let decoded: LiveIngestBatch = serde_json::from_value(value).unwrap();
        assert_eq!(decoded, batch);
    }

    #[test]
    fn legacy_aliases_are_one_way() {
        assert_eq!(canonical_trigger("tiktok.chat"), Some("live.chat"));
        assert_eq!(canonical_trigger("live.chat"), None);
        assert!(is_live_trigger("live.subscription"));
    }
}
