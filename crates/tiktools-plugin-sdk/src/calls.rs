use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{tiktools_plugin_api::DomainEventEnvelope, EventEnrichmentRequest};

/// A typed action call. The action descriptor remains a JSON value so plugin
/// authors can define their own config schema without host-side registration.
#[derive(Debug, Clone, PartialEq)]
pub struct ActionCall {
    pub action: Value,
    pub event: Value,
}

impl ActionCall {
    pub fn action_type(&self) -> Option<&str> {
        self.action
            .get("typeId")
            .or_else(|| self.action.get("type"))
            .and_then(Value::as_str)
    }

    pub fn config(&self) -> &Map<String, Value> {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        self.action
            .get("config")
            .and_then(Value::as_object)
            .unwrap_or_else(|| EMPTY.get_or_init(Map::new))
    }
}

/// Typed calls at the SDK boundary. The serialized shape remains compatible
/// with the existing `{"type":"action"|"poll"}` process protocol; `enrich`
/// is additive. Existing plugins can keep their default handlers and never
/// need to know about the newer call variants.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum PluginCall {
    Action {
        action: Value,
        #[serde(default)]
        event: Value,
    },
    Poll,
    Enrich {
        request: EventEnrichmentRequest,
    },
    Event {
        event: DomainEventEnvelope,
    },
}

impl PluginCall {
    pub fn action(action: Value, event: Value) -> Self {
        Self::Action { action, event }
    }

    pub fn enrich(request: EventEnrichmentRequest) -> Self {
        Self::Enrich { request }
    }

    pub fn event(event: DomainEventEnvelope) -> Self {
        Self::Event { event }
    }

    pub fn into_action(self) -> Option<ActionCall> {
        match self {
            Self::Action { action, event } => Some(ActionCall { action, event }),
            Self::Poll | Self::Enrich { .. } | Self::Event { .. } => None,
        }
    }

    pub fn into_enrich(self) -> Option<EventEnrichmentRequest> {
        match self {
            Self::Enrich { request } => Some(request),
            Self::Action { .. } | Self::Poll | Self::Event { .. } => None,
        }
    }

    pub fn into_event(self) -> Option<DomainEventEnvelope> {
        match self {
            Self::Event { event } => Some(event),
            Self::Action { .. } | Self::Poll | Self::Enrich { .. } => None,
        }
    }
}
