//! Pre-filter event processor orchestration.
//!
//! Processors enrich existing host events before automation filters run. The
//! pipeline is fail-open: any processor timeout, crash, invalid response, or
//! missing capability passes the original event through, optionally stamped
//! with `intel.processing.status = "degraded"`. Processors never mutate raw
//! event fields; derived data merges only under the reserved `intel`
//! namespace.

use super::*;
use std::time::{Duration, Instant};

use tiktools_plugin_api::manifest::{validate_processor_type, PluginProcessorDescriptor};

/// Bound on concurrent in-flight processor calls per event. Live chat must
/// never spawn an unbounded fan-out of plugin calls.
pub(crate) const MAX_CONCURRENT_PROCESSORS: usize = 4;
/// Largest serialized enrich request accepted on the hot path. Host-built
/// events are small; anything beyond this is rejected as input-too-large
/// instead of being handed to a plugin.
pub(crate) const MAX_PROCESSOR_EVENT_BYTES: usize = 256 * 1024;

/// Host-defined annotation keys promoted from the provider namespace into
/// stable top-level `intel.*` fields. Any processor may contribute them;
/// textintel is only the first implementation, not a special case.
const STABLE_INTEL_KEYS: [&str; 2] = ["comment", "user"];
/// Reserved key under `providers.<pluginId>` holding text views. An
/// annotation using this name is skipped during the provider merge.
const PROVIDER_VIEWS_KEY: &str = "views";

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
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct EligibleProcessor {
    pub(crate) plugin_id: String,
    pub(crate) processor_id: String,
    pub(crate) descriptor: PluginProcessorDescriptor,
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

#[derive(Debug, Clone, Default)]
pub(crate) struct ProcessorMetrics {
    pub(crate) calls: u64,
    pub(crate) successes: u64,
    pub(crate) failures: u64,
    pub(crate) timeouts: u64,
    pub(crate) skipped_circuit_open: u64,
    pub(crate) total_latency_ms: u64,
    pub(crate) max_latency_ms: u64,
    pub(crate) last_error: Option<String>,
    pub(crate) last_success_at: Option<u64>,
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

impl ProcessorMetrics {
    pub(crate) fn average_latency_ms(&self) -> u64 {
        self.total_latency_ms
            .checked_div(self.successes)
            .unwrap_or(0)
    }
}

pub(crate) struct ProcessorTestOutcome {
    pub(crate) ok: bool,
    pub(crate) duration_ms: u64,
    pub(crate) result: Value,
    pub(crate) error: Option<String>,
}

/// Validated processor descriptors from one plugin manifest, in manifest
/// order. Invalid entries are skipped so one bad descriptor never breaks
/// discovery, mirroring `declared_event_types`.
pub(crate) fn declared_processors(
    manifest: &tiktools_plugin_api::manifest::PluginManifest,
) -> Vec<PluginProcessorDescriptor> {
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

/// Collects the processors eligible for one event type, sorted
/// deterministically by `(plugin id, processor id)`. A plugin must be ready
/// and declare `events.enrich`; an empty `eventTypes` subscription matches
/// every event type.
pub(crate) fn collect_eligible_processors(
    plugins: &PluginManager,
    capabilities: &CapabilityBroker,
    ready: impl Fn(&str) -> bool,
    event_type: &str,
) -> Vec<EligibleProcessor> {
    let mut eligible = Vec::new();
    for plugin in plugins.list() {
        let plugin_id = plugin.manifest.id.clone();
        if !ready(&plugin_id) {
            continue;
        }
        if capabilities
            .require_capability(
                &plugin.manifest,
                tiktools_plugin_api::capabilities::EVENTS_ENRICH,
            )
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
            let matches = descriptor.event_types.is_empty()
                || descriptor
                    .event_types
                    .iter()
                    .any(|subscribed| subscribed == event_type);
            if matches {
                eligible.push(EligibleProcessor {
                    plugin_id: plugin_id.clone(),
                    processor_id: descriptor.id.clone(),
                    descriptor,
                });
            }
        }
    }
    eligible.sort_by(|left, right| {
        (&left.plugin_id, &left.processor_id).cmp(&(&right.plugin_id, &right.processor_id))
    });
    eligible
}

/// Executes eligible processors with bounded concurrency and returns their
/// outcomes in deterministic processor order. Health and metrics mutate here
/// in the awaiting task so concurrent plugin calls never contend on them.
pub(crate) async fn execute_processors(
    plugins: &Arc<PluginManager>,
    health: &Mutex<BTreeMap<String, PluginHealth>>,
    metrics: &Mutex<BTreeMap<String, ProcessorMetrics>>,
    load_settings: impl Fn(&str) -> Value,
    eligible: Vec<EligibleProcessor>,
    event: &Value,
) -> Vec<ProcessorOutcome> {
    let mut tasks = tokio::task::JoinSet::new();
    let mut outcomes = Vec::with_capacity(eligible.len());
    for (index, processor) in eligible.into_iter().enumerate() {
        if !processor_retry_allowed(health, &processor.plugin_id) {
            // Health and metrics record once below with the other outcomes.
            outcomes.push((
                index,
                processor.plugin_id,
                processor.processor_id,
                Err(ProcessorError::CircuitOpen),
            ));
            continue;
        }
        let plugins = Arc::clone(plugins);
        let event = event.clone();
        let settings = load_settings(&processor.plugin_id);
        tasks.spawn(async move {
            let deadline = processor.descriptor.timeout();
            let outcome = call_processor_once(
                &plugins,
                &processor.plugin_id,
                &processor.processor_id,
                &event,
                settings,
                deadline,
            )
            .await;
            (index, processor.plugin_id, processor.processor_id, outcome)
        });
        if tasks.len() >= MAX_CONCURRENT_PROCESSORS {
            if let Some(Ok(outcome)) = tasks.join_next().await {
                outcomes.push(outcome);
            }
        }
    }
    while let Some(joined) = tasks.join_next().await {
        if let Ok(outcome) = joined {
            outcomes.push(outcome);
        }
    }
    outcomes.sort_by_key(|outcome| outcome.0);
    for (_, plugin_id, processor_id, outcome) in &outcomes {
        let duration_ms = match outcome {
            Ok(enrichment) => enrichment.duration_ms,
            Err(_) => 0,
        };
        record_processor_metrics(metrics, plugin_id, processor_id, outcome, duration_ms);
        match outcome {
            Ok(_) => record_processor_success(health, plugin_id),
            // Circuit-open skips must not extend the backoff they observe.
            Err(ProcessorError::CircuitOpen) => {}
            Err(error) => record_processor_failure(health, plugin_id, error.to_string()),
        }
    }
    outcomes
}

async fn call_processor_once(
    plugins: &Arc<PluginManager>,
    plugin_id: &str,
    processor_id: &str,
    event: &Value,
    settings: Value,
    deadline: Duration,
) -> Result<TimedEnrichment, ProcessorError> {
    if let Err(error) = plugins.start(plugin_id) {
        return Err(ProcessorError::Unavailable(error.to_string()));
    }
    let request = serde_json::to_value(tiktools_plugin_sdk::PluginCall::enrich(
        tiktools_plugin_sdk::EventEnrichmentRequest::new(processor_id, event.clone())
            .settings(settings),
    ))
    .map_err(|error| {
        ProcessorError::PluginError(format!("could not encode enrich call: {error}"))
    })?;
    if serde_json::to_vec(&request)
        .map(|bytes| bytes.len() > MAX_PROCESSOR_EVENT_BYTES)
        .unwrap_or(true)
    {
        return Err(ProcessorError::InputTooLarge);
    }
    let started = Instant::now();
    let plugins_for_call = Arc::clone(plugins);
    let plugin_id_for_call = plugin_id.to_owned();
    let response = tokio::time::timeout(
        deadline,
        tokio::task::spawn_blocking(move || {
            plugins_for_call.call_with_timeout(&plugin_id_for_call, &request, deadline)
        }),
    )
    .await
    .map_err(|_| ProcessorError::Timeout)?
    .map_err(|error| ProcessorError::PluginError(format!("processor task failed: {error}")))?
    .map_err(|error| ProcessorError::PluginError(error.to_string()))?;
    let duration_ms = started.elapsed().as_millis().min(u64::MAX as u128) as u64;
    let result = tiktools_plugin_sdk::decode_enrichment_result(response)
        .map_err(|error| ProcessorError::InvalidResponse(error.to_string()))?;
    tracing::debug!(
        processor = %processor_id,
        plugin = %plugin_id,
        duration_ms,
        annotations = result.annotations.len(),
        views = result.views.len(),
        "processor enriched event"
    );
    for line in result.logs.iter().take(4) {
        tracing::trace!(processor = %processor_id, plugin = %plugin_id, "{line}");
    }
    Ok(TimedEnrichment {
        result,
        duration_ms,
    })
}

/// Merges processor outcomes into the event's reserved `intel` namespace.
/// Raw event fields are never touched: annotations land under
/// `intel.providers.<pluginId>`, views under
/// `intel.providers.<pluginId>.views`, and only the host-defined stable
/// keys (`comment`, `user`) are additionally promoted to top-level
/// `intel.*`. Any failure stamps `intel.processing.status = "degraded"`.
pub(crate) fn merge_processor_outcomes(mut event: Value, outcomes: &[ProcessorOutcome]) -> Value {
    let mut any_success = false;
    let mut any_failure = false;
    for (_, plugin_id, _, outcome) in outcomes {
        match outcome {
            Ok(enrichment) => {
                any_success = true;
                merge_enrichment_result(&mut event, plugin_id, &enrichment.result);
            }
            Err(_) => {
                any_failure = true;
            }
        }
    }
    if any_failure {
        let intel = ensure_child_object(&mut event, "intel");
        let processing = ensure_child_object(intel, "processing");
        if let Some(object) = processing.as_object_mut() {
            object.insert("status".to_owned(), Value::String("degraded".to_owned()));
        }
    }
    let _ = any_success;
    event
}

fn merge_enrichment_result(
    event: &mut Value,
    plugin_id: &str,
    result: &tiktools_plugin_sdk::EventEnrichmentResult,
) {
    let intel = ensure_child_object(event, "intel");
    let providers = ensure_child_object(intel, "providers");
    let provider = ensure_child_object(providers, plugin_id);
    for (key, value) in &result.annotations {
        if key == PROVIDER_VIEWS_KEY {
            continue;
        }
        let slot = ensure_child_value(provider, key);
        deep_merge(slot, value);
    }
    if !result.views.is_empty() {
        let views = ensure_child_object(provider, PROVIDER_VIEWS_KEY);
        for (name, view) in &result.views {
            let slot = ensure_child_value(views, name);
            *slot = serde_json::to_value(view).unwrap_or(Value::Null);
        }
    }
    for key in STABLE_INTEL_KEYS {
        if let Some(value) = result.annotations.get(key) {
            if value.is_object() {
                let stable = ensure_child_object(intel, key);
                deep_merge(stable, value);
            }
        }
    }
}

/// Returns the named child as an object, replacing missing or mistyped
/// values. The `intel` namespace is host-reserved, so replacement there is
/// always safe.
fn ensure_child_object<'a>(parent: &'a mut Value, key: &str) -> &'a mut Value {
    if !parent.is_object() {
        *parent = Value::Object(Default::default());
    }
    let object = parent.as_object_mut().expect("parent must be an object");
    if !object.get(key).is_some_and(Value::is_object) {
        object.insert(key.to_owned(), Value::Object(Default::default()));
    }
    object.get_mut(key).expect("child object must exist")
}

