//! Developer-facing SDK for TikTools plugins.
//!
//! The SDK is intentionally runtime-neutral. It provides typed calls/results,
//! a small plugin trait, and adapters for the existing framed process and
//! native ABI boundaries. It does not depend on Tokio, the desktop crate,
//! Wry, Winit, or a particular WASM engine.

use std::{collections::BTreeMap, env, io};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use thiserror::Error;
use tiktools_plugin_api::{
    read_frame, write_frame, AudioOverlap, CapabilitySet, FrameError, MediaFileRef, PermissionSet,
    PluginBuffer, PluginRequest, PluginResponse, PluginStatus, MAX_FRAME_BYTES, METHOD_CALL,
    TIKTOOLS_PLUGIN_PROTOCOL_VERSION,
};

pub use tiktools_plugin_api;
pub use tiktools_plugin_macros::{tiktools_export_native_plugin, tiktools_process_plugin};

/// Errors exposed to plugin authors. Protocol adapters deliberately serialize
/// only this display text, never a Rust backtrace.
#[derive(Debug, Error)]
pub enum PluginError {
    #[error("invalid request: {0}")]
    InvalidRequest(String),

    #[error("unsupported action: {0}")]
    UnsupportedAction(String),

    #[error("capability unavailable: {0}")]
    CapabilityUnavailable(String),

    #[error("plugin error: {0}")]
    Other(String),
}

impl PluginError {
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::InvalidRequest(message.into())
    }

    pub fn unsupported(action: impl Into<String>) -> Self {
        Self::UnsupportedAction(action.into())
    }

    pub fn capability_unavailable(capability: impl Into<String>) -> Self {
        Self::CapabilityUnavailable(capability.into())
    }

    pub fn other(message: impl Into<String>) -> Self {
        Self::Other(message.into())
    }
}

pub type PluginResult<T> = Result<T, PluginError>;

/// A plugin's stable identity, independent of its runtime adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginIdentity {
    pub id: String,
    pub version: String,
}

impl PluginIdentity {
    pub fn new(id: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            version: version.into(),
        }
    }
}

/// Runtime-neutral context supplied to plugin business logic.
///
/// File paths and host handles are intentionally absent. Future WASM
/// adapters can expose narrowly scoped host capabilities without changing the
/// trait or handing plugins arbitrary host internals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginContext {
    pub identity: PluginIdentity,
    /// Capabilities declared by the plugin manifest. These are not grants.
    pub declared_capabilities: CapabilitySet,
    /// Permissions declared by the plugin manifest. User grants are a
    /// separate policy layer and are intentionally not represented here yet.
    pub declared_permissions: PermissionSet,
}

impl PluginContext {
    pub fn new(
        identity: PluginIdentity,
        declared_capabilities: CapabilitySet,
        declared_permissions: PermissionSet,
    ) -> Self {
        Self {
            identity,
            declared_capabilities,
            declared_permissions,
        }
    }

    /// Builds the process context from the existing launcher contract.
    /// WASM adapters can construct the same shape from their manifest and
    /// explicit host policy without relying on environment variables.
    pub fn from_process_environment() -> Self {
        Self::new(
            PluginIdentity::new(
                env::var("TIKTOOLS_PLUGIN_ID").unwrap_or_else(|_| "unknown".to_owned()),
                env::var("TIKTOOLS_PLUGIN_VERSION").unwrap_or_else(|_| "0.0.0".to_owned()),
            ),
            CapabilitySet::from_strings(environment_list("TIKTOOLS_PLUGIN_CAPABILITIES")),
            PermissionSet::from_strings(environment_list("TIKTOOLS_PLUGIN_PERMISSIONS")),
        )
    }

    /// Returns the limited context available through native ABI v1.
    ///
    /// ABI v1 does not pass manifest metadata into `create`, so native
    /// plugins must not mistake this context for a manifest-backed grant.
    /// A future ABI revision can add an explicit initialization payload.
    pub fn for_native_abi_v1() -> Self {
        Self::new(
            PluginIdentity::new("unknown", "0.0.0"),
            CapabilitySet::default(),
            PermissionSet::default(),
        )
    }
}

fn environment_list(name: &str) -> Vec<String> {
    env::var(name)
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

/// Process-runtime helpers for paths supplied by the TikTools launcher.
///
/// These are intentionally outside `PluginContext`: WASM and native ABI
/// plugins do not necessarily have meaningful process paths.
pub mod process {
    use std::{env, ffi::OsString, path::PathBuf};

    use super::{PluginError, PluginResult};

    const DATA_DIR: &str = "TIKTOOLS_PLUGIN_DATA_DIR";
    const STORAGE_FILE: &str = "TIKTOOLS_PLUGIN_STORAGE_FILE";

    pub fn data_dir() -> PluginResult<PathBuf> {
        path_from_environment(DATA_DIR)
    }

    pub fn storage_file() -> PluginResult<PathBuf> {
        path_from_environment(STORAGE_FILE)
    }

    fn path_from_environment(name: &str) -> PluginResult<PathBuf> {
        path_from_os(name, env::var_os(name))
    }

    pub(crate) fn path_from_os(name: &str, value: Option<OsString>) -> PluginResult<PathBuf> {
        let value = value.ok_or_else(|| {
            PluginError::other(format!(
                "required process environment variable {name} is missing"
            ))
        })?;
        let path = PathBuf::from(value);
        if path.as_os_str().is_empty() {
            return Err(PluginError::other(format!(
                "required process environment variable {name} is empty"
            )));
        }
        Ok(path)
    }
}

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
/// is additive and the host only sends it to plugins that declare
/// `processorTypes`, so existing plugins never observe the new variant.
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
}

