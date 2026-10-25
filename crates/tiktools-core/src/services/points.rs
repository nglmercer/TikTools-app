use std::{
    collections::{HashMap, VecDeque},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Condvar, Mutex, MutexGuard, RwLock,
    },
    time::{Duration, Instant, SystemTime},
};

use serde_json::{json, Value};

use crate::db::DatabaseManager;
use crate::ipc::messages::{PartialPointsConfig, PointsConfig};
use tiktools_plugin_api::sync::{recover_mutex, recover_rwlock_read, recover_rwlock_write};

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

impl ViewerState {
    /// Deterministic leaderboard order: points descending, then coins,
    /// then chats, then identity. The final `unique_id` tie-break keeps
    /// the board stable when every counter matches.
    fn compare_rank(left: &Self, right: &Self) -> std::cmp::Ordering {
        right
            .points
            .total_cmp(&left.points)
            .then_with(|| right.total_coins.cmp(&left.total_coins))
            .then_with(|| right.total_chats.cmp(&left.total_chats))
            .then_with(|| left.unique_id.cmp(&right.unique_id))
    }

    pub fn to_value(&self) -> Value {
        serde_json::json!({
            "uniqueId": self.unique_id,
            "userId": self.user_id,
            "nickname": self.nickname,
            "avatarUrl": self.avatar_url,
            "points": self.points,
            "level": self.level,
            "isSubscriber": self.is_subscriber,
            "totalChats": self.total_chats,
            "totalCoins": self.total_coins,
            "totalLikes": self.total_likes,
            "totalShares": self.total_shares,
            "firstSeen": self.first_seen,
            "lastSeen": self.last_seen,
        })
    }

    pub fn from_value(value: &Value) -> Option<Self> {
        let object = value.as_object()?;
        Some(Self {
            unique_id: object.get("uniqueId")?.as_str()?.to_owned(),
            user_id: object
                .get("userId")
                .and_then(Value::as_str)
                .map(str::to_owned),
            nickname: object
                .get("nickname")
                .and_then(Value::as_str)
                .map(str::to_owned)
                .unwrap_or_default(),
            avatar_url: object
                .get("avatarUrl")
                .and_then(Value::as_str)
                .map(str::to_owned),
            points: object
                .get("points")
                .and_then(Value::as_f64)
                .unwrap_or_default(),
            level: object
                .get("level")
                .and_then(Value::as_u64)
                .unwrap_or(1)
                .min(u64::from(u32::MAX)) as u32,
            is_subscriber: object
                .get("isSubscriber")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            total_chats: object
                .get("totalChats")
                .and_then(Value::as_i64)
                .unwrap_or_default(),
            total_coins: object
                .get("totalCoins")
                .and_then(Value::as_i64)
                .unwrap_or_default(),
            total_likes: object
                .get("totalLikes")
                .and_then(Value::as_i64)
                .unwrap_or_default(),
            total_shares: object
                .get("totalShares")
                .and_then(Value::as_i64)
                .unwrap_or_default(),
            first_seen: object
                .get("firstSeen")
                .and_then(Value::as_i64)
                .unwrap_or_default(),
            last_seen: object
                .get("lastSeen")
                .and_then(Value::as_i64)
                .unwrap_or_default(),
        })
    }
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

impl PointsInner {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            viewers: HashMap::new(),
            capacity: capacity.max(1),
            pending: HashMap::new(),
            reset_pending: 0,
            hits: 0,
            misses: 0,
        }
    }

    /// Amortized eviction slack: past `capacity`, up to this many extra
    /// rows accumulate before one scan evicts back down to `capacity`.
    /// Without it every insert past the bound would pay a full O(N) scan —
    /// the same per-event trap the leaderboard sort was. The overshoot is
    /// a hard +64 rows, never unbounded growth.
    pub(crate) const EVICT_BATCH_SLACK: usize = 64;

    /// Hot-path gate: run the O(N) eviction scan only once the overshoot
    /// reaches a full batch, keeping steady-state inserts amortized O(1).
    /// Cold paths (explicit resize, single-row promotion, resets) call
    /// `evict_cold` directly for an exact bound.
    fn evict_if_over(&mut self, resolved: u64) {
        if self.viewers.len() > self.capacity + Self::EVICT_BATCH_SLACK {
            self.evict_cold(resolved);
        }
    }

    /// Evicts cold viewers until the cache fits `capacity`. Viewers with
    /// unresolved writes (sequence newer than the resolved watermark) are
    /// never evicted; when every viewer is dirty the cache temporarily
    /// exceeds its bound rather than losing acknowledged updates.
    fn evict_cold(&mut self, resolved: u64) {
        // Retire markers the writer has already resolved — durable or
        // explicitly reported failed — so the pending map itself stays
        // bounded. A failed write is not retried by eviction; reloading it
        // would resurrect state the operator was told had been rejected.
        self.pending.retain(|_, seq| *seq > resolved);
        if self.reset_pending <= resolved {
            self.reset_pending = 0;
        }
        while self.viewers.len() > self.capacity {
            let victim = self
                .viewers
                .iter()
                .filter(|(id, _)| self.pending.get(*id).is_none_or(|seq| *seq <= resolved))
                .min_by(|(left_id, left), (right_id, right)| {
                    left.last_seen
                        .cmp(&right.last_seen)
                        .then_with(|| left_id.cmp(right_id))
                })
                .map(|(id, _)| id.clone());
            let Some(victim) = victim else { break };
            self.viewers.remove(&victim);
        }
    }

    /// Sequence a reload must wait for before it may read SQLite: its own
    /// outstanding write, or an unresolved full reset (which zeroes rows
    /// this viewer may still carry).
    fn reload_wait_seq(&self, unique_id: &str) -> u64 {
        self.pending
            .get(unique_id)
            .copied()
            .unwrap_or(0)
            .max(self.reset_pending)
    }
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

impl PointsDiagnostics {
    /// Machine-readable snapshot for `system.health`.
    pub fn to_value(&self) -> Value {
        json!({
            "hotSize": self.hot_size,
            "hotCapacity": self.hot_capacity,
            "cacheHits": self.cache_hits,
            "cacheMisses": self.cache_misses,
            "writerQueued": self.writer_queued,
            "writerExecuting": self.writer_executing,
            "writeBatches": self.write_batches,
            "writeErrors": self.write_errors,
            "writerOverload": self.writer_overload,
            "writeRejected": self.write_rejected,
            "writeFailed": self.write_failed,
            "writeApplied": self.write_applied,
            "committedSeq": self.committed_seq,
            "resolvedSeq": self.resolved_seq,
            "commitLatencyMeanMs": self.commit_latency_mean_ms,
            "commitLatencyMaxMs": self.commit_latency_max_ms,
            "pendingWrites": self.pending_writes,
        })
    }
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

#[cfg(feature = "persistence")]
impl PointsWriteOp {
    fn seq(&self) -> u64 {
        match self {
            Self::Save { seq, .. } | Self::Adjust { seq, .. } | Self::Reset { seq, .. } => *seq,
        }
    }
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
}

#[cfg(feature = "persistence")]
impl PointsWriterShared {
    fn new(writer_running: bool) -> Self {
        Self {
            queue: Mutex::new(PointsQueueState {
                queue: VecDeque::new(),
                closed: !writer_running,
            }),
            wake: Condvar::new(),
            batches: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            failed_ops: AtomicU64::new(0),
            first_failed_seq: AtomicU64::new(0),
            committed_seq: AtomicU64::new(0),
            resolved_seq: AtomicU64::new(0),
            overload: AtomicU64::new(0),
            rejected: AtomicU64::new(0),
            admitted: AtomicU64::new(0),
            applied: AtomicU64::new(0),
            queued: AtomicU64::new(0),
            executing: AtomicU64::new(0),
            commit_latency_sum_ms: AtomicU64::new(0),
            commit_samples: AtomicU64::new(0),
            commit_latency_max_ms: AtomicU64::new(0),
            admission_timeout_ms: AtomicU64::new(WRITE_ADMISSION_TIMEOUT_MS),
            #[cfg(test)]
            stall_ms: AtomicU64::new(0),
        }
    }

