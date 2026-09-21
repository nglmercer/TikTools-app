//! Application event bus independent of the desktop event loop.
//!
//! [`DomainEvent`] is the single authoritative event bus: every client
//! (WebView, CLI, stdio, local IPC, tests, agents) observes the same typed
//! topics. There is no UI-oriented bus; legacy `HostMessage` pushes stay on
//! the emitter path, never on this bus.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::broadcast;

/// Domain event observed identically by every control client.
///
/// The serde form is `{ "topic": "<dotted.topic>", "data": { ... } }`, which
/// matches the JSON-RPC event notification envelope used on stdio and IPC.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "topic", content = "data")]
pub enum DomainEvent {
    #[serde(rename = "plugin.installed", rename_all = "camelCase")]
    PluginInstalled { plugin_id: String },
    #[serde(rename = "plugin.uninstalled", rename_all = "camelCase")]
    PluginUninstalled { plugin_id: String },
    #[serde(rename = "plugin.started", rename_all = "camelCase")]
    PluginStarted { plugin_id: String },
    #[serde(rename = "plugin.stopped", rename_all = "camelCase")]
    PluginStopped { plugin_id: String },
    #[serde(rename = "plugin.progress", rename_all = "camelCase")]
    PluginProgress {
        plugin_id: String,
        state: String,
        progress: Option<f32>,
        message: String,
    },
    #[serde(rename = "plugin.settings-changed", rename_all = "camelCase")]
    PluginSettingsChanged { plugin_id: String },
    /// Validated spontaneous plugin event (hotkeys, timers, watchers),
    /// published immediately after poll/validation and before automation
    /// enrichment/execution, so control-plane subscribers observe raw
    /// plugin events even when automation drops, throttles, or fails.
    #[serde(rename = "plugin.event", rename_all = "camelCase")]
    PluginEvent {
        plugin_id: String,
        event_type: String,
        event: serde_json::Value,
    },
    /// Plugin listener health snapshot (for example `hotkey.status`
    /// payloads), published alongside the typed plugin event so UIs can
    /// render backend state without polling.
    #[serde(rename = "plugin.status", rename_all = "camelCase")]
    PluginStatus {
        plugin_id: String,
        status: serde_json::Value,
    },
    /// One automation action finished (ok or error). Published with every
    /// recorded run so subscribers observe completions without polling.
    #[serde(rename = "automation.run.completed", rename_all = "camelCase")]
    AutomationRunCompleted { run: serde_json::Value },
    /// The authoritative recent-runs list changed. Carries the same list
    /// as the legacy `behavior-runs` push; new subscribers should use
    /// this topic instead of the compatibility push.
    #[serde(rename = "automation.runs.changed", rename_all = "camelCase")]
    AutomationRunsChanged { runs: Vec<serde_json::Value> },
    #[serde(rename = "live.connected", rename_all = "camelCase")]
    LiveConnected {
        unique_id: Option<String>,
        room_id: Option<String>,
    },
    #[serde(rename = "live.disconnected")]
    LiveDisconnected,
    #[serde(rename = "live.event", rename_all = "camelCase")]
    LiveEvent {
        event_type: String,
        event: serde_json::Value,
    },
    /// UI-ready live event (render shape), published alongside the
    /// automation-shaped `live.event` so the frontend can migrate off the
    /// legacy `live-event` push without depending on automation capacity.
    #[serde(rename = "live.ui-event", rename_all = "camelCase")]
    LiveUiEvent { event: serde_json::Value },
    #[serde(rename = "room.stats", rename_all = "camelCase")]
    RoomStats {
        viewers: u64,
        total_users: u64,
        top_viewers: Vec<serde_json::Value>,
    },
    #[serde(rename = "gifts.catalog", rename_all = "camelCase")]
    GiftsCatalog { gifts: Vec<serde_json::Value> },
    #[serde(rename = "live.reconnecting", rename_all = "camelCase")]
    LiveReconnecting { attempt: u64, delay_ms: u64 },
    #[serde(rename = "live.error", rename_all = "camelCase")]
    LiveError { phase: String, message: String },
    #[serde(rename = "points.changed", rename_all = "camelCase")]
    PointsChanged {
        unique_id: String,
        delta: f64,
        total_points: f64,
        level: u32,
    },
    #[serde(rename = "workflow.changed", rename_all = "camelCase")]
    WorkflowChanged {
        kind: String,
        id: String,
        change: String,
    },
    #[serde(rename = "creator.changed", rename_all = "camelCase")]
    CreatorChanged { unique_id: Option<String> },
    #[serde(rename = "analytics.updated", rename_all = "camelCase")]
    AnalyticsUpdated { creator_unique_id: String },
    #[serde(rename = "shutdown")]
    Shutdown,
}