impl PluginCall {
    pub fn action(action: Value, event: Value) -> Self {
        Self::Action { action, event }
    }

    pub fn enrich(request: EventEnrichmentRequest) -> Self {
        Self::Enrich { request }
    }

    pub fn into_action(self) -> Option<ActionCall> {
        match self {
            Self::Action { action, event } => Some(ActionCall { action, event }),
            Self::Poll | Self::Enrich { .. } => None,
        }
    }

    pub fn into_enrich(self) -> Option<EventEnrichmentRequest> {
        match self {
            Self::Enrich { request } => Some(request),
            Self::Action { .. } | Self::Poll => None,
        }
    }
}

/// A pre-filter enrichment call. The host sends the canonical automation
/// event; the plugin must never mutate it in place, only return derived
/// annotations the host merges under the reserved `intel` namespace.
/// `settings` carries the plugin's host-rendered settings object so
/// processors stay configurable on every runtime without file access.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EventEnrichmentRequest {
    pub processor_id: String,
    pub event: Value,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub settings: Value,
}

impl EventEnrichmentRequest {
    pub fn new(processor_id: impl Into<String>, event: Value) -> Self {
        Self {
            processor_id: processor_id.into(),
            event,
            settings: Value::Null,
        }
    }

    pub fn settings(mut self, settings: Value) -> Self {
        self.settings = settings;
        self
    }
}

/// Constrained enrichment result. Unlike `PluginCallResult` this carries no
/// side-effect intents: processors classify or re-represent data only.
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EventEnrichmentResult {
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub annotations: Map<String, Value>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub views: BTreeMap<String, TextView>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub logs: Vec<String>,
}

/// One normalized/spoken text view: the derived text plus the evidence a
/// consumer needs to decide whether to trust it (language, confidence,
/// source, optional pronunciation).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TextView {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ipa: Option<String>,
}

impl TextView {
    pub fn new(text: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            language: None,
            confidence: None,
            source: source.into(),
            ipa: None,
        }
    }

    pub fn language(mut self, language: impl Into<String>, confidence: f64) -> Self {
        self.language = Some(language.into());
        self.confidence = Some(confidence);
        self
    }

    pub fn ipa(mut self, ipa: impl Into<String>) -> Self {
        self.ipa = Some(ipa.into());
        self
    }
}

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

/// Runtime-neutral plugin business-logic trait.
pub trait Plugin: Send + 'static {
    fn initialize(&mut self, _context: &PluginContext) -> PluginResult<()> {
        Ok(())
    }

    fn action(
        &mut self,
        _context: &PluginContext,
        _call: ActionCall,
    ) -> PluginResult<ActionResult> {
        Err(PluginError::unsupported("action"))
    }

    fn poll(&mut self, _context: &PluginContext) -> PluginResult<PollResult> {
        Ok(PollResult::default())
    }

    /// Enriches one host event with derived annotations. The default is a
    /// no-op result so existing plugins compile unchanged; the host only
    /// calls this for plugins that declare `processorTypes`.
    ///
    /// ```rust
    /// use tiktools_plugin_sdk::prelude::*;
    ///
    /// #[derive(Default)]
    /// struct TextProcessor;
    ///
    /// impl Plugin for TextProcessor {
    ///     fn enrich(
    ///         &mut self,
    ///         _context: &PluginContext,
    ///         request: EventEnrichmentRequest,
    ///     ) -> PluginResult<EventEnrichmentResult> {
    ///         let comment = request
    ///             .event
    ///             .pointer("/data/comment")
    ///             .and_then(|value| value.as_str())
    ///             .unwrap_or_default();
    ///         let mut result = EventEnrichmentResult::default();
    ///         result.annotations.insert(
    ///             "comment".to_owned(),
    ///             serde_json::json!({"normalized": comment.trim()}),
    ///         );
    ///         Ok(result)
    ///     }
    /// }
    /// ```
    fn enrich(
        &mut self,
        _context: &PluginContext,
        _request: EventEnrichmentRequest,
    ) -> PluginResult<EventEnrichmentResult> {
        Ok(EventEnrichmentResult::default())
    }

    fn shutdown(&mut self, _context: &PluginContext) -> PluginResult<()> {
        Ok(())
    }
}

