//! Transport health operations: IPC/WebView degradation flags and
//! reliable-lane event-gap counters reported through `system.health`.

use crate::*;

impl AppCore {
    /// Records a control IPC failure so `system.health` reports degraded
    /// instead of silently losing CLI/agent connectivity while the GUI
    /// looks healthy. `None` clears the degraded state after a retry.
    pub fn set_ipc_error(&self, message: Option<String>) {
        *recover_rwlock_write(&self.transport_state.ipc_error, "ipc error") = message;
    }

    pub fn ipc_error(&self) -> Option<String> {
        recover_rwlock_read(&self.transport_state.ipc_error, "ipc error").clone()
    }

    /// Records a broken WebView transport (reliable outbox overflow) so
    /// `system.health` reports degraded instead of silently losing the UI
    /// while RPCs fail. `None` clears the degraded state after recovery.
    pub fn set_webview_error(&self, message: Option<String>) {
        *recover_rwlock_write(&self.transport_state.webview_error, "webview error") = message;
    }

    pub fn webview_error(&self) -> Option<String> {
        recover_rwlock_read(&self.transport_state.webview_error, "webview error").clone()
    }

    /// Records one reliable-lane event gap observed by a transport. The
    /// transport already sent the affected client an explicit `event.gap`
    /// notification; this cumulative counter plus timestamp lets
    /// `system.health` tell agents their local state may be stale.
    pub fn record_event_gap(&self) {
        self.transport_state
            .event_gaps
            .fetch_add(1, Ordering::Relaxed);
        self.transport_state
            .last_event_gap_at
            .store(now_millis(), Ordering::Relaxed);
    }

    /// Cumulative reliable gaps plus the most recent gap timestamp
    /// (`None` when no gap was recorded since boot/acknowledge).
    pub fn event_gap_stats(&self) -> (u64, Option<u64>) {
        let gaps = self.transport_state.event_gaps.load(Ordering::Relaxed);
        let at = self
            .transport_state
            .last_event_gap_at
            .load(Ordering::Relaxed);
        (gaps, if at == 0 { None } else { Some(at) })
    }

    /// Clears recorded event gaps after every affected client resynced
    /// authoritative state, returning health to OK. Only call this once
    /// resync completed; clearing early hides staleness from agents that
    /// have not refreshed yet.
    pub fn acknowledge_event_gaps(&self) {
        self.transport_state.event_gaps.store(0, Ordering::Relaxed);
        self.transport_state
            .last_event_gap_at
            .store(0, Ordering::Relaxed);
    }
}
