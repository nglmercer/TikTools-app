use super::*;

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

impl PointsService {
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
}
