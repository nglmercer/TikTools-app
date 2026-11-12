//! Public client handle, cooldown tracking and observation/disconnect controls.
use super::config::ensure_tls_provider;
use crate::{
    lifecycle, retry, ClientEvent, ConnectionSnapshot, NativeTikTokClient, NativeTikTokConfig,
    StampedClientEvent, TikTokError, TrackedReceiver,
};
use std::time::Duration;
use tokio::sync::broadcast;

impl NativeTikTokClient {
    pub fn new(config: NativeTikTokConfig) -> Self {
        ensure_tls_provider();
        let events = lifecycle::EventSender::new();
        Self {
            config,
            events,
            owner: lifecycle::ConnectionOwner::new(),
            cooldown: retry::Cooldown::default(),
        }
    }

    pub(in super::super) async fn check_cooldown(&self) -> Result<(), TikTokError> {
        if let Some(remaining) = self.cooldown.remaining().await {
            return Err(TikTokError::RateLimited {
                retry_after: Some(remaining),
            });
        }
        Ok(())
    }

    pub(in super::super) async fn remember_failure(&self, error: &TikTokError) {
        if let TikTokError::RateLimited { retry_after } = error {
            self.cooldown
                .extend(retry_after.unwrap_or(Duration::from_secs(30)))
                .await;
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ClientEvent> {
        self.events.subscribe()
    }

    pub fn subscribe_stamped(&self) -> broadcast::Receiver<StampedClientEvent> {
        self.events.subscribe_stamped()
    }

    /// Subscribe to the same bounded broadcast with advisory, generation-
    /// scoped attribution for skipped events when its history is sufficient.
    pub fn subscribe_stamped_with_loss(&self) -> TrackedReceiver {
        self.events.subscribe_stamped_with_loss()
    }

    pub fn connection_snapshot(&self) -> ConnectionSnapshot {
        self.owner.snapshot()
    }

    pub async fn disconnect_native(&self) -> Result<(), TikTokError> {
        let generation = self.owner.disconnect().await?;
        self.events.send(
            generation,
            ClientEvent::Disconnected {
                reason: "disconnect requested".into(),
            },
        );
        Ok(())
    }
}

impl Default for NativeTikTokClient {
    fn default() -> Self {
        Self::new(NativeTikTokConfig::default())
    }
}