fn ensure_child_value<'a>(parent: &'a mut Value, key: &str) -> &'a mut Value {
    if !parent.is_object() {
        *parent = Value::Object(Default::default());
    }
    let object = parent.as_object_mut().expect("parent must be an object");
    object.entry(key.to_owned()).or_insert(Value::Null)
}

fn deep_merge(target: &mut Value, source: &Value) {
    match (target.as_object_mut(), source.as_object()) {
        (Some(target), Some(source)) => {
            for (key, value) in source {
                match target.get_mut(key) {
                    Some(slot) => deep_merge(slot, value),
                    None => {
                        target.insert(key.clone(), value.clone());
                    }
                }
            }
        }
        _ => {
            *target = source.clone();
        }
    }
}

pub(crate) fn processor_retry_allowed(
    health: &Mutex<BTreeMap<String, PluginHealth>>,
    id: &str,
) -> bool {
    health
        .lock()
        .expect("processor health lock poisoned")
        .get(id)
        .and_then(|health| health.next_retry_at)
        .is_none_or(|next_retry_at| Instant::now() >= next_retry_at)
}

pub(crate) fn record_processor_failure(
    health: &Mutex<BTreeMap<String, PluginHealth>>,
    id: &str,
    error: String,
) {
    let mut health = health.lock().expect("processor health lock poisoned");
    let entry = health.entry(id.to_owned()).or_insert(PluginHealth {
        consecutive_failures: 0,
        next_retry_at: None,
    });
    entry.consecutive_failures = entry.consecutive_failures.saturating_add(1);
    let delay_seconds = plugin_backoff_seconds(entry.consecutive_failures);
    entry.next_retry_at = Some(Instant::now() + Duration::from_secs(delay_seconds));
    if entry.consecutive_failures <= 5 {
        tracing::warn!(
            plugin = %id,
            failures = entry.consecutive_failures,
            retry_in_seconds = delay_seconds,
            %error,
            "event processor entered retry backoff"
        );
    }
}

