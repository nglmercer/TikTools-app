//! Only completed, device-authenticated transports enter the shared RPC engine.
use crate::{
    negotiation::AuthenticatedChannel,
    relay::{io, RelayFramed, SecureChannel},
    signaling::Context,
    Error, Frame, Framed, Peer, Route,
};

pub enum Channel {
    Direct(Box<AuthenticatedChannel>),
    Relay(Box<SecureChannel<io::Stream>>),
}
impl From<AuthenticatedChannel> for Channel {
    fn from(channel: AuthenticatedChannel) -> Self {
        Self::Direct(Box::new(channel))
    }
}
impl From<SecureChannel<io::Stream>> for Channel {
    fn from(channel: SecureChannel<io::Stream>) -> Self {
        Self::Relay(Box::new(channel))
    }
}
impl Channel {
    pub fn local_context(&self) -> &Context {
        match self {
            Self::Direct(c) => c.local_context(),
            Self::Relay(c) => c.local_context(),
        }
    }
    pub fn remote_context(&self) -> &Context {
        match self {
            Self::Direct(c) => c.remote_context(),
            Self::Relay(c) => c.remote_context(),
        }
    }
    pub fn local_endpoint_id(&self) -> &str {
        match self {
            Self::Direct(c) => c.local_endpoint_id(),
            Self::Relay(c) => c.local_endpoint_id(),
        }
    }
    pub fn remote_endpoint_id(&self) -> &str {
        match self {
            Self::Direct(c) => c.remote_endpoint_id(),
            Self::Relay(c) => c.remote_endpoint_id(),
        }
    }
    pub fn into_transport(self) -> Transport {
        match self {
            Self::Direct(c) => {
                let (owner, framed) = c.into_transport();
                Transport::Direct { owner, framed }
            }
            Self::Relay(c) => Transport::Relay(Box::new(c.into_framed())),
        }
    }
}
pub enum Transport {
    Direct { owner: Peer, framed: Framed },
    Relay(Box<RelayFramed<io::Stream>>),
}
impl Transport {
    pub fn route(&self) -> Option<Route> {
        match self {
            Self::Direct { owner, .. } => owner.route(),
            Self::Relay(_) => Some(Route::Relay),
        }
    }
    pub async fn send(&mut self, frame: Frame) -> Result<(), Error> {
        match self {
            Self::Direct { framed, .. } => framed.send(frame).await,
            Self::Relay(framed) => framed.send(frame).await,
        }
    }
    pub async fn recv(&mut self) -> Result<Frame, Error> {
        match self {
            Self::Direct { framed, .. } => framed.recv().await,
            Self::Relay(framed) => framed.recv().await,
        }
    }
}