impl DomainEvent {
    pub fn topic(&self) -> &'static str {
        match self {
            Self::PluginInstalled { .. } => "plugin.installed",
            Self::PluginUninstalled { .. } => "plugin.uninstalled",
            Self::PluginStarted { .. } => "plugin.started",
            Self::PluginStopped { .. } => "plugin.stopped",
            Self::PluginProgress { .. } => "plugin.progress",
            Self::PluginSettingsChanged { .. } => "plugin.settings-changed",
            Self::PluginEvent { .. } => "plugin.event",
            Self::PluginStatus { .. } => "plugin.status",
            Self::AutomationRunCompleted { .. } => "automation.run.completed",
            Self::AutomationRunsChanged { .. } => "automation.runs.changed",
            Self::LiveConnected { .. } => "live.connected",
            Self::LiveDisconnected => "live.disconnected",
            Self::LiveEvent { .. } => "live.event",
            Self::LiveUiEvent { .. } => "live.ui-event",
            Self::RoomStats { .. } => "room.stats",
            Self::GiftsCatalog { .. } => "gifts.catalog",
            Self::LiveReconnecting { .. } => "live.reconnecting",
            Self::LiveError { .. } => "live.error",
            Self::PointsChanged { .. } => "points.changed",
            Self::WorkflowChanged { .. } => "workflow.changed",
            Self::CreatorChanged { .. } => "creator.changed",
            Self::AnalyticsUpdated { .. } => "analytics.updated",
            Self::Shutdown => "shutdown",
        }
    }
}

/// Stable conversion failure when a [`DomainEvent`] cannot be serialized
/// into its transport-neutral envelope. This indicates an invariant
/// violation (every variant must serialize); callers must log it and,
/// for reliable events, record a gap instead of delivering `null` data.
#[derive(Debug, thiserror::Error)]
pub enum EventEnvelopeError {
    #[error("domain event failed to serialize: {0}")]
    Serialization(String),
    #[error("serialized domain event is missing its data payload")]
    MissingData,
}

impl DomainEvent {
    /// Converts this event into its transport-neutral envelope
    /// (`{ topic, data }`), preserving the exact wire shape every client
    /// already receives. Never substitutes `null` on failure.
    pub fn to_envelope(
        &self,
    ) -> Result<tiktools_plugin_api::DomainEventEnvelope, EventEnvelopeError> {
        let value = serde_json::to_value(self)
            .map_err(|error| EventEnvelopeError::Serialization(error.to_string()))?;
        let data = match (value.get("data").cloned(), self) {
            (Some(data), _) => data,
            // Adjacently-tagged unit variants serialize as `{"topic": ...}`
            // with no content payload; their envelope data is null by
            // definition, matching what subscribers already receive.
            (None, Self::LiveDisconnected | Self::Shutdown) => Value::Null,
            (None, _) => return Err(EventEnvelopeError::MissingData),
        };
        Ok(tiktools_plugin_api::DomainEventEnvelope::new(
            self.topic(),
            data,
        ))
    }
}

impl DomainEvent {
    /// High-rate feed and superseding snapshots (live feed, room stats,
    /// analytics, gift catalog, plugin progress) travel the lossy lane:
    /// a lagged subscriber skips them and takes the latest. Every other
    /// event — connection/lifecycle transitions, errors, points changes,
    /// shutdown — travels the reliable lane, so a feed burst can never
    /// overwrite a state transition a subscriber has not read yet.
    /// Mirrors the WebView `Coalescable`/`Droppable` classes.
    pub fn is_lossy(&self) -> bool {
        matches!(
            self,
            Self::LiveEvent { .. }
                | Self::LiveUiEvent { .. }
                | Self::RoomStats { .. }
                | Self::AnalyticsUpdated { .. }
                | Self::GiftsCatalog { .. }
                | Self::PluginProgress { .. }
        )
    }
}

#[derive(Clone)]
pub struct EventBus {
    reliable: broadcast::Sender<DomainEvent>,
    lossy: broadcast::Sender<DomainEvent>,
}

impl EventBus {
    /// `capacity` sizes the reliable lane; the lossy lane gets four times
    /// that for high-rate feed bursts.
    pub fn new(capacity: usize) -> Self {
        let (reliable, _) = broadcast::channel(capacity);
        let (lossy, _) = broadcast::channel(capacity.saturating_mul(4).max(1));
        Self { reliable, lossy }
    }