pub(crate) fn record_processor_success(health: &Mutex<BTreeMap<String, PluginHealth>>, id: &str) {
    let was_unhealthy = health
        .lock()
        .expect("processor health lock poisoned")
        .remove(id)
        .is_some_and(|health| health.consecutive_failures > 0);
    if was_unhealthy {
        tracing::info!(plugin = %id, "event processor recovered");
    }
}

fn record_processor_metrics(
    metrics: &Mutex<BTreeMap<String, ProcessorMetrics>>,
    plugin_id: &str,
    processor_id: &str,
    outcome: &Result<TimedEnrichment, ProcessorError>,
    duration_ms: u64,
) {
    let key = format!("{plugin_id}/{processor_id}");
    let mut metrics = metrics.lock().expect("processor metrics lock poisoned");
    let entry = metrics.entry(key).or_default();
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
        Err(error) => {
            entry.failures = entry.failures.saturating_add(1);
            entry.last_error = Some(error.to_string().chars().take(240).collect());
        }
    }
}

/// Runs one processor against a caller-supplied event for preview and
/// diagnostics. The circuit gate is bypassed so an operator can probe
/// recovery, but the outcome still feeds health and metrics.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_single_processor(
    plugins: &Arc<PluginManager>,
    capabilities: &CapabilityBroker,
    health: &Mutex<BTreeMap<String, PluginHealth>>,
    metrics: &Mutex<BTreeMap<String, ProcessorMetrics>>,
    ready: impl Fn(&str) -> bool,
    load_settings: impl Fn(&str) -> Value,
    plugin_id: &str,
    processor_id: &str,
    event: Value,
) -> ProcessorTestOutcome {
    let plugin = plugins.get(plugin_id);
    let outcome = match plugin {
        None => Err(ProcessorError::Unavailable(format!(
            "plugin `{plugin_id}` is not installed"
        ))),
        Some(plugin) => {
            if !ready(plugin_id) {
                Err(ProcessorError::Unavailable(format!(
                    "plugin `{plugin_id}` is not installed, enabled, or available"
                )))
            } else if let Err(error) = capabilities.require_capability(
                &plugin.manifest,
                tiktools_plugin_api::capabilities::EVENTS_ENRICH,
            ) {
                Err(ProcessorError::CapabilityDenied(error.to_string()))
            } else {
                match declared_processors(&plugin.manifest)
                    .into_iter()
                    .find(|descriptor| descriptor.id == processor_id)
                {
                    None => Err(ProcessorError::Unavailable(format!(
                        "processor `{processor_id}` is not declared by plugin `{plugin_id}`"
                    ))),
                    Some(descriptor) => {
                        if !event.is_object() {
                            Err(ProcessorError::InvalidResponse(
                                "test event must be a JSON object".to_owned(),
                            ))
                        } else {
                            call_processor_once(
                                plugins,
                                plugin_id,
                                processor_id,
                                &event,
                                load_settings(plugin_id),
                                descriptor.timeout(),
                            )
                            .await
                        }
                    }
                }
            }
        }
    };
    let (ok, duration_ms, result, error) = match &outcome {
        Ok(enrichment) => (
            true,
            enrichment.duration_ms,
            serde_json::to_value(&enrichment.result).unwrap_or(Value::Null),
            None,
        ),
        Err(error) => (false, 0, Value::Null, Some(error.to_string())),
    };
    record_processor_metrics(metrics, plugin_id, processor_id, &outcome, duration_ms);
    match &outcome {
        Ok(_) => record_processor_success(health, plugin_id),
        Err(error) => record_processor_failure(health, plugin_id, error.to_string()),
    }
    ProcessorTestOutcome {
        ok,
        duration_ms,
        result,
        error,
    }
}

