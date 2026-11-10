//! Bounded, host-local recovery state. No credentials, SQL or business effects.
mod hub;
mod snapshots;
use crate::ResourceVersion;
pub use hub::StreamHub;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

pub const STREAM_EVENTS: usize = 4096;
pub const STREAM_BYTES: usize = 4 * 1024 * 1024;
pub const STREAM_SECONDS: u64 = 60;
pub const SNAPSHOT_BYTES: usize = 4 * 1024 * 1024;
pub const SNAPSHOT_SECONDS: u64 = 30;
// RPC IDs are limited to 128 bytes; JSON escaping can expand each to six.
// This also reserves the fixed success-response envelope.
const SNAPSHOT_RPC_RESERVE: usize = 1024;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum StreamDomain {
    Live,
    Points,
    Rankings,
    Competitions,
    Workflows,
    Automation,
    Plugins,
    Widgets,
    Commands,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SubscribeParams {
    pub domains: BTreeSet<StreamDomain>,
    #[serde(default)]
    pub feed: bool,
    /// None is the complete permitted set; Some(n) is a top-N display view.
    /// Some(0) requests only reliable invalidations, with no baseline point rows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub points_limit: Option<u16>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeResult {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub points_limit: Option<u16>,
    pub subscription_id: String,
    pub host_instance_id: String,
    pub grant_revision: String,
    pub domains: BTreeSet<StreamDomain>,
    pub retained_events: usize,
    pub retained_bytes: usize,
    pub retention_seconds: u64,
    pub snapshot_seconds: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SnapshotParams {
    pub subscription_id: String,
    #[serde(default)]
    pub page_token: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotPage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub points_limit: Option<u16>,
    pub subscription_id: String,
    pub snapshot_id: String,
    pub host_instance_id: String,
    pub grant_revision: String,
    pub through_sequence: String,
    pub domains: BTreeSet<StreamDomain>,
    pub workflows: Vec<Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub points: Vec<Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub automation: Vec<Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub competitions: Vec<Value>,
    pub resource_versions: Vec<ResourceVersion>,
    pub next_page_token: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResumeParams {
    pub subscription_id: String,
    pub host_instance_id: String,
    pub last_applied_sequence: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResumeResult {
    pub subscription_id: String,
    pub host_instance_id: String,
    pub grant_revision: String,
    pub after_sequence: String,
    pub through_sequence: String,
    pub replaying: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StreamActor {
    pub principal_id: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReliableEvent {
    pub topic: String,
    pub data: Value,
    pub host_instance_id: String,
    pub subscription_id: String,
    pub sequence: String,
    pub event_id: String,
    pub resource: ResourceVersion,
    pub actor: StreamActor,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub command_id: Option<String>,
}

pub fn stream_sequence(value: &str) -> Result<u64, crate::Error> {
    value
        .parse::<u64>()
        .ok()
        .filter(|n| *n <= i64::MAX as u64 && n.to_string() == value)
        .ok_or(crate::Error::Invalid("invalid stream sequence"))
}

/// Reserve enough for the JSON-RPC envelope and its longest escaped ID.
/// Capture additionally subtracts its exact page metadata before packing.
pub fn snapshot_page_bytes(frame_bytes: usize) -> usize {
    frame_bytes
        .saturating_sub(SNAPSHOT_RPC_RESERVE)
        .min(512 * 1024)
}

#[cfg(test)]
mod tests;
