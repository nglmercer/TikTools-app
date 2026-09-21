//! Event-bus subscription, worker registry, and dispatch fan-out.

use super::eligibility::{eligible_for_delivery, eligible_for_event};
use super::queue::{QueuedPluginEvent, WorkerHandle, PLUGIN_EVENT_QUEUE_CAPACITY};
use super::worker::run_plugin_queue;
use crate::events::{DomainEvent, DomainRecvError};
use crate::{now_millis, AppCore};
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tiktools_plugin_api::DomainEventEnvelope;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub(crate) async fn run_observer(
    core: Arc<AppCore>,
    shutdown: CancellationToken,
    mut subscription: crate::events::DomainSubscription,
) {
    let mut workers: BTreeMap<String, WorkerHandle> = BTreeMap::new();
    let mut tasks = tokio::task::JoinSet::new();

    loop {
        tokio::select! {
            biased;
            () = shutdown.cancelled() => break,
            received = subscription.recv() => match received {
                Ok(event) => dispatch_event(&core, event, &mut workers, &mut tasks, &shutdown),
                Err(DomainRecvError::ReliableLagged(lost)) => {
                    // The observer itself missed authoritative events. Keep
                    // the host health signal consistent with IPC/WebView and
                    // give subscribers a stable, transport-neutral gap topic.
                    core.record_event_gap();
                    dispatch_envelope(
                        &core,
                        DomainEventEnvelope::new(
                            "event.gap",
                            json!({"lost": lost, "resync": true, "at": now_millis()}),
                        ),
                        false,
                        true,
                        &mut workers,
                        &mut tasks,
                        &shutdown,
                    );
                }
                Err(DomainRecvError::LossyLagged(skipped)) => {
                    tracing::debug!(skipped, "plugin event observer skipped a lossy burst");
                }
                Err(DomainRecvError::Closed) => break,
            },
        }
    }

    // Cancelling first wakes every worker even if a queue is full; dropping
    // the senders then lets already-exiting workers finish draining.
    shutdown.cancel();
    for handle in workers.values() {
        handle.token.cancel();
    }
    drop(workers);
    while let Some(result) = tasks.join_next().await {
        if let Err(error) = result {
            tracing::debug!(%error, "plugin event observer worker stopped during shutdown");
        }
    }
    tracing::info!("plugin event observer stopped");
}

fn dispatch_event(
    core: &Arc<AppCore>,
    event: DomainEvent,
    workers: &mut BTreeMap<String, WorkerHandle>,
    tasks: &mut tokio::task::JoinSet<()>,
    shutdown: &CancellationToken,
) {
    let lossy = event.is_lossy();
    let envelope = match event.to_envelope() {
        Ok(envelope) => envelope,
        Err(error) => {
            // Invariant violation: drop the event loudly instead of
            // delivering fabricated `null` data. Reliable drops also
            // record a gap so subscribers refresh authoritative state.
            tracing::error!(topic = event.topic(), %error, "domain event failed envelope conversion");
            if !lossy {
                core.record_event_gap();
            }
            return;
        }
    };
    dispatch_envelope(core, envelope, lossy, false, workers, tasks, shutdown);
}

fn dispatch_envelope(
    core: &Arc<AppCore>,
    envelope: DomainEventEnvelope,
    lossy: bool,
    force_all_subscribers: bool,
    workers: &mut BTreeMap<String, WorkerHandle>,
    tasks: &mut tokio::task::JoinSet<()>,
    shutdown: &CancellationToken,
) {
    let plugins = core.plugins.list();
    for plugin in &plugins {
        if !eligible_for_event(core, plugin, &envelope.topic, force_all_subscribers) {
            continue;
        }
        let plugin_id = plugin.manifest.id.clone();
        let handle = workers.entry(plugin_id.clone()).or_insert_with(|| {
            let (sender, receiver) = mpsc::channel(PLUGIN_EVENT_QUEUE_CAPACITY);
            let pending_reliable_gaps = Arc::new(AtomicU64::new(0));
            let token = shutdown.child_token();
            let core = Arc::clone(core);
            tasks.spawn(run_plugin_queue(
                core,
                plugin_id.clone(),
                receiver,
                token.clone(),
                Arc::clone(&pending_reliable_gaps),
            ));
            WorkerHandle {
                sender,
                token,
                pending_reliable_gaps,
            }
        });
        let result = handle.sender.try_send(QueuedPluginEvent {
            envelope: envelope.clone(),
            lossy,
        });
        if let Err(error) = result {
            match error {
                mpsc::error::TrySendError::Full(_) => {
                    // Lossy events may be shed by design. Reliable events
                    // make the subscriber's stream incomplete, so surface a
                    // host-wide gap just as other reliable transports do.
                    if !lossy && envelope.topic != "event.gap" {
                        core.record_event_gap();
                        handle.pending_reliable_gaps.fetch_add(1, Ordering::Relaxed);
                    }
                    core.record_plugin_drops(&plugin_id, 1);
                    tracing::warn!(
                        plugin = %plugin_id,
                        topic = %envelope.topic,
                        lossy,
                        "plugin event queue is full; event dropped"
                    );
                }
                mpsc::error::TrySendError::Closed(_) => {
                    if let Some(handle) = workers.remove(&plugin_id) {
                        handle.token.cancel();
                    }
                    tracing::debug!(plugin = %plugin_id, "plugin event queue worker is closed");
                }
            }
        }
    }
    reconcile_workers(core, &plugins, workers);
}

/// Drops workers whose plugin can no longer receive events (uninstalled,
/// disabled, stopped, or unsubscribed) so repeated lifecycle churn cannot
/// leak worker tasks or queues. Never starts or restarts plugins.
pub(crate) fn reconcile_workers(
    core: &AppCore,
    plugins: &[tiktools_plugin_loader::DiscoveredPlugin],
    workers: &mut BTreeMap<String, WorkerHandle>,
) {
    workers.retain(|plugin_id, handle| {
        let alive = plugins
            .iter()
            .find(|plugin| plugin.manifest.id == *plugin_id)
            .is_some_and(|plugin| eligible_for_delivery(core, plugin));
        if !alive {
            handle.token.cancel();
            tracing::debug!(plugin = %plugin_id, "plugin event worker pruned");
        }
        alive
    });
}
