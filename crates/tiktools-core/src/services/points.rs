use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex, RwLock,
    },
    time::SystemTime,
};
#[cfg(feature = "persistence")]
use std::{
    collections::VecDeque,
    sync::{Condvar, MutexGuard},
    time::{Duration, Instant},
};

use serde_json::{json, Value};

use crate::db::DatabaseManager;
use crate::ipc::messages::{PartialPointsConfig, PointsConfig};
use tiktools_plugin_api::sync::{recover_mutex, recover_rwlock_read, recover_rwlock_write};

#[cfg(feature = "persistence")]
mod admission;
#[cfg(feature = "persistence")]
mod approved;
#[cfg(feature = "persistence")]
mod commands;

mod awards;
mod cache;
mod configuration;
mod diagnostics;
mod helpers;
mod lifecycle;
mod mutations;
mod queries;
mod queue;
mod viewer;
#[cfg(feature = "persistence")]
mod writer;

pub(crate) use helpers::valid_viewer_id;
use helpers::*;
#[cfg(feature = "persistence")]
use writer::{queue_wait_timeout, run_points_writer};

/// Default bound for the in-memory hot-viewer cache. SQLite remains the
/// durable source of truth; viewers outside the cache are reloaded on demand
/// through the indexed `viewers.unique_id` primary key.
pub const DEFAULT_HOT_VIEWER_CAPACITY: usize = 10_000;
/// Upper bound for queued durable writes. When the writer cannot keep up,
/// admission waits (bounded) for a free slot and then rejects the op
/// without mutating memory. There is deliberately no synchronous write
/// path: an inline write could jump the queue and win the FIFO race with a
/// stale row snapshot, silently dropping the increments already queued.
#[cfg(feature = "persistence")]
const WRITER_QUEUE_CAPACITY: usize = 4_096;
/// Maximum durable writes folded into one SQLite transaction.
#[cfg(feature = "persistence")]
const WRITER_BATCH_SIZE: usize = 128;
/// How long an award/adjust/reset waits for a free queue slot. Exceeding it
/// is an explicit rejection (`writeRejected` in diagnostics, an error log
/// with the exact sequence): memory is left untouched, so the caller's
/// view, the hot cache and SQLite all agree that the op did not happen.
#[cfg(feature = "persistence")]
const WRITE_ADMISSION_TIMEOUT_MS: u64 = 5_000;
/// Poll interval for an admission waiting on a free slot. The condvar
/// wakeup is the primary path; this bounds the wait if a wakeup is lost.
#[cfg(feature = "persistence")]
const WRITE_ADMISSION_POLL_MS: u64 = 50;
/// Bounded whole-transaction retries before the writer falls back to
/// per-op transactions. Storage that is merely busy recovers here; storage
/// that is broken moves on to an explicit per-op failure report.
#[cfg(feature = "persistence")]
const WRITE_BATCH_RETRIES: u32 = 3;
/// Base backoff between writer retries, doubled per attempt.
#[cfg(feature = "persistence")]
const WRITE_RETRY_BACKOFF_MS: u64 = 5;

/// Strongly typed viewer row. The wire shape stays JSON (`Value`) at the API
/// boundary; internally every hot-path field is typed so awards avoid dynamic
/// lookup, and the leaderboard orders without touching `serde_json`.
#[derive(Debug, Clone)]
pub struct ViewerState {
    pub unique_id: String,
    pub user_id: Option<String>,
    pub nickname: String,
    pub avatar_url: Option<String>,
    pub points: f64,
    pub level: u32,
    pub is_subscriber: bool,
    pub total_chats: i64,
    pub total_coins: i64,
    pub total_likes: i64,
    pub total_shares: i64,
    pub first_seen: i64,
    pub last_seen: i64,
}

