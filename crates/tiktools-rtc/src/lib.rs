//! Rust-only WebRTC transport. A signed negotiation identifies the peer; the
//! owning host still selects its local grant before accepting business RPC.
pub mod cloud_socket;
mod codec;
pub mod negotiation;
mod peer;
pub mod relay;
mod route;
pub use route::Route;
pub mod signaling;
mod transport;
pub use codec::{Frame, Framed, MAX_FRAME_BYTES};
pub use peer::{Config, Event, IceServer, Peer, Signal};
pub use transport::{Channel, Transport};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid WebRTC configuration")]
    Configuration,
    #[error("WebRTC transport failed or closed")]
    Transport,
    #[error("invalid or oversized data-channel frame")]
    Frame,
    #[error("WebRTC negotiation is out of order")]
    Negotiation,
}
