//! Processor fan-out execution with bounded concurrency.
//!
//! Each event runs at most [`MAX_CONCURRENT_PROCESSORS`](super::MAX_CONCURRENT_PROCESSORS)
//! calls, and all events together share the global slot semaphore so overlapping
//! live bursts cannot spawn an unbounded backlog of plugin calls.

use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use super::{
    state::{
        processor_retry_allowed, record_processor_failure, record_processor_metrics,
        record_processor_success,
    },
    EligibleProcessor, ProcessorError, ProcessorKey, ProcessorMetrics, ProcessorOutcome,
    ProcessorTestOutcome, TimedEnrichment, MAX_CONCURRENT_PROCESSORS, MAX_PROCESSOR_EVENT_BYTES,
};
use crate::{
    plugin_invoker::{InvokeError, PluginInvoker},
    services::CapabilityBroker,
    PluginHealth,
};
use serde_json::Value;
use tiktools_plugin_api::manifest::PluginProcessorDescriptor;
use tokio::sync::Semaphore;

/// Resolves a descriptor's declared input paths against one event into
/// `role -> value`, using the same `event.*` path language as filters and
/// templates. Unresolvable paths resolve to null so the processor sees
/// exactly what the host resolved, nothing more.
pub(crate) fn resolve_processor_inputs(
    descriptor: &PluginProcessorDescriptor,
    event: &Value,
) -> BTreeMap<String, Value> {
    let mut inputs = BTreeMap::new();
    for input in &descriptor.inputs {
        let value = crate::services::read_event_path(event, &input.path)
            .cloned()
            .unwrap_or(Value::Null);
        inputs.insert(input.role.clone(), value);
    }
    inputs
}

/// Executes eligible processors with bounded concurrency and returns their
/// outcomes in deterministic processor order. Health and metrics mutate here
/// in the awaiting task so concurrent plugin calls never contend on them.
/// Events that arrive while every global slot is occupied fail open with
/// [`ProcessorError::Overloaded`] instead of queueing behind the burst.
pub(crate) async fn execute_processors(
    invoker: &PluginInvoker,
    health: &Mutex<BTreeMap<ProcessorKey, PluginHealth>>,
    metrics: &Mutex<BTreeMap<ProcessorKey, ProcessorMetrics>>,
    slots: &Arc<Semaphore>,
    load_settings: impl Fn(&str) -> Value,
    eligible: Vec<EligibleProcessor>,
    event: &Value,
) -> Vec<ProcessorOutcome> {
    let mut tasks = tokio::task::JoinSet::new();
    let mut outcomes = Vec::with_capacity(eligible.len());
    for (index, processor) in eligible.into_iter().enumerate() {
        let key = ProcessorKey::of(&processor);
        if !processor_retry_allowed(health, &key) {
            // Health and metrics record once below with the other outcomes.
            outcomes.push((
                index,
                processor.plugin_id,
                processor.processor_id,
                Err(ProcessorError::CircuitOpen),
            ));
            continue;
        }
        let permit = match slots.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                outcomes.push((
                    index,
                    processor.plugin_id,
                    processor.processor_id,
                    Err(ProcessorError::Overloaded),
                ));
                continue;
            }
        };
        let invoker = invoker.clone();
        let event = event.clone();
        let settings = load_settings(&processor.plugin_id);
        tasks.spawn(async move {
            let _permit = permit;
            let deadline = processor.descriptor.timeout();
            let outcome =
                call_processor_once(&invoker, &processor, &event, settings, deadline).await;
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
        let key = ProcessorKey::new(plugin_id, processor_id);
        let duration_ms = match outcome {
            Ok(enrichment) => enrichment.duration_ms,
            Err(_) => 0,
        };
        record_processor_metrics(metrics, &key, outcome, duration_ms);
        match outcome {
            Ok(_) => record_processor_success(health, &key),
            // Circuit-open skips must not extend the backoff they observe,
            // and host-side overload is backpressure, not plugin failure.
            Err(ProcessorError::CircuitOpen | ProcessorError::Overloaded) => {}
            Err(error) => record_processor_failure(health, &key, error.to_string()),
        }
    }
    outcomes
}

async fn call_processor_once(
    invoker: &PluginInvoker,
    processor: &EligibleProcessor,
    event: &Value,
    settings: Value,
    deadline: Duration,
) -> Result<TimedEnrichment, ProcessorError> {
    let plugin_id = processor.plugin_id.as_str();
    let processor_id = processor.processor_id.as_str();
    let inputs = resolve_processor_inputs(&processor.descriptor, event);
    let request = serde_json::to_value(tiktools_plugin_sdk::PluginCall::enrich(
        tiktools_plugin_sdk::EventEnrichmentRequest::new(processor_id, event.clone())
            .settings(settings)
            .inputs(inputs),
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
    let response =
        invoker
            .call(plugin_id, &request, deadline)
            .await
            .map_err(|error| match error {
                InvokeError::Timeout => ProcessorError::Timeout,
                InvokeError::Unavailable(reason) => ProcessorError::Unavailable(reason),
                InvokeError::Plugin(reason) => ProcessorError::PluginError(reason),
            })?;
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

/// Runs one processor against a caller-supplied event for preview and
/// diagnostics. The circuit gate and the global slot bound are bypassed so an
/// operator can probe recovery, but the outcome still feeds health and metrics.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_single_processor(
    invoker: &PluginInvoker,
    capabilities: &CapabilityBroker,
    health: &Mutex<BTreeMap<ProcessorKey, PluginHealth>>,
    metrics: &Mutex<BTreeMap<ProcessorKey, ProcessorMetrics>>,
    ready: impl Fn(&str) -> bool,
    load_settings: impl Fn(&str) -> Value,
    plugin_id: &str,
    processor_id: &str,
    event: Value,
) -> ProcessorTestOutcome {
    let started = Instant::now();
    let plugin = invoker.plugins().get(plugin_id);
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
                match super::declared_processors(&plugin.manifest)
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
                            let deadline = descriptor.timeout();
                            let processor = EligibleProcessor {
                                plugin_id: plugin_id.to_owned(),
                                processor_id: processor_id.to_owned(),
                                descriptor,
                            };
                            call_processor_once(
                                invoker,
                                &processor,
                                &event,
                                load_settings(plugin_id),
                                deadline,
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
        // Report the time actually spent: a cold-start timeout waited the
        // full grace budget, and "took 0 ms" hides that from the operator.
        Err(error) => (
            false,
            started.elapsed().as_millis().min(u64::MAX as u128) as u64,
            Value::Null,
            Some(error.to_string()),
        ),
    };
    let key = ProcessorKey::new(plugin_id, processor_id);
    record_processor_metrics(metrics, &key, &outcome, duration_ms);
    match &outcome {
        Ok(_) => record_processor_success(health, &key),
        Err(error) => record_processor_failure(health, &key, error.to_string()),
    }
    ProcessorTestOutcome {
        ok,
        duration_ms,
        result,
        error,
    }
}
