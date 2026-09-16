//! Processor pipeline value types: errors, keys, and call outcomes.

use tiktools_plugin_api::manifest::PluginProcessorDescriptor;

/// One processor selected for an event, with its manifest descriptor.
#[derive(Debug, Clone)]
pub(crate) struct EligibleProcessor {
    pub(crate) plugin_id: String,
    pub(crate) processor_id: String,
    pub(crate) descriptor: PluginProcessorDescriptor,
}

/// Identity of one processor within its plugin. Health and metrics are keyed
/// by this instead of the plugin id so a failing processor never trips the
/// circuit for healthy siblings sharing its process.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ProcessorKey {
    pub(crate) plugin_id: String,
    pub(crate) processor_id: String,
}

impl ProcessorKey {
    pub(crate) fn new(plugin_id: &str, processor_id: &str) -> Self {
        Self {
            plugin_id: plugin_id.to_owned(),
            processor_id: processor_id.to_owned(),
        }
    }

    pub(crate) fn of(processor: &EligibleProcessor) -> Self {
        Self::new(&processor.plugin_id, &processor.processor_id)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TimedEnrichment {
    pub(crate) result: tiktools_plugin_sdk::EventEnrichmentResult,
    pub(crate) duration_ms: u64,
}

/// Outcome of one processor call, tagged with its deterministic order index.
pub(crate) type ProcessorOutcome = (
    usize,
    String,
    String,
    Result<TimedEnrichment, ProcessorError>,
);

/// Typed failure category for one processor call. Every variant fails open:
/// the host event continues without that processor's annotations.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ProcessorError {
    Timeout,
    Unavailable(String),
    InvalidResponse(String),
    CapabilityDenied(String),
    PluginError(String),
    InputTooLarge,
    CircuitOpen,
    Overloaded,
}

impl std::fmt::Display for ProcessorError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout => formatter.write_str("processor timed out"),
            Self::Unavailable(reason) => write!(formatter, "processor unavailable: {reason}"),
            Self::InvalidResponse(reason) => {
                write!(formatter, "invalid processor response: {reason}")
            }
            Self::CapabilityDenied(reason) => {
                write!(formatter, "processor capability denied: {reason}")
            }
            Self::PluginError(reason) => write!(formatter, "processor error: {reason}"),
            Self::InputTooLarge => formatter.write_str("event is too large to enrich"),
            Self::CircuitOpen => formatter.write_str("processor circuit is open"),
            Self::Overloaded => formatter.write_str("processor queue is full"),
        }
    }
}

pub(crate) struct ProcessorTestOutcome {
    pub(crate) ok: bool,
    pub(crate) duration_ms: u64,
    pub(crate) result: serde_json::Value,
    pub(crate) error: Option<String>,
}
