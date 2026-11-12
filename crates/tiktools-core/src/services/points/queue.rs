use super::*;

impl PointsService {
    pub(super) fn next_seq(&self) -> u64 {
        self.next_op.fetch_add(1, Ordering::Relaxed)
    }

    /// Bounded wait for one free slot in the ordered writer queue. Called
    /// while the caller holds the mutation lock through enqueue — that is
    /// what makes sequence allocation order and queue order the same order.
    ///
    /// There is no synchronous write fallback on this path: an inline write
    /// could jump the queue and win the FIFO race with the ops already in
    /// it, overwriting newer state with a stale snapshot. Saturation
    /// therefore delays the caller, and the timeout rejects the op without
    /// mutating memory so caller-visible state and SQLite stay consistent.
    #[cfg(feature = "persistence")]
    pub(super) fn wait_write_capacity(&self) -> bool {
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
    pub(super) fn wait_write_capacity(&self) -> bool {
        true
    }

    #[cfg(all(test, feature = "persistence"))]
    pub(super) fn pause_before_submit(&self) {
        let pause = recover_mutex(&self.before_submit, "points test admission").take();
        if let Some(pause) = pause {
            pause.reached.send(()).expect("test observes admission");
            pause.resume.recv().expect("test resumes admission");
        }
    }

    /// Pushes one op onto the ordered writer queue. Capacity was reserved
    /// through [`Self::wait_write_capacity`] inside the same critical
    /// section that allocated the sequence, so this push cannot block and
    /// cannot reorder.
    #[cfg(feature = "persistence")]
    pub(super) fn submit_points_op(&self, op: PointsWriteOp) {
        if self.database.is_none() {
            return;
        }
        let mut state = self.writer.lock_queue();
        if state.closed {
            // Shutdown shares the mutation lock, so a normal close cannot
            // race a successfully admitted producer. Keep this defensive
            // report for an unexpectedly closed writer.
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
