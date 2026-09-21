//! Plugin invocation, serialization, and error mapping.

use crate::AppCore;
use std::sync::Arc;
use std::time::Duration;
use tiktools_plugin_api::DomainEventEnvelope;

/// Event delivery is background work. A process that stops answering is
/// retired by the loader after this deadline and cannot hold the observer
/// forever.
const PLUGIN_EVENT_DELIVERY_TIMEOUT: Duration = Duration::from_secs(5);

pub(crate) async fn deliver_event(
    core: &Arc<AppCore>,
    invoker: &crate::plugin_invoker::PluginInvoker,
    plugin_id: &str,
    envelope: DomainEventEnvelope,
    lossy: bool,
) -> bool {
    let topic = envelope.topic.clone();
    let request = match serde_json::to_value(tiktools_plugin_sdk::PluginCall::event(envelope)) {
        Ok(request) => request,
        Err(error) => {
            tracing::warn!(plugin = %plugin_id, %error, "could not encode plugin event call");
            return false;
        }
    };
    match invoker
        .call_running(plugin_id, &request, PLUGIN_EVENT_DELIVERY_TIMEOUT)
        .await
    {
        Ok(_) => true,
        Err(crate::plugin_invoker::InvokeError::Unavailable(reason)) => {
            tracing::debug!(plugin = %plugin_id, %reason, "plugin event delivery skipped");
            false
        }
        Err(error) => {
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