fn serialize_call_result<T: Serialize>(result: T) -> PluginResult<Value> {
    serde_json::to_value(result)
        .map_err(|error| PluginError::other(format!("could not encode plugin result: {error}")))
}

/// Routes one typed call to plugin business logic and returns the serialized
/// typed result. Action and poll calls serialize to the same
/// `PluginCallResult` JSON as before; enrich calls serialize to
/// `EventEnrichmentResult` JSON, which carries no side-effect intents.
pub fn dispatch_plugin_call<P: Plugin>(
    plugin: &mut P,
    context: &PluginContext,
    call: PluginCall,
) -> PluginResult<Value> {
    match call {
        PluginCall::Action { action, event } => plugin
            .action(context, ActionCall { action, event })
            .map(PluginCallResult::from)
            .and_then(serialize_call_result),
        PluginCall::Poll => plugin
            .poll(context)
            .map(PluginCallResult::from)
            .and_then(serialize_call_result),
        PluginCall::Enrich { request } => plugin
            .enrich(context, request)
            .and_then(serialize_call_result),
    }
}

#[derive(Debug, Error)]
pub enum PluginProtocolError {
    #[error("plugin result must be a JSON object")]
    NotAnObject,
    #[error("plugin result field `{0}` is invalid")]
    InvalidField(&'static str),
    #[error("plugin result field `{field}` is invalid: {message}")]
    InvalidValue {
        field: &'static str,
        message: String,
    },
}

/// Converts both the current typed result shape and the legacy `emit` /
/// `playAudio` keys into one internal contract. This is the single legacy
/// compatibility boundary used by the host after a runtime call.
pub fn decode_plugin_result(value: Value) -> Result<PluginCallResult, PluginProtocolError> {
    let object = value.as_object().ok_or(PluginProtocolError::NotAnObject)?;
    let summary = decode_summary(object.get("summary"))?;
    let logs = decode_logs(object.get("logs"))?;
    let mut intents = decode_typed_intents(object.get("intents"))?;
    intents.extend(decode_legacy_emit_intents(object.get("emit"))?);
    intents.extend(decode_legacy_audio_intents(object.get("playAudio"))?);
    let events = decode_events(object.get("events"))?;
    Ok(PluginCallResult {
        summary,
        logs,
        intents,
        events,
    })
}

fn decode_summary(value: Option<&Value>) -> Result<Option<String>, PluginProtocolError> {
    let Some(value) = value else {
        return Ok(None);
    };
    value
        .as_str()
        .map(|value| Some(value.to_owned()))
        .ok_or(PluginProtocolError::InvalidField("summary"))
}

fn decode_logs(value: Option<&Value>) -> Result<Vec<String>, PluginProtocolError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    as_values(value)
        .into_iter()
        .map(|value| {
            value
                .as_str()
                .map(ToOwned::to_owned)
                .ok_or(PluginProtocolError::InvalidField("logs"))
        })
        .collect()
}

fn decode_typed_intents(value: Option<&Value>) -> Result<Vec<HostIntent>, PluginProtocolError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    as_values(value)
        .into_iter()
        .map(|value| {
            serde_json::from_value(value.clone()).map_err(|error| {
                PluginProtocolError::InvalidValue {
                    field: "intents",
                    message: error.to_string(),
                }
            })
        })
        .collect()
}

fn decode_legacy_emit_intents(
    value: Option<&Value>,
) -> Result<Vec<HostIntent>, PluginProtocolError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    as_values(value)
        .into_iter()
        .map(|value| {
            let object = value
                .as_object()
                .ok_or(PluginProtocolError::InvalidField("emit"))?;
            let event_type = object
                .get("type")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .ok_or(PluginProtocolError::InvalidField("emit"))?;
            Ok(HostIntent::Emit(EmitIntent::new(
                event_type,
                object.get("data").cloned().unwrap_or(Value::Null),
            )))
        })
        .collect()
}

fn decode_legacy_audio_intents(
    value: Option<&Value>,
) -> Result<Vec<HostIntent>, PluginProtocolError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    as_values(value)
        .into_iter()
        .map(|value| {
            let mut object = value
                .as_object()
                .cloned()
                .ok_or(PluginProtocolError::InvalidField("playAudio"))?;
            if object.get("fileRef").is_some_and(Value::is_string) {
                let file = object
                    .remove("fileRef")
                    .ok_or(PluginProtocolError::InvalidField("playAudio"))?;
                object.insert("fileRef".to_owned(), serde_json::json!({"path": file}));
            } else if object.get("fileRef").is_none() {
                for key in ["filePath", "file", "path"] {
                    if let Some(file) = object.remove(key) {
                        object.insert(
                            "fileRef".to_owned(),
                            if file.is_string() {
                                serde_json::json!({"path": file})
                            } else {
                                file
                            },
                        );
                        break;
                    }
                }
            }
            let intent = serde_json::from_value::<AudioPlayIntent>(Value::Object(object)).map_err(
                |error| PluginProtocolError::InvalidValue {
                    field: "playAudio",
                    message: error.to_string(),
                },
            )?;
            Ok(HostIntent::AudioPlay(intent))
        })
        .collect()
}

