use serde::{Deserialize, Serialize};
use serde_json::Value;
use tiktools_plugin_api::{AudioOverlap, MediaFileRef};

use crate::{PluginError, PluginResult};

/// A plugin-defined event. The event type remains a string so new plugin event
/// namespaces are discoverable without recompiling the host.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PluginEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: Value,
}

impl PluginEvent {
    pub fn new<T: Serialize>(event_type: impl Into<String>, data: T) -> PluginResult<Self> {
        let data = serde_json::to_value(data).map_err(|error| {
            PluginError::other(format!("could not encode plugin event: {error}"))
        })?;
        Ok(Self {
            event_type: event_type.into(),
            data,
        })
    }

    pub fn from_value(value: Value) -> PluginResult<Self> {
        serde_json::from_value(value)
            .map_err(|error| PluginError::invalid_request(format!("invalid plugin event: {error}")))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EmitIntent {
    pub event_type: String,
    pub data: Value,
}

impl EmitIntent {
    pub fn new(event_type: impl Into<String>, data: Value) -> Self {
        Self {
            event_type: event_type.into(),
            data,
        }
    }

    pub fn serialized<T: Serialize>(event_type: impl Into<String>, data: T) -> PluginResult<Self> {
        Ok(Self::new(
            event_type,
            serde_json::to_value(data).map_err(|error| {
                PluginError::other(format!("could not encode emit intent: {error}"))
            })?,
        ))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioPlayIntent {
    pub file_ref: MediaFileRef,
    #[serde(default = "default_volume")]
    pub volume: f32,
    #[serde(default)]
    pub overlap: AudioOverlap,
}

impl AudioPlayIntent {
    pub fn new(file_ref: MediaFileRef) -> Self {
        Self {
            file_ref,
            volume: 1.0,
            overlap: AudioOverlap::Allow,
        }
    }

    pub fn from_path(path: impl Into<String>) -> Self {
        Self::new(MediaFileRef::from_path(path))
    }

    pub fn volume(mut self, volume: f32) -> Self {
        self.volume = volume;
        self
    }

    pub fn overlap(mut self, overlap: AudioOverlap) -> Self {
        self.overlap = overlap;
        self
    }
}

fn default_volume() -> f32 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "data", rename_all = "kebab-case")]
pub enum HostIntent {
    Emit(EmitIntent),
    AudioPlay(AudioPlayIntent),
}

impl HostIntent {
    pub fn emit(event_type: impl Into<String>, data: Value) -> Self {
        Self::Emit(EmitIntent::new(event_type, data))
    }

    pub fn audio_play(intent: AudioPlayIntent) -> Self {
        Self::AudioPlay(intent)
    }
}

/// Typed result returned by an action or poll call.
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PluginCallResult {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub logs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub intents: Vec<HostIntent>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<PluginEvent>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ActionResult {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub logs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub intents: Vec<HostIntent>,
}

impl ActionResult {
    pub fn summary(summary: impl Into<String>) -> Self {
        Self {
            summary: Some(summary.into()),
            ..Self::default()
        }
    }

    pub fn log(mut self, message: impl Into<String>) -> Self {
        self.logs.push(message.into());
        self
    }

    pub fn intent(mut self, intent: HostIntent) -> Self {
        self.intents.push(intent);
        self
    }

    pub fn emit(mut self, event_type: impl Into<String>, data: Value) -> Self {
        self.intents.push(HostIntent::emit(event_type, data));
        self
    }

    pub fn emit_serialized<T: Serialize>(
        self,
        event_type: impl Into<String>,
        data: T,
    ) -> PluginResult<Self> {
        Ok(self.intent(HostIntent::Emit(EmitIntent::serialized(event_type, data)?)))
    }
}

/// Protocol batch size: one `poll` response carries at most this many
/// events. Plugins with more pending events retain the remainder in their
/// own queue for the next tick; the host accepts the complete bounded
/// response, so events are never removed from the producer queue only to
/// be discarded by the consumer.
pub const POLL_MAX_EVENTS_PER_RESPONSE: usize = 16;

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PollResult {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<PluginEvent>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub logs: Vec<String>,
}

impl PollResult {
    pub fn event(mut self, event: PluginEvent) -> Self {
        self.events.push(event);
        self
    }

    pub fn log(mut self, message: impl Into<String>) -> Self {
        self.logs.push(message.into());
        self
    }
}

impl From<ActionResult> for PluginCallResult {
    fn from(result: ActionResult) -> Self {
        Self {
            summary: result.summary,
            logs: result.logs,
            intents: result.intents,
            events: Vec::new(),
        }
    }
}

impl From<PollResult> for PluginCallResult {
    fn from(result: PollResult) -> Self {
        Self {
            summary: None,
            logs: result.logs,
            intents: Vec::new(),
            events: result.events,
        }
    }
}
