//! Two-lane WebView outbox with coalescing and drop policies.

/// Bound for the lossy lane (snapshots + feed). Past this point the
/// oldest lossy message is shed — never a reliable one.
pub(crate) const MAX_PENDING_WEBVIEW_MESSAGES: usize = 512;

/// Backlog size at which the reliable lane starts warning loudly. The
/// reliable lane is never dropped: a stuck/slow frontend shows up as a
/// growing backlog in logs instead of silently lost RPC responses.
const MAX_RELIABLE_WEBVIEW_BACKLOG: usize = 1024;

/// Reliable-lane safety policy: past either bound the transport is
/// broken (a stuck page that never drains), so it fails loudly —
/// health degrades, RPC is rejected, the WebView reloads — instead of
/// consuming unlimited memory. Reliable messages are never silently
/// evicted to stay under these bounds.
pub(crate) const MAX_RELIABLE_MESSAGES: usize = 4096;

pub(crate) const MAX_RELIABLE_BYTES: usize = 16 * 1024 * 1024;

/// Maximum messages delivered in one UI tick through one `evaluate_script`.
pub(crate) const MAX_BATCH_PER_TICK: usize = 128;

/// Delivery class for one queued WebView message. This is the single
/// classification shared by the queue policy and the tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WebviewMessageClass {
    /// Never evicted under saturation: RPC responses, lifecycle and
    /// connection transitions, errors, shutdown, and anything unrecognized
    /// (the core rule forbids silently dropping a possible state change).
    Critical,
    /// Snapshots where only the latest per coalesce key is kept.
    Coalescable,
    /// High-rate feed where shedding the oldest under saturation is safe.
    Droppable,
}

