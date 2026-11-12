//! Signed socket creation and reconnecting socket ownership.
use crate::{retry, ClientEvent, ConnectionInfo, ConnectionState, TikTokError};
use std::sync::Arc;

pub(in super::super) struct ManagedConnection {
    pub(in super::super) connection: Option<ttl_live_ws::LiveConnection>,
    pub(in super::super) backend: Arc<dyn ttl_sign_core::SignerBackend>,
    pub(in super::super) room_id: String,
    pub(in super::super) retry: retry::RetryContext,
    pub(in super::super) info: ConnectionInfo,
}

impl ManagedConnection {
    pub(in super::super) async fn next_message(
        &mut self,
    ) -> Option<Result<ttl_live_ws::LiveMessage, TikTokError>> {
        loop {
            let current = self.connection.as_mut()?;
            let cause = match current.next_message().await {
                Some(Ok(message)) => return Some(Ok(message)),
                Some(Err(error)) => TikTokError::from_socket(error),
                None => TikTokError::RetryableTransport("socket closed".into()),
            };
            self.connection.take();
            if !matches!(
                cause,
                TikTokError::RetryableTransport(_) | TikTokError::RateLimited { .. }
            ) {
                return Some(Err(cause));
            }
            let mut first = Some(cause);
            let result = self
                .retry
                .run(|| {
                    let initial = first.take();
                    let backend = &self.backend;
                    let room_id = &self.room_id;
                    async move {
                        if let Some(error) = initial {
                            Err(error)
                        } else {
                            open_socket(backend.as_ref(), room_id).await
                        }
                    }
                })
                .await;
            match result {
                Ok(connection) => {
                    self.connection = Some(connection);
                    self.retry
                        .state
                        .transition(self.retry.generation, ConnectionState::Connected);
                    self.retry.events.send(
                        self.retry.generation,
                        ClientEvent::Connected(self.info.clone()),
                    );
                }
                Err(error) => return Some(Err(error)),
            }
        }
    }

    pub(in super::super) async fn close(mut self) {
        if let Some(connection) = self.connection.take() {
            connection.close().await;
        }
    }
}

// Only the application retries. Every attempt asks the upstream signer for a
// fresh URL; the SDK still owns framing, ACKs, heartbeats, and socket cleanup.
pub(in super::super) async fn open_socket(
    backend: &dyn ttl_sign_core::SignerBackend,
    room_id: &str,
) -> Result<ttl_live_ws::LiveConnection, TikTokError> {
    let signed = match backend
        .transport(ttl_sign_core::TransportRequest::new(room_id))
        .await
    {
        ttl_sign_core::SignOutcome::Ok(signed) => signed,
        ttl_sign_core::SignOutcome::Rejected(ttl_sign_core::RejectReason::HttpStatus(429)) => {
            return Err(TikTokError::RateLimited { retry_after: None })
        }
        ttl_sign_core::SignOutcome::Rejected(ttl_sign_core::RejectReason::HttpStatus(
            status @ 500..=599,
        )) => {
            return Err(TikTokError::RetryableTransport(format!(
                "signing endpoint returned HTTP {status}"
            )))
        }
        ttl_sign_core::SignOutcome::Rejected(reason) => {
            return Err(TikTokError::Signer(reason.to_string()))
        }
        ttl_sign_core::SignOutcome::Transport(ttl_sign_core::SignError::Transport(message)) => {
            return Err(TikTokError::RetryableTransport(message))
        }
        ttl_sign_core::SignOutcome::Transport(error) => {
            return Err(TikTokError::Signer(error.to_string()))
        }
    };
    ttl_live_ws::LiveConnection::open_uri(
        &signed.signed_url,
        &signed.cookies,
        &signed.user_agent,
        "",
        &ttl_live_ws::ConnectConfig::default(),
    )
    .await
    .map_err(TikTokError::from_socket)
}