fn decode_events(value: Option<&Value>) -> Result<Vec<PluginEvent>, PluginProtocolError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    as_values(value)
        .into_iter()
        .map(|value| {
            serde_json::from_value(value.clone()).map_err(|error| {
                PluginProtocolError::InvalidValue {
                    field: "events",
                    message: error.to_string(),
                }
            })
        })
        .collect()
}

fn as_values(value: &Value) -> Vec<&Value> {
    match value {
        Value::Array(values) => values.iter().collect(),
        value => vec![value],
    }
}

/// Bounds for one enrichment result. Processors run per live event, so every
/// plugin-controlled collection and string accepted here is capped; the host
/// treats an oversized result as an invalid response and passes the raw
/// event through.
pub const MAX_ENRICHMENT_ANNOTATIONS: usize = 16;
pub const MAX_ENRICHMENT_ANNOTATION_BYTES: usize = 32 * 1024;
pub const MAX_ENRICHMENT_VIEWS: usize = 16;
pub const MAX_ENRICHMENT_VIEW_TEXT_CHARS: usize = 4_096;
pub const MAX_ENRICHMENT_LOGS: usize = 16;
pub const MAX_ENRICHMENT_LOG_CHARS: usize = 512;

/// Top-level keys that would smuggle side effects through the enrichment
/// channel. They are rejected, never executed or merged.
const FORBIDDEN_ENRICHMENT_KEYS: [&str; 8] = [
    "emit",
    "events",
    "intents",
    "playAudio",
    "audioPlay",
    "points",
    "http",
    "storage",
];

/// Decodes one `enrich` response into the constrained enrichment contract.
/// Unlike `decode_plugin_result` this accepts no legacy intent keys: any
/// side-effect field is a typed error so the host can fail open with a
/// precise diagnostic instead of executing it.
pub fn decode_enrichment_result(
    value: Value,
) -> Result<EventEnrichmentResult, PluginProtocolError> {
    let object = value.as_object().ok_or(PluginProtocolError::NotAnObject)?;
    for key in FORBIDDEN_ENRICHMENT_KEYS {
        if object.contains_key(key) {
            return Err(PluginProtocolError::InvalidValue {
                field: "enrich",
                message: format!("enrichment results must not contain `{key}`"),
            });
        }
    }
    let annotations = decode_enrichment_annotations(object.get("annotations"))?;
    let views = decode_enrichment_views(object.get("views"))?;
    let logs = decode_enrichment_logs(object.get("logs"))?;
    Ok(EventEnrichmentResult {
        annotations,
        views,
        logs,
    })
}

fn decode_enrichment_annotations(
    value: Option<&Value>,
) -> Result<Map<String, Value>, PluginProtocolError> {
    let Some(value) = value else {
        return Ok(Map::new());
    };
    let object = value
        .as_object()
        .ok_or(PluginProtocolError::InvalidField("annotations"))?;
    if object.len() > MAX_ENRICHMENT_ANNOTATIONS {
        return Err(PluginProtocolError::InvalidValue {
            field: "annotations",
            message: format!("at most {MAX_ENRICHMENT_ANNOTATIONS} annotations are allowed"),
        });
    }
    for key in object.keys() {
        if key.is_empty() || key.len() > 64 {
            return Err(PluginProtocolError::InvalidValue {
                field: "annotations",
                message: "annotation names must be 1..=64 characters".to_owned(),
            });
        }
    }
    if serde_json::to_vec(&object)
        .map(|bytes| bytes.len() > MAX_ENRICHMENT_ANNOTATION_BYTES)
        .unwrap_or(true)
    {
        return Err(PluginProtocolError::InvalidValue {
            field: "annotations",
            message: format!("annotations are larger than {MAX_ENRICHMENT_ANNOTATION_BYTES} bytes"),
        });
    }
    Ok(object.clone())
}

