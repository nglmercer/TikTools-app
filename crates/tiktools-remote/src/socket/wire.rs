use axum::extract::ws::{Message, WebSocket};

pub(crate) enum Wire {
    WebSocket(Box<WebSocket>),
    #[cfg(feature = "peer")]
    Peer(tiktools_peer::Framed),
    #[cfg(feature = "peer")]
    Rtc(tiktools_rtc::Transport),
}

impl From<WebSocket> for Wire {
    fn from(socket: WebSocket) -> Self {
        Self::WebSocket(Box::new(socket))
    }
}

impl Wire {
    pub async fn recv(&mut self) -> Option<Result<Message, ()>> {
        match self {
            Self::WebSocket(socket) => socket.recv().await.map(|r| r.map_err(|_| ())),
            #[cfg(feature = "peer")]
            Self::Peer(framed) => Some(
                framed
                    .recv()
                    .await
                    .map(|frame| match frame {
                        tiktools_peer::Frame::Text(text) => Message::Text(text.into()),
                        tiktools_peer::Frame::Ping => Message::Ping(Vec::new().into()),
                        tiktools_peer::Frame::Pong => Message::Pong(Vec::new().into()),
                        tiktools_peer::Frame::Close => Message::Close(None),
                    })
                    .map_err(|_| ()),
            ),
            #[cfg(feature = "peer")]
            Self::Rtc(framed) => Some(
                framed
                    .recv()
                    .await
                    .map(|frame| match frame {
                        tiktools_rtc::Frame::Text(text) => Message::Text(text.into()),
                        tiktools_rtc::Frame::Ping => Message::Ping(Vec::new().into()),
                        tiktools_rtc::Frame::Pong => Message::Pong(Vec::new().into()),
                        tiktools_rtc::Frame::Close => Message::Close(None),
                    })
                    .map_err(|_| ()),
            ),
        }
    }

    pub async fn send(&mut self, message: Message) -> Result<(), ()> {
        match self {
            Self::WebSocket(socket) => socket.send(message).await.map_err(|_| ()),
            #[cfg(feature = "peer")]
            Self::Peer(framed) => framed
                .send(match message {
                    Message::Text(text) => tiktools_peer::Frame::Text(text.to_string()),
                    Message::Ping(_) => tiktools_peer::Frame::Ping,
                    Message::Pong(_) => tiktools_peer::Frame::Pong,
                    Message::Close(_) => tiktools_peer::Frame::Close,
                    _ => return Err(()),
                })
                .await
                .map_err(|_| ()),
            #[cfg(feature = "peer")]
            Self::Rtc(framed) => framed
                .send(match message {
                    Message::Text(text) => tiktools_rtc::Frame::Text(text.to_string()),
                    Message::Ping(_) => tiktools_rtc::Frame::Ping,
                    Message::Pong(_) => tiktools_rtc::Frame::Pong,
                    Message::Close(_) => tiktools_rtc::Frame::Close,
                    _ => return Err(()),
                })
                .await
                .map_err(|_| ()),
        }
    }
}
