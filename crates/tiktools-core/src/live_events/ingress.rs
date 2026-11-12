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

mod admission;
mod dispatch;
mod statistics;
mod worker;

#[cfg(test)]
pub(crate) use admission::admit_event;
#[cfg(feature = "native-tiktok")]
pub(crate) use admission::admit_stamped_event;
#[cfg(all(test, feature = "native-tiktok"))]
use admission::classify_data_event;
#[cfg(feature = "native-tiktok")]
use dispatch::run_ingress_dispatch;
#[cfg(feature = "native-tiktok")]
pub(crate) use worker::run_ingress_worker;

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

/// Delivery semantics for one data event under overload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DataClass {
    /// Durable, money-grade side effects: bounded backpressure, then an
    /// explicit gap-reported drop. Never silently discarded.
    Critical,
    /// High-volume ticks: shed immediately with counters.
    Sheddable,
}

#[cfg(test)]
mod effects_tests;
#[cfg(all(test, feature = "native-tiktok"))]
mod tests;
