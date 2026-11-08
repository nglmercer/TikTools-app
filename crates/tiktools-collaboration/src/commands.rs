//! Shared command wire data. Caller identity is deliberately a separate,
//! non-deserializable host-side value in core, never part of these requests.
use crate::{ResourceKind, ResourceVersion};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CommandResource {
    pub kind: ResourceKind,
    pub id: String,
    pub scope: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CommandState {
    Accepted,
    Running,
    Succeeded,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CommandReceipt {
    pub command_id: String,
    pub state: CommandState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource: Option<CommandResource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub committed_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CommandOutcome {
    pub principal_id: String,
    pub method: String,
    pub host_instance_id: String,
    pub command: CommandReceipt,
    pub result: Value,
    pub resource_versions: Vec<ResourceVersion>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommandGet {
    pub command_id: String,
    /// Only the local owner can select another principal.
    #[serde(default)]
    pub principal_id: Option<String>,
}

/// Metadata supplied by the caller. The host supplies principal/session and
/// its current instance separately; these fields cannot grant authority.
#[derive(Debug, Clone)]
pub struct CommandRequest {
    pub command_id: String,
    pub host_instance_id: String,
    pub expected_revision: String,
}

/// Additive operations deliberately have no compare-and-set precondition.
#[derive(Debug, Clone)]
pub struct AdditiveCommandRequest {
    pub command_id: String,
    pub host_instance_id: String,
}
impl AdditiveCommandRequest {
    pub fn validate(&self) -> Result<(), crate::Error> {
        if !valid_command_id(&self.command_id) || !valid_command_id(&self.host_instance_id) {
            return Err(crate::Error::Invalid("invalid command metadata"));
        }
        Ok(())
    }
}
impl CommandRequest {
    pub fn validate(&self) -> Result<(), crate::Error> {
        if !valid_command_id(&self.command_id)
            || !valid_command_id(&self.host_instance_id)
            || self
                .expected_revision
                .parse::<i64>()
                .ok()
                .filter(|revision| *revision >= 0 && revision.to_string() == self.expected_revision)
                .is_none()
        {
            return Err(crate::Error::Invalid("invalid command metadata"));
        }
        Ok(())
    }
}
pub fn valid_command_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':'))
}