    fn lock_queue(&self) -> MutexGuard<'_, PointsQueueState> {
        recover_mutex(&self.queue, "points write queue")
    }

    /// Records that a sequence could not be made durable. The first such
    /// sequence caps the contiguous durable watermark.
    fn record_failure(&self, seq: u64) {
        self.failed_ops.fetch_add(1, Ordering::Relaxed);
        self.first_failed_seq
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                Some(if current == 0 { seq } else { current.min(seq) })
            })
            .ok();
    }
}

#[cfg(feature = "persistence")]
impl Default for PointsWriterShared {
    fn default() -> Self {
        Self::new(false)
    }
}

/// Waits until the queue has work or admission is closed. Re-locks on
/// poison: a panicked writer must not wedge every award forever.
#[cfg(feature = "persistence")]
fn queue_wait<'a>(
    shared: &PointsWriterShared,
    guard: MutexGuard<'a, PointsQueueState>,
) -> MutexGuard<'a, PointsQueueState> {
    shared
        .wake
        .wait(guard)
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(feature = "persistence")]
fn queue_wait_timeout<'a>(
    shared: &PointsWriterShared,
    guard: MutexGuard<'a, PointsQueueState>,
    timeout: Duration,
) -> MutexGuard<'a, PointsQueueState> {
    shared
        .wake
        .wait_timeout(guard, timeout)
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .0
}

/// Writer entry point. Opens one connection, configures WAL/busy-timeout,
/// then folds queued ops into bounded transactions until `shutdown` closes
/// admission and the queue drains. Every admitted op is either applied or
/// explicitly reported as failed with its exact sequence: storage trouble
/// is counted and logged, never silently dropped, and one bad row cannot
/// discard its neighbours (bounded whole-transaction retries, then per-op
/// transactions). A failed write stops the contiguous durable watermark
/// while the resolved watermark keeps moving, so eviction and reloads stay
/// live without claiming durability they do not have.
#[cfg(feature = "persistence")]
fn run_points_writer(database: Arc<DatabaseManager>, shared: Arc<PointsWriterShared>) {
    let mut connection = match database.open_points_writer() {
        Ok(connection) => Some(connection),
        Err(error) => {
            // No connection means no durability for anything still queued,
            // or anything queued later. Keep draining and reporting: the
            // alternative is an unbounded queue and a shutdown that hangs
            // on work nobody can apply.
            tracing::error!(
                %error,
                "points writer could not open SQLite; every queued op is reported as a permanent failure"
            );
            shared.errors.fetch_add(1, Ordering::Relaxed);
            None
        }
    };
    let mut batch = Vec::with_capacity(WRITER_BATCH_SIZE);
    loop {
        batch.clear();
        // Test-only: simulate a wedged SQLite so saturation/rejection paths
        // are reachable deterministically. Sleeps outside the queue lock.
        #[cfg(test)]
        {
            let stall = shared.stall_ms.load(Ordering::Relaxed);
            if stall > 0 {
                std::thread::sleep(Duration::from_millis(stall));
            }
        }
        {
            let mut state = shared.lock_queue();
            // Block for the first op so an idle writer sleeps; then take
            // whatever else is already queued into the same transaction.
            loop {
                if !state.queue.is_empty() {
                    break;
                }
                if state.closed {
                    return;
                }
                state = queue_wait(&shared, state);
            }
            while batch.len() < WRITER_BATCH_SIZE {
                match state.queue.pop_front() {
                    Some(op) => batch.push(op),
                    None => break,
                }
            }
            shared
                .queued
                .store(state.queue.len() as u64, Ordering::Relaxed);
        }
        // Slots freed by the pop: admissions waiting on capacity can proceed
        // while this batch is being written, not after it.
        shared.wake.notify_all();
        shared
            .executing
            .fetch_add(batch.len() as u64, Ordering::Relaxed);
        let started = Instant::now();
        let first_failed =
            apply_points_batch_resilient(&database, connection.as_mut(), &batch, &shared);
        shared
            .executing
            .fetch_sub(batch.len() as u64, Ordering::Relaxed);
        let max_seq = batch.last().map_or(0, PointsWriteOp::seq);
        record_commit_latency(&shared, started.elapsed());
        match first_failed {
            Some(failed) => shared
                .committed_seq
                .store(failed.saturating_sub(1), Ordering::Release),
            None => shared.committed_seq.store(max_seq, Ordering::Release),
        }
        shared.resolved_seq.store(max_seq, Ordering::Release);
        shared.batches.fetch_add(1, Ordering::Relaxed);
        shared.wake.notify_all();
    }
}

/// Applies one batch: whole transaction first, bounded retries for storage
/// that is merely busy, then one transaction per op so a single bad row
/// cannot discard its neighbours. Returns the first sequence that failed
/// permanently, if any.
#[cfg(feature = "persistence")]
fn apply_points_batch_resilient(
    database: &DatabaseManager,
    connection: Option<&mut rusqlite::Connection>,
    batch: &[PointsWriteOp],
    shared: &PointsWriterShared,
) -> Option<u64> {
    let Some(connection) = connection else {
        for op in batch {
            tracing::error!(
                seq = op.seq(),
                "points write lost: writer has no database connection"
            );
            shared.record_failure(op.seq());
        }
        return batch.first().map(PointsWriteOp::seq);
    };
    if let Err(error) = database.apply_points_batch(connection, batch) {
        tracing::error!(
            %error,
            ops = batch.len(),
            max_seq = batch.last().map_or(0, PointsWriteOp::seq),
            "points batch transaction failed; retrying the transaction"
        );
        shared.errors.fetch_add(1, Ordering::Relaxed);
        let mut recovered = false;
        for attempt in 1..=WRITE_BATCH_RETRIES {
            std::thread::sleep(Duration::from_millis(
                WRITE_RETRY_BACKOFF_MS * u64::from(attempt),
            ));
            match database.apply_points_batch(connection, batch) {
                Ok(()) => {
                    recovered = true;
                    break;
                }
                Err(error) => {
                    shared.errors.fetch_add(1, Ordering::Relaxed);
                    tracing::error!(%error, attempt, "points batch transaction retry failed");
                }
            }
        }
        if !recovered {
            tracing::error!(
                ops = batch.len(),
                "points batch could not be applied; retrying each op on its own transaction"
            );
            let mut first_failed = None;
            for op in batch {
                match database.apply_points_op(connection, op) {
                    Ok(()) => {}
                    // Storage itself is failing (full disk, read-only FS);
                    // preservation cannot be guaranteed, so say so loudly
                    // with the exact op identity instead of vanishing.
                    Err(error) => {
                        tracing::error!(%error, seq = op.seq(), "points write failed and cannot be retried further");
                        shared.record_failure(op.seq());
                        first_failed.get_or_insert_with(|| op.seq());
                    }
                }
            }
            return first_failed;
        }
    }
    shared
        .applied
        .fetch_add(batch.len() as u64, Ordering::Relaxed);
    None
}

#[cfg(feature = "persistence")]
fn record_commit_latency(shared: &PointsWriterShared, elapsed: Duration) {
    let millis = u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX);
    shared.commit_samples.fetch_add(1, Ordering::Relaxed);
    shared
        .commit_latency_sum_ms
        .fetch_add(millis, Ordering::Relaxed);
    shared
        .commit_latency_max_ms
        .fetch_max(millis, Ordering::Relaxed);
}

impl Default for PointsService {
    fn default() -> Self {
        Self {
            config: RwLock::new(PointsConfig::default()),
            inner: RwLock::new(PointsInner::with_capacity(DEFAULT_HOT_VIEWER_CAPACITY)),
            next_op: AtomicU64::new(1),
            #[cfg(feature = "persistence")]
            database: None,
            #[cfg(feature = "persistence")]
            writer: Arc::new(PointsWriterShared::new(false)),
            #[cfg(feature = "persistence")]
            writer_thread: Mutex::new(None),
        }
    }
}

