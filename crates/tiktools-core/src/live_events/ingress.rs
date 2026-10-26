//! Bounded staged LIVE ingestion.
//!
//! Pipeline:
//!
//! ```text
//! TikTok WebSocket (broadcast, 512)
//!   -> pump recv loop (lightweight: stamp generation, admit)
//!   -> bounded ingress queue (2,048 ordered items, single owner)
//!   -> fast ordered worker (dedup, counters, originals, UI journal)
//!   -> bounded effects queue (128, analytics/points, independent future)
//!   -> independent bounded consumers (automation slots, plugin queues, UI bus)
//! ```
//!
//! The pump never awaits processing: slow points persistence or slow
//! plugins delay the *worker*, never socket reception. The queue is
//! bounded, so sustained overload sheds predictably instead of growing
//! memory:
//!
//! - Lifecycle transitions (connect/disconnect/reconnect/error) apply
//!   backpressure (`send().await`): they are rare and must never jump the
//!   queue or vanish.
//! - All gift packets, follows, and shares apply bounded backpressure
//!   (1 s): they produce durable, money-grade side effects. If capacity
//!   still does not free up, the drop is counted as critical and reported
//!   as an event gap — never as successful delivery.
//! - Chat, likes, joins, room stats,
//!   and unknown payloads shed immediately (`try_send`) with explicit
//!   per-class counters. These are high-volume, low-value ticks; blocking
//!   the pump on them would trade explicit local counters for
//!   unpredictable upstream broadcast loss.
//!
//! Transport stamps the original generation before publication, including
//! lifecycle transitions. Admission never assigns identities from processing
//! state. The worker fences superseded lifecycle and data items alike.
//!
//! Capacity bounds event count, not decoded payload bytes. A transport-level
//! payload-size policy remains required before claiming a hard byte bound.
//!
//! ## Shutdown contract
//!
//! Admission and draining use two different cancellation scopes:
//!
//! - `AppCore::live_admission` stops *new* events from entering the queue.
//!   The pump observes it and drops the sender when it exits.
//! - `AppCore::live_ingress_stop` is the emergency stop. It is only cancelled
//!   after the graceful drain deadline expires, and it makes the worker stop
//!   *between* items — never in the middle of one.
//!
//! Graceful shutdown therefore processes every already-admitted event; only
//! an emergency shutdown releases queued payloads, and every released payload
//! is counted. `AppCore::stop_live_ingress` owns the ladder and the deadlines.
//!
//! ## Accounting invariant
//!
//! For an isolated interval with no counter resets, once the pipeline is
//! settled (`queued == 0` and `in_flight == 0`):
//!
//! ```text
//! accepted = processed + stale + cancelled + abandoned
//! ```
//!
//! While the worker holds an item, `in_flight` carries it and `queued` only
//! counts items still waiting, so at every await point (the only points where
//! the worker can be aborted):
//!
//! ```text
//! accepted = processed + stale + cancelled + abandoned + queued + in_flight
//! ```
//!
//! `stale` is *exclusive*: a fenced item is never also counted as processed,
//! because it performed no processing. `admission_cancelled` counts events
//! the pump received but never admitted (admission stopped concurrently); it
//! never entered `accepted`, so it is outside the invariant above.

use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::time::Duration;

use crate::*;

/// Bounded ingress capacity. 4× the upstream 512-event broadcast so a slow
/// worker has headroom before the pump must shed or exert backpressure.
pub(crate) const INGRESS_QUEUE_CAPACITY: usize = 2_048;
/// Bounded backpressure for critical data events before an explicit,
/// gap-reported drop.
const CRITICAL_SEND_TIMEOUT: Duration = Duration::from_secs(1);
/// Minimum interval between overload warnings; the counters stay exact
/// while the logs stay sparse.
const OVERLOAD_WARN_INTERVAL_MS: u64 = 1_000;

/// One admitted item: the event plus the connection generation stamped at
/// ingress for stale-generation fencing in the worker.
pub(crate) struct IngressItem {
    pub(crate) generation: u64,
    pub(crate) event: ClientEvent,
}

/// Explicit ingress lifecycle. Transitions are monotonic, so the pump, the
/// worker and the shutdown ladder can each advance it without regressing to
/// an earlier state:
///
/// ```text
/// Running -> StoppingAdmission -> Draining -> Stopped
/// ```
///
/// `Running` is the only state in which new events are admitted, and
/// `Stopped` is only reachable once no ingress task remains alive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub(crate) enum IngressLifecycle {
    Running = 0,
    StoppingAdmission = 1,
    Draining = 2,
    Stopped = 3,
}

impl IngressLifecycle {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::StoppingAdmission => "stoppingAdmission",
            Self::Draining => "draining",
            Self::Stopped => "stopped",
        }
    }
}

