use super::*;

impl PointsWriteOp {
    pub(super) fn seq(&self) -> u64 {
        match self {
            Self::Save { seq, .. } | Self::Adjust { seq, .. } | Self::Reset { seq, .. } => *seq,
        }
    }
}

impl PointsWriterShared {
    pub(super) fn new(writer_running: bool) -> Self {
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
            #[cfg(test)]
            before_apply: Mutex::new(None),
        }
    }

    pub(super) fn lock_queue(&self) -> MutexGuard<'_, PointsQueueState> {
        recover_mutex(&self.queue, "points write queue")
    }

    /// Records that a sequence could not be made durable. The first such
    /// sequence caps the contiguous durable watermark.
    pub(super) fn record_failure(&self, seq: u64) {
        self.failed_ops.fetch_add(1, Ordering::Relaxed);
        self.first_failed_seq
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                Some(if current == 0 { seq } else { current.min(seq) })
            })
            .ok();
    }
}

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
pub(super) fn queue_wait_timeout<'a>(
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
pub(super) fn run_points_writer(database: Arc<DatabaseManager>, shared: Arc<PointsWriterShared>) {
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
        #[cfg(test)]
        {
            let pause = recover_mutex(&shared.before_apply, "points test writer").take();
            if let Some(pause) = pause {
                pause.reached.send(()).expect("test observes writer gate");
                pause.resume.recv().expect("test releases writer gate");
            }
        }
        let started = Instant::now();
        apply_points_batch_resilient(&database, connection.as_mut(), &batch, &shared);
        shared
            .executing
            .fetch_sub(batch.len() as u64, Ordering::Relaxed);
        let max_seq = batch.last().map_or(0, PointsWriteOp::seq);
        record_commit_latency(&shared, started.elapsed());
        let failed = shared.first_failed_seq.load(Ordering::Relaxed);
        let committed = if failed == 0 {
            max_seq
        } else {
            max_seq.min(failed.saturating_sub(1))
        };
        shared.committed_seq.store(committed, Ordering::Release);
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
) {
    let Some(connection) = connection else {
        for op in batch {
            tracing::error!(
                seq = op.seq(),
                "points write lost: writer has no database connection"
            );
            shared.record_failure(op.seq());
        }
        return;
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
            for op in batch {
                match database.apply_points_op(connection, op) {
                    Ok(()) => {
                        shared.applied.fetch_add(1, Ordering::Relaxed);
                    }
                    // Storage itself is failing (full disk, read-only FS);
                    // preservation cannot be guaranteed, so say so loudly
                    // with the exact op identity instead of vanishing.
                    Err(error) => {
                        tracing::error!(%error, seq = op.seq(), "points write failed and cannot be retried further");
                        shared.record_failure(op.seq());
                    }
                }
            }
            return;
        }
    }
    shared
        .applied
        .fetch_add(batch.len() as u64, Ordering::Relaxed);
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