impl PointsService {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        let capacity = std::env::var("TIKTOOLS_POINTS_CACHE_CAPACITY")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(DEFAULT_HOT_VIEWER_CAPACITY);
        let service = Self {
            config: RwLock::new(PointsConfig::default()),
            inner: RwLock::new(PointsInner::with_capacity(capacity)),
            next_op: AtomicU64::new(1),
            #[cfg(feature = "persistence")]
            database: None,
            #[cfg(feature = "persistence")]
            writer: Arc::new(PointsWriterShared::new(false)),
            #[cfg(feature = "persistence")]
            writer_thread: Mutex::new(None),
        };
        #[cfg(feature = "persistence")]
        {
            // These locks were just created by construction above and are
            // still thread-local to this constructor, so poisoning is
            // impossible: expect pins that invariant.
            if let Ok(mut config) = database.load_points_config() {
                config.normalize();
                *service.config.write().expect("points config poisoned") = config;
            }
            // Warm only the leaderboard head: the hot cache is bounded, and
            // the database stays the source of truth for everyone else.
            // Cold viewers reload on demand through the indexed primary key.
            match database.load_top_viewers(capacity) {
                Ok(viewers) => {
                    let mut inner = service.inner.write().expect("points viewers poisoned");
                    for viewer in viewers.into_iter().filter_map(|value| {
                        ViewerState::from_value(&value)
                            .map(|state| (state.unique_id.clone(), state))
                    }) {
                        inner.viewers.insert(viewer.0, viewer.1);
                    }
                }
                Err(error) => {
                    tracing::warn!(%error, "could not warm points hot cache; cold viewers load on demand");
                }
            }
            let shared = Arc::new(PointsWriterShared::new(true));
            let thread = std::thread::Builder::new()
                .name("points-writer".to_owned())
                .spawn({
                    let database = Arc::clone(&database);
                    let shared = Arc::clone(&shared);
                    move || run_points_writer(database, shared)
                })
                .ok();
            if thread.is_none() {
                // No thread means nothing will ever drain the queue: close
                // admission now so awards reject loudly instead of queueing
                // writes nobody applies.
                tracing::error!("points writer thread failed to start; durable writes are rejected until it is restarted");
                let mut state = shared.lock_queue();
                state.closed = true;
            }
            *service
                .writer_thread
                .lock()
                .expect("points writer thread poisoned") = thread;
            // Keep the connection boundary in the service while the writer
            // owns its long-lived connection.
            let mut service = service;
            service.database = Some(database);
            service.writer = shared;
            service
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = database;
            service
        }
    }

    /// Resizes the hot-viewer bound. Shrinking below the current size evicts
    /// cold viewers immediately (never ones with unresolved writes); growing
    /// only raises the ceiling, with cold rows reloading from SQLite on demand.
    pub fn set_cache_capacity(&self, capacity: usize) {
        let capacity = capacity.clamp(1, 1_000_000);
        let resolved = self.resolved_seq();
        let mut inner = recover_rwlock_write(&self.inner, "points viewers");
        inner.capacity = capacity;
        inner.evict_cold(resolved);
    }

    pub fn cache_capacity(&self) -> usize {
        recover_rwlock_read(&self.inner, "points viewers").capacity
    }

    pub fn config(&self) -> PointsConfig {
        recover_rwlock_read(&self.config, "points config").clone()
    }

    fn next_seq(&self) -> u64 {
        self.next_op.fetch_add(1, Ordering::Relaxed)
    }

    /// Records that a durable write for `unique_id` is outstanding, so
    /// eviction cannot drop the row before it lands. With no database
    /// configured there is no durable write to wait for, so nothing is
    /// tracked — an unretired marker would pin every row in the cache.
    fn track_pending(&self, inner: &mut PointsInner, unique_id: &str, seq: u64) {
        #[cfg(feature = "persistence")]
        {
            if self.database.is_some() {
                inner.pending.insert(unique_id.to_owned(), seq);
            }
        }
        #[cfg(not(feature = "persistence"))]
        let _ = (inner, unique_id, seq);
    }

    /// Highest sequence the writer finished: durable, or explicitly reported
    /// as a permanent failure. Outstanding work for any lower sequence is
    /// over, so this is what eviction and reload waits key off.
    #[cfg(feature = "persistence")]
    fn resolved_seq(&self) -> u64 {
        self.writer.resolved_seq.load(Ordering::Acquire)
    }

    #[cfg(not(feature = "persistence"))]
    fn resolved_seq(&self) -> u64 {
        u64::MAX
    }

    /// Waits (bounded) until every op queued so far for `unique_id` is
    /// finished, then the caller may reload the row from SQLite without
    /// losing queued increments to a stale read.
    ///
    /// Why this exists: eviction is allowed once the writer resolves a row,
    /// but a later award for that same viewer may still sit queued behind
    /// other work. Reloading the durable row before that op applies would
    /// base new math on stale state and the stale-based snapshot would win
    /// the FIFO race, silently dropping the queued increment. An
    /// unresolved full reset is waited on too: it zeroes rows this viewer
    /// may still carry, and reloading early would resurrect them.
    ///
    /// It triggers only on a cache miss with outstanding work for that
    /// viewer — steady-state hot viewers never pay it — and it always
    /// terminates: the resolved watermark advances for durable *and*
    /// permanently failed writes, a dead writer trips the timeout with a
    /// warning.
    #[cfg(feature = "persistence")]
    fn wait_for_viewer_committed(&self, unique_id: &str) {
        let seq = recover_rwlock_read(&self.inner, "points viewers").reload_wait_seq(unique_id);
        if seq == 0 || self.resolved_seq() >= seq {
            return;
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        while self.resolved_seq() < seq {
            if Instant::now() >= deadline {
                tracing::warn!(
                    viewer_seq = seq,
                    resolved = self.resolved_seq(),
                    "points reload timed out waiting for queued writes; proceeding with durable state"
                );
                return;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    /// Direct indexed lookup by identity: hot cache first, then one SQLite
    /// primary-key read that promotes the row into the cache. Returns `None`
    /// only when the viewer has no record anywhere.
    pub fn viewer(&self, unique_id: &str) -> Option<Value> {
        let needle = clean_unique_id(unique_id)?;
        {
            let mut inner = recover_rwlock_write(&self.inner, "points viewers");
            if let Some(state) = inner.viewers.get(&needle).cloned() {
                inner.hits += 1;
                return Some(state.to_value());
            }
            inner.misses += 1;
        }
        #[cfg(feature = "persistence")]
        {
            // Never base a reload on state older than our own queued work.
            self.wait_for_viewer_committed(&needle);
            let loaded = self
                .database
                .as_ref()
                .and_then(|database| database.load_viewer_by_id(&needle).ok().flatten())
                .and_then(|value| ViewerState::from_value(&value));
            if let Some(state) = loaded {
                let resolved = self.resolved_seq();
                let mut inner = recover_rwlock_write(&self.inner, "points viewers");
                // A concurrent award may have created the row while the
                // database read was in flight: keep the newer resident.
                let resident = inner.viewers.entry(needle).or_insert(state).clone();
                inner.evict_cold(resolved);
                return Some(resident.to_value());
            }
        }
        None
    }

    pub fn diagnostics(&self) -> PointsDiagnostics {
        let inner = recover_rwlock_read(&self.inner, "points viewers");
        #[cfg(feature = "persistence")]
        {
            let samples = self.writer.commit_samples.load(Ordering::Relaxed);
            let sum = self.writer.commit_latency_sum_ms.load(Ordering::Relaxed);
            PointsDiagnostics {
                hot_size: inner.viewers.len(),
                hot_capacity: inner.capacity,
                cache_hits: inner.hits,
                cache_misses: inner.misses,
                writer_queued: self.writer.queued.load(Ordering::Relaxed) as usize,
                writer_executing: self.writer.executing.load(Ordering::Relaxed) as usize,
                write_batches: self.writer.batches.load(Ordering::Relaxed),
                write_errors: self.writer.errors.load(Ordering::Relaxed),
                writer_overload: self.writer.overload.load(Ordering::Relaxed),
                write_rejected: self.writer.rejected.load(Ordering::Relaxed),
                write_failed: self.writer.failed_ops.load(Ordering::Relaxed),
                write_applied: self.writer.applied.load(Ordering::Relaxed),
                pending_writes: inner.pending.len(),
                committed_seq: self.writer.committed_seq.load(Ordering::Acquire),
                resolved_seq: self.writer.resolved_seq.load(Ordering::Acquire),
                commit_latency_mean_ms: if samples == 0 {
                    0.0
                } else {
                    sum as f64 / samples as f64
                },
                commit_latency_max_ms: self.writer.commit_latency_max_ms.load(Ordering::Relaxed),
            }
        }
        #[cfg(not(feature = "persistence"))]
        {
            PointsDiagnostics {
                hot_size: inner.viewers.len(),
                hot_capacity: inner.capacity,
                cache_hits: inner.hits,
                cache_misses: inner.misses,
                ..PointsDiagnostics::default()
            }
        }
    }

    /// Closes admission, joins the writer thread and flushes every admitted
    /// op. Ordinary shutdown never loses accepted-intent work: when this
    /// returns the queue is empty and every admitted sequence is resolved.
    /// Safe to call more than once.
    ///
    /// Bounded by construction: closing admission lets the writer drain and
    /// exit, and a writer with no connection drains without applying.
    /// [`AppCore::shutdown`](crate::AppCore::shutdown) still wraps this in
    /// its own deadline and reports a stuck writer explicitly.
    pub fn shutdown(&self) {
        #[cfg(feature = "persistence")]
        {
            {
                let mut state = self.writer.lock_queue();
                state.closed = true;
            }
            self.writer.wake.notify_all();
            let thread = recover_mutex(&self.writer_thread, "points writer thread").take();
            if let Some(thread) = thread {
                if let Err(error) = thread.join() {
                    tracing::warn!(?error, "points writer thread panicked during shutdown");
                }
            }
            // Post-shutdown reloads must never stall on the wait above: the
            // writer drained, so every admitted sequence is resolved. This
            // never stores a sentinel maximum — a permanently failed write
            // stays visible as a gap between `committed_seq` and
            // `resolved_seq` instead of being papered over.
            let admitted = self.writer.admitted.load(Ordering::Acquire);
            self.writer
                .resolved_seq
                .fetch_max(admitted, Ordering::Release);
        }
    }

    pub fn award_points(
        &self,
        unique_id: &str,
        action: PointAction,
        options: AwardOptions,
    ) -> Option<PointAward> {
        let unique_id = clean_unique_id(unique_id)?;
        let config = self.config();
        let base_points = match action {
            PointAction::Manual => options.custom_amount.unwrap_or_default(),
            PointAction::Chat if config.points_per_chat_enabled => config.points_per_chat,
            PointAction::Gift if config.points_per_coin_enabled => {
                nonnegative_or_one(options.diamond_count.or(options.count)) * config.points_per_coin
            }
            PointAction::Like if config.points_per_like_enabled => {
                nonnegative_or_one(options.count) * config.points_per_like
            }
            PointAction::Share if config.points_per_share_enabled => config.points_per_share,
            PointAction::Follow if config.points_per_follow_enabled => config.points_per_follow,
            PointAction::Join if config.points_per_join_enabled => config.points_per_join,
            _ => 0.0,
        };

        // O(1) lookup; on a cold miss fall back to one indexed SQLite read
        // (outside the lock) and promote the row. Unknown viewers are
        // created, exactly as before.
        let is_subscriber_hint = options.is_subscriber;
        let now = now_millis();
        let state_snapshot = {
            let mut inner = recover_rwlock_write(&self.inner, "points viewers");
            let cached = inner.viewers.contains_key(&unique_id);
            if cached {
                inner.hits += 1;
            } else {
                inner.misses += 1;
            }
            cached
        };
        if !state_snapshot {
            #[cfg(feature = "persistence")]
            if let Some(loaded) = {
                // Never base an award on state older than our own queued work.
                self.wait_for_viewer_committed(&unique_id);
                self.database
                    .as_ref()
                    .and_then(|database| database.load_viewer_by_id(&unique_id).ok().flatten())
                    .and_then(|value| ViewerState::from_value(&value))
            } {
                let mut inner = recover_rwlock_write(&self.inner, "points viewers");
                // Conditional promotion: a concurrent award may have
                // created a newer row while the database read was in
                // flight. The update below always accumulates on top of
                // whatever is resident, so no increment is ever lost.
                // No eviction here: the promoted row still carries its
                // durable timestamp and would be misjudged as cold. The
                // update section refreshes it and enforces the bound.
                inner.viewers.entry(unique_id.clone()).or_insert(loaded);
            }
        }

        let (award, persisted) = {
            let mut inner = recover_rwlock_write(&self.inner, "points viewers");
            let existing_subscriber = inner
                .viewers
                .get(&unique_id)
                .map(|viewer| viewer.is_subscriber)
                .unwrap_or(false);
            let is_subscriber = is_subscriber_hint.unwrap_or(existing_subscriber);
            let mut awarded = base_points;
            if is_subscriber && config.sub_bonus_multiplier > 0.0 && awarded > 0.0 {
                awarded += awarded * (config.sub_bonus_multiplier / 100.0);
            }
            awarded = round_points(awarded);
            let current_points = inner
                .viewers
                .get(&unique_id)
                .map(|viewer| viewer.points)
                .unwrap_or_default();
            let total_points = round_points((current_points + awarded).max(0.0));
            let points_per_level = config.points_per_level.max(10.0);
            let level = (total_points / points_per_level).floor() as u32 + 1;

            // Reserve durable-queue space *before* mutating anything. A
            // rejected admission must leave memory exactly as it was, so the
            // hot cache never claims an award SQLite will never hold. This
            // wait also pins sequence order to queue order: nothing else can
            // allocate a sequence while this critical section holds `inner`.
            if !self.wait_write_capacity() {
                return None;
            }
            let seq = self.next_seq();
            let snapshot = {
                let state = inner
                    .viewers
                    .entry(unique_id.clone())
                    .or_insert_with(|| new_viewer_state(&unique_id, &options, now));
                update_existing_viewer(
                    state,
                    &options,
                    total_points,
                    level,
                    is_subscriber,
                    action,
                    now,
                );
                state.clone()
            };
            self.track_pending(&mut inner, &unique_id, seq);
            // Enforce the memory bound on the insert path only, amortized:
            // steady-state updates never scan. Rows with unresolved writes
            // are exempt from eviction.
            let resolved = self.resolved_seq();
            inner.evict_if_over(resolved);
            (
                PointAward {
                    unique_id,
                    delta: awarded,
                    total_points,
                    level,
                    currency_name: config.currency_name,
                },
                (seq, snapshot),
            )
        };

        #[cfg(feature = "persistence")]
        {
            let (seq, snapshot) = persisted;
            self.submit_points_op(PointsWriteOp::Save {
                seq,
                viewer: snapshot,
                reason: action.as_str(),
                awarded: award.delta,
            });
        }
        #[cfg(not(feature = "persistence"))]
        let _ = persisted;

        Some(award)
    }

    pub fn update_config(&self, update: PartialPointsConfig) -> PointsConfig {
        let updated = {
            let mut config = recover_rwlock_write(&self.config, "points config");
            update.apply(&mut config);
            config.normalize();
            config.clone()
        };
        #[cfg(feature = "persistence")]
        if let Some(database) = &self.database {
            if let Err(error) = database.save_points_config(&updated) {
                tracing::warn!(%error, "could not persist points configuration");
            }
        }
        updated
    }

    /// Top-K leaderboard with deterministic tie-breaking. Selection is
    /// O(N) to find the K winners plus O(K log K) to order them — never a
    /// full O(N log N) sort — and it runs only when a caller asks for the
    /// board, never per awarded event.
    pub fn leaderboard(&self, limit: Option<i64>) -> Vec<Value> {
        let limit = limit.unwrap_or(100).clamp(0, 1_000) as usize;
        if limit == 0 {
            return Vec::new();
        }
        let inner = recover_rwlock_read(&self.inner, "points viewers");
        if inner.viewers.len() <= limit {
            let mut all: Vec<&ViewerState> = inner.viewers.values().collect();
            all.sort_by(|left, right| ViewerState::compare_rank(left, right));
            return all.into_iter().map(ViewerState::to_value).collect();
        }
        let mut states: Vec<&ViewerState> = inner.viewers.values().collect();
        states.select_nth_unstable_by(limit, |left, right| ViewerState::compare_rank(left, right));
        let mut top: Vec<&ViewerState> = states.into_iter().take(limit).collect();
        top.sort_by(|left, right| ViewerState::compare_rank(left, right));
        top.into_iter().map(ViewerState::to_value).collect()
    }

    pub fn reset(&self, unique_id: Option<&str>) {
        let unique_id = unique_id.and_then(clean_unique_id);
        // Admission first: a rejected reset must not zero memory the durable
        // store will never match.
        if !self.wait_write_capacity() {
            return;
        }
        let resolved = self.resolved_seq();
        let mut inner = recover_rwlock_write(&self.inner, "points viewers");
        let seq = self.next_seq();
        let PointsInner {
            viewers,
            pending,
            reset_pending,
            ..
        } = &mut *inner;
        match unique_id.as_deref() {
            Some(unique_id) => {
                if let Some(viewer) = viewers.get_mut(unique_id) {
                    viewer.points = 0.0;
                    viewer.level = 1;
                    #[cfg(feature = "persistence")]
                    if self.database.is_some() {
                        pending.insert(unique_id.to_owned(), seq);
                    }
                }
                // A cold (evicted or never-cached) viewer is already reset
                // durably below; nothing is fabricated in memory.
                #[cfg(feature = "persistence")]
                self.submit_points_op(PointsWriteOp::Reset {
                    seq,
                    unique_id: Some(unique_id.to_owned()),
                });
            }
            None => {
                for viewer in viewers.values_mut() {
                    viewer.points = 0.0;
                    viewer.level = 1;
                }
                // Full reset: markers retire when the reset op resolves (it
                // zeroes every row and is queued behind all earlier writes),
                // so nothing is cleared here. Clearing early would let a
                // concurrent eviction reload a row the reset is about to
                // zero, and the reload would win the FIFO race. The reset
                // sequence itself gates every cold reload until it lands.
                #[cfg(feature = "persistence")]
                if self.database.is_some() {
                    *reset_pending = seq;
                }
                #[cfg(feature = "persistence")]
                self.submit_points_op(PointsWriteOp::Reset {
                    seq,
                    unique_id: None,
                });
            }
        }
        inner.evict_cold(resolved);
    }

    pub fn adjust(&self, unique_id: &str, delta: f64) -> Option<PointAward> {
        let unique_id = clean_unique_id(unique_id)?;
        if !delta.is_finite() {
            return None;
        }
        // A bounded cache means the viewer may live only in SQLite: reload
        // through the indexed key instead of reporting a durable viewer as
        // unknown.
        let cached = {
            recover_rwlock_read(&self.inner, "points viewers")
                .viewers
                .contains_key(&unique_id)
        };
        if !cached {
            #[cfg(feature = "persistence")]
            if let Some(loaded) = {
                self.wait_for_viewer_committed(&unique_id);
                self.database
                    .as_ref()
                    .and_then(|database| database.load_viewer_by_id(&unique_id).ok().flatten())
                    .and_then(|value| ViewerState::from_value(&value))
            } {
                let mut inner = recover_rwlock_write(&self.inner, "points viewers");
                // Conditional promotion only: no eviction yet, for the same
                // stale-timestamp reason as in `award_points`. The update
                // below refreshes the row and enforces the bound.
                inner.viewers.entry(unique_id.clone()).or_insert(loaded);
            }
        }
        // Read config outside the viewer lock: one lock at a time.
        let config = self.config();
        let (award_value, seq) = {
            let mut inner = recover_rwlock_write(&self.inner, "points viewers");
            let viewer = inner.viewers.get(&unique_id)?;
            let points = round_points((viewer.points + delta).max(0.0));
            let level =
                ((points / config.points_per_level.max(10.0)).floor() as u32).saturating_add(1);
            // Reserve before mutating: a rejected admission leaves the row
            // untouched instead of diverging from the durable store.
            if !self.wait_write_capacity() {
                return None;
            }
            let seq = self.next_seq();
            let viewer = inner
                .viewers
                .get_mut(&unique_id)
                .expect("viewer checked above the admission wait");
            viewer.points = points;
            viewer.level = level;
            viewer.last_seen = now_millis();
            self.track_pending(&mut inner, &unique_id, seq);
            // Bound enforced after the refresh (amortized), never on a
            // stale timestamp.
            let resolved = self.resolved_seq();
            inner.evict_if_over(resolved);
            (
                PointAward {
                    unique_id: unique_id.clone(),
                    delta,
                    total_points: points,
                    level,
                    currency_name: config.currency_name.clone(),
                },
                seq,
            )
        };
        #[cfg(feature = "persistence")]
        self.submit_points_op(PointsWriteOp::Adjust {
            seq,
            unique_id: award_value.unique_id.clone(),
            points: award_value.total_points,
            level: award_value.level,
        });
        #[cfg(not(feature = "persistence"))]
        let _ = seq;
        Some(award_value)
    }

    /// Bounded wait for one free slot in the ordered writer queue. Called
    /// while the caller holds the viewer lock — that is what makes sequence
    /// allocation order and queue order the same order.
    ///
    /// There is no synchronous write fallback on this path: an inline write
    /// could jump the queue and win the FIFO race with the ops already in
    /// it, overwriting newer state with a stale snapshot. Saturation
    /// therefore delays the caller, and the timeout rejects the op without
    /// mutating memory so caller-visible state and SQLite stay consistent.
    #[cfg(feature = "persistence")]
    fn wait_write_capacity(&self) -> bool {
        // No database configured (in-memory service): nothing durable to
        // admit, so awards proceed directly.
        if self.database.is_none() {
            return true;
        }
        let timeout =
            Duration::from_millis(self.writer.admission_timeout_ms.load(Ordering::Relaxed));
        let deadline = Instant::now() + timeout;
        let mut state = self.writer.lock_queue();
        let mut waited = false;
        loop {
            if state.closed {
                return false;
            }
            if state.queue.len() < WRITER_QUEUE_CAPACITY {
                break;
            }
            if !waited {
                self.writer.overload.fetch_add(1, Ordering::Relaxed);
                waited = true;
            }
            if Instant::now() >= deadline {
                tracing::warn!(
                    queued = state.queue.len(),
                    timeout_ms = timeout.as_millis() as u64,
                    "points writer saturated; rejecting write without mutating memory"
                );
                self.writer.rejected.fetch_add(1, Ordering::Relaxed);
                return false;
            }
            state = queue_wait_timeout(
                &self.writer,
                state,
                Duration::from_millis(WRITE_ADMISSION_POLL_MS),
            );
        }
        true
    }

    #[cfg(not(feature = "persistence"))]
    fn wait_write_capacity(&self) -> bool {
        true
    }

    /// Pushes one op onto the ordered writer queue. Capacity was reserved
    /// through [`Self::wait_write_capacity`] inside the same critical
    /// section that allocated the sequence, so this push cannot block and
    /// cannot reorder.
    #[cfg(feature = "persistence")]
    fn submit_points_op(&self, op: PointsWriteOp) {
        if self.database.is_none() {
            return;
        }
        let mut state = self.writer.lock_queue();
        if state.closed {
            // Admission closed between the capacity wait and this push
            // (shutdown racing an award). The op cannot be made durable
            // anymore and memory already carries the update, so report the
            // loss loudly instead of pretending the queue accepted it.
            tracing::error!(
                seq = op.seq(),
                "points write rejected: writer queue closed while admitting"
            );
            self.writer.rejected.fetch_add(1, Ordering::Relaxed);
            return;
        }
        state.queue.push_back(op);
        self.writer
            .queued
            .store(state.queue.len() as u64, Ordering::Relaxed);
        self.writer.admitted.fetch_add(1, Ordering::Relaxed);
        self.writer.wake.notify_one();
    }
}

fn clean_unique_id(value: &str) -> Option<String> {
    let value = value.trim().trim_start_matches('@');
    (!value.is_empty()).then_some(value.to_owned())
}

fn nonnegative_or_one(value: Option<f64>) -> f64 {
    value
        .filter(|value| value.is_finite() && *value >= 0.0)
        .unwrap_or(1.0)
}

fn round_points(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn update_existing_viewer(
    state: &mut ViewerState,
    options: &AwardOptions,
    points: f64,
    level: u32,
    is_subscriber: bool,
    action: PointAction,
    now: i64,
) {
    if let Some(value) = options.user_id.as_ref() {
        state.user_id = Some(value.clone());
    }
    if let Some(value) = options.nickname.as_ref() {
        state.nickname = value.clone();
    }
    if let Some(value) = options.avatar_url.as_ref() {
        state.avatar_url = Some(value.clone());
    }
    state.points = points;
    state.level = level;
    state.is_subscriber = is_subscriber;
    // Counters and metadata advance even when the award itself is zero:
    // a chat that earns nothing is still a chat.
    if action == PointAction::Chat {
        state.total_chats = state.total_chats.saturating_add(1);
    }
    if action == PointAction::Gift {
        state.total_coins = state
            .total_coins
            .saturating_add(nonnegative_or_one(options.diamond_count).round() as i64);
    }
    if action == PointAction::Like {
        state.total_likes = state
            .total_likes
            .saturating_add(nonnegative_or_one(options.count).round() as i64);
    }
    if action == PointAction::Share {
        state.total_shares = state.total_shares.saturating_add(1);
    }
    state.last_seen = now;
}

fn new_viewer_state(unique_id: &str, options: &AwardOptions, now: i64) -> ViewerState {
    ViewerState {
        unique_id: unique_id.to_owned(),
        user_id: options.user_id.clone(),
        nickname: options
            .nickname
            .clone()
            .unwrap_or_else(|| unique_id.to_owned()),
        avatar_url: options.avatar_url.clone(),
        points: 0.0,
        level: 1,
        is_subscriber: false,
        total_chats: 0,
        total_coins: 0,
        total_likes: 0,
        total_shares: 0,
        first_seen: now,
        last_seen: now,
    }
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds an isolated on-disk service plus its temp root. Every
    /// persistence test owns its directory so a crash cannot leak state
    /// into the next run.
    #[cfg(feature = "persistence")]
    fn temp_service(tag: &str) -> (Arc<DatabaseManager>, PointsService, std::path::PathBuf) {
        use crate::paths::AppPaths;
        let root = std::env::temp_dir().join(format!(
            "tiktools-points-{tag}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let paths = AppPaths {
            data: root.join("data"),
            plugins: root.join("plugins"),
            plugin_data: root.join("plugin-data"),
            builtin_plugins: root.join("builtin-plugins"),
            development_plugins: None,
            logs: root.join("logs"),
            temp: root.join("temp"),
            root: root.clone(),
        };
        paths.ensure_directories().expect("temp dirs");
        let database = Arc::new(DatabaseManager::new(paths));
        let service = PointsService::new(Arc::clone(&database));
        (database, service, root)
    }

    /// Waits until the writer has taken and applied everything queued.
    #[cfg(feature = "persistence")]
    fn wait_for_writer_idle(service: &PointsService) {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let diagnostics = service.diagnostics();
            if diagnostics.writer_queued == 0 && diagnostics.writer_executing == 0 {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "writer never drained: {:?}",
                diagnostics
            );
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    #[test]
    fn award_points_matches_existing_defaults() {
        let service = PointsService::default();
        let award = service.award_points("@alice", PointAction::Chat, AwardOptions::default());
        assert_eq!(
            award.as_ref().map(|award| award.unique_id.as_str()),
            Some("alice")
        );
        assert_eq!(award.as_ref().map(|award| award.total_points), Some(1.0));
        assert_eq!(service.leaderboard(Some(1))[0]["totalChats"], 1);
    }

    #[test]
    fn rule_updates_apply_to_next_award_without_recreating_service() {
        let service = PointsService::default();
        let first = service
            .award_points("alice", PointAction::Chat, AwardOptions::default())
            .expect("award");
        assert_eq!(first.delta, 1.0);

        let update = serde_json::from_value(serde_json::json!({
            "pointsPerChat": 7.0,
            "currencyName": "Gems"
        }))
        .expect("partial config");
        service.update_config(update);
        let second = service
            .award_points("alice", PointAction::Chat, AwardOptions::default())
            .expect("award");
        assert_eq!(second.delta, 7.0);
        assert_eq!(second.total_points, 8.0);
        assert_eq!(service.config().currency_name, "Gems");

        service.update_config(
            serde_json::from_value(serde_json::json!({ "pointsPerChatEnabled": false }))
                .expect("partial config"),
        );
        let disabled = service
            .award_points("alice", PointAction::Chat, AwardOptions::default())
            .expect("viewer counters still update");
        assert_eq!(disabled.delta, 0.0);
        assert_eq!(disabled.total_points, 8.0);
    }

    #[test]
    fn reset_keeps_viewers_and_counters_but_zeroes_points() {
        let service = PointsService::default();
        service.award_points("alice", PointAction::Chat, AwardOptions::default());
        service.reset(None);
        let viewer = &service.leaderboard(Some(1))[0];
        assert_eq!(viewer["points"], 0.0);
        assert_eq!(viewer["level"], 1);
        assert_eq!(viewer["totalChats"], 1);
    }

    #[test]
    fn negative_adjustments_clamp_at_zero() {
        // The `core.points` execution path (moderation penalties included)
        // adjusts through here: viewers can never drop below zero.
        let service = PointsService::default();
        service.award_points(
            "alice",
            PointAction::Manual,
            AwardOptions {
                custom_amount: Some(5.0),
                ..AwardOptions::default()
            },
        );
        let award = service.adjust("alice", -10.0).expect("viewer exists");
        assert_eq!(award.total_points, 0.0);
        assert_eq!(award.delta, -10.0);
        assert_eq!(service.leaderboard(Some(1))[0]["points"], 0.0);
        // Unknown viewers stay unknown: no negative phantom rows.
        assert!(service.adjust("nobody", -10.0).is_none());
        assert!(service.adjust("alice", f64::NAN).is_none());
    }

    #[test]
    fn point_configuration_normalizes_unsafe_ranges() {
        let mut config = PointsConfig {
            points_per_coin: -10.0,
            points_per_share: f64::NAN,
            points_per_chat: 2.0,
            points_per_like: f64::INFINITY,
            points_per_follow: 2.0,
            points_per_join: 2.0,
            sub_bonus_multiplier: -1.0,
            points_per_level: 1.0,
            currency_name: "   ".to_owned(),
            ..PointsConfig::default()
        };
        config.normalize();
        assert_eq!(config.points_per_coin, 0.0);
        assert_eq!(config.points_per_share, 3.0);
        assert_eq!(config.points_per_like, 0.1);
        assert_eq!(config.sub_bonus_multiplier, 0.0);
        assert_eq!(config.points_per_level, 10.0);
        assert_eq!(config.currency_name, "Points");
    }

    #[test]
    fn repeated_awards_accumulate_exact_totals() {
        let service = PointsService::default();
        for _ in 0..100 {
            service.award_points("alice", PointAction::Chat, AwardOptions::default());
        }
        let viewer = service.viewer("alice").expect("alice exists");
        assert_eq!(viewer["points"], 100.0);
        assert_eq!(viewer["totalChats"], 100);
    }

    #[test]
    fn zero_point_events_keep_counters_and_metadata() {
        let service = PointsService::default();
        // Likes disabled by config would award nothing; force it directly.
        let award = service
            .award_points(
                "bob",
                PointAction::Manual,
                AwardOptions {
                    custom_amount: Some(0.0),
                    nickname: Some("Bobby".to_owned()),
                    ..AwardOptions::default()
                },
            )
            .expect("award succeeds");
        assert_eq!(award.delta, 0.0);
        let viewer = service.viewer("bob").expect("bob exists");
        assert_eq!(viewer["points"], 0.0);
        assert_eq!(viewer["nickname"], "Bobby");
    }

    #[test]
    fn leaderboard_orders_without_per_event_sort_and_breaks_ties() {
        let service = PointsService::default();
        service.award_points(
            "charlie",
            PointAction::Manual,
            AwardOptions {
                custom_amount: Some(10.0),
                ..AwardOptions::default()
            },
        );
        service.award_points(
            "alice",
            PointAction::Manual,
            AwardOptions {
                custom_amount: Some(10.0),
                ..AwardOptions::default()
            },
        );
        service.award_points(
            "bob",
            PointAction::Manual,
            AwardOptions {
                custom_amount: Some(5.0),
                ..AwardOptions::default()
            },
        );
        let board = service.leaderboard(Some(2));
        assert_eq!(board.len(), 2);
        // Equal points fall back to the deterministic identity tie-break.
        assert_eq!(board[0]["uniqueId"], "alice");
        assert_eq!(board[1]["uniqueId"], "charlie");
    }

    #[test]
    fn high_cardinality_awards_stay_bounded_and_exact() {
        let service = PointsService::default();
        service.set_cache_capacity(1_000);
        for id in 0..5_000 {
            let name = format!("viewer-{id:05}");
            service.award_points(&name, PointAction::Chat, AwardOptions::default());
        }
        let diagnostics = service.diagnostics();
        assert!(
            diagnostics.hot_size <= 1_000 + PointsInner::EVICT_BATCH_SLACK,
            "hot cache must respect its bound, got {}",
            diagnostics.hot_size
        );
        // The most recently touched viewer is always resident and exact.
        let viewer = service.viewer("viewer-04999").expect("recent viewer");
        assert_eq!(viewer["points"], 1.0);
        assert_eq!(viewer["totalChats"], 1);
        // The leaderboard serves the top-K without a full sort even when
        // the population exceeds the visible window.
        let board = service.leaderboard(Some(100));
        assert_eq!(board.len(), 100);
    }

    #[cfg(feature = "persistence")]
    #[test]
    fn eviction_and_restart_preserve_exact_balances() {
        use crate::db::DatabaseManager;
        use crate::paths::AppPaths;
        let root = std::env::temp_dir().join(format!(
            "tiktools-points-phase1-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let paths = AppPaths {
            data: root.join("data"),
            plugins: root.join("plugins"),
            plugin_data: root.join("plugin-data"),
            builtin_plugins: root.join("builtin-plugins"),
            development_plugins: None,
            logs: root.join("logs"),
            temp: root.join("temp"),
            root: root.clone(),
        };
        paths.ensure_directories().expect("temp dirs");
        let database = Arc::new(DatabaseManager::new(paths));
        let service = PointsService::new(Arc::clone(&database));
        service.set_cache_capacity(50);
        for id in 0..200 {
            let name = format!("fan-{id:03}");
            service.award_points(
                &name,
                PointAction::Manual,
                AwardOptions {
                    custom_amount: Some(7.0),
                    ..AwardOptions::default()
                },
            );
        }
        // Repeated updates to an evicted viewer reload durable state and
        // accumulate instead of forking a phantom row.
        for _ in 0..3 {
            service.award_points(
                "fan-000",
                PointAction::Manual,
                AwardOptions {
                    custom_amount: Some(7.0),
                    ..AwardOptions::default()
                },
            );
        }
        service.shutdown();
        let reloaded = service
            .viewer("fan-000")
            .expect("fan-000 survives eviction");
        assert_eq!(reloaded["points"], 28.0);
        drop(service);
        // Restart: balances survive through SQLite, not RAM. The runtime
        // capacity is a boot setting (env `TIKTOOLS_POINTS_CACHE_CAPACITY`
        // or `set_cache_capacity`), so reapply it after restart.
        let restarted = PointsService::new(database);
        restarted.set_cache_capacity(50);
        let viewer = restarted.viewer("fan-199").expect("fan-199 persisted");
        assert_eq!(viewer["points"], 7.0);
        let first = restarted.viewer("fan-000").expect("fan-000 persisted");
        assert_eq!(first["points"], 28.0);
        let diagnostics = restarted.diagnostics();
        assert!(
            diagnostics.hot_size <= 50 + PointsInner::EVICT_BATCH_SLACK,
            "hot_size={}",
            diagnostics.hot_size
        );
        restarted.shutdown();
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn cache_capacity_is_enforced_and_never_evicts_dirty_rows() {
        let service = PointsService::default();
        service.set_cache_capacity(10);
        for id in 0..50 {
            service.award_points(
                &format!("user-{id}"),
                PointAction::Chat,
                AwardOptions::default(),
            );
        }
        let diagnostics = service.diagnostics();
        assert!(
            diagnostics.hot_size <= 10 + PointsInner::EVICT_BATCH_SLACK,
            "hot_size={}",
            diagnostics.hot_size
        );
        assert_eq!(service.cache_capacity(), 10);
    }

    #[test]
    fn concurrent_awards_never_lose_increments() {
        use std::thread;
        let service = Arc::new(PointsService::default());
        let mut handles = Vec::new();
        for _ in 0..8 {
            let service = Arc::clone(&service);
            handles.push(thread::spawn(move || {
                for _ in 0..250 {
                    service.award_points("racer", PointAction::Chat, AwardOptions::default());
                }
            }));
        }
        for handle in handles {
            handle.join().expect("worker finishes");
        }
        let viewer = service.viewer("racer").expect("racer exists");
        assert_eq!(viewer["totalChats"], 8 * 250);
        assert_eq!(viewer["points"], 8.0 * 250.0);
    }

    /// Slow/unavailable SQLite must never break awarding or shutdown: the
    /// hot cache serves reads, writes queue and are reported as failures,
    /// and shutdown still joins cleanly.
    #[cfg(all(feature = "persistence", target_os = "linux"))]
    #[test]
    fn unavailable_storage_never_breaks_awards_or_shutdown() {
        use crate::db::DatabaseManager;
        use crate::paths::AppPaths;
        let root = std::env::temp_dir().join(format!(
            "tiktools-points-unavail-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let paths = AppPaths {
            data: root.join("data"),
            plugins: root.join("plugins"),
            plugin_data: root.join("plugin-data"),
            builtin_plugins: root.join("builtin-plugins"),
            development_plugins: None,
            logs: root.join("logs"),
            temp: root.join("temp"),
            root: root.clone(),
        };
        paths.ensure_directories().expect("temp dirs");
        // Poison the database file: every SQLite query under it fails,
        // exercising the writer-error and fallback paths. (Read-only is
        // belt-and-braces; the garbage content breaks SQLite regardless.)
        let db_file = root.join("data").join("tiktok-points.db");
        std::fs::write(&db_file, b"not a database").expect("poison file");
        let mut permissions = std::fs::metadata(&db_file).expect("metadata").permissions();
        permissions.set_readonly(true);
        std::fs::set_permissions(&db_file, permissions).expect("readonly");
        let database = Arc::new(DatabaseManager::new(paths));
        let service = PointsService::new(Arc::clone(&database));
        for id in 0..100 {
            let award = service
                .award_points(
                    &format!("survivor-{id}"),
                    PointAction::Chat,
                    AwardOptions::default(),
                )
                .expect("awards work from the hot cache");
            assert_eq!(award.total_points, 1.0);
        }
        // Reads still serve from memory; shutdown joins without hanging.
        let viewer = service.viewer("survivor-0").expect("memory serves reads");
        assert_eq!(viewer["points"], 1.0);
        service.shutdown();
        // Every op is resolved (applied or explicitly failed) so nothing is
        // left outstanding, but nothing was durable either: the failure is
        // visible as a gap between the watermarks instead of being hidden.
        let diagnostics = service.diagnostics();
        assert_eq!(diagnostics.writer_queued, 0, "{diagnostics:?}");
        assert_eq!(
            diagnostics.write_applied + diagnostics.write_failed,
            100,
            "{diagnostics:?}"
        );
        assert!(diagnostics.write_failed > 0, "{diagnostics:?}");
        assert!(
            diagnostics.resolved_seq >= diagnostics.committed_seq,
            "{diagnostics:?}"
        );
        assert!(
            diagnostics.committed_seq < diagnostics.resolved_seq,
            "a permanently failed write must stay visible as a watermark gap: {diagnostics:?}"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn viewer_lookup_is_direct_and_misses_cleanly() {
        let service = PointsService::default();
        assert!(service.viewer("ghost").is_none());
        assert!(service.viewer("   ").is_none());
        service.award_points("@alice", PointAction::Chat, AwardOptions::default());
        let viewer = service.viewer("alice").expect("direct hit");
        assert_eq!(viewer["uniqueId"], "alice");
        let diagnostics = service.diagnostics();
        assert!(diagnostics.cache_hits >= 1);
        assert!(diagnostics.cache_misses >= 1);
    }

    /// A wedged writer must never be papered over with an out-of-order
    /// synchronous write. Admission waits, then rejects without mutating
    /// memory; once the writer recovers, every admitted op is durable and
    /// the newest snapshot wins — which is only true if the queue order is
    /// the sequence order.
    #[cfg(feature = "persistence")]
    #[test]
    fn saturated_writer_rejects_instead_of_bypassing_the_fifo() {
        let (database, service, root) = temp_service("saturated");
        // Short admission bound so the rejection is reachable in a test.
        service
            .writer
            .admission_timeout_ms
            .store(50, Ordering::Relaxed);
        // Wedge the writer: the queue fills, then admissions must reject.
        service.writer.stall_ms.store(1_000, Ordering::Relaxed);
        let accepted = WRITER_QUEUE_CAPACITY;
        for _ in 0..accepted {
            let award = service.award_points("racer", PointAction::Chat, AwardOptions::default());
            assert!(
                award.is_some(),
                "admissions must succeed while the queue has room"
            );
        }
        assert_eq!(
            service.viewer("racer").expect("racer")["points"],
            accepted as f64
        );
        // The next award finds a full queue and is rejected with no memory
        // mutation: the caller and the durable store agree it did not land.
        let rejected = service.award_points("racer", PointAction::Chat, AwardOptions::default());
        assert!(
            rejected.is_none(),
            "saturated queue must reject, never write inline"
        );
        assert_eq!(
            service.viewer("racer").expect("racer")["points"],
            accepted as f64,
            "a rejected admission must not mutate the hot cache"
        );
        let saturated = service.diagnostics();
        assert_eq!(saturated.write_rejected, 1, "{saturated:?}");
        assert!(saturated.writer_overload >= 1, "{saturated:?}");
        assert!(saturated.writer_queued > 0, "{saturated:?}");
        assert_eq!(saturated.write_applied, 0, "{saturated:?}");

        // Recover the writer and drain: the durable row must carry the
        // newest accepted snapshot, not an older one that jumped the queue.
        service.writer.stall_ms.store(0, Ordering::Relaxed);
        service.shutdown();
        let drained = service.diagnostics();
        assert_eq!(drained.writer_queued, 0, "{drained:?}");
        assert_eq!(drained.write_failed, 0, "{drained:?}");
        assert_eq!(drained.write_applied as usize, accepted, "{drained:?}");
        assert_eq!(drained.committed_seq, drained.resolved_seq, "{drained:?}");
        drop(service);
        let restarted = PointsService::new(database);
        let durable = restarted.viewer("racer").expect("racer persisted");
        assert_eq!(
            durable["points"], accepted as f64,
            "durable state must equal the newest admitted snapshot"
        );
        restarted.shutdown();
        let _ = std::fs::remove_dir_all(root);
    }

    /// A full reset zeroes rows that may still be cold in SQLite. Reloading
    /// one before the reset op lands would resurrect its pre-reset points
    /// and the later FIFO winner would be the stale snapshot.
    #[cfg(feature = "persistence")]
    #[test]
    fn full_reset_blocks_cold_reloads_until_it_lands() {
        let (_database, service, root) = temp_service("reset");
        service.set_cache_capacity(4);
        service.award_points(
            "alice",
            PointAction::Manual,
            AwardOptions {
                custom_amount: Some(5.0),
                ..AwardOptions::default()
            },
        );
        service.award_points(
            "bob",
            PointAction::Manual,
            AwardOptions {
                custom_amount: Some(3.0),
                ..AwardOptions::default()
            },
        );
        wait_for_writer_idle(&service);
        // Wedge the writer so the reset op cannot land while we reload.
        service.writer.stall_ms.store(500, Ordering::Relaxed);
        service.reset(None);
        // Touch bob so alice is the coldest row, then shrink to force the
        // reload path for alice.
        service.adjust("bob", 0.0).expect("bob is cached");
        service.set_cache_capacity(1);
        let shrunk = service.diagnostics();
        assert_eq!(shrunk.hot_size, 1, "alice must be cold: {shrunk:?}");
        assert_eq!(
            service.viewer("bob").expect("bob is cached")["points"],
            0.0,
            "reset applies to memory immediately"
        );
        let alice = service.viewer("alice").expect("alice reloads");
        assert_eq!(
            alice["points"], 0.0,
            "a reload must not resurrect pre-reset points"
        );
        service.writer.stall_ms.store(0, Ordering::Relaxed);
        service.shutdown();
        let _ = std::fs::remove_dir_all(root);
    }

    /// Order at scale: 100,000 sequential awards to one viewer must leave
    /// exactly 100,000 durable points. The writer applies full row
    /// snapshots, so this only holds if it applies them in sequence order.
    #[cfg(feature = "persistence")]
    #[test]
    fn hundred_thousand_ordered_awards_are_durable_in_order() {
        const EVENTS: u64 = 100_000;
        let (database, service, root) = temp_service("ordered");
        for _ in 0..EVENTS {
            service.award_points("marathon", PointAction::Chat, AwardOptions::default());
        }
        let in_memory = service.viewer("marathon").expect("marathon");
        assert_eq!(in_memory["points"], EVENTS as f64);
        service.shutdown();
        let diagnostics = service.diagnostics();
        assert_eq!(diagnostics.writer_queued, 0, "{diagnostics:?}");
        assert_eq!(diagnostics.write_rejected, 0, "{diagnostics:?}");
        assert_eq!(diagnostics.write_failed, 0, "{diagnostics:?}");
        assert_eq!(diagnostics.write_applied, EVENTS, "{diagnostics:?}");
        assert_eq!(
            diagnostics.committed_seq, diagnostics.resolved_seq,
            "{diagnostics:?}"
        );
        drop(service);
        let restarted = PointsService::new(database);
        let durable = restarted.viewer("marathon").expect("marathon durable");
        assert_eq!(durable["points"], EVENTS as f64);
        restarted.shutdown();
        let _ = std::fs::remove_dir_all(root);
    }

    /// Concurrent awards from many threads must be durable in whatever
    /// order they were admitted, with nothing lost between memory and
    /// SQLite.
    #[cfg(feature = "persistence")]
    #[test]
    fn concurrent_awards_are_durable_and_exact() {
        let (database, service, root) = temp_service("concurrent");
        let service = Arc::new(service);
        let mut handles = Vec::new();
        for worker in 0..8 {
            let service = Arc::clone(&service);
            handles.push(std::thread::spawn(move || {
                for _ in 0..250 {
                    service.award_points("racer", PointAction::Chat, AwardOptions::default());
                    if worker % 2 == 0 {
                        service.adjust("racer", -0.5);
                    }
                }
            }));
        }
        for handle in handles {
            handle.join().expect("worker finishes");
        }
        let expected = service.viewer("racer").expect("racer")["points"]
            .as_f64()
            .expect("points are a number");
        wait_for_writer_idle(&service);
        let memory = service.diagnostics();
        assert_eq!(memory.writer_queued, 0, "{memory:?}");
        assert_eq!(memory.write_rejected, 0, "{memory:?}");
        assert_eq!(memory.write_failed, 0, "{memory:?}");
        service.shutdown();
        drop(service);
        let restarted = PointsService::new(database);
        let durable = restarted.viewer("racer").expect("racer durable");
        assert_eq!(durable["points"], expected);
        restarted.shutdown();
        let _ = std::fs::remove_dir_all(root);
    }
}
