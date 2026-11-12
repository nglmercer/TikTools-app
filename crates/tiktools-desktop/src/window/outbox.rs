mod drain;
mod enqueue;
mod mailbox;
mod messages;
mod state;
mod wire;

#[cfg(test)]
pub(crate) use messages::classify_webview_message;
pub(crate) use wire::{encode_bounded, encode_owner_rpc_bounded};
use wire::{encode_with_limit, message_limit};

use std::io::Write;

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

pub(crate) const MAX_RELIABLE_BYTES: usize = 32 * 1024 * 1024;

/// Maximum messages delivered in one UI tick through one `evaluate_script`.
pub(crate) const MAX_BATCH_PER_TICK: usize = 128;
pub(crate) const MAX_BATCH_BYTES: usize = 4 * 1024 * 1024;
pub(crate) const MAX_OUTBOUND_BYTES: usize = 1024 * 1024;
pub(crate) const MAX_OWNER_RPC_BYTES: usize = tiktools_control_api::MAX_OWNER_RESPONSE_BYTES;
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

/// Bounds high-rate domain payloads before they enter Winit's proxy queue.
/// A single payload-free wake represents the whole mailbox.
#[derive(Default)]
pub(crate) struct ForwardMailbox {
    outbox: WebviewOutbox,
    wake_pending: bool,
    reported_lossy_dropped: u64,
}

#[cfg(test)]
mod admission_stress;
#[cfg(test)]
mod mailbox_tests;
