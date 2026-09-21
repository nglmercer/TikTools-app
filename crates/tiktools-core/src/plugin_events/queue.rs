//! Bounded per-plugin queue types and backpressure metadata.

use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use tiktools_plugin_api::DomainEventEnvelope;
use tokio::sync::mpsc;

/// A slow event observer is isolated behind this queue. `try_send` is used at
/// the bus boundary, so the publisher never waits for a plugin process.
pub(crate) const PLUGIN_EVENT_QUEUE_CAPACITY: usize = 64;

#[derive(Debug, Clone)]
pub(crate) struct QueuedPluginEvent {
    pub(crate) envelope: DomainEventEnvelope,
    pub(crate) lossy: bool,
}

/// Supervisor-owned handle for one plugin delivery worker.
pub(crate) struct WorkerHandle {
    pub(crate) sender: mpsc::Sender<QueuedPluginEvent>,
    /// Child of the observer shutdown token. Cancelled when the worker is
    /// pruned (plugin gone) or when the observer shuts down.
    pub(crate) token: tokio_util::sync::CancellationToken,
    /// Reliable events shed at the queue boundary are summarized and sent as
    /// one stable gap envelope when the worker next gets capacity.
    pub(crate) pending_reliable_gaps: Arc<AtomicU64>,
}