/// Low-overhead admission/processing counters. Atomics only; no per-event
/// logging, no high-cardinality labels.
pub(crate) struct LiveIngressStats {
    /// Items admitted into the ingress queue.
    pub(crate) accepted: AtomicU64,
    pub(crate) received: AtomicU64,
    pub(crate) decoded: AtomicU64,
    pub(crate) effects_queued: AtomicU64,
    pub(crate) effects_dropped: AtomicU64,
    pub(crate) effects_rejected: AtomicU64,
    pub(crate) effects_cancelled: AtomicU64,
    pub(crate) effects_abandoned: AtomicU64,
    pub(crate) effects_processed: AtomicU64,
    pub(crate) effects_in_flight: AtomicU64,
    pub(crate) ui_published: AtomicU64,
    pub(crate) ui_gifts: AtomicU64,
    pub(crate) gifts_without_artwork: AtomicU64,
    /// Items the worker finished processing (stale fences excluded).
    pub(crate) processed: AtomicU64,
    /// Data events shed immediately on a full queue.
    pub(crate) dropped_data: AtomicU64,
    /// Dropped like packets and their reported like units. Both are
    /// lower-bound loss measurements, not a way to reconstruct missed events.
    pub(crate) dropped_like_batches: AtomicU64,
    pub(crate) dropped_like_units: AtomicU64,
    /// Critical events shed after the backpressure timeout.
    pub(crate) dropped_critical: AtomicU64,
    /// Upstream broadcast lag reports observed by the pump.
    pub(crate) lagged: AtomicU64,
    /// Stale-generation data items fenced by the worker. Exclusive: a fenced
    /// item is never also counted as processed.
    pub(crate) stale: AtomicU64,
    /// Approximate queued items (incremented on admit, decremented on take).
    pub(crate) queued: AtomicU64,
    /// Items taken by the worker and not yet finished.
    pub(crate) in_flight: AtomicU64,
    /// Admitted payloads released on graceful/emergency shutdown before
    /// processing.
    pub(crate) cancelled: AtomicU64,
    /// In-flight items whose processing was cut off by an emergency abort.
    /// Their outcome is unknown and is never reported as a clean cancel.
    pub(crate) abandoned: AtomicU64,
    /// Events the pump received but never admitted because admission had
    /// already stopped. Outside the `accepted` invariant.
    pub(crate) admission_cancelled: AtomicU64,
    /// Summed worker processing latency (ms) for mean computation.
    pub(crate) processing_latency_sum_ms: AtomicU64,
    /// Finished items behind the latency sum.
    pub(crate) processing_samples: AtomicU64,
    /// Monotonic lifecycle state (see [`IngressLifecycle`]).
    lifecycle: AtomicU8,
    /// Last overload warn timestamp (millis), for rate limiting.
    last_overload_warn_at: AtomicU64,
}

impl Default for LiveIngressStats {
    fn default() -> Self {
        Self {
            accepted: AtomicU64::new(0),
            received: AtomicU64::new(0),
            decoded: AtomicU64::new(0),
            effects_queued: AtomicU64::new(0),
            effects_dropped: AtomicU64::new(0),
            effects_rejected: AtomicU64::new(0),
            effects_cancelled: AtomicU64::new(0),
            effects_abandoned: AtomicU64::new(0),
            effects_processed: AtomicU64::new(0),
            effects_in_flight: AtomicU64::new(0),
            ui_published: AtomicU64::new(0),
            ui_gifts: AtomicU64::new(0),
            gifts_without_artwork: AtomicU64::new(0),
            processed: AtomicU64::new(0),
            dropped_data: AtomicU64::new(0),
            dropped_like_batches: AtomicU64::new(0),
            dropped_like_units: AtomicU64::new(0),
            dropped_critical: AtomicU64::new(0),
            lagged: AtomicU64::new(0),
            stale: AtomicU64::new(0),
            queued: AtomicU64::new(0),
            in_flight: AtomicU64::new(0),
            cancelled: AtomicU64::new(0),
            abandoned: AtomicU64::new(0),
            admission_cancelled: AtomicU64::new(0),
            processing_latency_sum_ms: AtomicU64::new(0),
            processing_samples: AtomicU64::new(0),
            lifecycle: AtomicU8::new(IngressLifecycle::Running as u8),
            last_overload_warn_at: AtomicU64::new(0),
        }
    }
}

impl LiveIngressStats {
    /// Advances the lifecycle monotonically; a stale writer can never move
    /// the state backwards.
    pub(crate) fn advance_lifecycle(&self, next: IngressLifecycle) {
        self.lifecycle.fetch_max(next as u8, Ordering::AcqRel);
    }

    pub(crate) fn lifecycle(&self) -> IngressLifecycle {
        match self.lifecycle.load(Ordering::Acquire) {
            0 => IngressLifecycle::Running,
            1 => IngressLifecycle::StoppingAdmission,
            2 => IngressLifecycle::Draining,
            _ => IngressLifecycle::Stopped,
        }
    }

    /// Items that were admitted but are neither finished, released, nor
    /// still tracked: always `queued + in_flight` while the pipeline is
    /// consistent, and `0` once it settles.
    pub(crate) fn outstanding(&self) -> i64 {
        let accounted = self
            .processed
            .load(Ordering::Relaxed)
            .saturating_add(self.stale.load(Ordering::Relaxed))
            .saturating_add(self.cancelled.load(Ordering::Relaxed))
            .saturating_add(self.abandoned.load(Ordering::Relaxed));
        self.accepted.load(Ordering::Relaxed) as i64 - accounted as i64
    }

    /// Emergency accounting for work the worker can no longer reach. Called
    /// only after the worker task is confirmed dead: everything still
    /// outstanding becomes `cancelled` (queued payloads) or `abandoned`
    /// (the item that was mid-processing). Derived from `accepted` so an
    /// abort between two counter updates can never leave an item uncounted.
    pub(crate) fn abandon_outstanding(&self) {
        let in_flight = self.in_flight.swap(0, Ordering::AcqRel);
        let missing = self.outstanding().max(0) as u64;
        let queued = missing.saturating_sub(in_flight);
        self.cancelled.fetch_add(queued, Ordering::Relaxed);
        self.abandoned
            .fetch_add(missing - queued, Ordering::Relaxed);
        self.queued.store(0, Ordering::Release);
    }

