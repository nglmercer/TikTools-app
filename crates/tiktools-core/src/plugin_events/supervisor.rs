//! Event-bus subscription, worker registry, and dispatch fan-out.

use super::eligibility::eligible_for_delivery;
use super::fanout::FanoutCache;
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
    mut subscription: crate::events::PluginDomainSubscription,
) {
    let mut workers: BTreeMap<String, WorkerHandle> = BTreeMap::new();
    let mut tasks = tokio::task::JoinSet::new();
    let mut maintenance = tokio::time::interval(std::time::Duration::from_millis(250));
    maintenance.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        core.plugin_state
            .observer_worker_count
            .store(tasks.len() as u64, Ordering::Release);
        tokio::select! {
            biased;
            () = shutdown.cancelled() => break,
            result = tasks.join_next(), if !tasks.is_empty() => {
                if let Some(Err(error)) = result {
                    tracing::warn!(%error, "plugin event worker failed");
                }
                workers.retain(|_, handle| !handle.sender.is_closed());
            },
            _ = maintenance.tick() => {
                // Cold path (4 Hz): a full registry read here is fine. The
                // per-event hot path below never clones the registry.
                reconcile_workers(&core, &core.plugins.list(), &mut workers);
                workers.retain(|_, handle| !handle.sender.is_closed());
            },
            received = subscription.recv_shared() => match received {
                Ok(event) => dispatch_event(&core, event.as_ref(), &mut workers, &mut tasks, &shutdown),
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
    core.plugin_state
        .observer_worker_count
        .store(0, Ordering::Release);
}

fn dispatch_event(
    core: &Arc<AppCore>,
    event: &DomainEvent,
    workers: &mut BTreeMap<String, WorkerHandle>,
    tasks: &mut tokio::task::JoinSet<()>,
    shutdown: &CancellationToken,
) {
    // Most high-volume topics have no interested plugin. Consult the
    // existing revision-cached subscription index before cloning JSON.
    // Domain bus delivery and declared plugin subscriptions are unchanged.
    if FanoutCache::current(core)
        .matching(event.topic(), false)
        .is_empty()
    {
        return;
    }
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
    // Hot path: one revision check plus a topic-index lookup. The registry
    // is cloned only when install/uninstall/start/stop/enable/disable (or
    // a generation retirement) moved a revision — never per event.
    let cache = FanoutCache::current(core);
    let matched = cache.matching(&envelope.topic, force_all_subscribers);
    // One shared envelope: N `Arc` clones at fan-out instead of N deep
    // JSON clones. Workers clone for serialization at delivery time, off
    // the hot path and after admission.
    let shared = Arc::new(envelope);
    // The cache guard borrows the fanout mutex; matching above copied out
    // the subscriber indices, so release the guard before any live
    // per-plugin reads below (lock ordering: fanout never nests).
    let matched: Vec<(String, bool)> = matched
        .into_iter()
        .map(|index| {
            let entry = cache.entry(index);
            (entry.id.clone(), entry.static_eligible)
        })
        .collect();
    drop(cache);
    for (plugin_id, static_eligible) in matched {
        if !static_eligible {
            continue;
        }
        // Fast-changing facts stay live reads: the running generation
        // fences delivery, and activation readiness resolves races between
        // lifecycle changes and dispatch. Delivery must never resurrect a
        // stopped plugin, so workers for replaced generations are pruned
        // here, exactly as before.
        let Some(generation) = core.plugins.running_generation(&plugin_id) else {
            continue;
        };
        if !core.plugin_ready(&plugin_id) {
            continue;
        }
        if workers
            .get(&plugin_id)
            .is_some_and(|handle| handle.generation != generation)
        {
            if let Some(stale) = workers.remove(&plugin_id) {
                stale.token.cancel();
            }
        }
        let handle = workers.entry(plugin_id.clone()).or_insert_with(|| {
            let (sender, receiver) = mpsc::channel(PLUGIN_EVENT_QUEUE_CAPACITY);
            let pending_reliable_gaps = Arc::new(AtomicU64::new(0));
            let token = shutdown.child_token();
            let core = Arc::clone(core);
            core.record_plugin_worker_start(&plugin_id);
            tasks.spawn(run_plugin_queue(
                core,
                plugin_id.clone(),
                generation,
                receiver,
                token.clone(),
                Arc::clone(&pending_reliable_gaps),
            ));
            WorkerHandle {
                generation,
                sender,
                token,
                pending_reliable_gaps,
            }
        });
        let closed = match handle.sender.try_reserve() {
            Ok(permit) => {
                // Account before another Tokio worker can take the item.
                core.record_plugin_accepted(&plugin_id);
                permit.send(QueuedPluginEvent {
                    envelope: Arc::clone(&shared),
                    lossy,
                });
                false
            }
            Err(mpsc::error::TrySendError::Full(_)) => {
                if !lossy && shared.topic != "event.gap" {
                    core.record_event_gap();
                    handle.pending_reliable_gaps.fetch_add(1, Ordering::Relaxed);
                }
                core.record_plugin_drops(&plugin_id, 1);
                core.record_plugin_queue_drop(&plugin_id);
                note_dispatch_overload(core, &plugin_id, &shared.topic, lossy);
                false
            }
            Err(mpsc::error::TrySendError::Closed(_)) => true,
        };
        // The reserve result has been dropped before registry mutation.
        if closed {
            if let Some(handle) = workers.remove(&plugin_id) {
                handle.token.cancel();
            }
            tracing::debug!(plugin = %plugin_id, "plugin event queue worker is closed");
        }
    }
    // No per-event reconcile: the 250 ms maintenance tick prunes dead
    // workers, and generation fencing above handles replacement inline.
}

/// Rate-limited overload log: the per-plugin counters stay exact while
/// the log stays sparse under sustained saturation.
fn note_dispatch_overload(core: &AppCore, plugin_id: &str, topic: &str, lossy: bool) {
    let now = now_millis();
    let last = core
        .plugin_state
        .fanout_overload_warned_at
        .load(Ordering::Acquire);
    if last != 0 && now.saturating_sub(last) < 1_000 {
        return;
    }
    if core
        .plugin_state
        .fanout_overload_warned_at
        .compare_exchange(last, now, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
    {
        tracing::warn!(
            plugin = %plugin_id,
            topic = %topic,
            lossy,
            "plugin event queue is full; event dropped (further drops within 1 s counted silently)"
        );
    }
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
        let alive = core.plugins.running_generation(plugin_id) == Some(handle.generation)
            && plugins
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
