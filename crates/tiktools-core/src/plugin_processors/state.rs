//! Processor health, metrics, and settings caches.
//!
//! All state here is keyed by [`ProcessorKey`](super::ProcessorKey) so one
//! processor's failures never affect its siblings.

use std::{
    collections::BTreeMap,
    sync::Mutex,
    time::{Duration, Instant},
};

use super::{ProcessorError, ProcessorKey, TimedEnrichment};
use crate::{now_millis, plugin_backoff_seconds, PluginHealth};
use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub(crate) struct ProcessorMetrics {
    pub(crate) calls: u64,
    pub(crate) successes: u64,
    pub(crate) failures: u64,
    pub(crate) timeouts: u64,
    pub(crate) skipped_circuit_open: u64,
    pub(crate) skipped_overloaded: u64,
    pub(crate) total_latency_ms: u64,
    pub(crate) max_latency_ms: u64,
    pub(crate) last_error: Option<String>,
    pub(crate) last_success_at: Option<u64>,
}

impl ProcessorMetrics {
    pub(crate) fn average_latency_ms(&self) -> u64 {
        self.total_latency_ms
            .checked_div(self.successes)
            .unwrap_or(0)
    }
}

/// Cached processor settings. The host saves settings to disk on UI edits
/// and bumps `current_revision`; the enrich path reloads the file only when
/// it lags, so steady-state enrichment costs one lock plus a clone.
#[derive(Debug, Clone)]
pub(crate) struct CachedProcessorSettings {
    pub(crate) current_revision: u64,
    pub(crate) loaded_revision: u64,
    pub(crate) settings: Value,
}

impl Default for CachedProcessorSettings {
    fn default() -> Self {
        Self {
            current_revision: 0,
            loaded_revision: 0,
            settings: Value::Null,
        }
    }
}

/// Revision-checked settings cache shared by the enrich path and previews.
#[derive(Debug, Default)]
pub(crate) struct ProcessorSettingsStore {
    cache: Mutex<BTreeMap<String, CachedProcessorSettings>>,
}

impl ProcessorSettingsStore {
    /// Returns cached settings, invoking `load` only when the entry is
    /// missing or a UI save bumped its revision. The loader runs outside the
    /// lock so disk reads never block concurrent enrichment.
    pub(crate) fn settings_for(&self, plugin_id: &str, load: impl FnOnce() -> Value) -> Value {
        let revision = {
            let cache = self.cache.lock().expect("processor settings lock poisoned");
            match cache.get(plugin_id) {
                Some(entry) if entry.loaded_revision == entry.current_revision => {
                    return entry.settings.clone();
                }
                Some(entry) => entry.current_revision,
                None => 0,
            }
        };
        let settings = load();
        let mut cache = self.cache.lock().expect("processor settings lock poisoned");
        let entry = cache.entry(plugin_id.to_owned()).or_default();
        entry.settings = settings.clone();
        entry.loaded_revision = revision;
        settings
    }

    pub(crate) fn bump_revision(&self, plugin_id: &str) {
        let mut cache = self.cache.lock().expect("processor settings lock poisoned");
        let entry = cache.entry(plugin_id.to_owned()).or_default();
        entry.current_revision = entry.current_revision.saturating_add(1);
    }
}

pub(crate) fn processor_retry_allowed(
    health: &Mutex<BTreeMap<ProcessorKey, PluginHealth>>,
    key: &ProcessorKey,
) -> bool {
    health
        .lock()
        .expect("processor health lock poisoned")
        .get(key)
        .and_then(|health| health.next_retry_at)
        .is_none_or(|next_retry_at| Instant::now() >= next_retry_at)
}

pub(crate) fn record_processor_failure(
    health: &Mutex<BTreeMap<ProcessorKey, PluginHealth>>,
    key: &ProcessorKey,
    error: String,
) {
    let mut health = health.lock().expect("processor health lock poisoned");
    let entry = health.entry(key.clone()).or_insert(PluginHealth {
        consecutive_failures: 0,
        next_retry_at: None,
    });
    entry.consecutive_failures = entry.consecutive_failures.saturating_add(1);
    let delay_seconds = plugin_backoff_seconds(entry.consecutive_failures);
    entry.next_retry_at = Some(Instant::now() + Duration::from_secs(delay_seconds));
    if entry.consecutive_failures <= 5 {
        tracing::warn!(
            plugin = %key.plugin_id,
            processor = %key.processor_id,
            failures = entry.consecutive_failures,
            retry_in_seconds = delay_seconds,
            %error,
            "event processor entered retry backoff"
        );
    }
}

pub(crate) fn record_processor_success(
    health: &Mutex<BTreeMap<ProcessorKey, PluginHealth>>,
    key: &ProcessorKey,
) {
    let was_unhealthy = health
        .lock()
        .expect("processor health lock poisoned")
        .remove(key)
        .is_some_and(|health| health.consecutive_failures > 0);
    if was_unhealthy {
        tracing::info!(
            plugin = %key.plugin_id,
            processor = %key.processor_id,
            "event processor recovered"
        );
    }
}

pub(crate) fn record_processor_metrics(
    metrics: &Mutex<BTreeMap<ProcessorKey, ProcessorMetrics>>,
    key: &ProcessorKey,
    outcome: &Result<TimedEnrichment, ProcessorError>,
    duration_ms: u64,
) {
    let mut metrics = metrics.lock().expect("processor metrics lock poisoned");
    let entry = metrics.entry(key.clone()).or_default();
    entry.calls = entry.calls.saturating_add(1);
    match outcome {
        Ok(_) => {
            entry.successes = entry.successes.saturating_add(1);
            entry.total_latency_ms = entry.total_latency_ms.saturating_add(duration_ms);
            entry.max_latency_ms = entry.max_latency_ms.max(duration_ms);
            entry.last_success_at = Some(now_millis());
        }
        Err(ProcessorError::Timeout) => {
            entry.failures = entry.failures.saturating_add(1);
            entry.timeouts = entry.timeouts.saturating_add(1);
            entry.last_error = Some(ProcessorError::Timeout.to_string());
        }
        Err(ProcessorError::CircuitOpen) => {
            entry.skipped_circuit_open = entry.skipped_circuit_open.saturating_add(1);
        }
        Err(ProcessorError::Overloaded) => {
            entry.skipped_overloaded = entry.skipped_overloaded.saturating_add(1);
        }
        Err(error) => {
            entry.failures = entry.failures.saturating_add(1);
            entry.last_error = Some(error.to_string().chars().take(240).collect());
        }
    }
}
