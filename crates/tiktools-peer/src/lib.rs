//! Native encrypted connections. Transport identity never implies RPC authority.
mod codec;
mod identity;
#[cfg(test)]
mod relay_tests;
#[cfg(test)]
mod tests;
pub use codec::{Frame, Framed, MAX_FRAME_BYTES};
pub use identity::{load_key, load_or_create_key};
use iroh::{
    endpoint::{presets, QuicTransportConfig},
    Endpoint, RelayMode,
};
pub use iroh::{EndpointAddr, EndpointId, RelayUrl, SecretKey, TransportAddr};
use serde::{Deserialize, Serialize};
use std::{path::Path, time::Duration};

pub const ALPN: &[u8] = b"tiktools/collaboration/1";
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid peer transport configuration")]
    Configuration,
    #[error("peer transport connection failed")]
    Transport,
    #[error("peer frame exceeds its budget or has invalid framing")]
    Frame,
    #[error("peer transport identity is invalid or not private")]
    Identity,
    #[error("peer transport storage is unavailable")]
    Storage(#[from] std::io::Error),
}

/// No public third-party relay/discovery is implicitly enabled.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub relays: Vec<String>,
}

#[derive(Clone)]
pub struct PeerEndpoint {
    endpoint: Endpoint,
}

impl PeerEndpoint {
    pub async fn open(key_file: &Path, config: &Config) -> Result<Self, Error> {
        Self::bind(load_or_create_key(key_file)?, config).await
    }

    pub async fn bind(key: SecretKey, config: &Config) -> Result<Self, Error> {
        if config.relays.len() > 4 {
            return Err(Error::Configuration);
        }
        let mut relays = Vec::new();
        for raw in &config.relays {
            let url = url::Url::parse(raw).map_err(|_| Error::Configuration)?;
            let relay: RelayUrl = raw.parse().map_err(|_| Error::Configuration)?;
            if raw.len() > 2048
                || url.scheme() != "https"
                || url.host().is_none()
                || !url.username().is_empty()
                || url.password().is_some()
                || url.query().is_some()
                || url.fragment().is_some()
                || url.path() != "/"
            {
                return Err(Error::Configuration);
            }
            relays.push(relay);
        }
        let relay_mode = if relays.is_empty() {
            RelayMode::Disabled
        } else {
            RelayMode::Custom(relays.into_iter().collect())
        };
        let endpoint = Endpoint::builder(presets::Minimal)
            .secret_key(key)
            // One reliable control stream, no media or unsolicited uni streams.
            // Bound QUIC buffering independently of application frame parsing.
            .transport_config(
                QuicTransportConfig::builder()
                    .max_concurrent_bidi_streams(1u32.into())
                    .max_concurrent_uni_streams(0u32.into())
                    .stream_receive_window((2 * MAX_FRAME_BYTES as u32).into())
                    .receive_window((2 * MAX_FRAME_BYTES as u32).into())
                    .build(),
            )
            .alpns(vec![ALPN.to_vec()])
            .relay_mode(relay_mode)
            .bind()
            .await
            .map_err(|_| Error::Transport)?;
        Ok(Self { endpoint })
    }

    /// Proof of possession for trusted native Cloud registration.
    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        self.endpoint.secret_key().sign(message).to_bytes()
    }

    pub fn address(&self) -> EndpointAddr {
        self.endpoint.addr()
    }

    pub fn id(&self) -> EndpointId {
        self.endpoint.id()
    }

    pub async fn connect(&self, address: EndpointAddr) -> Result<PeerConnection, Error> {
        let connection =
            tokio::time::timeout(CONNECT_TIMEOUT, self.endpoint.connect(address, ALPN))
                .await
                .map_err(|_| Error::Transport)?
                .map_err(|_| Error::Transport)?;
        Ok(PeerConnection { connection })
    }

    pub async fn accept(&self) -> Result<PeerConnection, Error> {
        let incoming = self.incoming().await.ok_or(Error::Transport)?;
        Self::finish_accept(incoming).await
    }

    pub async fn incoming(&self) -> Option<iroh::endpoint::Incoming> {
        self.endpoint.accept().await
    }

    pub async fn finish_accept(
        incoming: iroh::endpoint::Incoming,
    ) -> Result<PeerConnection, Error> {
        let connection = tokio::time::timeout(CONNECT_TIMEOUT, incoming)
            .await
            .map_err(|_| Error::Transport)?
            .map_err(|_| Error::Transport)?;
        Ok(PeerConnection { connection })
    }

    pub async fn close(&self) {
        self.endpoint.close().await;
    }
}

#[derive(Clone)]
pub struct PeerConnection {
    connection: iroh::endpoint::Connection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Route {
    Direct,
    Relay,
    Reconnecting,
}

impl PeerConnection {
    pub fn route(&self) -> Route {
        for path in self.connection.paths().iter() {
            if path.is_selected() {
                return if path.is_relay() {
                    Route::Relay
                } else {
                    Route::Direct
                };
            }
        }
        Route::Reconnecting
    }
    pub fn remote_id(&self) -> EndpointId {
        self.connection.remote_id()
    }

    pub async fn open(&self) -> Result<Framed, Error> {
        let (send, recv) = self
            .connection
            .open_bi()
            .await
            .map_err(|_| Error::Transport)?;
        Ok(Framed::new(send, recv))
    }

    pub async fn accept(&self) -> Result<Framed, Error> {
        let (send, recv) = self
            .connection
            .accept_bi()
            .await
            .map_err(|_| Error::Transport)?;
        Ok(Framed::new(send, recv))
    }

    pub fn close(&self) {
        self.connection.close(0u32.into(), b"closed");
    }
}
