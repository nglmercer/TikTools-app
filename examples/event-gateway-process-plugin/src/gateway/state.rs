//! Shared server state and event broadcast.

use super::config::GatewayConfig;
use std::sync::Arc;
use tiktools_plugin_sdk::DomainEventEnvelope;
use tokio::sync::{broadcast, Notify, Semaphore};

const EVENT_CHANNEL_CAPACITY: usize = 256;

/// Upper bound on concurrent client connections. Accepted sockets beyond
/// this limit are dropped instead of spawning unbounded tasks.
const MAX_CONCURRENT_CONNECTIONS: usize = 64;

#[derive(Clone)]
pub(crate) struct GatewayState {
    pub(crate) config: Arc<GatewayConfig>,
    pub(crate) events: broadcast::Sender<DomainEventEnvelope>,
    pub(crate) server_shutdown: Arc<Notify>,
    pub(crate) clients_shutdown: Arc<Notify>,
    pub(crate) connection_permits: Arc<Semaphore>,
}

impl GatewayState {
    pub(crate) fn new(config: GatewayConfig) -> Arc<Self> {
        let (events, _) = broadcast::channel(EVENT_CHANNEL_CAPACITY);
        Arc::new(Self {
            config: Arc::new(config),
            events,
            server_shutdown: Arc::new(Notify::new()),
            clients_shutdown: Arc::new(Notify::new()),
            connection_permits: Arc::new(Semaphore::new(MAX_CONCURRENT_CONNECTIONS)),
        })
    }

    pub(crate) fn publish(&self, event: DomainEventEnvelope) {
        let _ = self.events.send(event);
    }
}
