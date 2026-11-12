//! Ordered ingress consumption with stale-generation fencing.
use super::*;

#[cfg(feature = "native-tiktok")]
pub(super) async fn run_ingress_dispatch(
    core: Arc<AppCore>,
    mut receiver: tokio::sync::mpsc::Receiver<IngressItem>,
    emergency: tokio_util::sync::CancellationToken,
    effects: tokio::sync::mpsc::Sender<crate::live_events::handlers::LiveEffect>,
) {
    loop {
        let item = tokio::select! {
            biased;
            () = emergency.cancelled() => break,
            received = receiver.recv() => received,
        };
        let Some(item) = item else {
            break; // Sender dropped and queue drained: clean exit.
        };
        // Paired with the admission increment: `admit_event` is the sole
        // sender in production, so every take here matches exactly one
        // admit. Direct test injection must go through admission too.
        // The two counter updates below are synchronous, so an abort can
        // only ever observe a consistent `queued`/`in_flight` pair.
        core.live_ingress.queued.fetch_sub(1, Ordering::Relaxed);
        core.live_ingress.in_flight.fetch_add(1, Ordering::Relaxed);
        // Stale-generation fence: a newer connection exists, so this data
        // item belongs to an older session and must not be credited to the
        // current creator. Stale lifecycle items are fenced as well; an
        // accepted Connected item establishes the new worker generation.
        let current = core
            .live
            .connection_snapshot()
            .generation
            .max(core.connection_sequence.load(Ordering::Acquire));
        if item.generation < current {
            if matches!(&item.event, ClientEvent::Event(_)) {
                core.events.record_live_ingress_gap(item.generation);
            }
            core.live_ingress.in_flight.fetch_sub(1, Ordering::Relaxed);
            core.live_ingress.stale.fetch_add(1, Ordering::Relaxed);
            continue;
        }
        if matches!(&item.event, ClientEvent::Connected(_)) {
            core.connection_sequence
                .store(item.generation, Ordering::Release);
        }
        let started = now_millis();
        if let ClientEvent::Event(event) = item.event {
            if let Some(effect) =
                core.prepare_live_event_with_generation(event, Some(item.generation))
            {
                core.route_live_competition_command(&effect).await;
                core.route_live_competition_score(&effect).await;
                // Reserve synchronously: storage can never hold ingress hostage.
                match effects.try_reserve() {
                    Ok(permit) => {
                        core.live_ingress
                            .effects_queued
                            .fetch_add(1, Ordering::Relaxed);
                        permit.send(effect);
                    }
                    Err(error) => {
                        core.live_ingress
                            .effects_rejected
                            .fetch_add(1, Ordering::Relaxed);
                        core.live_ingress
                            .effects_dropped
                            .fetch_add(1, Ordering::Relaxed);
                        core.record_event_gap();
                        tracing::error!(%error, msg_id = %effect.event.msg_id(), method = effect.event.method(), "LIVE persistence effects discarded; original event delivered");
                    }
                }
            }
        } else {
            core.handle_native_event(item.event).await;
        }
        core.live_ingress.in_flight.fetch_sub(1, Ordering::Relaxed);
        core.live_ingress.processed.fetch_add(1, Ordering::Relaxed);
        core.live_ingress
            .processing_samples
            .fetch_add(1, Ordering::Relaxed);
        core.live_ingress
            .processing_latency_sum_ms
            .fetch_add(now_millis().saturating_sub(started), Ordering::Relaxed);
    }
    core.live_ingress
        .advance_lifecycle(IngressLifecycle::Draining);
    // Released payloads are counted, never silent: graceful shutdown drains
    // everything (this loop runs with an empty queue), emergency shutdown
    // accounts for whatever is still buffered.
    receiver.close();
    while let Ok(item) = receiver.try_recv() {
        if matches!(item.event, ClientEvent::Event(_)) {
            core.events.record_live_ingress_gap(item.generation);
        }
        core.live_ingress.queued.fetch_sub(1, Ordering::Relaxed);
        core.live_ingress.cancelled.fetch_add(1, Ordering::Relaxed);
    }
    tracing::info!(
        lifecycle = core.live_ingress.lifecycle().as_str(),
        "live ingress worker stopped"
    );
}
