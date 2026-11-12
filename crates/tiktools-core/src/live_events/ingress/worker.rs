//! Owned dispatch/effect futures and shutdown-safe effect accounting guards.
use super::*;

/// Ordered worker: the sole consumer of the ingress queue. Sequential
/// processing preserves event order (points sequence, gift streaks,
/// session/creator attribution) without any parallel-handler reasoning.
/// Owned by `AppCore` and joined at shutdown.
///
/// The `emergency` token is *not* the admission token: the worker keeps
/// draining everything already admitted until the sender closes. It only
/// observes `emergency`, which the shutdown ladder cancels after the graceful
/// drain deadline — and even then the worker stops between items, never
/// inside one, so a running points award is never reported as cancelled.
#[cfg(feature = "native-tiktok")]
pub(crate) async fn run_ingress_worker(
    core: Arc<AppCore>,
    receiver: tokio::sync::mpsc::Receiver<IngressItem>,
    emergency: tokio_util::sync::CancellationToken,
) {
    let (sender, mut effects) =
        tokio::sync::mpsc::channel::<crate::live_events::handlers::LiveEffect>(128);
    let dispatch = run_ingress_dispatch(Arc::clone(&core), receiver, emergency.clone(), sender);
    let persist = async {
        let _pending_guard = PendingEffectsGuard(&core.live_ingress);
        loop {
            let effect = tokio::select! {
                biased;
                () = emergency.cancelled() => break,
                effect = effects.recv() => effect,
            };
            let Some(effect) = effect else {
                break;
            };
            core.live_ingress
                .effects_queued
                .fetch_sub(1, Ordering::Relaxed);
            core.live_ingress
                .effects_in_flight
                .fetch_add(1, Ordering::Relaxed);
            let mut guard = EffectGuard {
                stats: &core.live_ingress,
                complete: false,
            };
            core.apply_live_effect(effect).await;
            guard.complete = true;
            core.live_ingress
                .effects_processed
                .fetch_add(1, Ordering::Relaxed);
        }
        effects.close();
        while effects.try_recv().is_ok() {
            core.live_ingress
                .effects_queued
                .fetch_sub(1, Ordering::Relaxed);
            core.live_ingress
                .effects_dropped
                .fetch_add(1, Ordering::Relaxed);
            core.live_ingress
                .effects_cancelled
                .fetch_add(1, Ordering::Relaxed);
            core.record_event_gap();
        }
    };
    // Both futures belong to this worker. Emergency abort drops both; no
    // detached task or per-event task can outlive the ingress owner.
    tokio::join!(dispatch, persist);
    core.live_ingress
        .advance_lifecycle(IngressLifecycle::Stopped);
}

struct PendingEffectsGuard<'a>(&'a LiveIngressStats);

struct EffectGuard<'a> {
    stats: &'a LiveIngressStats,
    complete: bool,
}

impl Drop for PendingEffectsGuard<'_> {
    fn drop(&mut self) {
        let queued = self.0.effects_queued.swap(0, Ordering::Relaxed);
        if queued > 0 {
            self.0.effects_dropped.fetch_add(queued, Ordering::Relaxed);
            self.0
                .effects_cancelled
                .fetch_add(queued, Ordering::Relaxed);
            tracing::error!(
                queued,
                "queued LIVE persistence effects cancelled during shutdown"
            );
        }
    }
}

impl Drop for EffectGuard<'_> {
    fn drop(&mut self) {
        self.stats.effects_in_flight.fetch_sub(1, Ordering::Relaxed);
        if !self.complete {
            self.stats.effects_dropped.fetch_add(1, Ordering::Relaxed);
            self.stats.effects_abandoned.fetch_add(1, Ordering::Relaxed);
            tracing::error!("LIVE persistence effect abandoned during shutdown; outcome unknown");
        }
    }
}
