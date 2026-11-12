//! Host message forwarding and bounded leaderboard snapshot publication.
use super::*;

impl AppCore {
    pub fn emit(&self, message: HostMessage) {
        self.emitter.emit(message);
    }

    /// Publishes a leaderboard snapshot at a bounded rate. Callers that need
    /// an immediate snapshot (for example an explicit UI request) still emit
    /// directly; high-rate live events use this coalescing boundary.
    pub(crate) fn emit_leaderboard_if_due(&self) {
        if !self.emitter.wants_leaderboard_snapshots() {
            return;
        }
        const MIN_INTERVAL_MS: u64 = 250;
        let now = now_millis();
        let last = self.last_leaderboard_emit_at.load(Ordering::Acquire);
        if last != 0 && now.saturating_sub(last) < MIN_INTERVAL_MS {
            return;
        }
        if self
            .last_leaderboard_emit_at
            .compare_exchange(last, now, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            self.emit(HostMessage::Leaderboard {
                viewers: self.points.leaderboard(Some(50)),
            });
        }
    }
}
