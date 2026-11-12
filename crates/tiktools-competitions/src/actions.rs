//! Host-owned winner action references and execution records.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A reference to an existing automation action; execution remains the
/// responsibility of the host automation runtime and its capability checks.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WinnerAction {
    pub action_id: String,
    pub inputs: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ActionClaim {
    pub competition_id: String,
    pub generation: u64,
    pub idempotency_key: String,
    pub action: WinnerAction,
}

/// Public execution status omits action inputs and idempotency keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Uncertain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActionResolution {
    Completed,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ActionExecution {
    pub competition_id: String,
    pub generation: u64,
    pub action_id: String,
    pub status: ActionStatus,
    /// Sanitized, machine-readable reason; never includes action inputs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
}
