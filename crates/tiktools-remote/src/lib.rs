//! Optional remote transport. This crate never constructs AppCore, installs
//! plugins or converts a remote request into a trusted-local IPC call.
mod auth;
mod http;
#[cfg(feature = "peer")]
mod peer;
#[cfg(feature = "peer")]
mod rtc;
#[cfg(feature = "cloud-peer")]
pub use rtc::CloudRtcHost;
#[cfg(feature = "peer")]
pub use rtc::RtcGateway;
mod server;
mod socket;
mod state;
#[cfg(feature = "peer")]
pub use peer::{start_peer, PeerGateway};

#[cfg(feature = "browser-app")]
mod browser_assets {
    include!(concat!(env!("OUT_DIR"), "/browser_assets.rs"));
}

pub use server::{start, start_on, supervise, RemoteGateway};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[cfg(feature = "peer")]
    #[error("WebRTC transport is unavailable")]
    Rtc(#[from] tiktools_rtc::Error),
    #[cfg(feature = "peer")]
    #[error("peer transport is unavailable")]
    Peer(#[from] tiktools_peer::Error),
    #[error("collaboration transport is not configured")]
    Configuration,
    #[error("collaboration TLS configuration is invalid or unreadable")]
    Tls,
    #[error("collaboration listener is unavailable")]
    Io(#[from] std::io::Error),
    #[error("collaboration service is unavailable")]
    Service(#[from] tiktools_collaboration::Error),
    #[error("collaboration transport task failed")]
    Task,
}

#[cfg(test)]
pub(crate) const MAX_FRAME: usize = 1024 * 1024;

pub(crate) fn random_secret() -> Result<String, auth::HttpError> {
    let mut bytes = [0; 32];
    getrandom::fill(&mut bytes).map_err(|_| auth::HttpError::internal())?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}
