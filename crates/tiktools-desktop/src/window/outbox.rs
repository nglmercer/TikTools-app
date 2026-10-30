use std::io::Write;

/// Stop serialization before a host-created JSON string exceeds its item
/// budget. The source value is owned by its producer, not by this transport.
pub(crate) fn encode_bounded(value: &impl serde::Serialize) -> Result<String, serde_json::Error> {
    encode_with_limit(value, MAX_OUTBOUND_BYTES)
}

fn encode_with_limit(
    value: &impl serde::Serialize,
    limit: usize,
) -> Result<String, serde_json::Error> {
    struct Bounded {
        bytes: Vec<u8>,
        limit: usize,
    }
    impl Write for Bounded {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > self.limit.saturating_sub(self.bytes.len()) {
                return Err(std::io::Error::other(
                    "outbound payload exceeds transport limit",
                ));
            }
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut output = Bounded {
        bytes: Vec::new(),
        limit,
    };
    serde_json::to_writer(&mut output, value)?;
    Ok(String::from_utf8(output.bytes).expect("JSON serializer produces UTF-8"))
}

// Two-lane WebView outbox with coalescing and drop policies.

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
pub(crate) const MAX_BATCH_BYTES: usize = 4 * 1024 * 1024;
pub(crate) const MAX_OUTBOUND_BYTES: usize = 1024 * 1024;
pub(crate) const MAX_LOSSY_BYTES: usize = 8 * 1024 * 1024;

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
    if topic == "ranking.updated" {
        let ranking = value.pointer("/params/data/snapshot/rankingId")?.as_str()?;
        return Some(format!("domain:{topic}:{ranking}"));
    }
    if topic == "competition.result" {
        let competition = value
            .pointer("/params/data/result/competitionId")?
            .as_str()?;
        return Some(format!("domain:{topic}:{competition}"));
    }
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
            "live-event" | "points-awarded" => WebviewMessageClass::Droppable,
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
            "room.stats"
            | "analytics.updated"
            | "gifts.catalog"
            | "ranking.updated"
            | "competition.result"
            | "live.likes-ranking"
            | "live.likes-ranking-full"
            | "ui.points.snapshot"
            | "ui.automation.runs"
            | "ui.projection.stats" => WebviewMessageClass::Coalescable,
            "live.event" | "live.ui-event" => WebviewMessageClass::Droppable,
            // Connection/lifecycle/state transitions, errors, shutdown,
            // and unknown topics: never evicted.
            _ => WebviewMessageClass::Critical,
        };
    }
    // Unrecognized shapes default to Critical per the core rule.
    WebviewMessageClass::Critical
}

/// String entry point kept for the classification contract tests; the
/// production paths parse once (`push`) or never (`push_value`).
#[cfg(test)]
pub(crate) fn classify_webview_message(message: &str) -> WebviewMessageClass {
    match serde_json::from_str::<serde_json::Value>(message) {
        Ok(value) => classify_value(&value),
        Err(_) => WebviewMessageClass::Critical,
    }
}

/// One lossy-lane message with its delivery class computed once at
/// enqueue time, so saturation scans never re-parse JSON on the UI thread.
#[derive(Debug, Clone)]
pub(crate) struct QueuedWebviewMessage {
    pub(crate) body: std::sync::Arc<str>,
    pub(crate) javascript_literal: Option<std::sync::Arc<str>>,
    class: WebviewMessageClass,
    coalesce_key: Option<String>,
    /// Persisted competition generation, used only for committed reveals.
    coalesce_generation: Option<u64>,
    /// Revisions are comparable only within the same ranking instance.
    ranking_revision: Option<(String, u64)>,
    pub(crate) plugin: Option<PluginRoute>,
}

#[derive(Debug, Clone)]
pub(crate) struct PluginRoute {
    pub(crate) plugin_id: String,
    pub(crate) topic: String,
}

impl std::ops::Deref for QueuedWebviewMessage {
    type Target = str;
    fn deref(&self) -> &str {
        &self.body
    }
}

