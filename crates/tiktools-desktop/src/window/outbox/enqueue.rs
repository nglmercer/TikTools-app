//! Reliable and lossy lane admission, coalescing and overflow accounting.
use super::*;

impl WebviewOutbox {
    /// Enqueues an already-parsed notification: one serialization, zero
    /// parses. The domain forwarder uses this so the hot path never
    /// round-trips through a string.
    pub(crate) fn push_value(&mut self, value: &serde_json::Value) {
        if self.transport_failed {
            self.dropped_while_failed = self.dropped_while_failed.saturating_add(1);
            return;
        }
        match QueuedWebviewMessage::from_value(value, None) {
            Ok(message) => self.push_prepared(message),
            Err(_) => self.fail(),
        }
    }

    /// Producer-only parser; the UI receives prepared classification, routing,
    /// and JavaScript literals without another parse/serialize round-trip.
    pub(crate) fn push(&mut self, body: String) {
        if self.transport_failed {
            self.dropped_while_failed = self.dropped_while_failed.saturating_add(1);
            return;
        }
        if body.len() > MAX_OWNER_RPC_BYTES {
            self.fail();
            return;
        }
        match serde_json::from_str::<serde_json::Value>(&body) {
            Ok(value) => match QueuedWebviewMessage::from_value(&value, Some(body)) {
                Ok(message) => self.push_prepared(message),
                Err(_) => self.fail(),
            },
            Err(_) if body.len() > MAX_OUTBOUND_BYTES => self.fail(),
            Err(_) => match encode_with_limit(&body, 2 * MAX_OUTBOUND_BYTES + 2) {
                Ok(literal) => self.push_prepared(QueuedWebviewMessage {
                    body: body.into(),
                    javascript_literal: Some(literal.into()),
                    class: WebviewMessageClass::Critical,
                    coalesce_key: None,
                    coalesce_generation: None,
                    ranking_revision: None,
                    plugin: None,
                }),
                Err(_) => self.fail(),
            },
        }
    }

    /// Host-generated native broker envelopes need no UI-thread classification.
    pub(crate) fn push_critical(&mut self, body: std::sync::Arc<str>) {
        if body.len() > MAX_OUTBOUND_BYTES {
            self.fail();
            return;
        }
        self.push_prepared(QueuedWebviewMessage {
            body,
            javascript_literal: None,
            class: WebviewMessageClass::Critical,
            coalesce_key: None,
            coalesce_generation: None,
            ranking_revision: None,
            plugin: None,
        });
    }

    pub(crate) fn push_prepared(&mut self, message: QueuedWebviewMessage) {
        if self.transport_failed {
            self.dropped_while_failed = self.dropped_while_failed.saturating_add(1);
            return;
        }
        if message.class == WebviewMessageClass::Critical {
            self.push_reliable(message);
            return;
        }
        if let Some(key) = message.coalesce_key.as_deref() {
            if let Some((instance, revision)) = &message.ranking_revision {
                if self.lossy.iter().any(|queued| {
                    queued.coalesce_key.as_deref() == Some(key)
                        && queued.ranking_revision.as_ref().is_some_and(
                            |(prior_instance, prior_revision)| {
                                prior_instance == instance && prior_revision > revision
                            },
                        )
                }) {
                    return;
                }
            }
            if let Some(generation) = message.coalesce_generation {
                if self.lossy.iter().any(|queued| {
                    queued.coalesce_key.as_deref() == Some(key)
                        && queued
                            .coalesce_generation
                            .is_some_and(|prior| prior > generation)
                }) {
                    return;
                }
            }
            self.lossy.retain(|queued| {
                let keep = queued.coalesce_key.as_deref() != Some(key);
                if !keep {
                    self.lossy_bytes -= queued.bytes();
                }
                keep
            });
        }
        self.lossy_bytes += message.bytes();
        self.lossy.push_back(message);
        self.shed_lossy_overflow();
        self.record_high_water();
    }

    /// Reliable-lane enqueue shared by both entry points: never shed, with
    /// the loud-backlog warnings and the fail-loud safety policy.
    pub(super) fn push_reliable(&mut self, body: QueuedWebviewMessage) {
        if self.transport_failed {
            self.dropped_while_failed += 1;
            if self.dropped_while_failed == 1 || self.dropped_while_failed.is_multiple_of(256) {
                tracing::error!(
                    dropped = self.dropped_while_failed,
                    "WebView transport failed; refusing reliable backlog for a dead page"
                );
            }
            return;
        }
        if self.reliable.len() >= MAX_RELIABLE_MESSAGES
            || body.bytes() > MAX_RELIABLE_BYTES.saturating_sub(self.reliable_bytes)
        {
            self.fail();
            tracing::error!("WebView reliable admission exhausted; failing transport without retaining overflow");
            return;
        }
        self.reliable_bytes += body.bytes();
        self.reliable.push_back(body);
        self.record_high_water();
        // Loud backlog instead of silent drops: warn once when the
        // budget is crossed, then once per additional budget of lag.
        let backlog = self.reliable.len();
        if backlog == MAX_RELIABLE_WEBVIEW_BACKLOG + 1
            || (backlog > MAX_RELIABLE_WEBVIEW_BACKLOG
                && backlog.is_multiple_of(MAX_RELIABLE_WEBVIEW_BACKLOG))
        {
            tracing::error!(
                backlog,
                "WebView reliable backlog growing; RPC responses and transitions are held, not dropped"
            );
        }
    }

    /// Bounded lossy lane: shed oldest droppable feed first, then the
    /// oldest snapshot. Reliable messages live in the other lane and
    /// are never candidates here.
    pub(super) fn shed_lossy_overflow(&mut self) {
        while self.lossy.len() > MAX_PENDING_WEBVIEW_MESSAGES || self.lossy_bytes > MAX_LOSSY_BYTES
        {
            if let Some(index) = self
                .lossy
                .iter()
                .position(|queued| queued.class == WebviewMessageClass::Droppable)
            {
                if let Some(item) = self.lossy.remove(index) {
                    self.lossy_bytes -= item.bytes();
                }
            } else if let Some(item) = self.lossy.pop_front() {
                self.lossy_bytes -= item.bytes();
            }
            self.lossy_dropped = self.lossy_dropped.saturating_add(1);
        }
    }
}
