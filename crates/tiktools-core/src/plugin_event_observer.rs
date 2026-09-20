//! Generic asynchronous delivery of domain events to subscribing plugins.
//!
//! This module is deliberately transport-neutral. It knows about the stable
//! plugin event envelope and plugin lifecycle/capability policy, but it does
//! not know whether a plugin forwards events to WebSockets, MQTT, OSC,
//! Discord, or another system.

use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use serde_json::{json, Value};
use tiktools_plugin_api::{
    capabilities::EVENTS_SUBSCRIBE, event_subscription_matches, DomainEventEnvelope,
};
use tiktools_plugin_loader::DiscoveredPlugin;
use tokio::sync::{mpsc, Notify};

use crate::events::{DomainEvent, DomainRecvError};
use crate::{now_millis, AppCore};

/// A slow event observer is isolated behind this queue. `try_send` is used at
/// the bus boundary, so the publisher never waits for a plugin process.
pub(crate) const PLUGIN_EVENT_QUEUE_CAPACITY: usize = 64;
/// Event delivery is background work. A process that stops answering is
/// retired by the loader after this deadline and cannot hold the observer
/// forever.
const PLUGIN_EVENT_DELIVERY_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
struct QueuedPluginEvent {
    envelope: DomainEventEnvelope,
    lossy: bool,
}

struct PluginQueue {
    sender: mpsc::Sender<QueuedPluginEvent>,
    /// Reliable events shed at the queue boundary are summarized and sent as
    /// one stable gap envelope when the worker next gets capacity.
    pending_reliable_gaps: Arc<AtomicU64>,
}

impl AppCore {
    /// Starts the generic host-side observer once. This is separate from the
    /// plugin poller so future hosts can opt into either lifecycle component.
    pub fn spawn_plugin_event_observer(self: &Arc<Self>, runtime: &tokio::runtime::Handle) {
        if self
            .plugin_event_observer_started
            .swap(true, std::sync::atomic::Ordering::AcqRel)
        {
            return;
        }
        let core = Arc::clone(self);
        let shutdown = Arc::clone(&self.plugin_event_observer_shutdown);
        // Subscribe before spawning so an event published immediately after
        // this method returns cannot race the observer's first poll.
        let subscription = self.events.subscribe_domain();
        tracing::info!("plugin event observer started");
        let task = runtime.spawn(async move {
            run_observer(core, shutdown, subscription).await;
        });
        *self
            .plugin_event_observer_task
            .lock()
            .expect("plugin event observer task lock poisoned") = Some(task);
    }
}

async fn run_observer(
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

fn eligible_for_event(
    core: &AppCore,
    plugin: &DiscoveredPlugin,
    topic: &str,
    force_all_subscribers: bool,
) -> bool {
    // Declarative packages have no process call boundary and cannot implement
    // the SDK event hook. A plugin must also be both enabled and running;
    // observing events must never resurrect a stopped or crashed instance.
    plugin.manifest.runtime != tiktools_plugin_api::PluginRuntimeKind::Declarative
        && plugin.running
        && core.plugin_ready(&plugin.manifest.id)
        && core
            .capabilities
            .require_capability(&plugin.manifest, EVENTS_SUBSCRIBE)
            .is_ok()
        && (!plugin.manifest.event_subscriptions.is_empty()
            && (force_all_subscribers
                || plugin
                    .manifest
                    .event_subscriptions
                    .iter()
                    .any(|subscription| event_subscription_matches(subscription, topic))))
}

async fn run_plugin_queue(
    core: Arc<AppCore>,
    plugin_id: String,
    mut receiver: mpsc::Receiver<QueuedPluginEvent>,
    shutdown: Arc<Notify>,
    pending_reliable_gaps: Arc<AtomicU64>,
) {
    let invoker = crate::plugin_invoker::PluginInvoker::new(Arc::clone(&core.plugins));
    loop {
        let queued = tokio::select! {
            biased;
            _ = shutdown.notified() => break,
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
        let _ = deliver_event(&core, &invoker, &plugin_id, queued.envelope, queued.lossy).await;
    }
}

async fn deliver_event(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_domain_payload_does_not_expose_the_internal_enum() {
        let event = DomainEvent::PointsChanged {
            unique_id: "viewer".to_owned(),
            delta: 2.0,
            total_points: 4.0,
            level: 1,
        };
        let envelope = DomainEventEnvelope::new(
            event.topic(),
            serde_json::to_value(&event).unwrap()["data"].clone(),
        );
        assert_eq!(envelope.topic, "points.changed");
        assert_eq!(envelope.data["uniqueId"], "viewer");
        assert!(serde_json::to_value(envelope)
            .unwrap()
            .get("topic")
            .is_some());
    }
}