impl QueuedWebviewMessage {
    fn bytes(&self) -> usize {
        self.body.len()
            + self
                .javascript_literal
                .as_ref()
                .map_or(0, |value| value.len())
            + self.coalesce_key.as_ref().map_or(0, String::len)
            + self.ranking_revision.as_ref().map_or(0, |(id, _)| id.len())
            + self
                .plugin
                .as_ref()
                .map_or(0, |route| route.plugin_id.len() + route.topic.len())
    }

    fn from_value(
        value: &serde_json::Value,
        body: Option<String>,
    ) -> Result<Self, serde_json::Error> {
        let plugin = plugin_route(value);
        #[derive(serde::Serialize)]
        struct Compatible<'a> {
            #[serde(rename = "apiVersion")]
            version: u32,
            #[serde(flatten)]
            value: &'a serde_json::Value,
        }
        let body = if plugin.is_some() && value.get("apiVersion").is_none() {
            encode_bounded(&Compatible { version: 1, value })?
        } else if let Some(body) = body {
            body
        } else {
            encode_bounded(value)?
        };
        let javascript_literal = encode_with_limit(&body, 2 * MAX_OUTBOUND_BYTES + 2)?;
        Ok(Self {
            body: body.into(),
            javascript_literal: Some(javascript_literal.into()),
            class: classify_value(value),
            coalesce_key: webview_coalesce_key(value),
            coalesce_generation: (value
                .pointer("/params/topic")
                .and_then(serde_json::Value::as_str)
                == Some("competition.result"))
            .then(|| {
                value
                    .pointer("/params/data/result/generation")
                    .and_then(serde_json::Value::as_u64)
            })
            .flatten(),
            ranking_revision: (value
                .pointer("/params/topic")
                .and_then(serde_json::Value::as_str)
                == Some("ranking.updated"))
            .then(|| {
                let id = value
                    .pointer("/params/data/snapshot/instanceId")?
                    .as_str()?;
                if id.len() != 32 || !id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                    return None;
                }
                Some((
                    id.to_owned(),
                    value.pointer("/params/data/snapshot/revision")?.as_u64()?,
                ))
            })
            .flatten(),
            plugin,
        })
    }
}

