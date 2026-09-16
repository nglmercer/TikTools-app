use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const DEFAULT_PLUGIN_ACTION_TIMEOUT_MS: u64 = 30_000;
pub const MAX_PLUGIN_ACTION_TIMEOUT_MS: u64 = 180_000;

/// Default deadline for one pre-filter processor call. Processors run on the
/// live-message hot path and fail open, so the default stays far below the
/// action timeout. Tune after benchmarking local processor latency.
pub const DEFAULT_PLUGIN_PROCESSOR_TIMEOUT_MS: u64 = 250;
/// Upper bound for a manifest-declared processor `timeoutMs`. Live enrichment
/// must never wait seconds for one chat event.
pub const MAX_PLUGIN_PROCESSOR_TIMEOUT_MS: u64 = 5_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginRuntimeKind {
    Native,
    Wasm,
    Process,
}

/// Runtime boundary semantics used for host policy and documentation. This
/// is intentionally separate from the serialized `trust` field so schema v2
/// values remain compatible while process isolation is not mislabeled as a
/// sandbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PluginSecurityModel {
    Trusted,
    Isolated,
    Sandboxed,
}

impl PluginRuntimeKind {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "native" => Some(Self::Native),
            "wasm" => Some(Self::Wasm),
            "process" => Some(Self::Process),
            _ => None,
        }
    }

    pub const fn security_model(self) -> PluginSecurityModel {
        match self {
            Self::Native => PluginSecurityModel::Trusted,
            Self::Process => PluginSecurityModel::Isolated,
            Self::Wasm => PluginSecurityModel::Sandboxed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PluginTrust {
    /// Trusted in-process native code, or a process executable whose OS
    /// permissions remain outside TikTools' protocol policy.
    #[default]
    Trusted,
    /// A runtime that supplies an actual execution sandbox, currently the
    /// intended label for WASM. WASI grants still depend on host policy.
    Sandboxed,
    /// Declarative package metadata; this value is not an OS sandbox by
    /// itself and is preserved for manifest compatibility.
    Untrusted,
}

impl PluginTrust {
    /// Returns the schema-v2 compatibility default for a runtime when a
    /// manifest omits `trust`. The separate `security_model()` API describes
    /// the actual runtime boundary; this value preserves the old manifest
    /// interpretation until a deliberate schema revision changes it.
    pub const fn default_for_runtime(runtime: PluginRuntimeKind) -> Self {
        match runtime {
            PluginRuntimeKind::Native => Self::Trusted,
            PluginRuntimeKind::Wasm | PluginRuntimeKind::Process => Self::Sandboxed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PluginManifest {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub runtime: PluginRuntimeKind,
    pub entry: String,
    pub trust: PluginTrust,
    pub capabilities: Vec<String>,
    pub permissions: Vec<String>,
    pub protocol_version: u32,
    pub abi_version: Option<u32>,
    pub targets: Vec<String>,
    /// JSON action descriptors exposed by the plugin, if any.
    pub action_types: Vec<Value>,
    /// JSON event-type descriptors a plugin can publish (hotkeys, timers).
    /// Entries are validated when the host merges its catalog; the raw list
    /// is kept here so discovery never fails on a single bad entry.
    pub event_types: Vec<Value>,
    /// JSON processor descriptors a plugin offers for pre-filter event
    /// enrichment. Like `event_types`, entries are validated when the host
    /// builds its processor catalog; the raw list is kept here so discovery
    /// never fails on a single bad entry.
    pub processor_types: Vec<Value>,
    /// Host-rendered settings schema, kept as data and never executed.
    pub settings_schema: Option<Value>,
    pub settings_ui_hints: Option<Value>,
}
impl fmt::Display for PluginRuntimeKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Native => "native",
            Self::Wasm => "wasm",
            Self::Process => "process",
        })
    }
}
