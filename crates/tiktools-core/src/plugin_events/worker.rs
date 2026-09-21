//! One ordered delivery worker per subscribed plugin.

use super::delivery::deliver_event;
use super::queue::QueuedPluginEvent;
use crate::{now_millis, AppCore};
use serde_json::json;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tiktools_plugin_api::DomainEventEnvelope;
use tokio::sync::mpsc;

pub(crate) async fn run_plugin_queue(
    core: Arc<AppCore>,
    plugin_id: String,
    mut receiver: mpsc::Receiver<QueuedPluginEvent>,
    shutdown: tokio_util::sync::CancellationToken,
    pending_reliable_gaps: Arc<AtomicU64>,
) {
    let invoker = crate::plugin_invoker::PluginInvoker::new(Arc::clone(&core.plugins));
    loop {
        let queued = tokio::select! {
            biased;
            () = shutdown.cancelled() => break,
            queued = receiver.recv() => match queued {
                Some(queued) => queued,
                None => break,
            },
        };

        // Drop everything queued while the plugin was disabled/stopped. The
        // lifecycle operation owns the authoritative state; event delivery
        // must never restart a plugin as a side effect.
        if !core.plugin_ready(&plugin_id) || !core.plugins.is_running(&plugin_id) {
            continue;
        }

        let lost = pending_reliable_gaps.swap(0, Ordering::AcqRel);
        if lost > 0 {
            let gap = DomainEventEnvelope::new(
                "event.gap",
                json!({"lost": lost, "resync": true, "at": now_millis()}),
            );
            if !deliver_event(&core, &invoker, &plugin_id, gap, false).await {
                // Keep the summary if a transient delivery failure happened
                // while the plugin remained active; a stopped/crashed plugin
                // will simply drain it when it is explicitly restarted.
                pending_reliable_gaps.fetch_add(lost, Ordering::Relaxed);
            }
        }
        // A cancelled shutdown token does not preempt an in-flight delivery,
        // but no new delivery starts after cancellation is observed.
        if shutdown.is_cancelled() {
            break;
        }
        let _ = deliver_event(&core, &invoker, &plugin_id, queued.envelope, queued.lossy).await;
    }
}
