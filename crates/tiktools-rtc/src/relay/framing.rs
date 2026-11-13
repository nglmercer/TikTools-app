//! Bounded RPC framing above TLS. Partial reads survive cancellation; partial
//! writes poison the stream so an uncertain mutation is never replayed here.
use crate::{Error, Frame, MAX_FRAME_BYTES};
use bytes::{Buf, BytesMut};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio_rustls::TlsStream;

pub struct RelayFramed<S> {
    stream: TlsStream<S>,
    incoming: BytesMut,
    sending: bool,
    failed: bool,
}
impl<S: AsyncRead + AsyncWrite + Unpin> RelayFramed<S> {
    pub(super) fn new(stream: TlsStream<S>) -> Self {
        Self {
            stream,
            incoming: BytesMut::new(),
            sending: false,
            failed: false,
        }
    }

    pub async fn send(&mut self, frame: Frame) -> Result<(), Error> {
        if self.sending || self.failed {
            return Err(Error::Transport);
        }
        let (kind, payload) = match frame {
            Frame::Text(s) => (0, s.into_bytes()),
            Frame::Ping => (1, vec![]),
            Frame::Pong => (2, vec![]),
            Frame::Close => (3, vec![]),
        };
        if payload.len() > MAX_FRAME_BYTES {
            return Err(Error::Frame);
        }
        self.sending = true;
        let mut header = [0; 5];
        header[0] = kind;
        header[1..].copy_from_slice(&(payload.len() as u32).to_be_bytes());
        self.stream
            .write_all(&header)
            .await
            .map_err(|_| Error::Transport)?;
        self.stream
            .write_all(&payload)
            .await
            .map_err(|_| Error::Transport)?;
        self.stream.flush().await.map_err(|_| Error::Transport)?;
        self.sending = false;
        if kind == 3 {
            self.failed = true;
        }
        Ok(())
    }

    pub async fn recv(&mut self) -> Result<Frame, Error> {
        if self.failed {
            return Err(Error::Transport);
        }
        let result = self.receive().await;
        if result.is_err() || matches!(result, Ok(Frame::Close)) {
            self.failed = true;
        }
        result
    }

    async fn receive(&mut self) -> Result<Frame, Error> {
        loop {
            if self.incoming.len() >= 5 {
                let kind = self.incoming[0];
                let len =
                    u32::from_be_bytes(self.incoming[1..5].try_into().map_err(|_| Error::Frame)?)
                        as usize;
                if kind > 3 || len > MAX_FRAME_BYTES || (kind != 0 && len != 0) {
                    return Err(Error::Frame);
                }
                if self.incoming.len() >= len + 5 {
                    self.incoming.advance(5);
                    let payload = self.incoming.split_to(len);
                    return Ok(match kind {
                        0 => Frame::Text(
                            String::from_utf8(payload.to_vec()).map_err(|_| Error::Frame)?,
                        ),
                        1 => Frame::Ping,
                        2 => Frame::Pong,
                        3 => Frame::Close,
                        _ => return Err(Error::Frame),
                    });
                }
            }
            // read_buf is cancellation safe. Limit each allocation/read even
            // when an untrusted peer advertises a maximum-sized frame.
            let remaining = MAX_FRAME_BYTES + 5 - self.incoming.len();
            let n = (&mut self.stream)
                .take(remaining.min(16 * 1024) as u64)
                .read_buf(&mut self.incoming)
                .await
                .map_err(|_| Error::Transport)?;
            if n == 0 {
                return Err(Error::Transport);
            }
        }
    }
}