    pub(crate) fn diagnostics(&self) -> Value {
        let samples = self.processing_samples.load(Ordering::Relaxed);
        json!({
            "effects": { "capacity": 128, "queued": self.effects_queued.load(Ordering::Relaxed), "dropped": self.effects_dropped.load(Ordering::Relaxed), "rejected": self.effects_rejected.load(Ordering::Relaxed), "cancelled": self.effects_cancelled.load(Ordering::Relaxed), "abandoned": self.effects_abandoned.load(Ordering::Relaxed), "processed": self.effects_processed.load(Ordering::Relaxed), "inFlight": self.effects_in_flight.load(Ordering::Relaxed) },
            "capacity": INGRESS_QUEUE_CAPACITY,
            "lifecycle": self.lifecycle().as_str(),
            "queued": self.queued.load(Ordering::Relaxed),
            "inFlight": self.in_flight.load(Ordering::Relaxed),
            "cancelled": self.cancelled.load(Ordering::Relaxed),
            "abandoned": self.abandoned.load(Ordering::Relaxed),
            "admissionCancelled": self.admission_cancelled.load(Ordering::Relaxed),
            "received": self.received.load(Ordering::Relaxed),
            "decoded": self.decoded.load(Ordering::Relaxed),
            "accepted": self.accepted.load(Ordering::Relaxed),
            "uiPublished": self.ui_published.load(Ordering::Relaxed),
            "uiGifts": self.ui_gifts.load(Ordering::Relaxed),
            "giftsWithoutArtwork": self.gifts_without_artwork.load(Ordering::Relaxed),
            "processed": self.processed.load(Ordering::Relaxed),
            "droppedData": self.dropped_data.load(Ordering::Relaxed),
            "droppedLikeBatches": self.dropped_like_batches.load(Ordering::Relaxed),
            "droppedLikeUnits": self.dropped_like_units.load(Ordering::Relaxed),
            "droppedCritical": self.dropped_critical.load(Ordering::Relaxed),
            "broadcastLagged": self.lagged.load(Ordering::Relaxed),
            "staleFenced": self.stale.load(Ordering::Relaxed),
            "outstanding": self.outstanding(),
            "processingSamples": samples,
            "meanProcessingLatencyMs": if samples == 0 {
                Value::Null
            } else {
                json!(self.processing_latency_sum_ms.load(Ordering::Relaxed) as f64 / samples as f64)
            },
        })
    }

    fn note_overload(&self, message: &str) {
        let now = now_millis();
        let last = self.last_overload_warn_at.load(Ordering::Acquire);
        if last != 0 && now.saturating_sub(last) < OVERLOAD_WARN_INTERVAL_MS {
            return;
        }
        if self
            .last_overload_warn_at
            .compare_exchange(last, now, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            tracing::warn!(
                message,
                "live ingress saturated; shedding with explicit counters"
            );
        }
    }
}

/// Delivery semantics for one data event under overload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DataClass {
    /// Durable, money-grade side effects: bounded backpressure, then an
    /// explicit gap-reported drop. Never silently discarded.
    Critical,
    /// High-volume ticks: shed immediately with counters.
    Sheddable,
}

#[cfg(feature = "native-tiktok")]
pub(crate) fn classify_data_event(event: &NativeLiveEvent) -> DataClass {
    use tiktools_tiktok::events::CanonicalLiveEvent;
    match &event.base {
        // Every gift packet can contribute a monetary increment. Protect
        // progress as well as finals with the existing bounded backpressure.
        CanonicalLiveEvent::Gift(_) => DataClass::Critical,
        CanonicalLiveEvent::Social(social) => {
            match tiktools_tiktok::events::classify_social_action(social.action) {
                tiktools_tiktok::events::SocialKind::Follow
                | tiktools_tiktok::events::SocialKind::Share => DataClass::Critical,
                // An unconfirmed social action has no durable side effect yet,
                // so it sheds under overload like the other high-volume ticks.
                tiktools_tiktok::events::SocialKind::Other
                | tiktools_tiktok::events::SocialKind::Unknown => DataClass::Sheddable,
            }
        }
        // Chat, likes, joins, room stats, and unknown payloads are
        // high-volume ticks: shed with counters, never block the pump.
        _ => DataClass::Sheddable,
    }
}

/// Admits one broadcast event into the bounded ingress queue without
/// awaiting processing. Shed decisions are counted, never propagated.
#[cfg(test)]
pub(crate) async fn admit_event(
    core: &AppCore,
    sender: &tokio::sync::mpsc::Sender<IngressItem>,
    event: ClientEvent,
) {
    admit_stamped_event(
        core,
        sender,
        tiktools_tiktok::StampedClientEvent {
            generation: core.connection_sequence.load(Ordering::Acquire),
            event,
        },
    )
    .await;
}

/// Admits one stamped event into the bounded ingress queue without awaiting
/// processing. Admission observes `AppCore::live_admission`, which stops *new*
/// events only; already-admitted items stay owned by the queue and drain
/// normally. An event released here is counted in `admission_cancelled` so it
/// can never vanish between receive and admission.
pub(crate) async fn admit_stamped_event(
    core: &AppCore,
    sender: &tokio::sync::mpsc::Sender<IngressItem>,
    stamped: tiktools_tiktok::StampedClientEvent,
) {
    let tiktools_tiktok::StampedClientEvent { generation, event } = stamped;
    let stats = &core.live_ingress;
    stats.received.fetch_add(1, Ordering::Relaxed);
    if matches!(&event, ClientEvent::Event(_)) {
        stats.decoded.fetch_add(1, Ordering::Relaxed);
    }
    if core.live_admission.is_cancelled() {
        stats.admission_cancelled.fetch_add(1, Ordering::Relaxed);
        return;
    }
    match event {
        // Lifecycle transitions are rare and ordering-sensitive: wait for
        // capacity rather than drop or reorder them — unless admission has
        // stopped, in which case the wait is abandoned explicitly.
        lifecycle @ (ClientEvent::Connected(_)
        | ClientEvent::Reconnecting { .. }
        | ClientEvent::Disconnected { .. }
        | ClientEvent::Error { .. }) => {
            tokio::select! {
                biased;
                () = core.live_admission.cancelled() => {
                    stats.admission_cancelled.fetch_add(1, Ordering::Relaxed);
                }
                reserved = sender.reserve() => match reserved {
                    Ok(permit) => publish_reserved(
                        stats,
                        permit,
                        IngressItem {
                            generation,
                            event: lifecycle,
                        },
                    ),
                    Err(_) => tracing::debug!("live ingress worker is gone; pump stops"),
                }
            }
        }
        ClientEvent::Event(data) => {
            let item = IngressItem {
                generation,
                event: ClientEvent::Event(data),
            };
            // Fast path: capacity available.
            match sender.try_reserve() {
                Ok(permit) => publish_reserved(stats, permit, item),
                Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                    tracing::debug!("live ingress worker is gone; pump stops");
                }
                Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                    admit_full(core, sender, item).await;
                }
            }
        }
    }
}

