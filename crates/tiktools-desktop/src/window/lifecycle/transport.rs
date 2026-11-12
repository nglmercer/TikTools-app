//! Batch acknowledgment, mailbox draining and aggregate diagnostics on the UI thread.
//! Delivery counters and transport failure admission keep their existing ordering.

use super::super::DesktopApp;
use crate::{
    event::{CoalescedSignals, DesktopCommand, DesktopEvent},
    platform,
};
use std::{sync::atomic::Ordering, time::Instant};

impl DesktopApp {
    pub(super) fn acknowledge_host_batch(&mut self) {
        if self.emitter.consume_batch_ack()
            && self.pending_batch.as_ref().is_some_and(|batch| {
                batch.generation == self.emitter.generation.load(Ordering::Acquire)
                    && batch.sequence == self.emitter.batch_sequence.load(Ordering::Acquire)
            })
        {
            if let Some(batch) = self.pending_batch.as_ref() {
                let latency = Instant::now()
                    .saturating_duration_since(batch.deadline - std::time::Duration::from_secs(5))
                    .as_micros() as u64;
                self.emitter
                    .ack_latency_us
                    .store(latency, Ordering::Relaxed);
                self.emitter
                    .ack_latency_max_us
                    .fetch_max(latency, Ordering::Relaxed);
                self.emitter
                    .ack_latency_total_us
                    .fetch_add(latency, Ordering::Relaxed);
            }
            self.pending_batch = None;
            self.emitter.clear_batch();
            self.emitter
                .acknowledged_batches
                .fetch_add(1, Ordering::Relaxed);
        }
    }
    pub(super) fn poll_desktop_diagnostics(&mut self) {
        if self.diagnostics_at.elapsed() >= std::time::Duration::from_secs(10) {
            self.diagnostics_at = Instant::now();
            *self
                .emitter
                .ui_snapshot
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = serde_json::json!({
                "outbox": self.pending_host_messages.diagnostics(),
                "retainedBatchCount": self.pending_batch.as_ref().map(|batch| batch.messages.len()).unwrap_or(0),
                "plugins": self.plugin_ui.diagnostics(),
                "nativeLifecycle": platform::native_lifecycle_diagnostics(),
                "webviewLifecycle": {
                    "state": format!("{:?}", self.reload.state()),
                    "failureReason": self.reload.failure_reason(),
                    "navigationRecoveries": self.reload.recovery_counts().0,
                    "webviewRecreates": self.reload.recovery_counts().1,
                    "pageReady": self.page_ready,
                },
            });
            if self.diagnostic_logging {
                use tiktools_core::HostEmitter;
                tracing::info!(desktop = %self.emitter.diagnostics(), "aggregate desktop diagnostics");
            }
        }
    }
    pub(super) fn drain_domain_mailbox(&mut self) {
        let (messages, failed, wake, dropped) = {
            let mut mailbox = self
                .domain_mailbox
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let (messages, failed, wake) = mailbox.drain();
            (messages, failed, wake, mailbox.take_lossy_drops())
        };
        if dropped > 0 && !failed {
            self.pending_host_messages
                .push_value(&tiktools_control_api::gap_notification(dropped));
        }
        if failed {
            self.core.record_event_gap();
            self.fail_webview_transport("producer mailbox admission exhausted");
        }
        for message in messages {
            self.emit_to_webview(message);
        }
        if wake {
            let _ = self
                .proxy
                .send_coalesced(DesktopEvent::Command(DesktopCommand::FlushWebviewBatch));
        }
    }
}