struct PointsInner {
    viewers: HashMap<String, ViewerState>,
    capacity: usize,
    /// Sequences with durable writes still queued in the writer. Eviction
    /// must never drop these: a reload would read stale SQLite state.
    pending: HashMap<String, u64>,
    /// Sequence of the newest full reset whose durable op has not resolved
    /// yet. A cold reload before that reset lands would resurrect
    /// pre-reset rows, so reloads wait on it exactly like a per-viewer
    /// marker. Cleared when the reset resolves.
    reset_pending: u64,
    hits: u64,
    misses: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PointAward {
    pub unique_id: String,
    pub delta: f64,
    pub total_points: f64,
    pub level: u32,
    pub currency_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointAction {
    Chat,
    Gift,
    Like,
    Share,
    Follow,
    Join,
    Manual,
}

impl PointAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Chat => "chat",
            Self::Gift => "gift",
            Self::Like => "like",
            Self::Share => "share",
            Self::Follow => "follow",
            Self::Join => "join",
            Self::Manual => "manual",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AwardOptions {
    pub user_id: Option<String>,
    pub nickname: Option<String>,
    pub avatar_url: Option<String>,
    pub count: Option<f64>,
    pub diamond_count: Option<f64>,
    pub is_subscriber: Option<bool>,
    pub custom_amount: Option<f64>,
}

pub struct PointsService {
    /// Serializes a producer's complete read/compute/admit/enqueue operation.
    /// The viewer lock alone cannot do this: cold reads and enqueue happen
    /// outside it. The writer never takes this lock and can drain a full
    /// queue while a producer waits for admission. Lock order is always
    /// mutation -> config/viewers -> writer queue.
    mutation: Mutex<()>,
    config: RwLock<PointsConfig>,
    inner: RwLock<PointsInner>,
    next_op: AtomicU64,
    #[cfg(feature = "persistence")]
    database: Option<Arc<DatabaseManager>>,
    /// Ordered durable-write queue plus the counters both sides update.
    /// Always present under `persistence`; when no writer thread runs the
    /// queue starts closed so admissions reject loudly instead of piling
    /// up work that nothing will ever apply.
    #[cfg(feature = "persistence")]
    writer: Arc<PointsWriterShared>,
    #[cfg(feature = "persistence")]
    writer_thread: Mutex<Option<std::thread::JoinHandle<()>>>,
    #[cfg(all(test, feature = "persistence"))]
    before_submit: Mutex<Option<AdmissionPause>>,
    #[cfg(all(test, feature = "persistence"))]
    after_command_admission: Mutex<Option<AdmissionPause>>,
}

#[cfg(all(test, feature = "persistence"))]
#[derive(Debug)]
pub(crate) struct AdmissionPause {
    reached: std::sync::mpsc::SyncSender<()>,
    resume: std::sync::mpsc::Receiver<()>,
}

#[cfg(all(test, feature = "persistence"))]
impl AdmissionPause {
    pub(crate) fn wait(self) {
        self.reached
            .send(())
            .expect("test observes admission barrier");
        self.resume.recv().expect("test releases admission barrier");
    }
}

/// Durable-write diagnostics for `system.health`. Contains counts only, never
/// viewer identities or chat text.
#[derive(Debug, Clone, Default)]
pub struct PointsDiagnostics {
    pub hot_size: usize,
    pub hot_capacity: usize,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub writer_queued: usize,
    pub write_batches: u64,
    pub write_errors: u64,
    /// Admissions that found the queue full and had to wait for space.
    pub writer_overload: u64,
    /// Admissions the writer refused (queue closed or saturated for the
    /// full admission timeout). Nothing was mutated or queued for these.
    pub write_rejected: u64,
    /// Ops the writer could not make durable even after bounded retries.
    pub write_failed: u64,
    pub write_applied: u64,
    pub writer_executing: usize,
    pub pending_writes: usize,
    /// Contiguous durable watermark: every sequence <= this is on disk.
    pub committed_seq: u64,
    /// Resolved watermark: every sequence <= this is either durable or was
    /// explicitly reported as a permanent failure.
    pub resolved_seq: u64,
    pub commit_latency_mean_ms: f64,
    pub commit_latency_max_ms: u64,
}

/// One durable intent sent to the owned points writer. The single writer
/// thread applies ops FIFO, so per-viewer award/adjust/reset ordering is
/// preserved exactly as the callers issued it.
#[cfg(feature = "persistence")]
#[derive(Debug)]
pub(crate) enum PointsWriteOp {
    Save {
        seq: u64,
        viewer: ViewerState,
        reason: &'static str,
        awarded: f64,
    },
    Adjust {
        seq: u64,
        unique_id: String,
        points: f64,
        level: u32,
    },
    Reset {
        seq: u64,
        unique_id: Option<String>,
    },
}

/// FIFO state behind one lock. Sequence allocation and the push that
/// admits an op both happen while the caller holds the viewer lock, so
/// sequence order and queue order are the same order by construction.
#[cfg(feature = "persistence")]
#[derive(Debug, Default)]
struct PointsQueueState {
    queue: VecDeque<PointsWriteOp>,
    /// Set by `shutdown` (or by construction when no writer thread runs).
    /// Once set the queue drains and then stops accepting.
    closed: bool,
}

/// Counters shared between the service and its writer thread.
///
/// Two watermarks are kept on purpose:
/// - `committed_seq` is contiguous and durable: a permanently failed write
///   stops it, so it never claims more than SQLite actually holds.
/// - `resolved_seq` is finished: durable *or* explicitly reported failed.
///   Eviction and reload waits key off this, because outstanding work for
///   a failed op is over — pinning the cache or stalling a reload forever
///   would hide the failure instead of surfacing it.
#[cfg(feature = "persistence")]
#[derive(Debug)]
struct PointsWriterShared {
    queue: Mutex<PointsQueueState>,
    wake: Condvar,
    /// Batches the writer folded into one transaction.
    batches: AtomicU64,
    /// Transient storage errors the writer recovered from by retrying.
    errors: AtomicU64,
    /// Ops that could not be made durable even after bounded retries.
    failed_ops: AtomicU64,
    /// First sequence that permanently failed, if any.
    first_failed_seq: AtomicU64,
    committed_seq: AtomicU64,
    resolved_seq: AtomicU64,
    /// Admissions that found the queue full and waited for a free slot.
    overload: AtomicU64,
    /// Admissions refused (queue closed, or saturated past the timeout).
    rejected: AtomicU64,
    /// Ops admitted into the queue.
    admitted: AtomicU64,
    /// Ops applied to SQLite (applied or retried successfully).
    applied: AtomicU64,
    /// Gauge: ops waiting in the queue.
    queued: AtomicU64,
    /// Gauge: ops currently inside a batch the writer is applying.
    executing: AtomicU64,
    commit_latency_sum_ms: AtomicU64,
    commit_samples: AtomicU64,
    commit_latency_max_ms: AtomicU64,
    /// Admission wait bound in milliseconds. Per service (not a global) so a
    /// saturation test cannot shorten another service's timeout.
    admission_timeout_ms: AtomicU64,
    /// Test-only stall gate (like the LIVE ingress gate): simulates a
    /// wedged SQLite without touching the rest of the service.
    #[cfg(test)]
    stall_ms: AtomicU64,
    #[cfg(test)]
    before_apply: Mutex<Option<AdmissionPause>>,
}

#[cfg(test)]
mod tests;
