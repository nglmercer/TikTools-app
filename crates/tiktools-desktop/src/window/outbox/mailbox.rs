//! One-wake forward mailbox ownership and page retirement.
use super::*;

impl ForwardMailbox {
    pub(crate) fn fail(&mut self) -> bool {
        self.outbox.fail();
        self.arm_wake()
    }

    pub(crate) fn retire_page(&mut self) {
        self.outbox.clear_for_reload();
        // Keep wake_pending: an already posted wake remains valid and drains
        // any new-generation producer work. No second wake is needed.
    }

    pub(crate) fn diagnostics(&self) -> serde_json::Value {
        self.outbox.diagnostics()
    }

    pub(crate) fn push(&mut self, body: String) -> bool {
        self.outbox.push(body);
        self.arm_wake()
    }

    fn arm_wake(&mut self) -> bool {
        if self.wake_pending {
            return false;
        }
        if self.outbox.is_empty() && !self.outbox.failure_pending {
            return false;
        }
        self.wake_pending = true;
        true
    }

    pub(crate) fn shutdown(&mut self) {
        self.outbox.clear_for_reload();
        self.outbox.transport_failed = true;
        self.outbox.failure_pending = false;
        self.wake_pending = false;
    }

    /// Value entry point for the domain forwarder: no string round-trip.
    /// This is the only mailbox admission: every producer funnels through
    /// the bounded outbox, never around it.
    pub(crate) fn push_value(&mut self, value: &serde_json::Value) -> bool {
        self.outbox.push_value(value);
        self.arm_wake()
    }

    pub(crate) fn take_lossy_drops(&mut self) -> u64 {
        let dropped = self
            .outbox
            .lossy_dropped
            .saturating_sub(self.reported_lossy_dropped);
        self.reported_lossy_dropped = self.outbox.lossy_dropped;
        dropped
    }

    pub(crate) fn drain(&mut self) -> (Vec<QueuedWebviewMessage>, bool, bool) {
        self.wake_pending = false;
        let failed = self.outbox.take_transport_failure();
        if failed {
            self.outbox.clear_for_reload();
        }
        let batch = self.outbox.take_prepared_batch();
        let wake = !self.outbox.is_empty();
        self.wake_pending = wake;
        (batch, failed, wake)
    }

    pub(crate) fn recover(&mut self) {
        self.outbox.recover_transport();
    }
}
