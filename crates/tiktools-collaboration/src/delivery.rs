//! Closed metadata shared by durable delivery, audit and future scoped streams.
//! No graph, script, settings body, chat or credential belongs in these types.
use crate::{CommandReceipt, CommandResource, ResourceVersion};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuditRecord {
    pub event_id: String,
    pub timestamp: i64,
    pub principal_id: String,
    pub session_id: String,
    pub method: String,
    pub resource: CommandResource,
    pub command_id: Option<String>,
    pub before_revision: String,
    pub after_revision: String,
    pub result_code: String,
    pub correlation_id: String,
}
impl AuditRecord {
    pub fn validate(&self) -> Result<(), crate::Error> {
        let text = |value: &str, limit| {
            !value.is_empty() && value.len() <= limit && !value.chars().any(char::is_control)
        };
        let revision = |value: &str| {
            value
                .parse::<i64>()
                .ok()
                .is_some_and(|number| number >= 0 && number.to_string() == value)
        };
        let code = |value: &str, limit| {
            text(value, limit)
                && value.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b':')
                })
        };
        if self.event_id.len() != 64
            || !self
                .event_id
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            || self.timestamp < 0
            || !text(&self.principal_id, 256)
            || !text(&self.session_id, 256)
            || !code(&self.method, 128)
            || !text(&self.resource.id, 256)
            || !text(&self.resource.scope, 256)
            || self
                .command_id
                .as_ref()
                .is_some_and(|id| !crate::valid_command_id(id))
            || !revision(&self.before_revision)
            || !revision(&self.after_revision)
            || !code(&self.result_code, 64)
            || !text(&self.correlation_id, 128)
        {
            return Err(crate::Error::Invalid("invalid coordination audit metadata"));
        }
        Ok(())
    }
}

/// Projection notifications only. They must never enter the DomainEvent bus
/// that feeds automation/plugin execution. Subscribers handle lag by resyncing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum CoordinationEvent {
    Invalidated {
        event_id: String,
        resource: ResourceVersion,
        principal_id: String,
        command_id: Option<String>,
        change: String,
        committed_at: i64,
    },
    CommandChanged {
        event_id: String,
        resource: ResourceVersion,
        principal_id: String,
        command: CommandReceipt,
        committed_at: i64,
    },
}