/// Coalesce identity for snapshots, e.g. `legacy:room-stats`,
/// `domain:room.stats`, `domain:plugin.progress:<plugin-id>`.
fn webview_coalesce_key(value: &serde_json::Value) -> Option<String> {
    if classify_value(value) != WebviewMessageClass::Coalescable {
        return None;
    }
    if let Some(kind) = value.get("type").and_then(serde_json::Value::as_str) {
        return Some(format!("legacy:{kind}"));
    }
    let topic = value
        .get("params")
        .and_then(|params| params.get("topic"))
        .and_then(serde_json::Value::as_str)?;
    if topic == "plugin.progress" {
        let plugin = value
            .get("params")
            .and_then(|params| params.get("data"))
            .and_then(|data| data.get("pluginId"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("?");
        return Some(format!("domain:{topic}:{plugin}"));
    }
    Some(format!("domain:{topic}"))
}

fn classify_value(value: &serde_json::Value) -> WebviewMessageClass {
    if let Some(kind) = value.get("type").and_then(serde_json::Value::as_str) {
        return match kind {
            // Compat duplicates of authoritative domain twins: safe to shed.
            "live-event" | "points-awarded" | "plugin-progress" => WebviewMessageClass::Droppable,
            "room-stats" | "leaderboard" | "analytics-summary" | "processor-status"
            | "automation-context" | "gift-catalog" => WebviewMessageClass::Coalescable,
            // rpc-response, connection/error/reconnecting transitions, and
            // every other rare state/result message: never evicted.
            _ => WebviewMessageClass::Critical,
        };
    }
    if value.get("method").and_then(serde_json::Value::as_str) == Some("event") {
        let topic = value
            .get("params")
            .and_then(|params| params.get("topic"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        return match topic {
            "room.stats" | "analytics.updated" | "plugin.progress" | "gifts.catalog" => {
                WebviewMessageClass::Coalescable
            }
            "live.event" | "live.ui-event" => WebviewMessageClass::Droppable,
            // Connection/lifecycle/state transitions, errors, shutdown,
            // and unknown topics: never evicted.
            _ => WebviewMessageClass::Critical,
        };
    }
    // Unrecognized shapes default to Critical per the core rule.
    WebviewMessageClass::Critical
}

pub(crate) fn classify_webview_message(message: &str) -> WebviewMessageClass {
    match serde_json::from_str::<serde_json::Value>(message) {
        Ok(value) => classify_value(&value),
        Err(_) => WebviewMessageClass::Critical,
    }
}

/// One lossy-lane message with its delivery class computed once at
/// enqueue time, so saturation scans never re-parse JSON on the UI thread.
#[derive(Debug)]
pub(crate) struct QueuedWebviewMessage {
    pub(crate) body: String,
    class: WebviewMessageClass,
    coalesce_key: Option<String>,
}

/// Two-lane outbox enforcing the core invariant: RPC responses and
/// critical state transitions must never silently disappear.
///
/// * `reliable` holds Critical messages (RPC responses, connection and
///   lifecycle transitions, errors, shutdown). It is never shed: a slow
///   frontend builds a loud backlog instead of losing results, up to the
///   `MAX_RELIABLE_*` safety policy, past which the whole transport
///   fails loudly (health, RPC rejection, reload) instead of growing
///   memory without bound.
/// * `lossy` holds Coalescable snapshots (latest per key wins) and
///   Droppable feed, bounded by `MAX_PENDING_WEBVIEW_MESSAGES` with
///   oldest-first shedding.
///
/// Ordering contract: FIFO within each lane; each batch drains reliable
/// first, then fills the remainder of the tick from lossy. A snapshot or
/// feed item may therefore be delivered after a reliable message that was
/// enqueued later — acceptable because snapshots supersede and feed is
/// explicitly lossy, while results and transitions stay prompt.
#[derive(Debug, Default)]
pub(crate) struct WebviewOutbox {
    pub(crate) reliable: std::collections::VecDeque<String>,
    pub(crate) lossy: std::collections::VecDeque<QueuedWebviewMessage>,
    /// Serialized bytes currently held in the reliable lane.
    reliable_bytes: usize,
    /// Latched when the reliable lane exceeds its safety policy. While
    /// set, new reliable messages are counted (never queued) and inbound
    /// RPC is rejected, until a clean reload recovers the transport.
    transport_failed: bool,
    /// Edge flag: set on the trip, taken by the UI loop to run the
    /// fail-loud handling exactly once per trip.
    failure_pending: bool,
    /// Reliable messages refused while the transport was failed. Counted
    /// and logged, never silent — but the page is dead, so queueing more
    /// would only pin memory no reader can consume.
    pub(crate) dropped_while_failed: u64,
}

impl WebviewOutbox {
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
        self.reliable.clear();
        self.lossy.clear();
        self.reliable_bytes = 0;
        tracing::error!(
            reliable,
            lossy,
            "cleared WebView outbox for transport reload"
        );
    }

    /// Releases the failure latch after the reloaded page handshakes. New
    /// messages queue again; the rebooted frontend resyncs through its
    /// mount reads.
    pub(crate) fn recover_transport(&mut self) {
        self.transport_failed = false;
        self.dropped_while_failed = 0;
    }

    pub(crate) fn push(&mut self, body: String) {
        if classify_webview_message(&body) == WebviewMessageClass::Critical {
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
            self.reliable_bytes += body.len();
            self.reliable.push_back(body);
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
            // Safety policy: never evict to stay under the bounds — fail
            // the whole transport loudly instead.
            if self.reliable.len() > MAX_RELIABLE_MESSAGES
                || self.reliable_bytes > MAX_RELIABLE_BYTES
            {
                self.transport_failed = true;
                self.failure_pending = true;
                tracing::error!(
                    messages = self.reliable.len(),
                    bytes = self.reliable_bytes,
                    "WebView reliable backlog exceeded the safety policy; failing the transport loudly"
                );
            }
            return;
        }
        let coalesce_key = serde_json::from_str::<serde_json::Value>(&body)
            .ok()
            .and_then(|value| webview_coalesce_key(&value));
        let class = classify_webview_message(&body);
        if class == WebviewMessageClass::Coalescable {
            // Keep only the latest snapshot per coalesce key.
            if let Some(key) = coalesce_key.as_deref() {
                self.lossy
                    .retain(|queued| queued.coalesce_key.as_deref() != Some(key));
            }
        }
        self.lossy.push_back(QueuedWebviewMessage {
            body,
            class,
            coalesce_key,
        });
        // Bounded lossy lane: shed oldest droppable feed first, then the
        // oldest snapshot. Reliable messages live in the other lane and
        // are never candidates here.
        while self.lossy.len() > MAX_PENDING_WEBVIEW_MESSAGES {
            if let Some(index) = self
                .lossy
                .iter()
                .position(|queued| queued.class == WebviewMessageClass::Droppable)
            {
                self.lossy.remove(index);
            } else {
                self.lossy.pop_front();
            }
        }
    }

    /// Takes at most one UI tick's worth of messages: reliable first,
    /// then lossy fill. FIFO within each lane.
    pub(crate) fn take_batch(&mut self) -> Vec<String> {
        let mut batch = Vec::new();
        while batch.len() < MAX_BATCH_PER_TICK {
            if let Some(body) = self.reliable.pop_front() {
                self.reliable_bytes = self.reliable_bytes.saturating_sub(body.len());
                batch.push(body);
            } else {
                break;
            }
        }
        while batch.len() < MAX_BATCH_PER_TICK {
            if let Some(queued) = self.lossy.pop_front() {
                batch.push(queued.body);
            } else {
                break;
            }
        }
        batch
    }
}