fn plugin_route(value: &serde_json::Value) -> Option<PluginRoute> {
    if value.get("method")?.as_str()? != "event" {
        return None;
    }
    let params = value.get("params")?;
    let topic = params.get("topic")?.as_str()?;
    if !topic.starts_with("plugin.") || topic.len() > 128 {
        return None;
    }
    let plugin_id = params.get("data")?.get("pluginId")?.as_str()?;
    if !crate::plugin_webview::is_valid_ui_id(plugin_id) {
        return None;
    }
    Some(PluginRoute {
        plugin_id: plugin_id.to_owned(),
        topic: topic.to_owned(),
    })
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
    pub(crate) reliable: std::collections::VecDeque<QueuedWebviewMessage>,
    pub(crate) lossy: std::collections::VecDeque<QueuedWebviewMessage>,
    /// Serialized bytes currently held in the reliable lane.
    reliable_bytes: usize,
    lossy_bytes: usize,
    high_water_bytes: usize,
    high_water_count: usize,
    pub(crate) lossy_dropped: u64,
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
        })
    }

    fn record_high_water(&mut self) {
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

    pub(crate) fn fail(&mut self) {
        self.transport_failed = true;
        self.failure_pending = true;
    }

    /// Producer-only parser; the UI receives prepared classification, routing,
    /// and JavaScript literals without another parse/serialize round-trip.
    pub(crate) fn push(&mut self, body: String) {
        if self.transport_failed {
            self.dropped_while_failed = self.dropped_while_failed.saturating_add(1);
            return;
        }
        if body.len() > MAX_OUTBOUND_BYTES {
            self.fail();
            return;
        }
        match serde_json::from_str::<serde_json::Value>(&body) {
            Ok(value) => match QueuedWebviewMessage::from_value(&value, Some(body)) {
                Ok(message) => self.push_prepared(message),
                Err(_) => self.fail(),
            },
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
    fn push_reliable(&mut self, body: QueuedWebviewMessage) {
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
    fn shed_lossy_overflow(&mut self) {
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

    /// Takes at most one UI tick's worth of messages: reliable first,
    /// then lossy fill. FIFO within each lane.
    #[cfg(test)]
    pub(crate) fn take_batch(&mut self) -> Vec<String> {
        self.take_prepared_batch()
            .into_iter()
            .map(|message| message.body.to_string())
            .collect()
    }

    pub(crate) fn take_prepared_batch(&mut self) -> Vec<QueuedWebviewMessage> {
        let mut batch = Vec::new();
        let mut bytes = 0;
        while batch.len() < MAX_BATCH_PER_TICK {
            if self
                .reliable
                .front()
                .is_some_and(|body| body.bytes() > MAX_BATCH_BYTES.saturating_sub(bytes))
            {
                break;
            }
            if let Some(body) = self.reliable.pop_front() {
                self.reliable_bytes = self.reliable_bytes.saturating_sub(body.bytes());
                bytes += body.bytes();
                batch.push(body);
            } else {
                break;
            }
        }
        while batch.len() < MAX_BATCH_PER_TICK {
            if self
                .lossy
                .front()
                .is_some_and(|item| item.bytes() > MAX_BATCH_BYTES.saturating_sub(bytes))
            {
                break;
            }
            if let Some(queued) = self.lossy.pop_front() {
                self.lossy_bytes -= queued.bytes();
                bytes += queued.bytes();
                batch.push(queued);
            } else {
                break;
            }
        }
        batch
    }
}

/// Bounds high-rate domain payloads before they enter Winit's proxy queue.
/// A single payload-free wake represents the whole mailbox.
#[derive(Default)]
pub(crate) struct ForwardMailbox {
    outbox: WebviewOutbox,
    wake_pending: bool,
    reported_lossy_dropped: u64,
}

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

#[cfg(test)]
mod mailbox_tests {
    use super::*;

    #[test]
    fn high_rate_snapshots_have_one_wake_and_one_latest_snapshot() {
        let mut mailbox = ForwardMailbox::default();
        for n in 0..10_000 {
            let wake = mailbox.push_value(&serde_json::json!({"type":"room-stats", "viewers":n}));
            assert_eq!(wake, n == 0);
        }
        let (batch, failed, wake) = mailbox.drain();
        assert!(!failed);
        assert!(!wake);
        assert_eq!(batch.len(), 1);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&batch[0]).unwrap()["viewers"],
            9999
        );
    }

    #[test]
    fn value_path_classifies_without_reparsing() {
        let mut mailbox = ForwardMailbox::default();
        // Critical via the value path lands in the reliable lane.
        assert!(mailbox.push_value(&serde_json::json!({"type": "rpc-response", "id": 1})));
        // Feed via the value path is droppable; snapshots coalesce.
        let feed = serde_json::json!({"method": "event", "params": {"topic": "live.ui-event"}});
        for _ in 0..10 {
            mailbox.push_value(&feed);
        }
        let stats =
            serde_json::json!({"method": "event", "params": {"topic": "room.stats", "n": 1}});
        mailbox.push_value(&stats);
        let stats2 =
            serde_json::json!({"method": "event", "params": {"topic": "room.stats", "n": 2}});
        mailbox.push_value(&stats2);
        let (batch, failed, wake) = mailbox.drain();
        assert!(!failed);
        assert!(!wake);
        // 1 reliable + 10 feed + 1 coalesced snapshot (n=2 replaced n=1).
        assert_eq!(batch.len(), 12);
        assert!(batch[0].contains("rpc-response"));
        assert!(
            batch
                .iter()
                .filter(|line| line.contains("room.stats"))
                .count()
                == 1
        );
        assert!(batch.last().is_some_and(|line| line.contains("\"n\":2")));
    }

    #[test]
    fn forced_teardown_while_saturated_releases_everything() {
        let mut mailbox = ForwardMailbox::default();
        // Saturate the reliable lane past the safety policy while feed
        // keeps arriving, as during teardown under load.
        for index in 0..MAX_RELIABLE_MESSAGES + 64 {
            mailbox.push_value(&serde_json::json!({"type": "rpc-response", "id": index}));
        }
        for _ in 0..MAX_PENDING_WEBVIEW_MESSAGES + 64 {
            mailbox.push_value(
                &serde_json::json!({"method": "event", "params": {"topic": "live.ui-event"}}),
            );
        }
        // Forced teardown: the page is gone mid-burst.
        let (batch, failed, wake) = mailbox.drain();
        assert!(failed, "saturation must surface as transport failure");
        assert!(
            batch.is_empty(),
            "failed transport clears instead of delivering"
        );
        assert!(!wake);
        // Recovery bounds the fresh page: functional, empty, and quiet.
        mailbox.recover();
        assert!(mailbox.push_value(&serde_json::json!({"type": "rpc-response", "id": "after"})));
        let (batch, failed, wake) = mailbox.drain();
        assert!(!failed);
        assert!(!wake);
        assert_eq!(batch.len(), 1);
    }

    #[test]
    fn reliable_proxy_backlog_fails_loudly_and_recovers() {
        let mut mailbox = ForwardMailbox::default();
        for _ in 0..MAX_RELIABLE_MESSAGES + 1 {
            mailbox.push_value(&serde_json::json!({}));
        }
        let (batch, failed, wake) = mailbox.drain();
        assert!(failed);
        assert!(batch.is_empty());
        assert!(!wake);
        assert!(mailbox.outbox.transport_failed());
        mailbox.recover();
        assert!(mailbox.push_value(&serde_json::json!({})));
        assert_eq!(mailbox.drain().0.len(), 1);
    }
}

#[cfg(test)]
mod admission_stress {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn paused_ui_concurrent_feed_producers_are_bounded_and_account_loss() {
        let mailbox = Arc::new(Mutex::new(ForwardMailbox::default()));
        let workers: Vec<_> = (0..4)
            .map(|worker| {
                let mailbox = mailbox.clone();
                std::thread::spawn(move || {
                    for id in 0..25_000 {
                        let mut queue = mailbox.lock().unwrap();
                        queue.push_value(
                            &serde_json::json!({"type":"live-event", "worker":worker, "id":id}),
                        );
                        assert!(queue.outbox.lossy.len() <= MAX_PENDING_WEBVIEW_MESSAGES);
                        assert!(queue.outbox.lossy_bytes <= MAX_LOSSY_BYTES);
                    }
                })
            })
            .collect();
        for id in 0..100 {
            mailbox
                .lock()
                .unwrap()
                .push_value(&serde_json::json!({"type":"rpc-response", "id":id}));
        }
        for worker in workers {
            worker.join().unwrap();
        }
        let mut mailbox = mailbox.lock().unwrap();
        assert_eq!(mailbox.outbox.reliable.len(), 100);
        assert_eq!(
            mailbox.take_lossy_drops(),
            100_000 - MAX_PENDING_WEBVIEW_MESSAGES as u64
        );
        assert_eq!(mailbox.take_lossy_drops(), 0);
        assert!(!mailbox.outbox.transport_failed());
        let (batch, failed, _) = mailbox.drain();
        assert!(!failed);
        for (id, body) in batch.iter().take(100).enumerate() {
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(body).unwrap()["id"],
                id
            );
        }
    }

    #[test]
    fn outbound_encoding_stops_at_item_budget_and_shutdown_releases_backlog() {
        let huge = serde_json::json!({"blob": "x".repeat(MAX_OUTBOUND_BYTES + 1)});
        assert!(encode_bounded(&huge).is_err());
        let mut mailbox = ForwardMailbox::default();
        mailbox.push_value(&huge);
        assert!(mailbox.drain().1);
        mailbox.shutdown();
        assert!(!mailbox.push("{}".to_owned()));
        assert!(mailbox.drain().0.is_empty());
    }

    #[test]
    fn lossy_bytes_are_bounded_even_below_count_limit() {
        let mut outbox = WebviewOutbox::new();
        let body = serde_json::json!({"type":"live-event", "blob":"x".repeat(512 * 1024)});
        for _ in 0..100 {
            outbox.push_value(&body);
        }
        assert!(outbox.lossy.len() < MAX_PENDING_WEBVIEW_MESSAGES);
        assert!(outbox.lossy_bytes <= MAX_LOSSY_BYTES);
        assert_eq!(outbox.lossy_dropped + outbox.lossy.len() as u64, 100);
        while !outbox.is_empty() {
            let batch = outbox.take_batch();
            assert!(batch.iter().map(String::len).sum::<usize>() <= MAX_BATCH_BYTES);
        }
        assert_eq!(outbox.lossy_bytes, 0);
    }
}