fn decode_enrichment_views(
    value: Option<&Value>,
) -> Result<BTreeMap<String, TextView>, PluginProtocolError> {
    let Some(value) = value else {
        return Ok(BTreeMap::new());
    };
    let object = value
        .as_object()
        .ok_or(PluginProtocolError::InvalidField("views"))?;
    if object.len() > MAX_ENRICHMENT_VIEWS {
        return Err(PluginProtocolError::InvalidValue {
            field: "views",
            message: format!("at most {MAX_ENRICHMENT_VIEWS} views are allowed"),
        });
    }
    let mut views = BTreeMap::new();
    for (name, view) in object {
        if name.is_empty() || name.len() > 64 {
            return Err(PluginProtocolError::InvalidValue {
                field: "views",
                message: "view names must be 1..=64 characters".to_owned(),
            });
        }
        let view: TextView = serde_json::from_value(view.clone()).map_err(|error| {
            PluginProtocolError::InvalidValue {
                field: "views",
                message: error.to_string(),
            }
        })?;
        if view.text.chars().count() > MAX_ENRICHMENT_VIEW_TEXT_CHARS {
            return Err(PluginProtocolError::InvalidValue {
                field: "views",
                message: format!(
                    "view text is longer than {MAX_ENRICHMENT_VIEW_TEXT_CHARS} characters"
                ),
            });
        }
        if view.source.is_empty() || view.source.len() > 64 {
            return Err(PluginProtocolError::InvalidValue {
                field: "views",
                message: "view source must be 1..=64 characters".to_owned(),
            });
        }
        if view
            .language
            .as_ref()
            .is_some_and(|language| language.is_empty() || language.len() > 32)
        {
            return Err(PluginProtocolError::InvalidValue {
                field: "views",
                message: "view language must be 1..=32 characters".to_owned(),
            });
        }
        if view
            .confidence
            .is_some_and(|confidence| !confidence.is_finite() || !(0.0..=1.0).contains(&confidence))
        {
            return Err(PluginProtocolError::InvalidValue {
                field: "views",
                message: "view confidence must be within 0.0..=1.0".to_owned(),
            });
        }
        if view.ipa.as_ref().is_some_and(|ipa| ipa.len() > 256) {
            return Err(PluginProtocolError::InvalidValue {
                field: "views",
                message: "view ipa is longer than 256 characters".to_owned(),
            });
        }
        views.insert(name.clone(), view);
    }
    Ok(views)
}

fn decode_enrichment_logs(value: Option<&Value>) -> Result<Vec<String>, PluginProtocolError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    as_values(value)
        .into_iter()
        .take(MAX_ENRICHMENT_LOGS)
        .map(|value| {
            value
                .as_str()
                .map(|line| line.chars().take(MAX_ENRICHMENT_LOG_CHARS).collect())
                .ok_or(PluginProtocolError::InvalidField("logs"))
        })
        .collect()
}

fn response_ok(id: String, result: Value) -> PluginResponse {
    PluginResponse {
        protocol_version: TIKTOOLS_PLUGIN_PROTOCOL_VERSION,
        id,
        ok: true,
        result: Some(result),
        error: None,
    }
}

fn response_failure(id: String, error: impl Into<String>) -> PluginResponse {
    PluginResponse {
        protocol_version: TIKTOOLS_PLUGIN_PROTOCOL_VERSION,
        id,
        ok: false,
        result: None,
        error: Some(error.into()),
    }
}

/// Runs a plugin using the existing framed stdin/stdout process protocol.
pub fn run_process_plugin<P>() -> PluginResult<()>
where
    P: Plugin + Default,
{
    let context = PluginContext::from_process_environment();
    let mut plugin = P::default();
    plugin.initialize(&context)?;
    run_process_plugin_with(&mut plugin, &context)
}

pub fn run_process_plugin_with<P>(plugin: &mut P, context: &PluginContext) -> PluginResult<()>
where
    P: Plugin,
{
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = io::BufReader::new(stdin.lock());
    let mut writer = io::BufWriter::new(stdout.lock());

    let loop_result = loop {
        let request = match read_frame::<_, PluginRequest>(&mut reader) {
            Ok(request) => request,
            Err(FrameError::Io(error)) if error.kind() == io::ErrorKind::UnexpectedEof => {
                break Ok(())
            }
            Err(error) => break Err(PluginError::other(error.to_string())),
        };
        let response = handle_process_request(plugin, context, request);
        write_frame(&mut writer, &response)
            .map_err(|error| PluginError::other(error.to_string()))?;
    };
    let shutdown = plugin.shutdown(context);
    loop_result.and(shutdown)
}

fn handle_process_request<P: Plugin>(
    plugin: &mut P,
    context: &PluginContext,
    request: PluginRequest,
) -> PluginResponse {
    if request.protocol_version != TIKTOOLS_PLUGIN_PROTOCOL_VERSION {
        return response_failure(
            request.id,
            format!("unsupported protocol version {}", request.protocol_version),
        );
    }
    if request.method != METHOD_CALL {
        return response_failure(
            request.id,
            format!("unsupported method `{}`", request.method),
        );
    }
    let call = match serde_json::from_value::<PluginCall>(request.payload) {
        Ok(call) => call,
        Err(error) => return response_failure(request.id, format!("invalid plugin call: {error}")),
    };
    match dispatch_plugin_call(plugin, context, call) {
        Ok(result) => response_ok(request.id, result),
        Err(error) => response_failure(request.id, error.to_string()),
    }
}

pub mod native {
    //! Native ABI adapter used by `tiktools_export_native_plugin!`.

    use super::*;
    use std::{
        ffi::c_void,
        mem::ManuallyDrop,
        panic::{catch_unwind, AssertUnwindSafe},
        slice,
    };

    struct NativePluginState<P: Plugin> {
        plugin: P,
        context: PluginContext,
        initialization_error: Option<String>,
    }

