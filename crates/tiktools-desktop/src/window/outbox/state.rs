//! Queue health, reset and transport failure state.
use super::*;

impl WebviewOutbox {
    pub(crate) fn diagnostics(&self) -> serde_json::Value {
        serde_json::json!({
            "reliableCount": self.reliable.len(), "reliableBytes": self.reliable_bytes,
            "lossyCount": self.lossy.len(), "lossyBytes": self.lossy_bytes,
            "highWaterBytes": self.high_water_bytes, "highWaterCount": self.high_water_count,
            "lossyDropped": self.lossy_dropped, "failed": self.transport_failed,
            "refusedWhileFailed": self.dropped_while_failed,
            "reliableCapacity": MAX_RELIABLE_MESSAGES, "reliableByteCapacity": MAX_RELIABLE_BYTES,
            "lossyCapacity": MAX_PENDING_WEBVIEW_MESSAGES, "lossyByteCapacity": MAX_LOSSY_BYTES,
            "maxItemBytes": MAX_OUTBOUND_BYTES,
            "maxOwnerRpcBytes": MAX_OWNER_RPC_BYTES,
        })
    }

    pub(super) fn record_high_water(&mut self) {
        self.high_water_bytes = self
            .high_water_bytes
            .max(self.reliable_bytes + self.lossy_bytes);
        self.high_water_count = self
            .high_water_count
            .max(self.reliable.len() + self.lossy.len());
    }

    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.reliable.is_empty() && self.lossy.is_empty()
    }

    pub(crate) fn transport_failed(&self) -> bool {
        self.transport_failed
    }

    #[cfg(test)]
    pub(crate) fn reliable_bytes(&self) -> usize {
        self.reliable_bytes
    }

    /// Takes the failure edge exactly once per trip so the UI loop runs
    /// the fail-loud handling (health + reload) a single time.
    pub(crate) fn take_transport_failure(&mut self) -> bool {
        std::mem::take(&mut self.failure_pending)
    }

    /// Clears both lanes after the transport failed. The page is dead or
    /// rebooting, so retained messages are undeliverable; the rebooted
    /// frontend resyncs authoritative state through its mount reads.
    pub(crate) fn clear_for_reload(&mut self) {
        let reliable = self.reliable.len();
        let lossy = self.lossy.len();
        self.discard_pending();
        tracing::error!(
            reliable,
            lossy,
            "cleared WebView outbox for transport reload"
        );
    }

    pub(crate) fn discard_pending(&mut self) {
        self.reliable.clear();
        self.lossy.clear();
        self.reliable_bytes = 0;
        self.lossy_bytes = 0;
    }

    /// Releases the failure latch after the reloaded page handshakes. New
    /// messages queue again; the rebooted frontend resyncs through its
    /// mount reads.
    pub(crate) fn recover_transport(&mut self) {
        self.transport_failed = false;
        self.dropped_while_failed = 0;
    }

    pub(crate) fn fail(&mut self) {
        self.transport_failed = true;
        self.failure_pending = true;
    }
}
