//! Transport adaptation only. RPC negotiation, limits and recovery stay shared.
use super::*;

pub(super) enum Socket {
    Ws(Box<WebSocketStream<MaybeTlsStream<TcpStream>>>),
    #[cfg(feature = "peer")]
    Peer(PeerSocket),
    #[cfg(feature = "peer")]
    Rtc {
        framed: tiktools_rtc::Transport,
        remote: tiktools_peer::EndpointId,
    },
}

impl From<WebSocketStream<MaybeTlsStream<TcpStream>>> for Socket {
    fn from(socket: WebSocketStream<MaybeTlsStream<TcpStream>>) -> Self {
        Self::Ws(Box::new(socket))
    }
}

#[cfg(feature = "peer")]
pub(super) struct PeerSocket {
    pub framed: tiktools_peer::Framed,
    pub connection: tiktools_peer::PeerConnection,
    pub _endpoint: tiktools_peer::PeerEndpoint,
}

#[cfg(feature = "peer")]
impl Drop for PeerSocket {
    fn drop(&mut self) {
        self.connection.close();
    }
}

impl Socket {
    #[cfg(feature = "peer")]
    pub fn uses_rtc(&self) -> bool {
        matches!(self, Self::Rtc { .. })
    }
    #[cfg(feature = "peer")]
    pub fn peer_route(&self) -> Option<tiktools_peer::Route> {
        match self {
            Self::Peer(socket) => Some(socket.connection.route()),
            Self::Rtc { framed, .. } => framed.route().map(|route| match route {
                tiktools_rtc::Route::Direct => tiktools_peer::Route::Direct,
                tiktools_rtc::Route::Relay => tiktools_peer::Route::Relay,
            }),
            _ => None,
        }
    }
    #[cfg(feature = "peer")]
    pub fn peer_endpoint_id(&self) -> Option<tiktools_peer::EndpointId> {
        match self {
            Self::Peer(socket) => Some(socket.connection.remote_id()),
            Self::Rtc { remote, .. } => Some(*remote),
            _ => None,
        }
    }
    pub async fn send(&mut self, message: Message) -> Result<(), ()> {
        match self {
            Self::Ws(socket) => socket.send(message).await.map_err(|_| ()),
            #[cfg(feature = "peer")]
            Self::Peer(socket) => {
                use tiktools_peer::Frame;
                let frame = match message {
                    Message::Text(text) => Frame::Text(text.to_string()),
                    Message::Ping(_) => Frame::Ping,
                    Message::Pong(_) => Frame::Pong,
                    Message::Close(_) => Frame::Close,
                    _ => return Err(()),
                };
                socket.framed.send(frame).await.map_err(|_| ())
            }
            #[cfg(feature = "peer")]
            Self::Rtc { framed, .. } => {
                use tiktools_rtc::Frame;
                let frame = match message {
                    Message::Text(text) => Frame::Text(text.to_string()),
                    Message::Ping(_) => Frame::Ping,
                    Message::Pong(_) => Frame::Pong,
                    Message::Close(_) => Frame::Close,
                    _ => return Err(()),
                };
                framed.send(frame).await.map_err(|_| ())
            }
        }
    }

    pub async fn next(&mut self) -> Option<Result<Message, ()>> {
        match self {
            Self::Ws(socket) => socket.next().await.map(|result| result.map_err(|_| ())),
            #[cfg(feature = "peer")]
            Self::Peer(socket) => {
                use tiktools_peer::Frame;
                Some(
                    socket
                        .framed
                        .recv()
                        .await
                        .map(|frame| match frame {
                            Frame::Text(text) => Message::Text(text.into()),
                            Frame::Ping => Message::Ping(Vec::new().into()),
                            Frame::Pong => Message::Pong(Vec::new().into()),
                            Frame::Close => Message::Close(None),
                        })
                        .map_err(|_| ()),
                )
            }
            #[cfg(feature = "peer")]
            Self::Rtc { framed, .. } => {
                use tiktools_rtc::Frame;
                Some(
                    framed
                        .recv()
                        .await
                        .map(|frame| match frame {
                            Frame::Text(text) => Message::Text(text.into()),
                            Frame::Ping => Message::Ping(Vec::new().into()),
                            Frame::Pong => Message::Pong(Vec::new().into()),
                            Frame::Close => Message::Close(None),
                        })
                        .map_err(|_| ()),
                )
            }
        }
    }
}
