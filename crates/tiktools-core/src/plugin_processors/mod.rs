//! Pre-filter event processor orchestration.
//!
//! Processors enrich existing host events before automation filters run. The
//! pipeline is fail-open: any processor timeout, crash, invalid response, or
//! missing capability passes the original event through, optionally stamped
//! with `intel.processing.status = "degraded"`. Processors never mutate raw
//! event fields; derived data merges only under the reserved `intel`
//! namespace.
//!
//! The module is split along the pipeline's natural boundaries:
//!
//! - [`index`] — the [`ContributionIndex`], rebuilt on lifecycle changes so
//!   manifest scans and activation reads stay off the hot path.
//! - [`execution`] — bounded fan-out over the shared [`PluginInvoker`](crate::plugin_invoker::PluginInvoker).
//! - [`merge`] — stable projection of outcomes into `event.intel`.
//! - [`state`] — per-processor health, metrics, and settings caches.
//! - [`status`] — the host-owned catalog behind the processors panel.

mod execution;
mod index;
mod merge;
mod state;
mod status;
#[cfg(test)]
mod tests;
mod types;

use super::*;

#[cfg(test)]
pub(crate) use execution::resolve_processor_inputs;
pub(crate) use execution::{execute_processors, run_single_processor};
#[cfg(test)]
pub(crate) use index::collect_eligible_processors;
pub(crate) use index::{declared_processors, ContributionIndex};
pub(crate) use merge::merge_processor_outcomes;
#[cfg(test)]
pub(crate) use state::{processor_retry_allowed, record_processor_success};
pub(crate) use state::{ProcessorMetrics, ProcessorSettingsStore};
pub(crate) use status::processor_status_entries;
pub(crate) use types::{
    EligibleProcessor, ProcessorError, ProcessorKey, ProcessorOutcome, ProcessorTestOutcome,
    TimedEnrichment,
};

/// Bound on concurrent in-flight processor calls per event. Live chat must
/// never spawn an unbounded fan-out of plugin calls.
pub(crate) const MAX_CONCURRENT_PROCESSORS: usize = 4;
/// Bound on concurrent in-flight processor calls across all events.
/// Overlapping live bursts shed load with `Overloaded` instead of queueing.
pub(crate) const MAX_TOTAL_PROCESSOR_SLOTS: usize = 16;
/// Largest serialized enrich request accepted on the hot path. Host-built
/// events are small; anything beyond this is rejected as input-too-large
/// instead of being handed to a plugin.
pub(crate) const MAX_PROCESSOR_EVENT_BYTES: usize = 256 * 1024;

impl AppCore {
    /// Rebuilds the contribution index from current plugin availability and
    /// activation. Called on install, uninstall, enable, and disable — never
    /// on the enrich hot path.
    pub(crate) fn rebuild_processor_index(&self) {
        let index = ContributionIndex::build(&self.plugins, &self.capabilities, |id| {
            self.plugin_ready(id)
        });
        *recover_rwlock_write(&self.processor_state.index, "processor index") = index;
    }

    pub(crate) fn eligible_processors(&self, event_type: &str) -> Vec<EligibleProcessor> {
        recover_rwlock_read(&self.processor_state.index, "processor index").eligible_for(event_type)
    }

    /// Runs the pre-filter enrichment pipeline over one canonical automation
    /// event. Without eligible processors the event returns unchanged;
    /// otherwise every failure fails open to the raw event.
    pub(crate) async fn enrich_automation_event(self: &Arc<Self>, event: Value) -> Value {
        let Some(event_type) = event.get("type").and_then(Value::as_str) else {
            return event;
        };
        let eligible = self.eligible_processors(event_type);
        if eligible.is_empty() {
            return event;
        }
        let invoker = crate::plugin_invoker::PluginInvoker::new(Arc::clone(&self.plugins));
        let outcomes = execute_processors(
            &invoker,
            &self.processor_state.health,
            &self.processor_state.metrics,
            &self.processor_state.slots,
            |id| self.processor_settings_for(id),
            eligible,
            &event,
        )
        .await;
        merge_processor_outcomes(event, &outcomes)
    }

    /// Settings object delivered inside enrich requests. Reloads from disk
    /// only after a UI save bumped the revision; corrupt or missing files
    /// degrade to null so the plugin falls back to its defaults.
    pub(crate) fn processor_settings_for(&self, plugin_id: &str) -> Value {
        self.processor_state.settings.settings_for(plugin_id, || {
            self.plugins
                .get(plugin_id)
                .and_then(|plugin| {
                    self.capabilities
                        .load_plugin_settings(&plugin.manifest)
                        .ok()
                })
                .filter(|settings| settings.is_object() || settings.is_null())
                .unwrap_or(Value::Null)
        })
    }

    pub(crate) fn bump_processor_settings_revision(&self, plugin_id: &str) {
        self.processor_state.settings.bump_revision(plugin_id);
    }

    pub(crate) async fn test_processor(
        &self,
        plugin_id: &str,
        processor_id: &str,
        event: Value,
    ) -> ProcessorTestOutcome {
        let invoker = crate::plugin_invoker::PluginInvoker::new(Arc::clone(&self.plugins));
        run_single_processor(
            &invoker,
            &self.capabilities,
            &self.processor_state.health,
            &self.processor_state.metrics,
            |id| self.plugin_ready(id),
            |id| self.processor_settings_for(id),
            plugin_id,
            processor_id,
            event,
        )
        .await
    }

    pub(crate) fn processor_status_snapshot(&self) -> Value {
        processor_status_entries(
            &self.plugins,
            &self.capabilities,
            |id| self.plugin_ready(id),
            &self.processor_state.health,
            &self.processor_state.metrics,
        )
    }
}
