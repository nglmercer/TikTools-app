//! Plugin invocation, serialization, and error mapping.

use crate::AppCore;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tiktools_plugin_api::DomainEventEnvelope;

/// Event delivery is background work. A process that stops answering is
/// retired by the loader after this deadline and cannot hold the observer
/// forever.
// Tests shrink the fuse so shutdown-during-stuck-delivery scenarios do not
// each wait out the full production window; delivery semantics are unchanged.
const PLUGIN_EVENT_DELIVERY_TIMEOUT: Duration = Duration::from_secs(if cfg!(test) { 1 } else { 5 });

pub(crate) async fn deliver_event(
    core: &Arc<AppCore>,
    invoker: &crate::plugin_invoker::PluginInvoker,
    plugin_id: &str,
    generation: u64,
    envelope: &DomainEventEnvelope,
    lossy: bool,
) -> bool {
    let started = Instant::now();
    let topic = envelope.topic.clone();
    // The fan-out shares one `Arc`; this clone for serialization happens
    // once per delivery, off the hot path and after queue admission.
    let request =
        match serde_json::to_value(tiktools_plugin_sdk::PluginCall::event(envelope.clone())) {
            Ok(request) => request,
            Err(error) => {
                tracing::warn!(plugin = %plugin_id, %error, "could not encode plugin event call");
                core.record_plugin_delivered(
                    plugin_id,
                    started.elapsed().as_millis() as u64,
                    true,
                    false,
                );
                return false;
            }
        };
    match invoker
        .call_running(
            plugin_id,
            &request,
            PLUGIN_EVENT_DELIVERY_TIMEOUT,
            generation,
        )
        .await
    {
        Ok(_) => {
            core.record_plugin_delivered(
                plugin_id,
                started.elapsed().as_millis() as u64,
                false,
                false,
            );
            true
        }
        Err(crate::plugin_invoker::InvokeError::Timeout) => {
            core.record_plugin_delivered(
                plugin_id,
                started.elapsed().as_millis() as u64,
                true,
                true,
            );
            core.record_plugin_failure(plugin_id, "plugin call timed out".to_owned());
            tracing::warn!(
                plugin = %plugin_id,
                topic = %topic,
                lossy,
                "plugin event delivery timed out"
            );
            false
        }
        Err(crate::plugin_invoker::InvokeError::Unavailable(reason)) => {
            // Skipped, not failed: the plugin stopped mid-flight. No
            // completion recorded either way, matching previous behavior.
            tracing::debug!(plugin = %plugin_id, %reason, "plugin event delivery skipped");
            false
        }
        Err(error) => {
            core.record_plugin_delivered(
                plugin_id,
                started.elapsed().as_millis() as u64,
                true,
                false,
            );
            core.record_plugin_failure(plugin_id, error.to_string());
            tracing::warn!(
                plugin = %plugin_id,
                topic = %topic,
                lossy,
                error = %error,
                "plugin event delivery failed"
            );
            false
        }
    }
}