/// Account before publishing: the receiver may run on another Tokio worker
/// immediately after `send`. A reserved slot has no payload and cancellation
/// before this synchronous section simply releases the permit.
fn publish_reserved(
    stats: &LiveIngressStats,
    permit: tokio::sync::mpsc::Permit<'_, IngressItem>,
    item: IngressItem,
) {
    stats.accepted.fetch_add(1, Ordering::Relaxed);
    stats.queued.fetch_add(1, Ordering::Relaxed);
    permit.send(item);
}

#[cfg(feature = "native-tiktok")]
async fn admit_full(
    core: &AppCore,
    sender: &tokio::sync::mpsc::Sender<IngressItem>,
    item: IngressItem,
) {
    let stats = &core.live_ingress;
    let critical = match &item.event {
        ClientEvent::Event(data) => classify_data_event(data) == DataClass::Critical,
        _ => false,
    };
    if !critical {
        if let ClientEvent::Event(data) = &item.event {
            if let tiktools_tiktok::events::CanonicalLiveEvent::Like(like) = &data.base {
                stats.dropped_like_batches.fetch_add(1, Ordering::Relaxed);
                stats.dropped_like_units.fetch_add(like.count, Ordering::Relaxed);
                // Like goals can no longer claim an exact observed count:
                // distinguish this gap from a rule whose filter did not match.
                core.record_event_gap();
            }
        }
        stats.dropped_data.fetch_add(1, Ordering::Relaxed);
        stats.note_overload("shedding live data event on saturated ingress");
        return;
    }
    // Critical data: bounded backpressure, then an explicit, gap-reported
    // drop. Delivery beyond this point cannot be guaranteed, and the
    // counters say exactly that. Admission stop abandons the wait too, so a
    // shutdown can never block behind a saturated queue.
    let reserved = tokio::select! {
        biased;
        () = core.live_admission.cancelled() => {
            stats.admission_cancelled.fetch_add(1, Ordering::Relaxed);
            return;
        }
        reserved = tokio::time::timeout(CRITICAL_SEND_TIMEOUT, sender.reserve()) => reserved,
    };
    match reserved {
        Ok(Ok(permit)) => publish_reserved(stats, permit, item),
        Ok(Err(_)) => {
            tracing::debug!("live ingress worker is gone; pump stops");
        }
        Err(_) => {
            stats.dropped_critical.fetch_add(1, Ordering::Relaxed);
            core.record_event_gap();
            stats.note_overload("dropping critical live event after backpressure timeout");
            tracing::warn!(
                "critical live event shed after 1 s of ingress backpressure; gap recorded"
            );
        }
    }
}

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
    let (sender, mut effects) = tokio::sync::mpsc::channel::<super::handlers::LiveEffect>(128);
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
struct EffectGuard<'a> {
    stats: &'a LiveIngressStats,
    complete: bool,
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
#[cfg(feature = "native-tiktok")]
async fn run_ingress_dispatch(
    core: Arc<AppCore>,
    mut receiver: tokio::sync::mpsc::Receiver<IngressItem>,
    emergency: tokio_util::sync::CancellationToken,
    effects: tokio::sync::mpsc::Sender<super::handlers::LiveEffect>,
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
        // current creator. Lifecycle items always pass; they establish the
        // generation this fence compares against.
        let current = core
            .live
            .connection_snapshot()
            .generation
            .max(core.connection_sequence.load(Ordering::Acquire));
        if item.generation < current {
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
            if let Some(effect) = core.prepare_live_event(event) {
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
    while receiver.try_recv().is_ok() {
        core.live_ingress.queued.fetch_sub(1, Ordering::Relaxed);
        core.live_ingress.cancelled.fetch_add(1, Ordering::Relaxed);
    }
    tracing::info!(
        lifecycle = core.live_ingress.lifecycle().as_str(),
        "live ingress worker stopped"
    );
}

#[cfg(all(test, feature = "native-tiktok"))]
mod tests {
    use super::*;
    use tiktools_tiktok::events::{
        CanonicalLiveEvent, ChatEvent, EventMetadata, EventUser, GiftEnrichment, GiftEvent,
        LikeEvent, RoomUserEvent, SocialEvent,
    };

    struct NoopEmitter;
    impl HostEmitter for NoopEmitter {
        fn emit(&self, _: HostMessage) {}
    }

    fn user() -> EventUser {
        EventUser {
            id: 7,
            unique_id: "spectator".to_owned(),
            nickname: "Spectator".to_owned(),
            sec_uid: "sec-spectator".to_owned(),
            avatar_url: None,
        }
    }

    fn wrap(base: CanonicalLiveEvent, gift: Option<GiftEnrichment>) -> NativeLiveEvent {
        NativeLiveEvent {
            base,
            metadata: EventMetadata {
                method: "WebcastTest".to_owned(),
                msg_id: 11,
                is_history: false,
            },
            gift,
        }
    }

    fn chat() -> NativeLiveEvent {
        wrap(
            CanonicalLiveEvent::Chat(ChatEvent {
                user: user(),
                comment: "hello".to_owned(),
                ..Default::default()
            }),
            None,
        )
    }

    fn gift_final() -> NativeLiveEvent {
        wrap(
            CanonicalLiveEvent::Gift(GiftEvent {
                user: user(),
                gift_id: 1,
                gift_name: "Rose".to_owned(),
                gift_image_url: None,
                diamond_count: 1,
                repeat_count: 1,
                combo_count: 1,
                group_id: 0,
                repeat_end: true,
            }),
            Some(GiftEnrichment {
                name: "Rose".to_owned(),
                diamond_count: 1,
                streakable: true,
                icon_url: None,
            }),
        )
    }

    fn gift_streak_mid() -> NativeLiveEvent {
        wrap(
            CanonicalLiveEvent::Gift(GiftEvent {
                user: user(),
                gift_id: 1,
                gift_name: "Rose".to_owned(),
                gift_image_url: None,
                diamond_count: 1,
                repeat_count: 3,
                combo_count: 3,
                group_id: 9,
                repeat_end: false,
            }),
            Some(GiftEnrichment {
                name: "Rose".to_owned(),
                diamond_count: 1,
                streakable: true,
                icon_url: None,
            }),
        )
    }

    fn follow() -> NativeLiveEvent {
        wrap(
            CanonicalLiveEvent::Social(SocialEvent {
                user: user(),
                action: 1,
                follow_count: 2,
                share_count: 0,
                ..Default::default()
            }),
            None,
        )
    }

    fn room_stats() -> NativeLiveEvent {
        wrap(
            CanonicalLiveEvent::RoomUser(RoomUserEvent {
                total: 10,
                total_user: 12,
                popularity: 0,
                anonymous: 0,
                top_viewers: Vec::new(),
                ranked_viewers: Vec::new(),
            }),
            None,
        )
    }

    #[test]
    fn critical_events_cover_gifts_follows_and_shares() {
        assert_eq!(classify_data_event(&gift_final()), DataClass::Critical);
        assert_eq!(classify_data_event(&follow()), DataClass::Critical);
        // Chat, likes, and room stats shed under overload; gift progress is protected.
        assert_eq!(classify_data_event(&gift_streak_mid()), DataClass::Critical);
        assert_eq!(classify_data_event(&chat()), DataClass::Sheddable);
        assert_eq!(
            classify_data_event(&wrap(
                CanonicalLiveEvent::Like(LikeEvent {
                    user: user(),
                    count: 5,
                    total: 5,
                }),
                None,
            )),
            DataClass::Sheddable
        );
        assert_eq!(classify_data_event(&room_stats()), DataClass::Sheddable);
    }

    #[tokio::test]
    async fn full_queue_sheds_data_without_blocking_admission() {
        let core = Arc::new(AppCore::new(Arc::new(NoopEmitter)));
        // Capacity 1 held by an unreceived item: every further data event
        // meets a full queue.
        let (sender, _held) = tokio::sync::mpsc::channel::<IngressItem>(1);
        sender
            .send(IngressItem {
                generation: 0,
                event: ClientEvent::Event(room_stats()),
            })
            .await
            .unwrap();
        let before = core.live_ingress.dropped_data.load(Ordering::Relaxed);
        tokio::time::timeout(
            std::time::Duration::from_secs(3),
            admit_event(&core, &sender, ClientEvent::Event(chat())),
        )
        .await
        .expect("sheddable admission never blocks");
        assert_eq!(
            core.live_ingress.dropped_data.load(Ordering::Relaxed),
            before + 1,
            "shed data must be counted, never silently lost"
        );
        // Like drops are still non-blocking, but goal reliability degrades.
        // The packet count is the number of likes lost (not just one).
        let like_gaps_before = core.event_gap_stats().0;
        admit_event(
            &core,
            &sender,
            ClientEvent::Event(wrap(
                CanonicalLiveEvent::Like(LikeEvent {
                    user: user(),
                    count: 15,
                    total: 1_000,
                }),
                None,
            )),
        )
        .await;
        assert_eq!(core.live_ingress.dropped_like_batches.load(Ordering::Relaxed), 1);
        assert_eq!(core.live_ingress.dropped_like_units.load(Ordering::Relaxed), 15);
        assert_eq!(core.event_gap_stats().0, like_gaps_before + 1);
        assert_eq!(core.live_ingress.diagnostics()["droppedLikeUnits"], 15);
        // Critical data on a full queue applies bounded backpressure and,
        // when capacity cannot free up, reports a gap instead of success.
        let gaps_before = core.event_gap_stats().0;
        let critical_before = core.live_ingress.dropped_critical.load(Ordering::Relaxed);
        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            admit_event(&core, &sender, ClientEvent::Event(gift_final())),
        )
        .await
        .expect("critical admission is time-bounded");
        assert_eq!(
            core.live_ingress.dropped_critical.load(Ordering::Relaxed),
            critical_before + 1
        );
        assert_eq!(core.event_gap_stats().0, gaps_before + 1);
    }

    /// Sustained overload far past queue capacity: admission stays
    /// responsive (sheddable events never block), accounting is exact
    /// (every sent item is accepted or counted shed), and memory stays
    /// bounded (the queue never exceeds its capacity).
    #[tokio::test]
    async fn sustained_overload_sheds_with_exact_accounting() {
        let core = Arc::new(AppCore::new(Arc::new(NoopEmitter)));
        let (sender, _held) = tokio::sync::mpsc::channel::<IngressItem>(INGRESS_QUEUE_CAPACITY);
        // Occupy the queue the same way a stalled worker would: the held
        // receiver never drains, so every further admit meets pressure.
        for _ in 0..INGRESS_QUEUE_CAPACITY {
            sender
                .send(IngressItem {
                    generation: 0,
                    event: ClientEvent::Event(room_stats()),
                })
                .await
                .unwrap();
        }
        let sent = 10_000;
        let started = now_millis();
        for id in 0..sent {
            let event = if id % 2 == 0 { chat() } else { room_stats() };
            tokio::time::timeout(
                std::time::Duration::from_secs(5),
                admit_event(&core, &sender, ClientEvent::Event(event)),
            )
            .await
            .expect("sheddable admission never blocks");
        }
        let elapsed = now_millis().saturating_sub(started);
        let accepted = core.live_ingress.accepted.load(Ordering::Relaxed);
        let dropped = core.live_ingress.dropped_data.load(Ordering::Relaxed);
        // The prefill injected pressure directly (never drained, never
        // counted); every item sent through admission is either admitted
        // or counted shed — none vanish.
        assert_eq!(
            accepted + dropped,
            sent as u64,
            "every admitted item is accepted or counted shed"
        );
        assert_eq!(accepted, 0, "a saturated queue admits nothing further");
        assert!(
            core.live_ingress.queued.load(Ordering::Relaxed) <= INGRESS_QUEUE_CAPACITY as u64,
            "the queue never exceeds its bound"
        );
        println!(
            "  sustained overload: {sent} shed-path admits in {elapsed} ms, dropped={dropped}"
        );
    }

    /// Shutdown while the queue is saturated: the worker drains every
    /// accepted item (stale ones fence fast) and joins — no retained
    /// payloads, no detached task.
    #[tokio::test]
    async fn saturated_queue_drains_and_joins_on_shutdown() {
        let core = Arc::new(AppCore::new(Arc::new(NoopEmitter)));
        let (sender, receiver) = tokio::sync::mpsc::channel::<IngressItem>(64);
        // Admit through the real path (the sole production sender), so the
        // queue gauge pairs exactly with worker takes. Stamps read
        // generation 0 here ...
        for _ in 0..64 {
            admit_event(&core, &sender, ClientEvent::Event(chat())).await;
        }
        // ... and a newer connection arrives before the drain: every item
        // fences on the fast path, so the drain is quick and side-effect
        // free.
        core.connection_sequence.store(9, Ordering::Release);
        assert_eq!(core.live_ingress.queued.load(Ordering::Relaxed), 64);
        drop(sender);
        let shutdown = tokio_util::sync::CancellationToken::new();
        tokio::time::timeout(
            std::time::Duration::from_secs(10),
            run_ingress_worker(Arc::clone(&core), receiver, shutdown),
        )
        .await
        .expect("saturated worker drains and exits");
        assert_eq!(core.live_ingress.stale.load(Ordering::Relaxed), 64);
        // Stale is exclusive: a fenced item performed no processing.
        assert_eq!(core.live_ingress.processed.load(Ordering::Relaxed), 0);
        assert_eq!(core.live_ingress.queued.load(Ordering::Relaxed), 0);
        assert_eq!(core.live_ingress.in_flight.load(Ordering::Relaxed), 0);
        assert_eq!(core.live_ingress.outstanding(), 0);
    }

    #[tokio::test]
    async fn worker_fences_stale_generations_and_joins() {
        let core = Arc::new(AppCore::new(Arc::new(NoopEmitter)));
        // A newer connection exists: the queued item belongs to the old one.
        core.connection_sequence.store(4, Ordering::Release);
        let (sender, receiver) = tokio::sync::mpsc::channel::<IngressItem>(8);
        // Admission is the sole production entry point, so the item is
        // stamped and accounted there instead of being pushed into the queue
        // behind the counters' backs.
        admit_stamped_event(
            &core,
            &sender,
            tiktools_tiktok::StampedClientEvent {
                generation: 2,
                event: ClientEvent::Event(chat()),
            },
        )
        .await;
        assert_eq!(core.live_ingress.accepted.load(Ordering::Relaxed), 1);
        assert_eq!(core.live_ingress.queued.load(Ordering::Relaxed), 1);
        let shutdown = tokio_util::sync::CancellationToken::new();
        let worker = tokio::spawn(run_ingress_worker(
            Arc::clone(&core),
            receiver,
            shutdown.clone(),
        ));
        // Let the worker take the item, then stop it cleanly.
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        shutdown.cancel();
        tokio::time::timeout(std::time::Duration::from_secs(3), worker)
            .await
            .expect("worker joins")
            .expect("worker task succeeds");
        assert_eq!(core.live_ingress.stale.load(Ordering::Relaxed), 1);
        assert_eq!(core.live_ingress.processed.load(Ordering::Relaxed), 0);
        assert_eq!(core.live_ingress.outstanding(), 0);
        // The fenced chat never reached points: no side effects, no viewer.
        assert!(core.points.viewer("spectator").is_none());
        let diagnostics = core.live_ingress.diagnostics();
        assert_eq!(diagnostics["capacity"], INGRESS_QUEUE_CAPACITY);
        assert_eq!(diagnostics["staleFenced"], 1);
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn concurrent_consumption_never_underflows_admission_gauge() {
        let core = Arc::new(AppCore::new(Arc::new(NoopEmitter)));
        let (sender, mut receiver) = tokio::sync::mpsc::channel::<IngressItem>(64);
        let consumer_core = Arc::clone(&core);
        let consumer = tokio::spawn(async move {
            let mut taken = 0;
            while receiver.recv().await.is_some() {
                let before = consumer_core
                    .live_ingress
                    .queued
                    .fetch_sub(1, Ordering::Relaxed);
                assert!((1..=65).contains(&before), "invalid queue depth: {before}");
                taken += 1;
            }
            taken
        });
        for _ in 0..100_000 {
            // Lifecycle admission always waits, so this exercises 100,000
            // genuine publications and concurrent takes without shedding.
            admit_event(
                &core,
                &sender,
                ClientEvent::Reconnecting {
                    attempt: 1,
                    delay_ms: 0,
                },
            )
            .await;
        }
        drop(sender);
        assert_eq!(consumer.await.unwrap(), 100_000);
        assert_eq!(core.live_ingress.queued.load(Ordering::Relaxed), 0);
        assert_eq!(core.live_ingress.accepted.load(Ordering::Relaxed), 100_000);
    }

    #[tokio::test]
    async fn cancelled_full_queue_releases_payloads_and_accounting() {
        let core = Arc::new(AppCore::new(Arc::new(NoopEmitter)));
        let (sender, receiver) = tokio::sync::mpsc::channel(64);
        for _ in 0..64 {
            admit_event(&core, &sender, ClientEvent::Event(chat())).await;
        }
        let shutdown = tokio_util::sync::CancellationToken::new();
        shutdown.cancel();
        run_ingress_worker(Arc::clone(&core), receiver, shutdown).await;
        assert_eq!(core.live_ingress.queued.load(Ordering::Relaxed), 0);
        assert_eq!(core.live_ingress.cancelled.load(Ordering::Relaxed), 64);
        assert_eq!(core.live_ingress.processed.load(Ordering::Relaxed), 0);
        assert!(sender.is_closed());
        assert!(core.points.viewer("spectator").is_none());
    }
    #[tokio::test]
    async fn admission_preserves_original_generation_before_connected_processing() {
        let core = Arc::new(AppCore::new(Arc::new(NoopEmitter)));
        let (sender, mut receiver) = tokio::sync::mpsc::channel(8);
        for event in [
            ClientEvent::Reconnecting {
                attempt: 1,
                delay_ms: 0,
            },
            ClientEvent::Event(chat()),
            ClientEvent::Disconnected {
                reason: "old".into(),
            },
        ] {
            admit_stamped_event(
                &core,
                &sender,
                tiktools_tiktok::StampedClientEvent {
                    generation: 7,
                    event,
                },
            )
            .await;
        }
        core.connection_sequence.store(9, Ordering::Release);
        for _ in 0..3 {
            let item = receiver.recv().await.unwrap();
            assert_eq!(item.generation, 7);
            core.live_ingress.queued.fetch_sub(1, Ordering::Relaxed);
        }
    }

    #[tokio::test]
    async fn superseded_disconnect_cannot_clear_new_session() {
        let core = Arc::new(AppCore::new(Arc::new(NoopEmitter)));
        core.connection_sequence.store(9, Ordering::Release);
        *recover_rwlock_write(&core.connection_context, "connection context") = Some(LiveContext {
            unique_id: "new".into(),
            room_id: "9".into(),
            connection_id: "connection-9".into(),
        });
        let (sender, receiver) = tokio::sync::mpsc::channel(8);
        admit_stamped_event(
            &core,
            &sender,
            tiktools_tiktok::StampedClientEvent {
                generation: 2,
                event: ClientEvent::Disconnected {
                    reason: "stale".into(),
                },
            },
        )
        .await;
        drop(sender);
        run_ingress_worker(
            Arc::clone(&core),
            receiver,
            tokio_util::sync::CancellationToken::new(),
        )
        .await;
        assert_eq!(
            recover_rwlock_read(&core.connection_context, "connection context")
                .as_ref()
                .unwrap()
                .unique_id,
            "new"
        );
        assert_eq!(core.live_ingress.stale.load(Ordering::Relaxed), 1);
    }

    /// Graceful shutdown contract: with the sender gone (pump stopped) the
    /// worker processes every accepted event before it exits. Nothing is
    /// released, nothing is lost, and the accounting closes exactly.
    #[tokio::test]
    async fn graceful_shutdown_drains_thousands_of_accepted_events() {
        let core = Arc::new(AppCore::new(Arc::new(NoopEmitter)));
        let (sender, receiver) = tokio::sync::mpsc::channel::<IngressItem>(INGRESS_QUEUE_CAPACITY);
        for _ in 0..INGRESS_QUEUE_CAPACITY {
            admit_event(&core, &sender, ClientEvent::Event(chat())).await;
        }
        assert_eq!(
            core.live_ingress.accepted.load(Ordering::Relaxed),
            INGRESS_QUEUE_CAPACITY as u64
        );
        assert_eq!(
            core.live_ingress.queued.load(Ordering::Relaxed),
            INGRESS_QUEUE_CAPACITY as u64
        );
        // A newer connection fences every queued item, so the drain is
        // deterministic and side-effect free but still exercises the full
        // "thousands of accepted events" path.
        core.connection_sequence.store(9, Ordering::Release);
        drop(sender);
        let emergency = tokio_util::sync::CancellationToken::new();
        tokio::time::timeout(
            std::time::Duration::from_secs(10),
            run_ingress_worker(Arc::clone(&core), receiver, emergency),
        )
        .await
        .expect("graceful drain must complete");
        assert_eq!(
            core.live_ingress.stale.load(Ordering::Relaxed),
            INGRESS_QUEUE_CAPACITY as u64
        );
        assert_eq!(core.live_ingress.processed.load(Ordering::Relaxed), 0);
        assert_eq!(core.live_ingress.cancelled.load(Ordering::Relaxed), 0);
        assert_eq!(core.live_ingress.queued.load(Ordering::Relaxed), 0);
        assert_eq!(core.live_ingress.in_flight.load(Ordering::Relaxed), 0);
        assert_eq!(core.live_ingress.outstanding(), 0);
        assert_eq!(
            core.live_ingress.lifecycle(),
            IngressLifecycle::Stopped,
            "worker exit must reach the terminal lifecycle state"
        );
    }

    /// Emergency stop (drain deadline already expired): every payload still
    /// buffered is released *and counted*, so the invariant still holds.
    #[tokio::test]
    async fn emergency_stop_accounts_every_released_queue_payload() {
        let core = Arc::new(AppCore::new(Arc::new(NoopEmitter)));
        let (sender, receiver) = tokio::sync::mpsc::channel::<IngressItem>(64);
        for _ in 0..64 {
            admit_event(&core, &sender, ClientEvent::Event(chat())).await;
        }
        let emergency = tokio_util::sync::CancellationToken::new();
        emergency.cancel();
        run_ingress_worker(Arc::clone(&core), receiver, emergency).await;
        assert_eq!(core.live_ingress.cancelled.load(Ordering::Relaxed), 64);
        assert_eq!(core.live_ingress.abandoned.load(Ordering::Relaxed), 0);
        assert_eq!(core.live_ingress.processed.load(Ordering::Relaxed), 0);
        assert_eq!(core.live_ingress.queued.load(Ordering::Relaxed), 0);
        assert_eq!(core.live_ingress.outstanding(), 0);
        assert_eq!(core.live_ingress.lifecycle(), IngressLifecycle::Stopped);
    }

    #[tokio::test]
    async fn shutdown_with_empty_queue_exits_cleanly() {
        let core = Arc::new(AppCore::new(Arc::new(NoopEmitter)));
        let (sender, receiver) = tokio::sync::mpsc::channel::<IngressItem>(8);
        drop(sender);
        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            run_ingress_worker(
                Arc::clone(&core),
                receiver,
                tokio_util::sync::CancellationToken::new(),
            ),
        )
        .await
        .expect("empty shutdown must not hang");
        assert_eq!(core.live_ingress.accepted.load(Ordering::Relaxed), 0);
        assert_eq!(core.live_ingress.cancelled.load(Ordering::Relaxed), 0);
        assert_eq!(core.live_ingress.outstanding(), 0);
        assert_eq!(core.live_ingress.lifecycle(), IngressLifecycle::Stopped);
    }

    /// Cancellation during queue reservation: a lifecycle event that would
    /// otherwise wait forever on a full queue is released when admission
    /// stops, and is counted as never admitted — not silently swallowed.
    #[tokio::test]
    async fn admission_stop_releases_event_waiting_on_queue_reservation() {
        let core = Arc::new(AppCore::new(Arc::new(NoopEmitter)));
        let (sender, _held) = tokio::sync::mpsc::channel::<IngressItem>(1);
        sender
            .send(IngressItem {
                generation: 0,
                event: ClientEvent::Event(room_stats()),
            })
            .await
            .unwrap();
        let accepted_before = core.live_ingress.accepted.load(Ordering::Relaxed);
        core.live_admission.cancel();
        tokio::time::timeout(
            std::time::Duration::from_secs(3),
            admit_event(
                &core,
                &sender,
                ClientEvent::Reconnecting {
                    attempt: 1,
                    delay_ms: 0,
                },
            ),
        )
        .await
        .expect("admission stop must release a blocked reservation");
        assert_eq!(
            core.live_ingress
                .admission_cancelled
                .load(Ordering::Relaxed),
            1
        );
        assert_eq!(
            core.live_ingress.accepted.load(Ordering::Relaxed),
            accepted_before,
            "a released reservation must never be counted as accepted"
        );
    }

    /// Shutdown racing an active producer: every attempt is either admitted
    /// or explicitly counted, and the consumer settles the invariant.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn admission_stop_while_producer_admits_accounts_every_event() {
        let core = Arc::new(AppCore::new(Arc::new(NoopEmitter)));
        let (sender, mut receiver) = tokio::sync::mpsc::channel::<IngressItem>(64);
        let consumer_core = Arc::clone(&core);
        let consumer = tokio::spawn(async move {
            let mut taken = 0u64;
            while receiver.recv().await.is_some() {
                consumer_core
                    .live_ingress
                    .queued
                    .fetch_sub(1, Ordering::Relaxed);
                consumer_core
                    .live_ingress
                    .in_flight
                    .fetch_add(1, Ordering::Relaxed);
                consumer_core
                    .live_ingress
                    .in_flight
                    .fetch_sub(1, Ordering::Relaxed);
                consumer_core
                    .live_ingress
                    .processed
                    .fetch_add(1, Ordering::Relaxed);
                taken += 1;
            }
            taken
        });
        let producer_core = Arc::clone(&core);
        let producer = tokio::spawn(async move {
            let mut attempted = 0u64;
            for _ in 0..10_000 {
                attempted += 1;
                admit_event(
                    &producer_core,
                    &sender,
                    ClientEvent::Reconnecting {
                        attempt: 1,
                        delay_ms: 0,
                    },
                )
                .await;
            }
            attempted
        });
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        core.live_admission.cancel();
        let attempted = tokio::time::timeout(std::time::Duration::from_secs(30), producer)
            .await
            .expect("producer never deadlocks on a stopped admission")
            .expect("producer task");
        let taken = tokio::time::timeout(std::time::Duration::from_secs(10), consumer)
            .await
            .expect("consumer joins")
            .expect("consumer task");
        let accepted = core.live_ingress.accepted.load(Ordering::Relaxed);
        let admission_cancelled = core
            .live_ingress
            .admission_cancelled
            .load(Ordering::Relaxed);
        assert_eq!(
            accepted + admission_cancelled,
            attempted,
            "every attempted admission is accepted or explicitly released"
        );
        assert_eq!(taken, accepted, "every accepted event was consumed");
        assert_eq!(core.live_ingress.queued.load(Ordering::Relaxed), 0);
        assert_eq!(core.live_ingress.outstanding(), 0);
    }
}

