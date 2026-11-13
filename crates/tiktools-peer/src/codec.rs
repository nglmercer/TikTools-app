use crate::Error;
use iroh::endpoint::{RecvStream, SendStream};

pub const MAX_FRAME_BYTES: usize = 1024 * 1024;

/// Explicit heartbeat frames let existing native RPC semantics share one wire.
pub enum Frame {
    Text(String),
    Ping,
    Pong,
    Close,
}

pub struct Framed {
    send: SendStream,
    recv: RecvStream,
    header: [u8; 5],
    header_read: usize,
    payload: Vec<u8>,
    payload_read: usize,
}

impl Framed {
    pub(crate) fn new(send: SendStream, recv: RecvStream) -> Self {
        Self {
            send,
            recv,
            header: [0; 5],
            header_read: 0,
            payload: Vec::new(),
            payload_read: 0,
        }
    }

    pub async fn send(&mut self, frame: Frame) -> Result<(), Error> {
        let (kind, bytes) = match frame {
            Frame::Text(text) => (0u8, text.into_bytes()),
            Frame::Ping => (1, Vec::new()),
            Frame::Pong => (2, Vec::new()),
            Frame::Close => (3, Vec::new()),
        };
        if bytes.len() > MAX_FRAME_BYTES {
            return Err(Error::Frame);
        }
        self.send
            .write_all(&[kind])
            .await
            .map_err(|_| Error::Transport)?;
        self.send
            .write_all(&(bytes.len() as u32).to_be_bytes())
            .await
            .map_err(|_| Error::Transport)?;
        self.send
            .write_all(&bytes)
            .await
            .map_err(|_| Error::Transport)?;
        Ok(())
    }

    pub async fn recv(&mut self) -> Result<Frame, Error> {
        // All progress lives on self: select!/heartbeat cancellation must not
        // discard a partial header or payload and desynchronize the stream.
        while self.header_read < self.header.len() {
            let count = self
                .recv
                .read(&mut self.header[self.header_read..])
                .await
                .map_err(|_| Error::Transport)?
                .ok_or(Error::Transport)?;
            self.header_read += count;
        }
        let len =
            u32::from_be_bytes(self.header[1..].try_into().map_err(|_| Error::Frame)?) as usize;
        let kind = self.header[0];
        if len > MAX_FRAME_BYTES || kind > 3 || (kind != 0 && len != 0) {
            return Err(Error::Frame);
        }
        if self.payload.len() != len {
            self.payload.resize(len, 0);
        }
        while self.payload_read < len {
            let count = self
                .recv
                .read(&mut self.payload[self.payload_read..])
                .await
                .map_err(|_| Error::Transport)?
                .ok_or(Error::Transport)?;
            self.payload_read += count;
        }
        let bytes = std::mem::take(&mut self.payload);
        self.header_read = 0;
        self.payload_read = 0;
        match kind {
            0 => Ok(Frame::Text(
                String::from_utf8(bytes).map_err(|_| Error::Frame)?,
            )),
            1 => Ok(Frame::Ping),
            2 => Ok(Frame::Pong),
            3 => Ok(Frame::Close),
            _ => Err(Error::Frame),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::pair;
    use std::time::Duration;

    #[tokio::test]
    async fn partial_frame_survives_cancelled_receive_and_oversize_is_rejected_before_allocation() {
        tokio::time::timeout(Duration::from_secs(10), async {
            let (host, client, accepted, connected) = pair().await;
            let mut sender = connected.open().await.unwrap();
            sender.send.write_all(&[0, 0]).await.unwrap();
            let mut receiver = accepted.accept().await.unwrap();
            assert!(
                tokio::time::timeout(Duration::from_millis(50), receiver.recv())
                    .await
                    .is_err()
            );
            sender.send.write_all(&[0, 0, 2, b'o', b'k']).await.unwrap();
            assert!(matches!(receiver.recv().await.unwrap(), Frame::Text(s) if s == "ok"));
            sender.send.write_all(&[0]).await.unwrap();
            sender
                .send
                .write_all(&((MAX_FRAME_BYTES + 1) as u32).to_be_bytes())
                .await
                .unwrap();
            assert!(matches!(receiver.recv().await, Err(Error::Frame)));
            assert!(receiver.payload.is_empty());
            host.close().await;
            client.close().await;
        })
        .await
        .unwrap();
    }
}
