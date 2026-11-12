use super::*;

impl PointsInner {
    pub(super) fn with_capacity(capacity: usize) -> Self {
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
    pub(super) fn evict_if_over(&mut self, resolved: u64) {
        if self.viewers.len() > self.capacity + Self::EVICT_BATCH_SLACK {
            self.evict_cold(resolved);
        }
    }

    /// Evicts cold viewers until the cache fits `capacity`. Viewers with
    /// unresolved writes (sequence newer than the resolved watermark) are
    /// never evicted; when every viewer is dirty the cache temporarily
    /// exceeds its bound rather than losing acknowledged updates.
    pub(super) fn evict_cold(&mut self, resolved: u64) {
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
    #[cfg(feature = "persistence")]
    pub(super) fn reload_wait_seq(&self, unique_id: &str) -> u64 {
        self.pending
            .get(unique_id)
            .copied()
            .unwrap_or(0)
            .max(self.reset_pending)
    }
}

impl PointsService {
    /// Resizes the hot-viewer bound. Shrinking below the current size evicts
    /// cold viewers immediately (never ones with unresolved writes); growing
    /// only raises the ceiling, with cold rows reloading from SQLite on demand.
    pub fn set_cache_capacity(&self, capacity: usize) {
        let _mutation = recover_mutex(&self.mutation, "points mutation");
        let capacity = capacity.clamp(1, 1_000_000);
        let resolved = self.resolved_seq();
        let mut inner = recover_rwlock_write(&self.inner, "points viewers");
        inner.capacity = capacity;
        inner.evict_cold(resolved);
    }

    pub fn cache_capacity(&self) -> usize {
        recover_rwlock_read(&self.inner, "points viewers").capacity
    }

    /// Records that a durable write for `unique_id` is outstanding, so
    /// eviction cannot drop the row before it lands. With no database
    /// configured there is no durable write to wait for, so nothing is
    /// tracked — an unretired marker would pin every row in the cache.
    pub(super) fn track_pending(&self, inner: &mut PointsInner, unique_id: &str, seq: u64) {
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
    pub(super) fn resolved_seq(&self) -> u64 {
        self.writer.resolved_seq.load(Ordering::Acquire)
    }

    #[cfg(not(feature = "persistence"))]
    pub(super) fn resolved_seq(&self) -> u64 {
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
    pub(super) fn wait_for_viewer_resolved(&self, unique_id: &str) -> bool {
        let seq = recover_rwlock_read(&self.inner, "points viewers").reload_wait_seq(unique_id);
        if seq == 0 || self.resolved_seq() >= seq {
            return true;
        }
        let deadline = Instant::now()
            + Duration::from_millis(self.writer.admission_timeout_ms.load(Ordering::Relaxed));
        while self.resolved_seq() < seq {
            if Instant::now() >= deadline {
                tracing::warn!(
                    viewer_seq = seq,
                    resolved = self.resolved_seq(),
                    "points reload timed out waiting for queued writes; refusing a stale reload"
                );
                return false;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        true
    }
}