/// Point-in-time processor catalog with health and metrics for the
/// host-owned processors panel and diagnostics.
pub(crate) fn processor_status_entries(
    plugins: &PluginManager,
    capabilities: &CapabilityBroker,
    ready: impl Fn(&str) -> bool,
    health: &Mutex<BTreeMap<String, PluginHealth>>,
    metrics: &Mutex<BTreeMap<String, ProcessorMetrics>>,
) -> Value {
    let health = health.lock().expect("processor health lock poisoned");
    let metrics = metrics.lock().expect("processor metrics lock poisoned");
    let mut entries = Vec::new();
    for plugin in plugins.list() {
        let plugin_id = plugin.manifest.id.clone();
        let declares_enrich = capabilities
            .require_capability(
                &plugin.manifest,
                tiktools_plugin_api::capabilities::EVENTS_ENRICH,
            )
            .is_ok();
        for descriptor in declared_processors(&plugin.manifest) {
            let key = format!("{plugin_id}/{}", descriptor.id);
            let metric = metrics.get(&key).cloned().unwrap_or_default();
            let status = if !ready(&plugin_id) {
                "disabled"
            } else if !declares_enrich {
                "unavailable"
            } else if health
                .get(&plugin_id)
                .and_then(|health| health.next_retry_at)
                .is_some_and(|next_retry_at| Instant::now() < next_retry_at)
            {
                "circuit-open"
            } else if health
                .get(&plugin_id)
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
                "status": status,
                "metrics": {
                    "calls": metric.calls,
                    "successes": metric.successes,
                    "failures": metric.failures,
                    "timeouts": metric.timeouts,
                    "skippedCircuitOpen": metric.skipped_circuit_open,
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

impl AppCore {
    pub(crate) fn eligible_processors(&self, event_type: &str) -> Vec<EligibleProcessor> {
        collect_eligible_processors(
            &self.plugins,
            &self.capabilities,
            |id| self.plugin_ready(id),
            event_type,
        )
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
        let outcomes = execute_processors(
            &self.plugins,
            &self.processor_health,
            &self.processor_metrics,
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
        let revision = {
            let cache = self
                .processor_settings
                .lock()
                .expect("processor settings lock poisoned");
            match cache.get(plugin_id) {
                Some(entry) if entry.loaded_revision == entry.current_revision => {
                    return entry.settings.clone();
                }
                Some(entry) => entry.current_revision,
                None => 0,
            }
        };
        let settings = self
            .plugins
            .get(plugin_id)
            .and_then(|plugin| {
                self.capabilities
                    .load_plugin_settings(&plugin.manifest)
                    .ok()
            })
            .filter(|settings| settings.is_object() || settings.is_null())
            .unwrap_or(Value::Null);
        let mut cache = self
            .processor_settings
            .lock()
            .expect("processor settings lock poisoned");
        let entry = cache.entry(plugin_id.to_owned()).or_default();
        entry.settings = settings.clone();
        entry.loaded_revision = revision;
        settings
    }

    pub(crate) fn bump_processor_settings_revision(&self, plugin_id: &str) {
        let mut cache = self
            .processor_settings
            .lock()
            .expect("processor settings lock poisoned");
        let entry = cache.entry(plugin_id.to_owned()).or_default();
        entry.current_revision = entry.current_revision.saturating_add(1);
    }

    pub(crate) async fn test_processor(
        &self,
        plugin_id: &str,
        processor_id: &str,
        event: Value,
    ) -> ProcessorTestOutcome {
        run_single_processor(
            &self.plugins,
            &self.capabilities,
            &self.processor_health,
            &self.processor_metrics,
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
            &self.processor_health,
            &self.processor_metrics,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::sync::atomic::{AtomicU64, Ordering};
    use tiktools_plugin_loader::{
        PluginInstance, PluginLoaderError, PluginRoot, PluginRuntime, PluginSource, RuntimeRegistry,
    };

    type Handler = Arc<dyn Fn(&[u8]) -> Result<Vec<u8>, PluginLoaderError> + Send + Sync>;

    struct FakeRuntime {
        handler: Handler,
    }

    impl PluginRuntime for FakeRuntime {
        fn kind(&self) -> tiktools_plugin_api::manifest::PluginRuntimeKind {
            tiktools_plugin_api::manifest::PluginRuntimeKind::Process
        }

        fn load(
            &self,
            manifest: &tiktools_plugin_api::manifest::PluginManifest,
            _directory: &Path,
        ) -> Result<Box<dyn PluginInstance>, PluginLoaderError> {
            Ok(Box::new(FakeInstance {
                id: manifest.id.clone(),
                handler: Arc::clone(&self.handler),
            }))
        }
    }

    struct FakeInstance {
        id: String,
        handler: Handler,
    }

    impl PluginInstance for FakeInstance {
        fn id(&self) -> &str {
            &self.id
        }

        fn handle_message(&mut self, request: &[u8]) -> Result<Vec<u8>, PluginLoaderError> {
            (self.handler)(request)
        }

        fn shutdown(&mut self) -> Result<(), PluginLoaderError> {
            Ok(())
        }
    }

    static HARNESS_COUNTER: AtomicU64 = AtomicU64::new(0);

    struct Harness {
        plugins: Arc<PluginManager>,
        capabilities: CapabilityBroker,
        health: Mutex<BTreeMap<String, PluginHealth>>,
        metrics: Mutex<BTreeMap<String, ProcessorMetrics>>,
    }

    fn scripted(
        handler: impl Fn(&[u8]) -> Result<Vec<u8>, PluginLoaderError> + Send + Sync + 'static,
    ) -> Handler {
        Arc::new(handler)
    }

    fn enrich_response(annotations: Value, views: Value) -> Vec<u8> {
        serde_json::to_vec(&json!({
            "annotations": annotations,
            "views": views,
            "logs": ["analyzed"],
        }))
        .unwrap()
    }

    fn chat_event() -> Value {
        json!({
            "id": "evt-1",
            "type": "tiktok.chat",
            "timestamp": 1,
            "connectionId": "connection-1",
            "creator": {"uniqueId": "creator", "roomId": "1"},
            "user": {"uniqueId": "alice", "nickname": "Alice", "secUid": "", "userId": "7"},
            "data": {"comment": "Hello there", "method": "m", "msgId": "1", "isHistory": false}
        })
    }

    fn processor_manifest(id: &str, capabilities: &[&str], processors: Value) -> Value {
        json!({
            "schemaVersion": 2,
            "id": id,
            "name": id,
            "version": "0.1.0",
            "runtime": "process",
            "entry": "entry.bin",
            "capabilities": capabilities,
            "processorTypes": processors,
        })
    }

    fn make_harness(manifests: &[(&str, Value)], handler: Handler) -> Harness {
        let tag = HARNESS_COUNTER.fetch_add(1, Ordering::AcqRel);
        let root = std::env::temp_dir().join(format!(
            "tiktools-processor-test-{}-{tag}",
            std::process::id()
        ));
        for (id, manifest) in manifests {
            let directory = root.join(id);
            std::fs::create_dir_all(&directory).unwrap();
            std::fs::write(
                directory.join("plugin.json"),
                serde_json::to_vec_pretty(manifest).unwrap(),
            )
            .unwrap();
            std::fs::write(directory.join("entry.bin"), b"fake").unwrap();
        }
        let mut runtimes = RuntimeRegistry::default();
        runtimes.register(Arc::new(FakeRuntime { handler }) as Arc<dyn PluginRuntime>);
        let plugins = Arc::new(PluginManager::with_runtimes(
            vec![PluginRoot {
                path: root.clone(),
                source: PluginSource::Development,
            }],
            runtimes,
        ));
        plugins.scan().unwrap();
        Harness {
            plugins,
            capabilities: CapabilityBroker::new(root.join("data")),
            health: Mutex::new(BTreeMap::new()),
            metrics: Mutex::new(BTreeMap::new()),
        }
    }

    fn drop_harness(harness: Harness) {
        harness.plugins.stop_all();
        let _ = harness;
    }

    async fn enrich(harness: &Harness, event: Value) -> Value {
        let event_type = event
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let eligible = collect_eligible_processors(
            &harness.plugins,
            &harness.capabilities,
            |_| true,
            &event_type,
        );
        let outcomes = execute_processors(
            &harness.plugins,
            &harness.health,
            &harness.metrics,
            |_| Value::Null,
            eligible,
            &event,
        )
        .await;
        merge_processor_outcomes(event, &outcomes)
    }

    fn raw_fields(event: &Value) -> Value {
        let mut event = event.clone();
        if let Some(object) = event.as_object_mut() {
            object.remove("intel");
        }
        event
    }

    #[tokio::test]
    async fn eligible_processor_enriches_matching_event() {
        let manifest = processor_manifest(
            "textintel",
            &["events.enrich"],
            json!([{
                "id": "textintel.analyze",
                "title": {"default": "Text Intelligence"},
                "eventTypes": ["tiktok.chat"],
            }]),
        );
        let harness = make_harness(
            &[("textintel", manifest)],
            scripted(|request| {
                let call: tiktools_plugin_sdk::PluginCall =
                    serde_json::from_slice(request).unwrap();
                let request = call.into_enrich().expect("must be an enrich call");
                assert_eq!(request.processor_id, "textintel.analyze");
                assert_eq!(request.event["data"]["comment"], "Hello there");
                Ok(enrich_response(
                    json!({"comment": {"normalized": "hello there"}}),
                    json!({"tts": {"text": "hello there", "source": "normalized"}}),
                ))
            }),
        );
        let event = chat_event();
        let enriched = enrich(&harness, event.clone()).await;
        assert_eq!(raw_fields(&enriched), raw_fields(&event));
        assert_eq!(enriched["intel"]["comment"]["normalized"], "hello there");
        assert_eq!(
            enriched["intel"]["providers"]["textintel"]["comment"]["normalized"],
            "hello there"
        );
        assert_eq!(
            enriched["intel"]["providers"]["textintel"]["views"]["tts"]["text"],
            "hello there"
        );
        assert!(enriched["intel"].get("processing").is_none());
        drop_harness(harness);
    }

    #[tokio::test]
    async fn enrich_requests_carry_plugin_settings() {
        let manifest = processor_manifest(
            "configured",
            &["events.enrich"],
            json!([{"id": "configured.analyze", "title": {"default": "Configured"}}]),
        );
        let harness = make_harness(
            &[("configured", manifest)],
            scripted(|request| {
                let call: tiktools_plugin_sdk::PluginCall =
                    serde_json::from_slice(request).unwrap();
                let request = call.into_enrich().unwrap();
                assert_eq!(request.settings["analyzeComments"], false);
                Ok(enrich_response(json!({}), json!({})))
            }),
        );
        let event = chat_event();
        let eligible = collect_eligible_processors(
            &harness.plugins,
            &harness.capabilities,
            |_| true,
            "tiktok.chat",
        );
        assert_eq!(eligible.len(), 1);
        let outcomes = execute_processors(
            &harness.plugins,
            &harness.health,
            &harness.metrics,
            |_| json!({"analyzeComments": false}),
            eligible,
            &event,
        )
        .await;
        assert!(outcomes[0].3.is_ok());
        drop_harness(harness);
    }

    #[tokio::test]
    async fn processor_not_matching_event_type_is_skipped() {
        let manifest = processor_manifest(
            "gifts",
            &["events.enrich"],
            json!([{
                "id": "gifts.describe",
                "title": {"default": "Gifts"},
                "eventTypes": ["tiktok.gift"],
            }]),
        );
        let harness = make_harness(
            &[("gifts", manifest)],
            scripted(|_| panic!("must not be called")),
        );
        let event = chat_event();
        let enriched = enrich(&harness, event.clone()).await;
        assert_eq!(enriched, event);
        drop_harness(harness);
    }

    #[tokio::test]
    async fn missing_capability_and_disabled_plugins_are_skipped() {
        let without_capability = processor_manifest(
            "nocap",
            &[],
            json!([{"id": "nocap.analyze", "title": {"default": "No cap"}}]),
        );
        let harness = make_harness(
            &[("nocap", without_capability)],
            scripted(|_| panic!("must not be called")),
        );
        let event = chat_event();
        assert!(collect_eligible_processors(
            &harness.plugins,
            &harness.capabilities,
            |_| true,
            "tiktok.chat"
        )
        .is_empty());
        assert_eq!(enrich(&harness, event.clone()).await, event);

        let disabled = processor_manifest(
            "off",
            &["events.enrich"],
            json!([{"id": "off.analyze", "title": {"default": "Off"}}]),
        );
        let harness = make_harness(
            &[("off", disabled)],
            scripted(|_| panic!("must not be called")),
        );
        assert!(collect_eligible_processors(
            &harness.plugins,
            &harness.capabilities,
            |_| false,
            "tiktok.chat"
        )
        .is_empty());
        drop_harness(harness);
    }

    #[tokio::test]
    async fn processor_timeout_passes_raw_event_through() {
        let manifest = processor_manifest(
            "slow",
            &["events.enrich"],
            json!([{
                "id": "slow.analyze",
                "title": {"default": "Slow"},
                "timeoutMs": 5,
            }]),
        );
        let harness = make_harness(
            &[("slow", manifest)],
            scripted(|_| {
                std::thread::sleep(Duration::from_millis(300));
                Ok(enrich_response(json!({}), json!({})))
            }),
        );
        let event = chat_event();
        let enriched = enrich(&harness, event.clone()).await;
        assert_eq!(raw_fields(&enriched), raw_fields(&event));
        assert_eq!(enriched["intel"]["processing"]["status"], "degraded");
        assert!(enriched["intel"].get("providers").is_none());
        let metrics = harness.metrics.lock().unwrap();
        let metric = &metrics["slow/slow.analyze"];
        assert_eq!(metric.timeouts, 1);
        assert_eq!(metric.failures, 1);
        drop(metrics);
        drop_harness(harness);
    }

    #[tokio::test]
    async fn processor_errors_pass_raw_event_through() {
        for (tag, handler) in [
            (
                "crash",
                scripted(|_| Err(PluginLoaderError::Runtime("boom".to_owned()))),
            ),
            ("invalid-json", scripted(|_| Ok(b"not json".to_vec()))),
            (
                "side-effect",
                scripted(|_| {
                    Ok(serde_json::to_vec(&json!({
                        "annotations": {},
                        "emit": [{"type": "x", "data": {}}],
                    }))
                    .unwrap())
                }),
            ),
        ] {
            let manifest = processor_manifest(
                tag,
                &["events.enrich"],
                json!([{"id": format!("{tag}.analyze"), "title": {"default": tag}}]),
            );
            // Rebind per-case plugin id for the manifest directory layout.
            let id = tag.to_owned();
            let manifest = {
                let mut manifest = manifest;
                manifest["id"] = Value::String(id.clone());
                manifest["name"] = Value::String(id.clone());
                manifest
            };
            let harness = make_harness(&[(tag, manifest)], handler);
            let event = chat_event();
            let enriched = enrich(&harness, event.clone()).await;
            assert_eq!(raw_fields(&enriched), raw_fields(&event), "{tag}");
            assert_eq!(
                enriched["intel"]["processing"]["status"], "degraded",
                "{tag}"
            );
            drop_harness(harness);
        }
    }

    #[tokio::test]
    async fn two_processors_merge_deterministically() {
        let first = processor_manifest(
            "aaa",
            &["events.enrich"],
            json!([{"id": "aaa.analyze", "title": {"default": "A"}}]),
        );
        let second = processor_manifest(
            "zzz",
            &["events.enrich"],
            json!([{"id": "zzz.analyze", "title": {"default": "Z"}}]),
        );
        let harness = make_harness(
            &[("zzz", second), ("aaa", first)],
            scripted(|request| {
                let call: tiktools_plugin_sdk::PluginCall =
                    serde_json::from_slice(request).unwrap();
                let request = call.into_enrich().unwrap();
                let tag = if request.processor_id.starts_with("aaa") {
                    "aaa"
                } else {
                    "zzz"
                };
                let mut comment = serde_json::Map::new();
                comment.insert("normalized".to_owned(), Value::String(tag.to_owned()));
                comment.insert(format!("only-{tag}"), Value::Bool(true));
                Ok(enrich_response(
                    json!({"comment": Value::Object(comment)}),
                    json!({}),
                ))
            }),
        );
        let event = chat_event();
        let enriched = enrich(&harness, event.clone()).await;
        // Both providers stay namespaced; the stable field resolves by the
        // deterministic (plugin id, processor id) order with later wins.
        assert_eq!(
            enriched["intel"]["providers"]["aaa"]["comment"]["normalized"],
            "aaa"
        );
        assert_eq!(
            enriched["intel"]["providers"]["zzz"]["comment"]["normalized"],
            "zzz"
        );
        assert_eq!(enriched["intel"]["comment"]["normalized"], "zzz");
        assert_eq!(enriched["intel"]["comment"]["only-aaa"], true);
        assert_eq!(enriched["intel"]["comment"]["only-zzz"], true);
        drop_harness(harness);
    }

    #[tokio::test]
    async fn oversized_enrichment_result_is_rejected() {
        let manifest = processor_manifest(
            "big",
            &["events.enrich"],
            json!([{"id": "big.analyze", "title": {"default": "Big"}}]),
        );
        let harness = make_harness(
            &[("big", manifest)],
            scripted(|_| {
                Ok(enrich_response(
                    json!({"comment": {"blob": "x".repeat(40 * 1024)}}),
                    json!({}),
                ))
            }),
        );
        let event = chat_event();
        let enriched = enrich(&harness, event.clone()).await;
        assert_eq!(raw_fields(&enriched), raw_fields(&event));
        assert_eq!(enriched["intel"]["processing"]["status"], "degraded");
        assert!(enriched["intel"].get("providers").is_none());
        drop_harness(harness);
    }

    #[tokio::test]
    async fn circuit_breaker_opens_and_recovers() {
        assert_eq!(plugin_backoff_seconds(1), 1);
        assert_eq!(plugin_backoff_seconds(2), 2);
        assert_eq!(plugin_backoff_seconds(4), 10);
        assert_eq!(plugin_backoff_seconds(9), 30);
        let manifest = processor_manifest(
            "flaky",
            &["events.enrich"],
            json!([{"id": "flaky.analyze", "title": {"default": "Flaky"}}]),
        );
        let calls = Arc::new(AtomicU64::new(0));
        let calls_for_handler = Arc::clone(&calls);
        let harness = make_harness(
            &[("flaky", manifest)],
            scripted(move |_| {
                calls_for_handler.fetch_add(1, Ordering::AcqRel);
                Err(PluginLoaderError::Runtime("boom".to_owned()))
            }),
        );
        let event = chat_event();
        let enriched = enrich(&harness, event.clone()).await;
        assert_eq!(enriched["intel"]["processing"]["status"], "degraded");
        assert_eq!(calls.load(Ordering::Acquire), 1);
        assert!(!processor_retry_allowed(&harness.health, "flaky"));

        // While the circuit is open the plugin is not called again.
        let enriched = enrich(&harness, event.clone()).await;
        assert_eq!(enriched["intel"]["processing"]["status"], "degraded");
        assert_eq!(calls.load(Ordering::Acquire), 1);
        let metrics = harness.metrics.lock().unwrap();
        assert_eq!(metrics["flaky/flaky.analyze"].skipped_circuit_open, 1);
        drop(metrics);

        // A successful retry closes the circuit.
        record_processor_success(&harness.health, "flaky");
        assert!(processor_retry_allowed(&harness.health, "flaky"));
        drop_harness(harness);
    }

    #[tokio::test]
    async fn single_processor_preview_reports_typed_outcomes() {
        let manifest = processor_manifest(
            "demo",
            &["events.enrich"],
            json!([{"id": "demo.analyze", "title": {"default": "Demo"}}]),
        );
        let harness = make_harness(
            &[("demo", manifest)],
            scripted(|_| Ok(enrich_response(json!({"comment": {"ok": true}}), json!({})))),
        );
        let outcome = run_single_processor(
            &harness.plugins,
            &harness.capabilities,
            &harness.health,
            &harness.metrics,
            |_| true,
            |_| Value::Null,
            "demo",
            "demo.analyze",
            chat_event(),
        )
        .await;
        assert!(outcome.ok);
        assert!(outcome.error.is_none());
        assert_eq!(outcome.result["annotations"]["comment"]["ok"], true);

        let missing = run_single_processor(
            &harness.plugins,
            &harness.capabilities,
            &harness.health,
            &harness.metrics,
            |_| true,
            |_| Value::Null,
            "missing",
            "missing.analyze",
            chat_event(),
        )
        .await;
        assert!(!missing.ok);
        assert!(missing.error.unwrap_or_default().contains("not installed"));

        let unknown = run_single_processor(
            &harness.plugins,
            &harness.capabilities,
            &harness.health,
            &harness.metrics,
            |_| true,
            |_| Value::Null,
            "demo",
            "demo.unknown",
            chat_event(),
        )
        .await;
        assert!(!unknown.ok);
        assert!(unknown.error.unwrap_or_default().contains("not declared"));

        let shaped = run_single_processor(
            &harness.plugins,
            &harness.capabilities,
            &harness.health,
            &harness.metrics,
            |_| true,
            |_| Value::Null,
            "demo",
            "demo.analyze",
            Value::String("nope".to_owned()),
        )
        .await;
        assert!(!shaped.ok);
        drop_harness(harness);
    }

    #[tokio::test]
    async fn status_snapshot_reports_ready_and_degraded() {
        let manifest = processor_manifest(
            "demo",
            &["events.enrich"],
            json!([
                {"id": "demo.analyze", "title": {"default": "Demo"}, "eventTypes": ["tiktok.chat"]},
                {"id": "BAD ID", "title": {"default": "Bad"}},
            ]),
        );
        let failing = processor_manifest(
            "broken",
            &["events.enrich"],
            json!([
                {"id": "broken.analyze", "title": {"default": "Broken"}},
            ]),
        );
        let harness = make_harness(
            &[("demo", manifest), ("broken", failing)],
            scripted(|_| Err(PluginLoaderError::Runtime("boom".to_owned()))),
        );
        // Invalid descriptors never appear in the catalog.
        let snapshot = processor_status_entries(
            &harness.plugins,
            &harness.capabilities,
            |_| true,
            &harness.health,
            &harness.metrics,
        );
        assert_eq!(snapshot.as_array().unwrap().len(), 2);
        assert_eq!(
            snapshot[0]["processorId"],
            "broken/broken.analyze".split('/').nth(1).unwrap()
        );
        assert_eq!(snapshot[0]["status"], "ready");

        // After a failure the entry degrades and then opens its circuit.
        let _ = enrich(&harness, chat_event()).await;
        let snapshot = processor_status_entries(
            &harness.plugins,
            &harness.capabilities,
            |_| true,
            &harness.health,
            &harness.metrics,
        );
        assert_eq!(snapshot[0]["status"], "circuit-open");
        assert_eq!(snapshot[0]["metrics"]["failures"], 1);

        // Disabled plugins report as disabled.
        let snapshot = processor_status_entries(
            &harness.plugins,
            &harness.capabilities,
            |_| false,
            &harness.health,
            &harness.metrics,
        );
        assert!(snapshot
            .as_array()
            .unwrap()
            .iter()
            .all(|entry| entry["status"] == "disabled"));
        drop_harness(harness);
    }

    #[tokio::test]
    async fn processor_fan_out_stays_within_its_concurrency_limit() {
        let manifests: Vec<(String, Value)> = (0..8)
            .map(|index| {
                let id = format!("burst{index}");
                let manifest = processor_manifest(
                    &id,
                    &["events.enrich"],
                    json!([{"id": format!("{id}.analyze"), "title": {"default": id}}]),
                );
                (id, manifest)
            })
            .collect();
        let in_flight = Arc::new(AtomicU64::new(0));
        let max_in_flight = Arc::new(AtomicU64::new(0));
        let in_flight_for_handler = Arc::clone(&in_flight);
        let max_for_handler = Arc::clone(&max_in_flight);
        let manifest_refs: Vec<(&str, Value)> = manifests
            .iter()
            .map(|(id, manifest)| (id.as_str(), manifest.clone()))
            .collect();
        let harness = make_harness(
            &manifest_refs,
            scripted(move |_| {
                let current = in_flight_for_handler.fetch_add(1, Ordering::AcqRel) + 1;
                max_for_handler.fetch_max(current, Ordering::AcqRel);
                std::thread::sleep(Duration::from_millis(20));
                in_flight_for_handler.fetch_sub(1, Ordering::AcqRel);
                Ok(enrich_response(json!({"comment": {"ok": true}}), json!({})))
            }),
        );
        let event = chat_event();
        let enriched = enrich(&harness, event.clone()).await;
        assert_eq!(raw_fields(&enriched), raw_fields(&event));
        assert!(enriched["intel"]["comment"]["ok"] == true);
        let max = max_in_flight.load(Ordering::Acquire);
        assert!(
            max <= MAX_CONCURRENT_PROCESSORS as u64,
            "in-flight peak {max} exceeds the limit"
        );
        assert!(max > 1, "burst did not overlap; the limit was not stressed");
        drop_harness(harness);
    }

    #[tokio::test]
    async fn chat_burst_flows_raw_while_a_processor_fails() {
        let manifest = processor_manifest(
            "down",
            &["events.enrich"],
            json!([{"id": "down.analyze", "title": {"default": "Down"}}]),
        );
        let calls = Arc::new(AtomicU64::new(0));
        let calls_for_handler = Arc::clone(&calls);
        let harness = make_harness(
            &[("down", manifest)],
            scripted(move |_| {
                calls_for_handler.fetch_add(1, Ordering::AcqRel);
                Err(PluginLoaderError::Runtime("boom".to_owned()))
            }),
        );
        for index in 0..50 {
            let mut event = chat_event();
            event["id"] = Value::String(format!("evt-{index}"));
            let enriched = enrich(&harness, event.clone()).await;
            assert_eq!(raw_fields(&enriched), raw_fields(&event), "event {index}");
            assert_eq!(enriched["intel"]["processing"]["status"], "degraded");
        }
        // The first failure opens the circuit; the burst does not retry it.
        assert_eq!(calls.load(Ordering::Acquire), 1);
        drop_harness(harness);
    }

    #[test]
    fn merge_promotes_only_object_stable_keys_and_reserves_views() {
        let event = chat_event();
        let mut result = tiktools_plugin_sdk::EventEnrichmentResult::default();
        result.annotations.insert(
            "comment".to_owned(),
            Value::String("not-an-object".to_owned()),
        );
        result
            .annotations
            .insert("views".to_owned(), json!({"smuggled": true}));
        result.views.insert(
            "tts".to_owned(),
            tiktools_plugin_sdk::TextView::new("hello", "raw"),
        );
        let enriched = merge_processor_outcomes(
            event,
            &[(
                0,
                "demo".to_owned(),
                "demo.analyze".to_owned(),
                Ok(TimedEnrichment {
                    result,
                    duration_ms: 3,
                }),
            )],
        );
        // Non-object stable annotations stay provider-namespaced only.
        assert!(enriched["intel"].get("comment").is_none());
        assert_eq!(
            enriched["intel"]["providers"]["demo"]["comment"],
            "not-an-object"
        );
        // The reserved views key is never overwritten by annotations.
        assert_eq!(
            enriched["intel"]["providers"]["demo"]["views"]["tts"]["text"],
            "hello"
        );
        assert!(enriched["intel"].get("processing").is_none());
    }
}