    pub fn create<P>() -> *mut c_void
    where
        P: Plugin + Default,
    {
        let context = PluginContext::for_native_abi_v1();
        let mut plugin = P::default();
        let initialization_error = plugin
            .initialize(&context)
            .err()
            .map(|error| error.to_string());
        Box::into_raw(Box::new(NativePluginState {
            plugin,
            context,
            initialization_error,
        })) as *mut c_void
    }

    /// # Safety
    ///
    /// `context` must be a pointer returned by `create::<P>` that has not
    /// already been destroyed.
    pub unsafe fn destroy<P>(context: *mut c_void)
    where
        P: Plugin,
    {
        if context.is_null() {
            return;
        }
        // SAFETY: the pointer was allocated by `create::<P>` and is consumed
        // exactly once by the native ABI destroy function.
        let mut state = unsafe { Box::from_raw(context.cast::<NativePluginState<P>>()) };
        let _ = state.plugin.shutdown(&state.context);
    }

    /// # Safety
    ///
    /// `context` must come from `create::<P>`, `request_ptr` must refer to a
    /// readable request buffer for `request_len` bytes, and `response` must
    /// point to writable storage owned by the ABI caller.
    pub unsafe fn handle_message<P>(
        context: *mut c_void,
        request_ptr: *const u8,
        request_len: usize,
        response: *mut PluginBuffer,
    ) -> PluginStatus
    where
        P: Plugin,
    {
        let result =
            catch_unwind(AssertUnwindSafe(|| {
                if context.is_null() || response.is_null() {
                    return Err(PluginStatus::InvalidRequest);
                }
                if request_ptr.is_null() || request_len > MAX_FRAME_BYTES {
                    return Err(PluginStatus::InvalidRequest);
                }
                // SAFETY: the host supplies the request pointer and length for the
                // duration of this call; validation above bounds the slice.
                let request = unsafe { slice::from_raw_parts(request_ptr, request_len) };
                // SAFETY: the pointer was allocated by `create::<P>`.
                let state = unsafe { &mut *context.cast::<NativePluginState<P>>() };
                if state.initialization_error.is_some() {
                    return Err(PluginStatus::InternalError);
                }
                // Native ABI v1 receives the raw typed call. The process
                // adapter has an outer PluginRequest envelope, but adding
                // that envelope here would break existing native plugins.
                let call = serde_json::from_slice::<PluginCall>(request)
                    .map_err(|_| PluginStatus::InvalidRequest)?;
                let result = dispatch_plugin_call(&mut state.plugin, &state.context, call)
                    .map_err(|error| match error {
                        PluginError::InvalidRequest(_) | PluginError::UnsupportedAction(_) => {
                            PluginStatus::InvalidRequest
                        }
                        PluginError::CapabilityUnavailable(_) | PluginError::Other(_) => {
                            PluginStatus::InternalError
                        }
                    })?;
                let bytes = serde_json::to_vec(&result).map_err(|_| PluginStatus::InternalError)?;
                write_buffer(response, bytes);
                Ok(())
            }));
        match result {
            Ok(Ok(())) => PluginStatus::Ok,
            Ok(Err(status)) => status,
            Err(_) => PluginStatus::InternalError,
        }
    }

    /// # Safety
    ///
    /// `response` must point to a buffer previously initialized by this
    /// adapter, or to writable `PluginBuffer` storage.
    pub unsafe fn free_buffer(response: *mut PluginBuffer) {
        if response.is_null() {
            return;
        }
        // SAFETY: the host passes the same buffer that this adapter allocated.
        let buffer = unsafe { &mut *response };
        if !buffer.ptr.is_null() && buffer.capacity >= buffer.len {
            // SAFETY: pointer, length, and capacity came from `write_buffer`.
            unsafe { drop(Vec::from_raw_parts(buffer.ptr, buffer.len, buffer.capacity)) };
        }
        *buffer = PluginBuffer::empty();
    }

    fn write_buffer(response: *mut PluginBuffer, bytes: Vec<u8>) {
        // SAFETY: the caller validated that `response` is non-null.
        let response = unsafe { &mut *response };
        if bytes.is_empty() {
            *response = PluginBuffer::empty();
            return;
        }
        let mut bytes = ManuallyDrop::new(bytes);
        *response = PluginBuffer {
            ptr: bytes.as_mut_ptr(),
            len: bytes.len(),
            capacity: bytes.capacity(),
        };
    }
}

pub mod prelude {
    pub use crate::{
        tiktools_export_native_plugin, tiktools_process_plugin, ActionCall, ActionResult,
        AudioPlayIntent, EmitIntent, EventEnrichmentRequest, EventEnrichmentResult, HostIntent,
        Plugin, PluginCall, PluginCallResult, PluginContext, PluginError, PluginEvent,
        PluginIdentity, PluginResult, PollResult, TextView,
    };
    pub use tiktools_plugin_api::{AudioOverlap, MediaFileRef};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct TestPlugin;

