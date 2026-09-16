//! Contribution index: the precomputed set of processors eligible to run.
//!
//! Scanning every manifest and hitting the database per event does not scale
//! with live chat rates. Instead the host rebuilds this index only when
//! plugin availability or activation changes (install, uninstall, enable,
//! disable), and the enrich hot path filters the cached list by event type.

use super::EligibleProcessor;
use crate::services::CapabilityBroker;
use tiktools_plugin_api::{
    capabilities::EVENTS_ENRICH,
    manifest::{validate_processor_type, PluginManifest, PluginProcessorDescriptor},
};
use tiktools_plugin_loader::PluginManager;

/// Validated processor descriptors from one plugin manifest, in manifest
/// order. Invalid entries are skipped so one bad descriptor never breaks
/// discovery, mirroring `declared_event_types`.
pub(crate) fn declared_processors(manifest: &PluginManifest) -> Vec<PluginProcessorDescriptor> {
    manifest
        .processor_types
        .iter()
        .filter_map(|entry| {
            if validate_processor_type(entry).is_err() {
                tracing::debug!(
                    plugin = %manifest.id,
                    "skipping invalid processorTypes entry"
                );
                return None;
            }
            serde_json::from_value::<PluginProcessorDescriptor>(entry.clone()).ok()
        })
        .collect()
}

/// Collects every processor whose plugin is ready and declares
/// `events.enrich`, sorted in selection order: higher priority first, then
/// deterministically by `(plugin id, processor id)`. The first contributor
/// owns each stable key, so this order is the merge order.
pub(crate) fn collect_indexed_processors(
    plugins: &PluginManager,
    capabilities: &CapabilityBroker,
    ready: impl Fn(&str) -> bool,
) -> Vec<EligibleProcessor> {
    let mut eligible = Vec::new();
    for plugin in plugins.list() {
        let plugin_id = plugin.manifest.id.clone();
        if !ready(&plugin_id) {
            continue;
        }
        if capabilities
            .require_capability(&plugin.manifest, EVENTS_ENRICH)
            .is_err()
        {
            if !plugin.manifest.processor_types.is_empty() {
                tracing::debug!(
                    plugin = %plugin_id,
                    "skipping processors: events.enrich capability not declared"
                );
            }
            continue;
        }
        for descriptor in declared_processors(&plugin.manifest) {
            eligible.push(EligibleProcessor {
                plugin_id: plugin_id.clone(),
                processor_id: descriptor.id.clone(),
                descriptor,
            });
        }
    }
    sort_selection_order(&mut eligible);
    eligible
}

/// One-shot eligibility check without a cached index, for tests and
/// diagnostics. Production enrichment filters the shared [`ContributionIndex`]
/// instead so manifest scans stay off the hot path.
#[cfg(test)]
pub(crate) fn collect_eligible_processors(
    plugins: &PluginManager,
    capabilities: &CapabilityBroker,
    ready: impl Fn(&str) -> bool,
    event_type: &str,
) -> Vec<EligibleProcessor> {
    ContributionIndex::build(plugins, capabilities, ready).eligible_for(event_type)
}

fn sort_selection_order(eligible: &mut [EligibleProcessor]) {
    eligible.sort_by(|left, right| {
        right
            .descriptor
            .priority_value()
            .cmp(&left.descriptor.priority_value())
            .then_with(|| {
                (&left.plugin_id, &left.processor_id).cmp(&(&right.plugin_id, &right.processor_id))
            })
    });
}

fn matches_event(descriptor: &PluginProcessorDescriptor, event_type: &str) -> bool {
    descriptor.event_types.is_empty()
        || descriptor
            .event_types
            .iter()
            .any(|subscribed| subscribed == event_type)
}

/// Cached processor catalog in selection order. Rebuilt on lifecycle changes;
/// [`eligible_for`](Self::eligible_for) is the only hot-path query.
#[derive(Debug, Clone, Default)]
pub(crate) struct ContributionIndex {
    processors: Vec<EligibleProcessor>,
}

impl ContributionIndex {
    pub(crate) fn build(
        plugins: &PluginManager,
        capabilities: &CapabilityBroker,
        ready: impl Fn(&str) -> bool,
    ) -> Self {
        Self {
            processors: collect_indexed_processors(plugins, capabilities, ready),
        }
    }

    /// Processors subscribed to `event_type`, in selection order. An empty
    /// `eventTypes` subscription matches every event type.
    pub(crate) fn eligible_for(&self, event_type: &str) -> Vec<EligibleProcessor> {
        self.processors
            .iter()
            .filter(|processor| matches_event(&processor.descriptor, event_type))
            .cloned()
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.processors.len()
    }

    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.processors.is_empty()
    }
}
