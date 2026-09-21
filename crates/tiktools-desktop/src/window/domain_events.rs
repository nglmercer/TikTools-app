//! Domain-event forwarding to the WebView.

use std::sync::Arc;
use tiktools_core::AppCore;

/// Forwards domain events to a UI sink as JSON-RPC `event` notifications.
/// Reliable lag emits an explicit `event.gap` notification (and records
/// it for `system.health`) so the frontend resyncs instead of assuming a
/// complete stream; lossy lag just skips. Only a closed channel or the
/// shutdown event terminates the forwarder, so a temporary burst never
/// permanently disables WebView events.
pub(crate) async fn forward_domain_events(
    mut events: tiktools_core::events::DomainSubscription,
    core: Arc<AppCore>,
    mut send: impl FnMut(String),
) {
    use tiktools_core::events::DomainRecvError;
    loop {
        let event = match events.recv().await {
            Ok(event) => event,
            Err(DomainRecvError::ReliableLagged(lost)) => {
                tracing::warn!(
                    lost,
                    "WebView domain event receiver lagged on the reliable lane"
                );
                core.record_event_gap();
                send(tiktools_control_api::gap_notification(lost).to_string());
                continue;
            }
            Err(DomainRecvError::LossyLagged(skipped)) => {
                tracing::debug!(
                    skipped,
                    "WebView domain event receiver skipped a lossy burst"
                );
                continue;
            }
            Err(DomainRecvError::Closed) => break,
        };
        let shutdown = matches!(event, tiktools_core::events::DomainEvent::Shutdown);
        let notification = tiktools_control_api::event_notification(&event);
        send(notification.to_string());
        if shutdown {
            break;
        }
    }
}
