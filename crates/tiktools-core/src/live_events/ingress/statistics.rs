//! Shared atomic accounting, lifecycle observations and bounded overload diagnostics.
use super::*;

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

    pub(super) fn note_overload(&self, message: &str) {
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
