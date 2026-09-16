use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// A pre-filter enrichment call. The host sends the canonical automation
/// event; the plugin must never mutate it in place, only return derived
/// annotations the host merges under the reserved `intel` namespace.
/// `settings` carries the plugin's host-rendered settings object so
/// processors stay configurable on every runtime without file access.
/// `inputs` carries the descriptor-declared event paths resolved by the host
/// (`role -> value`); `None` means an older host that never resolved them,
/// in which case the processor falls back to reading event pointers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EventEnrichmentRequest {
    pub processor_id: String,
    pub event: Value,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub settings: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inputs: Option<BTreeMap<String, Value>>,
}

impl EventEnrichmentRequest {
    pub fn new(processor_id: impl Into<String>, event: Value) -> Self {
        Self {
            processor_id: processor_id.into(),
            event,
            settings: Value::Null,
            inputs: None,
        }
    }

    pub fn settings(mut self, settings: Value) -> Self {
        self.settings = settings;
        self
    }

    pub fn inputs(mut self, inputs: BTreeMap<String, Value>) -> Self {
        self.inputs = Some(inputs);
        self
    }

    /// Resolves one input role: the host-resolved value when present,
    /// otherwise the legacy event pointer.
    pub fn input(&self, role: &str, legacy_pointer: &str) -> Option<&Value> {
        if let Some(inputs) = &self.inputs {
            return inputs.get(role);
        }
        self.event.pointer(legacy_pointer)
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
/// consumer needs to decide whether to trust it. `language`/`confidence`
/// describe text selection; pronunciation evidence lives separately under
/// `pronunciation` so the two confidences never mix. Views are the canonical
/// processor contribution for spoken text: the host projects the selected
/// view into `event.intel.*.tts`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TextView {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    pub source: String,
    #[serde(default = "default_view_speak", skip_serializing_if = "is_view_speak")]
    pub speak: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pronunciation: Option<TextPronunciation>,
}

/// Pronunciation evidence for one text view, independent of text selection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct TextPronunciation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ipa: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dialect: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
}

fn default_view_speak() -> bool {
    true
}

fn is_view_speak(speak: &bool) -> bool {
    *speak
}

impl TextView {
    pub fn new(text: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            language: None,
            confidence: None,
            source: source.into(),
            speak: true,
            reason: None,
            pronunciation: None,
        }
    }

    pub fn language(mut self, language: impl Into<String>, confidence: f64) -> Self {
        self.language = Some(language.into());
        self.confidence = Some(confidence);
        self
    }

    pub fn pronunciation(mut self, pronunciation: TextPronunciation) -> Self {
        self.pronunciation = Some(pronunciation);
        self
    }

    pub fn no_speak(mut self, reason: impl Into<String>) -> Self {
        self.speak = false;
        self.reason = Some(reason.into());
        self
    }
}