    /// Publishes to the reliable or lossy lane by event class. Like
    /// `broadcast::send`, this never blocks and never fails for a slow
    /// subscriber: laggards skip on their own cursor only.
    pub fn publish_domain(&self, event: DomainEvent) {
        if event.is_lossy() {
            let _ = self.lossy.send(event);
        } else {
            let _ = self.reliable.send(event);
        }
    }

    pub fn subscribe_domain(&self) -> DomainSubscription {
        DomainSubscription {
            reliable: self.reliable.subscribe(),
            lossy: self.lossy.subscribe(),
        }
    }
}

/// Lane-aware `recv` failure. A lagged lane reports once and then
/// streams its retained backlog; the other lane is unaffected. Reliable
/// lag is never silent: the caller must emit an explicit gap/resync
/// signal so clients refresh authoritative state instead of assuming a
/// complete stream. Lossy lag only skipped feed/snapshots and needs no
/// signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainRecvError {
    /// Authoritative events were lost on the reliable lane; `u64` counts
    /// the skipped messages. The caller must send a gap notification.
    ReliableLagged(u64),
    /// Feed/snapshot messages were skipped on the lossy lane; safe to
    /// log at debug level and continue with no gap signal.
    LossyLagged(u64),
    /// The bus is gone and both lanes are drained.
    Closed,
}

/// Lane-aware `try_recv` failure. See [`DomainRecvError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainTryRecvError {
    /// Authoritative events were lost on the reliable lane; `u64` counts
    /// the skipped messages. The caller must send a gap notification and
    /// keep draining, never stop.
    ReliableLagged(u64),
    /// Feed/snapshot messages were skipped on the lossy lane; safe to
    /// continue draining with no gap signal.
    LossyLagged(u64),
    Empty,
    Closed,
}

/// Merged view over the reliable and lossy lanes. `recv`/`try_recv`
/// report lane-aware lag (recoverable: the caller handles the lag and
/// continues) or `Closed` (the bus is gone). Reliable messages are
/// always drained first, so a state transition never waits behind feed
/// backlog.
pub struct DomainSubscription {
    reliable: broadcast::Receiver<DomainEvent>,
    lossy: broadcast::Receiver<DomainEvent>,
}

impl DomainSubscription {
    /// Receives the next event, reliable lane first. A lagged lane
    /// reports once and then streams its retained backlog; the other
    /// lane is unaffected.
    pub async fn recv(&mut self) -> Result<DomainEvent, DomainRecvError> {
        match self.try_recv() {
            Err(DomainTryRecvError::Empty) => {}
            Ok(event) => return Ok(event),
            Err(DomainTryRecvError::ReliableLagged(skipped)) => {
                return Err(DomainRecvError::ReliableLagged(skipped));
            }
            Err(DomainTryRecvError::LossyLagged(skipped)) => {
                return Err(DomainRecvError::LossyLagged(skipped));
            }
            Err(DomainTryRecvError::Closed) => return Err(DomainRecvError::Closed),
        }
        // Both lanes are empty: wait biased toward reliable. A lossy
        // wake may overtake a reliable message published a moment later;
        // the next `recv` takes reliable first, so control events stay
        // prompt while feed stays merely ordered.
        tokio::select! {
            biased;
            message = self.reliable.recv() => {
                match message {
                    // The bus died mid-wait: drain lossy stragglers buffered
                    // before the drop instead of losing them to Closed.
                    Err(broadcast::error::RecvError::Closed) => {
                        match self.lossy.try_recv() {
                            Ok(straggler) => Ok(straggler),
                            Err(broadcast::error::TryRecvError::Lagged(skipped)) => {
                                Err(DomainRecvError::LossyLagged(skipped))
                            }
                            Err(_) => Err(DomainRecvError::Closed),
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        Err(DomainRecvError::ReliableLagged(skipped))
                    }
                    Ok(event) => Ok(event),
                }
            }
            message = self.lossy.recv() => {
                match message {
                    // The reliable lane may still hold buffered messages
                    // (or stay open), so never strand them behind a lossy
                    // close: fall back to the reliable backlog first.
                    Err(broadcast::error::RecvError::Closed) => match self.reliable.try_recv() {
                        Ok(event) => Ok(event),
                        Err(broadcast::error::TryRecvError::Lagged(skipped)) => {
                            Err(DomainRecvError::ReliableLagged(skipped))
                        }
                        Err(_) => Err(DomainRecvError::Closed),
                    },
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        Err(DomainRecvError::LossyLagged(skipped))
                    }
                    Ok(event) => Ok(event),
                }
            }
        }
    }

