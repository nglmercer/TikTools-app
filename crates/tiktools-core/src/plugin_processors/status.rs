//! Host-owned processor catalog for the processors panel and diagnostics.

use std::{collections::BTreeMap, sync::Mutex, time::Instant};

use super::{declared_processors, ProcessorKey, ProcessorMetrics};
use crate::{services::CapabilityBroker, PluginHealth};
use serde_json::{json, Value};
use tiktools_plugin_loader::PluginManager;

/// Point-in-time processor catalog with health and metrics.
pub(crate) fn processor_status_entries(
    plugins: &PluginManager,
    capabilities: &CapabilityBroker,
    ready: impl Fn(&str) -> bool,
    health: &Mutex<BTreeMap<ProcessorKey, PluginHealth>>,
    metrics: &Mutex<BTreeMap<ProcessorKey, ProcessorMetrics>>,
) -> Value {
    let health = health.lock().expect("processor health lock poisoned");
    let metrics = metrics.lock().expect("processor metrics lock poisoned");
    let mut entries = Vec::new();
    for plugin in plugins.list() {
        let plugin_id = plugin.manifest.id.clone();
        if plugin.manifest.runtime == tiktools_plugin_api::PluginRuntimeKind::Declarative {
            continue;
        }
        let declares_enrich = capabilities
            .require_capability(
                &plugin.manifest,
                tiktools_plugin_api::capabilities::EVENTS_ENRICH,
            )
            .is_ok();
        for descriptor in declared_processors(&plugin.manifest) {
            let key = ProcessorKey::new(&plugin_id, &descriptor.id);
            let metric = metrics.get(&key).cloned().unwrap_or_default();
            let status = if !ready(&plugin_id) {
                "disabled"
            } else if !declares_enrich {
                "unavailable"
            } else if health
                .get(&key)
                .and_then(|health| health.next_retry_at)
                .is_some_and(|next_retry_at| Instant::now() < next_retry_at)
            {
                "circuit-open"
            } else if health
                .get(&key)
                .is_some_and(|health| health.consecutive_failures > 0)
            {
                "degraded"
            } else {
                "ready"
            };
            entries.push(json!({
                "pluginId": plugin_id,
                "processorId": descriptor.id,
                "eventTypes": descriptor.event_types,
                "timeoutMs": descriptor.timeout_ms.unwrap_or(
                    tiktools_plugin_api::manifest::DEFAULT_PLUGIN_PROCESSOR_TIMEOUT_MS
                ),
                "priority": descriptor.priority_value(),
                "status": status,
                "metrics": {
                    "calls": metric.calls,
                    "successes": metric.successes,
                    "failures": metric.failures,
                    "timeouts": metric.timeouts,
                    "skippedCircuitOpen": metric.skipped_circuit_open,
                    "skippedOverloaded": metric.skipped_overloaded,
                    "averageLatencyMs": metric.average_latency_ms(),
                    "maxLatencyMs": metric.max_latency_ms,
                    "lastError": metric.last_error,
                    "lastSuccessAt": metric.last_success_at,
                },
            }));
        }
    }
    entries.sort_by(|left, right| {
        fn key(entry: &Value) -> (&str, &str) {
            (
                entry.get("pluginId").and_then(Value::as_str).unwrap_or(""),
                entry
                    .get("processorId")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
            )
        }
        key(left).cmp(&key(right))
    });
    Value::Array(entries)
}