    impl Plugin for TestPlugin {
        fn action(
            &mut self,
            _context: &PluginContext,
            _call: ActionCall,
        ) -> PluginResult<ActionResult> {
            Ok(ActionResult::summary("action handled"))
        }
    }

    fn test_context() -> PluginContext {
        PluginContext::new(
            PluginIdentity::new("test.plugin", "1.0.0"),
            CapabilitySet::default(),
            PermissionSet::default(),
        )
    }

    #[test]
    fn typed_plugin_call_preserves_legacy_wire_shape() {
        let call = PluginCall::action(
            serde_json::json!({"typeId": "demo.action", "config": {"value": 1}}),
            serde_json::json!({"type": "tiktok.chat"}),
        );
        let value = serde_json::to_value(call).unwrap();
        assert_eq!(value["type"], "action");
        assert_eq!(value["action"]["typeId"], "demo.action");
    }

    #[test]
    fn compatibility_decoder_maps_legacy_intents_once() {
        let result = decode_plugin_result(serde_json::json!({
            "summary": "done",
            "logs": ["one"],
            "emit": [{"type": "demo.event", "data": {"ok": true}}],
            "playAudio": {"fileRef": {"path": "/tmp/alert.wav"}, "volume": 0.5}
        }))
        .unwrap();
        assert_eq!(result.summary.as_deref(), Some("done"));
        assert_eq!(result.logs, vec!["one"]);
        assert_eq!(result.intents.len(), 2);
        assert!(matches!(result.intents[0], HostIntent::Emit(_)));
        assert!(matches!(result.intents[1], HostIntent::AudioPlay(_)));
    }

    #[test]
    fn compatibility_decoder_rejects_malformed_legacy_fields() {
        for (field, value) in [
            ("summary", serde_json::json!(42)),
            ("logs", serde_json::json!(["ok", 42])),
            ("emit", serde_json::json!([{"data": {}}])),
            ("playAudio", serde_json::json!([{"volume": "loud"}])),
            ("events", serde_json::json!([{"type": "missing-data"}])),
        ] {
            let error = decode_plugin_result(serde_json::json!({field: value})).unwrap_err();
            assert!(error.to_string().contains(field), "{field}: {error}");
        }
    }

    #[test]
    fn audio_intent_serializes_with_typed_file_reference() {
        let result = ActionResult::default().intent(HostIntent::audio_play(
            AudioPlayIntent::from_path("/tmp/alert.wav"),
        ));
        let value = serde_json::to_value(PluginCallResult::from(result)).unwrap();
        assert_eq!(value["intents"][0]["type"], "audio-play");
        assert_eq!(
            value["intents"][0]["data"]["fileRef"]["path"],
            "/tmp/alert.wav"
        );
    }

    #[test]
    fn process_path_helpers_reject_missing_and_empty_values() {
        assert!(process::path_from_os("TIKTOOLS_PLUGIN_DATA_DIR", None).is_err());
        assert!(process::path_from_os(
            "TIKTOOLS_PLUGIN_STORAGE_FILE",
            Some(std::ffi::OsString::new())
        )
        .is_err());
    }

    #[test]
    fn process_request_dispatches_raw_typed_call_inside_the_wire_envelope() {
        let call = PluginCall::action(
            serde_json::json!({"typeId": "demo.action"}),
            serde_json::json!({"type": "demo.event"}),
        );
        let request = PluginRequest::new(
            METHOD_CALL,
            METHOD_CALL,
            serde_json::to_value(call).unwrap(),
        );
        let response = handle_process_request(&mut TestPlugin, &test_context(), request);
        assert!(response.ok);
        assert_eq!(response.id, METHOD_CALL);
        let result: PluginCallResult = serde_json::from_value(response.result.unwrap()).unwrap();
        assert_eq!(result.summary.as_deref(), Some("action handled"));
    }

    #[test]
    fn process_request_rejects_bad_protocol_method_and_call() {
        let context = test_context();
        let mut bad_version = PluginRequest::new(
            "version",
            METHOD_CALL,
            serde_json::json!({
                "type": "poll"
            }),
        );
        bad_version.protocol_version += 1;
        assert!(!handle_process_request(&mut TestPlugin, &context, bad_version).ok);

        let bad_method = PluginRequest::new(
            "method",
            "other",
            serde_json::json!({
                "type": "poll"
            }),
        );
        assert!(!handle_process_request(&mut TestPlugin, &context, bad_method).ok);

        let bad_call = PluginRequest::new(
            "call",
            METHOD_CALL,
            serde_json::json!({
                "type": "unknown"
            }),
        );
        assert!(!handle_process_request(&mut TestPlugin, &context, bad_call).ok);
    }

