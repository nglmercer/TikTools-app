//! Native collaboration transports. No host runtime is created by this module.
#[cfg(feature = "peer")]
pub use tiktools_peer::Route as PeerRoute;
#[cfg(feature = "cloud-peer")]
pub mod cloud_peer;
#[cfg(feature = "remote-profiles")]
pub mod controller;
mod credentials;
#[cfg(test)]
mod delegation_tests;
mod invitation;
pub use invitation::parse_invitation;
mod recovery;
pub use recovery::{ReconnectBackoff, RecoveryCursor, RecoveryMode, RecoveryStream};
mod http;
#[cfg(feature = "remote-profiles")]
pub mod profiles;
#[cfg(test)]
pub(super) mod tests;
mod transport;
mod trust;
use crate::ClientError;
pub use credentials::NativeCredentials;
pub use http::{HostInfo, PairingPoll, PendingPairing, RemoteEndpoint};
pub use secrecy::SecretString;
use std::time::{SystemTime, UNIX_EPOCH};
pub use tiktools_collaboration::{
    ClientKind, ReliableEvent, ResumeParams, SnapshotParams, StreamDomain, SubscribeParams,
};
pub use transport::RemoteClient;
pub use trust::{normalize_fingerprint, probe_fingerprint, TlsTrust};

pub(super) fn fail(code: &str, message: &str) -> ClientError {
    ClientError::new(code, message)
}
pub(super) fn protocol() -> ClientError {
    fail(
        "protocol",
        "remote host returned an invalid protocol message",
    )
}
pub(super) fn network() -> ClientError {
    fail(
        "transport",
        "remote transport failed; no request was retried",
    )
}
pub(super) fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
}
pub(super) fn valid_id(id: &str) -> bool {
    uuid::Uuid::parse_str(id).is_ok_and(|v| v.to_string() == id)
}
pub(super) fn secret() -> Result<SecretString, ClientError> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes)
        .map_err(|_| fail("internal", "secure randomness is unavailable"))?;
    Ok(SecretString::from(
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
    ))
}

pub(super) struct LimitedBuffer(pub Vec<u8>, pub usize);
impl std::io::Write for LimitedBuffer {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.0.len().saturating_add(bytes.len()) > self.1 {
            return Err(std::io::Error::other("message exceeds configured limit"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
