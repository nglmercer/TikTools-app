//! Converts ordered, bounded Cloud relay records into an opaque byte stream
//! for TLS. This stream provides no authentication by itself: only Identity's
//! mutual TLS handshake may produce a SecureChannel used for business RPC.
use crate::{signaling::Context, Error};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    pin::Pin,
    task::{Context as TaskContext, Poll},
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, DuplexStream, ReadBuf},
    sync::mpsc,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const RECORD_BYTES: usize = 16 * 1024;
const RECORD_QUEUE: usize = 16;

#[derive(Deserialize)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum Record {
    #[serde(rename = "relay.data", rename_all = "camelCase")]
    Data {
        tunnel_id: Uuid,
        session_id: Uuid,
        from_device_id: Uuid,
        sequence: i64,
        data: String,
    },
    #[serde(rename = "relay.close", rename_all = "camelCase")]
    Close {
        tunnel_id: Uuid,
        session_id: Uuid,
        from_device_id: Uuid,
        sequence: i64,
    },
}
enum Incoming {
    Data(Vec<u8>),
    Close,
}
impl Record {
    pub fn position(&self) -> (Uuid, Uuid, Uuid, i64) {
        match self {
            Self::Data {
                tunnel_id,
                session_id,
                from_device_id,
                sequence,
                ..
            }
            | Self::Close {
                tunnel_id,
                session_id,
                from_device_id,
                sequence,
            } => (*tunnel_id, *session_id, *from_device_id, *sequence),
        }
    }
}

/// Owns the bounded pump. Dropping either live endpoint cancels it.
pub struct Stream {
    inner: DuplexStream,
    stop: CancellationToken,
}
pub struct Records {
    remote: Context,
    next: i64,
    tx: mpsc::Sender<Incoming>,
    stop: CancellationToken,
    closed: bool,
}

pub fn stream(
    local: Context,
    remote: Context,
    outgoing: mpsc::Sender<Value>,
) -> Result<(Stream, Records), Error> {
    let mut expected = local.clone();
    expected.from_device_id = remote.from_device_id;
    if expected != remote
        || local.grant_revision <= 0
        || local.from_device_id == remote.from_device_id
        || ((local.from_device_id == local.host_device_id)
            == (remote.from_device_id == remote.host_device_id))
    {
        return Err(Error::Configuration);
    }
    let (stream, bridge) = tokio::io::duplex(64 * 1024);
    let (tx, mut incoming) = mpsc::channel(RECORD_QUEUE);
    let stop = CancellationToken::new();
    let cancelled = stop.clone();
    tokio::spawn(async move {
        let (mut reader, mut writer) = tokio::io::split(bridge);
        let sending = async {
            let mut bytes = vec![0; RECORD_BYTES];
            for sequence in 0..i64::MAX {
                let n = reader
                    .read(&mut bytes)
                    .await
                    .map_err(|_| Error::Transport)?;
                let mut frame = json!({"type":if n==0 {"relay.close"} else {"relay.data"},
                    "tunnelId":local.negotiation_id,"sessionId":local.session_id,"sequence":sequence});
                if n > 0 {
                    frame["data"] = json!(URL_SAFE_NO_PAD.encode(&bytes[..n]));
                }
                outgoing.send(frame).await.map_err(|_| Error::Transport)?;
                if n == 0 {
                    return Ok::<_, Error>(());
                }
            }
            Err(Error::Transport)
        };
        let receiving = async {
            while let Some(item) = incoming.recv().await {
                match item {
                    Incoming::Data(bytes) => writer
                        .write_all(&bytes)
                        .await
                        .map_err(|_| Error::Transport)?,
                    Incoming::Close => return Ok::<_, Error>(()),
                }
            }
            Err(Error::Transport)
        };
        tokio::select! { biased; _=cancelled.cancelled()=>{}, _=sending=>{}, _=receiving=>{} }
        cancelled.cancel();
    });
    Ok((
        Stream {
            inner: stream,
            stop: stop.clone(),
        },
        Records {
            remote,
            next: 0,
            tx,
            stop,
            closed: false,
        },
    ))
}

impl Records {
    pub fn is_closed(&self) -> bool {
        self.closed || self.stop.is_cancelled()
    }
    pub fn cancel(&mut self) {
        self.closed = true;
        self.stop.cancel();
    }

    /// The WSS dispatcher must never wait on a slow TLS consumer. Overflow or
    /// sequence/context errors close this tunnel rather than dropping a record.
    pub fn apply(&mut self, record: Record) -> Result<(), Error> {
        let result = self.apply_inner(record);
        if result.is_err() {
            self.cancel();
        }
        result
    }
    fn apply_inner(&mut self, record: Record) -> Result<(), Error> {
        if self.is_closed() {
            return Err(Error::Transport);
        }
        let (id, session, device, sequence) = match &record {
            Record::Data {
                tunnel_id,
                session_id,
                from_device_id,
                sequence,
                ..
            }
            | Record::Close {
                tunnel_id,
                session_id,
                from_device_id,
                sequence,
            } => (*tunnel_id, *session_id, *from_device_id, *sequence),
        };
        if id != self.remote.negotiation_id
            || session != self.remote.session_id
            || device != self.remote.from_device_id
            || sequence != self.next
            || sequence == i64::MAX
        {
            return Err(Error::Negotiation);
        }
        let item = match record {
            Record::Data { data, .. } => {
                if data.len() > RECORD_BYTES * 4 / 3 + 4 {
                    return Err(Error::Frame);
                }
                let bytes = URL_SAFE_NO_PAD.decode(data).map_err(|_| Error::Frame)?;
                if bytes.is_empty() || bytes.len() > RECORD_BYTES {
                    return Err(Error::Frame);
                }
                Incoming::Data(bytes)
            }
            Record::Close { .. } => {
                self.closed = true;
                Incoming::Close
            }
        };
        self.tx.try_send(item).map_err(|_| Error::Transport)?;
        self.next += 1;
        Ok(())
    }
}
impl Drop for Records {
    fn drop(&mut self) {
        if !self.closed {
            self.stop.cancel();
        }
    }
}
impl Drop for Stream {
    fn drop(&mut self) {
        self.stop.cancel();
    }
}
impl AsyncRead for Stream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}
impl AsyncWrite for Stream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write(cx, buf)
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }
    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests;