    #[test]
    fn enrich_call_serializes_additively_without_changing_action_and_poll() {
        // Existing wire shapes are byte-identical to the pre-enrich protocol.
        let action = PluginCall::action(serde_json::json!({}), serde_json::json!({}));
        assert_eq!(
            serde_json::to_value(&action).unwrap(),
            serde_json::json!({"type": "action", "action": {}, "event": {}})
        );
        let poll = PluginCall::Poll;
        assert_eq!(
            serde_json::to_value(&poll).unwrap(),
            serde_json::json!({"type": "poll"})
        );
        // The new variant round-trips through the same envelope.
        let enrich = PluginCall::enrich(EventEnrichmentRequest::new(
            "textintel.analyze",
            serde_json::json!({"type": "tiktok.chat"}),
        ));
        let value = serde_json::to_value(&enrich).unwrap();
        assert_eq!(value["type"], "enrich");
        assert_eq!(value["request"]["processorId"], "textintel.analyze");
        assert_eq!(serde_json::from_value::<PluginCall>(value).unwrap(), enrich);
        assert!(enrich.into_action().is_none());
        assert!(poll.into_enrich().is_none());
    }

    #[test]
    fn default_enrich_is_a_noop_and_dispatch_routes_it() {
        // Existing plugins that never implemented `enrich` keep working.
        let context = test_context();
        let request = EventEnrichmentRequest::new("demo.enrich", serde_json::json!({}));
        let result = TestPlugin.enrich(&context, request.clone()).unwrap();
        assert_eq!(result, EventEnrichmentResult::default());

        let value =
            dispatch_plugin_call(&mut TestPlugin, &context, PluginCall::enrich(request)).unwrap();
        assert_eq!(
            decode_enrichment_result(value).unwrap(),
            EventEnrichmentResult::default()
        );
    }

    #[derive(Default)]
    struct EnrichPlugin;

    impl Plugin for EnrichPlugin {
        fn enrich(
            &mut self,
            _context: &PluginContext,
            request: EventEnrichmentRequest,
        ) -> PluginResult<EventEnrichmentResult> {
            let comment = request
                .event
                .pointer("/data/comment")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let mut result = EventEnrichmentResult::default();
            result.annotations.insert(
                "comment".to_owned(),
                serde_json::json!({"normalized": comment.trim()}),
            );
            result.views.insert(
                "tts".to_owned(),
                TextView::new(comment.trim(), "normalized").language("es", 0.9),
            );
            Ok(result)
        }
    }

    #[test]
    fn enrich_dispatch_flows_through_the_process_envelope() {
        let call = PluginCall::enrich(EventEnrichmentRequest::new(
            "demo.enrich",
            serde_json::json!({"type": "tiktok.chat", "data": {"comment": "  Hola  "}}),
        ));
        let request =
            PluginRequest::new("enrich-1", METHOD_CALL, serde_json::to_value(call).unwrap());
        let response = handle_process_request(&mut EnrichPlugin, &test_context(), request);
        assert!(response.ok);
        let result = decode_enrichment_result(response.result.unwrap()).unwrap();
        assert_eq!(
            result.annotations["comment"]["normalized"],
            serde_json::json!("Hola")
        );
        assert_eq!(result.views["tts"].text, "Hola");
        assert_eq!(result.views["tts"].language.as_deref(), Some("es"));
    }

    #[test]
    fn enrichment_decoder_rejects_side_effects_and_oversized_results() {
        for key in [
            "emit",
            "events",
            "intents",
            "playAudio",
            "points",
            "http",
            "storage",
        ] {
            let error = decode_enrichment_result(serde_json::json!({key: {}})).unwrap_err();
            assert!(
                error.to_string().contains("must not contain"),
                "{key}: {error}"
            );
        }
        assert!(decode_enrichment_result(serde_json::json!([])).is_err());
        assert!(decode_enrichment_result(serde_json::json!({"annotations": []})).is_err());
        assert!(decode_enrichment_result(serde_json::json!({"views": []})).is_err());
        assert!(decode_enrichment_result(serde_json::json!({"logs": [42]})).is_err());
        // Oversized annotation payloads are rejected, never truncated.
        let big = "x".repeat(MAX_ENRICHMENT_ANNOTATION_BYTES + 1);
        assert!(decode_enrichment_result(serde_json::json!({
            "annotations": {"comment": {"normalized": big}}
        }))
        .is_err());
        // View bounds are enforced.
        assert!(decode_enrichment_result(serde_json::json!({
            "views": {"tts": {"text": "hi"}}
        }))
        .is_err());
        assert!(decode_enrichment_result(serde_json::json!({
            "views": {"tts": {"text": "hi", "source": "raw", "confidence": 2.0}}
        }))
        .is_err());
        let long_text = "x".repeat(MAX_ENRICHMENT_VIEW_TEXT_CHARS + 1);
        assert!(decode_enrichment_result(serde_json::json!({
            "views": {"tts": {"text": long_text, "source": "raw"}}
        }))
        .is_err());
        // Unknown non-side-effect keys stay forward-compatible.
        assert!(decode_enrichment_result(serde_json::json!({
            "annotations": {},
            "futureField": {"nested": true}
        }))
        .is_ok());
    }
}