    pub fn try_recv(&mut self) -> Result<DomainEvent, DomainTryRecvError> {
        use broadcast::error::TryRecvError;
        match self.reliable.try_recv() {
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Lagged(skipped)) => {
                return Err(DomainTryRecvError::ReliableLagged(skipped));
            }
            Err(TryRecvError::Closed) => {
                // The bus is gone: drain lossy stragglers before Closed.
                return match self.lossy.try_recv() {
                    Ok(event) => Ok(event),
                    Err(TryRecvError::Empty) | Err(TryRecvError::Closed) => {
                        Err(DomainTryRecvError::Closed)
                    }
                    Err(TryRecvError::Lagged(skipped)) => {
                        Err(DomainTryRecvError::LossyLagged(skipped))
                    }
                };
            }
            Ok(event) => return Ok(event),
        }
        match self.lossy.try_recv() {
            // Both lanes empty and the reliable sender is alive (it would
            // report Closed, not Empty, otherwise), so more events can
            // still arrive: report Empty, never Closed. Both senders live
            // in one `EventBus` and drop together, so a lossy Closed here
            // is unreachable; map it to Empty for the same reason.
            Err(TryRecvError::Empty) | Err(TryRecvError::Closed) => Err(DomainTryRecvError::Empty),
            Err(TryRecvError::Lagged(skipped)) => Err(DomainTryRecvError::LossyLagged(skipped)),
            Ok(event) => Ok(event),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One representative value per variant plus its stable topic and a
    /// data-shape assertion. Adding a variant without extending this table
    /// fails the count assertion in the conversion test below.
    type VariantCase = (DomainEvent, &'static str, fn(&Value));

    fn all_variant_cases() -> Vec<VariantCase> {
        vec![
            (
                DomainEvent::PluginInstalled {
                    plugin_id: "demo".to_owned(),
                },
                "plugin.installed",
                |data| assert_eq!(data["pluginId"], "demo"),
            ),
            (
                DomainEvent::PluginUninstalled {
                    plugin_id: "demo".to_owned(),
                },
                "plugin.uninstalled",
                |data| assert_eq!(data["pluginId"], "demo"),
            ),
            (
                DomainEvent::PluginStarted {
                    plugin_id: "demo".to_owned(),
                },
                "plugin.started",
                |data| assert_eq!(data["pluginId"], "demo"),
            ),
            (
                DomainEvent::PluginStopped {
                    plugin_id: "demo".to_owned(),
                },
                "plugin.stopped",
                |data| assert_eq!(data["pluginId"], "demo"),
            ),
            (
                DomainEvent::PluginProgress {
                    plugin_id: "demo".to_owned(),
                    state: "running".to_owned(),
                    progress: Some(0.5),
                    message: "half".to_owned(),
                },
                "plugin.progress",
                |data| {
                    assert_eq!(data["pluginId"], "demo");
                    assert_eq!(data["progress"], 0.5);
                },
            ),
            (
                DomainEvent::PluginSettingsChanged {
                    plugin_id: "demo".to_owned(),
                },
                "plugin.settings-changed",
                |data| assert_eq!(data["pluginId"], "demo"),
            ),
            (
                DomainEvent::PluginEvent {
                    plugin_id: "demo".to_owned(),
                    event_type: "hotkey.pressed".to_owned(),
                    event: serde_json::json!({"key": "k"}),
                },
                "plugin.event",
                |data| {
                    assert_eq!(data["eventType"], "hotkey.pressed");
                    assert_eq!(data["event"]["key"], "k");
                },
            ),
            (
                DomainEvent::PluginStatus {
                    plugin_id: "demo".to_owned(),
                    status: serde_json::json!({"state": "active"}),
                },
                "plugin.status",
                |data| assert_eq!(data["status"]["state"], "active"),
            ),
            (
                DomainEvent::AutomationRunCompleted {
                    run: serde_json::json!({"id": "r1"}),
                },
                "automation.run.completed",
                |data| assert_eq!(data["run"]["id"], "r1"),
            ),
            (
                DomainEvent::AutomationRunsChanged {
                    runs: vec![serde_json::json!({"id": "r1"})],
                },
                "automation.runs.changed",
                |data| assert_eq!(data["runs"][0]["id"], "r1"),
            ),
            (
                DomainEvent::LiveConnected {
                    unique_id: Some("viewer".to_owned()),
                    room_id: None,
                },
                "live.connected",
                |data| assert_eq!(data["uniqueId"], "viewer"),
            ),
            (DomainEvent::LiveDisconnected, "live.disconnected", |data| {
                assert!(data.is_null(), "unit variants carry null data")
            }),
            (
                DomainEvent::LiveEvent {
                    event_type: "chat".to_owned(),
                    event: serde_json::json!({}),
                },
                "live.event",
                |data| assert_eq!(data["eventType"], "chat"),
            ),
            (
                DomainEvent::LiveUiEvent {
                    event: serde_json::json!({"kind": "chat"}),
                },
                "live.ui-event",
                |data| assert_eq!(data["event"]["kind"], "chat"),
            ),
            (
                DomainEvent::RoomStats {
                    viewers: 7,
                    total_users: 9,
                    top_viewers: vec![],
                },
                "room.stats",
                |data| {
                    assert_eq!(data["viewers"], 7);
                    assert_eq!(data["totalUsers"], 9);
                },
            ),
            (
                DomainEvent::GiftsCatalog {
                    gifts: vec![serde_json::json!({"id": 1})],
                },
                "gifts.catalog",
                |data| assert_eq!(data["gifts"][0]["id"], 1),
            ),
            (
                DomainEvent::LiveReconnecting {
                    attempt: 2,
                    delay_ms: 500,
                },
                "live.reconnecting",
                |data| {
                    assert_eq!(data["attempt"], 2);
                    assert_eq!(data["delayMs"], 500);
                },
            ),
            (
                DomainEvent::LiveError {
                    phase: "connect".to_owned(),
                    message: "boom".to_owned(),
                },
                "live.error",
                |data| {
                    assert_eq!(data["phase"], "connect");
                    assert_eq!(data["message"], "boom");
                },
            ),
            (
                DomainEvent::PointsChanged {
                    unique_id: "viewer".to_owned(),
                    delta: 2.0,
                    total_points: 4.0,
                    level: 1,
                },
                "points.changed",
                |data| {
                    assert_eq!(data["uniqueId"], "viewer");
                    assert_eq!(data["level"], 1);
                },
            ),
            (
                DomainEvent::WorkflowChanged {
                    kind: "graph".to_owned(),
                    id: "w1".to_owned(),
                    change: "saved".to_owned(),
                },
                "workflow.changed",
                |data| {
                    assert_eq!(data["id"], "w1");
                    assert_eq!(data["change"], "saved");
                },
            ),
            (
                DomainEvent::CreatorChanged { unique_id: None },
                "creator.changed",
                |data| assert!(data.get("uniqueId").is_some()),
            ),
            (
                DomainEvent::AnalyticsUpdated {
                    creator_unique_id: "creator".to_owned(),
                },
                "analytics.updated",
                |data| assert_eq!(data["creatorUniqueId"], "creator"),
            ),
            (DomainEvent::Shutdown, "shutdown", |data| {
                assert!(data.is_null(), "unit variants carry null data")
            }),
        ]
    }

    #[test]
    fn every_variant_converts_to_a_stable_envelope() {
        let cases = all_variant_cases();
        assert_eq!(
            cases.len(),
            23,
            "a DomainEvent variant is missing test coverage"
        );
        for (event, topic, check_data) in &cases {
            assert_eq!(event.topic(), *topic);
            let envelope = event
                .to_envelope()
                .unwrap_or_else(|error| panic!("{topic} must convert: {error}"));
            assert_eq!(envelope.topic, *topic);
            check_data(&envelope.data);
            // The envelope preserves the exact historical wire shape: the
            // same data every transport already extracted from the event.
            let serialized = serde_json::to_value(event).expect("variant must serialize");
            match serialized.get("data") {
                Some(expected) => assert_eq!(&envelope.data, expected),
                None => assert!(envelope.data.is_null()),
            }
            // The envelope itself round-trips through the plugin boundary.
            let round_trip: tiktools_plugin_api::DomainEventEnvelope = serde_json::from_value(
                serde_json::to_value(&envelope).expect("envelope serializes"),
            )
            .expect("envelope deserializes");
            assert_eq!(round_trip, envelope);
        }
    }

    #[test]
    fn topic_matches_serialized_tag_for_every_variant() {
        for (event, topic, _) in all_variant_cases() {
            let serialized = serde_json::to_value(&event).expect("variant must serialize");
            assert_eq!(serialized.get("topic").and_then(Value::as_str), Some(topic));
        }
    }
}
