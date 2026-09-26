//! Developer-facing SDK for TikTools plugins.
//!
//! The SDK is intentionally runtime-neutral. It provides typed calls/results,
//! a small plugin trait, and adapters for the existing framed process and
//! native ABI boundaries. It does not depend on Tokio, the desktop crate,
//! Wry, Winit, or a particular WASM engine.

mod calls;
mod context;
mod decode;
mod enrich;
mod error;
pub mod native;
#[cfg(feature = "providers")]
pub mod native_providers;
pub mod native_stage;
mod plugin;
pub mod process;
mod results;
#[cfg(test)]
mod tests;

pub use calls::{ActionCall, PluginCall};
pub use context::{PluginContext, PluginIdentity};
pub use decode::{decode_enrichment_result, decode_plugin_result};
pub use decode::{
    MAX_ENRICHMENT_ANNOTATIONS, MAX_ENRICHMENT_ANNOTATION_BYTES, MAX_ENRICHMENT_LOGS,
    MAX_ENRICHMENT_LOG_CHARS, MAX_ENRICHMENT_VIEWS, MAX_ENRICHMENT_VIEW_TEXT_CHARS,
};
pub use enrich::{EventEnrichmentRequest, EventEnrichmentResult, TextPronunciation, TextView};
pub use error::{PluginError, PluginProtocolError, PluginResult};
pub use plugin::{dispatch_plugin_call, Plugin};
pub use process::{data_dir, storage_file};
pub use process::{run_process_plugin, run_process_plugin_with};
pub use results::{
    ActionResult, AudioPlayIntent, EmitIntent, HostIntent, PluginCallResult, PluginEvent,
    PollResult, POLL_MAX_EVENTS_PER_RESPONSE,
};
pub use tiktools_plugin_api;
pub use tiktools_plugin_api::DomainEventEnvelope;
pub use tiktools_plugin_macros::{tiktools_export_native_plugin, tiktools_process_plugin};

pub mod prelude {
    pub use crate::{
        tiktools_export_native_plugin, tiktools_process_plugin, ActionCall, ActionResult,
        AudioPlayIntent, EmitIntent, EventEnrichmentRequest, EventEnrichmentResult, HostIntent,
        Plugin, PluginCall, PluginCallResult, PluginContext, PluginError, PluginEvent,
        PluginIdentity, PluginResult, PollResult, TextPronunciation, TextView,
    };
    pub use tiktools_plugin_api::{
        intel::{
            EventIntel, IntelComment, IntelComposition, IntelHandle, IntelLanguage,
            IntelLanguageCandidate, IntelNickname, IntelObfuscation, IntelPronunciation,
            IntelRebus, IntelSpam, IntelTts, IntelUnicode, IntelUser,
        },
        AudioOverlap, DomainEventEnvelope, MediaFileRef,
    };
}