#[cfg(test)]
mod effects_tests {
    use super::*;
    use tiktools_tiktok::events::{CanonicalLiveEvent, ChatEvent, EventMetadata, EventUser};
    struct Emitter;
    impl HostEmitter for Emitter {
        fn emit(&self, _: HostMessage) {}
    }
    #[tokio::test]
    async fn stalled_persistence_never_hides_originals_and_saturation_is_accounted() {
        let core = Arc::new(AppCore::new(Arc::new(Emitter)));
        core.live_points_work
            .stall_for_test(Duration::from_millis(200));
        let (sender, receiver) = tokio::sync::mpsc::channel(INGRESS_QUEUE_CAPACITY);
        for id in 1..=300 {
            admit_stamped_event(
                &core,
                &sender,
                tiktools_tiktok::StampedClientEvent {
                    generation: 0,
                    event: ClientEvent::Event(NativeLiveEvent {
                        base: CanonicalLiveEvent::Chat(ChatEvent {
                            user: EventUser {
                                id: 1,
                                unique_id: "effects-test".into(),
                                ..Default::default()
                            },
                            comment: "same".into(),
                            ..Default::default()
                        }),
                        metadata: EventMetadata {
                            method: "WebcastChatMessage".into(),
                            msg_id: id,
                            is_history: false,
                        },
                        gift: None,
                    }),
                },
            )
            .await;
        }
        drop(sender);
        let worker = tokio::spawn(run_ingress_worker(
            Arc::clone(&core),
            receiver,
            tokio_util::sync::CancellationToken::new(),
        ));
        tokio::time::timeout(Duration::from_secs(5), async {
            while core.live_ingress.ui_published.load(Ordering::Relaxed) < 300 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert!(core.live_ingress.effects_dropped.load(Ordering::Relaxed) > 0);
        core.live_points_work.stall_for_test(Duration::ZERO);
        tokio::time::timeout(Duration::from_secs(5), worker)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            core.live_ingress.effects_processed.load(Ordering::Relaxed)
                + core.live_ingress.effects_dropped.load(Ordering::Relaxed),
            300
        );
        assert_eq!(core.live_ingress.effects_queued.load(Ordering::Relaxed), 0);
        assert_eq!(
            core.live_ingress.effects_in_flight.load(Ordering::Relaxed),
            0
        );
        core.shutdown().await;
    }
}
