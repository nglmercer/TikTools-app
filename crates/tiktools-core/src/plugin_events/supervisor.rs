//! Event-bus subscription, worker registry, and dispatch fan-out.

use super::eligibility::eligible_for_event;
use super::queue::{PluginQueue, QueuedPluginEvent, PLUGIN_EVENT_QUEUE_CAPACITY};
use super::worker::run_plugin_queue;
use crate::events::{DomainEvent, DomainRecvError};
use crate::{now_millis, AppCore};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tiktools_plugin_api::DomainEventEnvelope;
use tokio::sync::mpsc;
use tokio::sync::Notify;

pub(crate) async fn run_observer(
    core: Arc<AppCore>,
    shutdown: Arc<Notify>,
    mut subscription: crate::events::DomainSubscription,
) {
    let mut queues: BTreeMap<String, PluginQueue> = BTreeMap::new();
    let mut workers = tokio::task::JoinSet::new();

    loop {
        tokio::select! {
            biased;
            _ = shutdown.notified() => break,
            received = subscription.recv() => match received {
                Ok(event) => dispatch_event(&core, event, &mut queues, &mut workers, &shutdown),
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
                        &mut queues,
                        &mut workers,
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

    // Dropping every sender closes the worker queues. Workers also observe
    // shutdown directly so a queue full of events cannot delay clean exit.
    drop(queues);
    while let Some(result) = workers.join_next().await {
        if let Err(error) = result {
            tracing::debug!(%error, "plugin event observer worker stopped during shutdown");
        }
    }
    tracing::info!("plugin event observer stopped");
}

fn dispatch_event(
    core: &Arc<AppCore>,
    event: DomainEvent,
    queues: &mut BTreeMap<String, PluginQueue>,
    workers: &mut tokio::task::JoinSet<()>,
    shutdown: &Arc<Notify>,
) {
    let lossy = event.is_lossy();
    let envelope = DomainEventEnvelope::new(
        event.topic(),
        serde_json::to_value(&event)
            .ok()
            .and_then(|value| value.get("data").cloned())
            .unwrap_or(Value::Null),
    );
    dispatch_envelope(core, envelope, lossy, false, queues, workers, shutdown);
}

fn dispatch_envelope(
    core: &Arc<AppCore>,
    envelope: DomainEventEnvelope,
    lossy: bool,
    force_all_subscribers: bool,
    queues: &mut BTreeMap<String, PluginQueue>,
    workers: &mut tokio::task::JoinSet<()>,
    shutdown: &Arc<Notify>,
) {
    let plugins = core.plugins.list();
    for plugin in plugins {
        if !eligible_for_event(core, &plugin, &envelope.topic, force_all_subscribers) {
            continue;
        }
        let plugin_id = plugin.manifest.id.clone();
        let queue = queues.entry(plugin_id.clone()).or_insert_with(|| {
            let (sender, receiver) = mpsc::channel(PLUGIN_EVENT_QUEUE_CAPACITY);
            let pending_reliable_gaps = Arc::new(AtomicU64::new(0));
            let core = Arc::clone(core);
            let shutdown = Arc::clone(shutdown);
            workers.spawn(run_plugin_queue(
                core,
                plugin_id.clone(),
                receiver,
                shutdown,
                Arc::clone(&pending_reliable_gaps),
            ));
            PluginQueue {
                sender,
                pending_reliable_gaps,
            }
        });
        let result = queue.sender.try_send(QueuedPluginEvent {
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
                        queue.pending_reliable_gaps.fetch_add(1, Ordering::Relaxed);
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
                    queues.remove(&plugin_id);
                    tracing::debug!(plugin = %plugin_id, "plugin event queue worker is closed");
                }
            }
        }
    }
}
