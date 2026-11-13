use crate::Error;
use bytes::BytesMut;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use webrtc::data_channel::DataChannel;

pub const MAX_FRAME_BYTES: usize = 1024 * 1024;
pub(crate) const MAX_PACKET_BYTES: usize = 16 * 1024;
const HEADER: usize = 12;
const PAYLOAD: usize = MAX_PACKET_BYTES - HEADER;

pub enum Frame {
    Text(String),
    Ping,
    Pong,
    Close,
}

#[derive(Default)]
pub(crate) struct Decoder {
    current: Option<(u8, usize, Vec<u8>)>,
}
impl Decoder {
    pub(crate) fn push(&mut self, packet: &[u8]) -> Result<Option<Frame>, Error> {
        if !(HEADER..=MAX_PACKET_BYTES).contains(&packet.len()) || packet[..3] != *b"TK\x01" {
            return Err(Error::Frame);
        }
        let kind = packet[3];
        let total = u32::from_be_bytes(packet[4..8].try_into().map_err(|_| Error::Frame)?) as usize;
        let offset =
            u32::from_be_bytes(packet[8..12].try_into().map_err(|_| Error::Frame)?) as usize;
        if kind > 3 || total > MAX_FRAME_BYTES || (kind != 0 && total != 0) {
            return Err(Error::Frame);
        }
        let payload = &packet[HEADER..];
        if offset
            .checked_add(payload.len())
            .is_none_or(|end| end > total)
            || total > 0 && payload.is_empty()
        {
            return Err(Error::Frame);
        }
        if self.current.is_none() {
            if offset != 0 {
                return Err(Error::Frame);
            }
            self.current = Some((kind, total, Vec::with_capacity(total)));
        }
        let (current_kind, current_total, bytes) = self.current.as_mut().ok_or(Error::Frame)?;
        if *current_kind != kind || *current_total != total || offset != bytes.len() {
            return Err(Error::Frame);
        }
        bytes.extend_from_slice(payload);
        if bytes.len() != total {
            return Ok(None);
        }
        let (_, _, bytes) = self.current.take().ok_or(Error::Frame)?;
        Ok(Some(match kind {
            0 => Frame::Text(String::from_utf8(bytes).map_err(|_| Error::Frame)?),
            1 => Frame::Ping,
            2 => Frame::Pong,
            3 => Frame::Close,
            _ => return Err(Error::Frame),
        }))
    }
}

/// The receiver and partial decoder belong to the connection, so cancelling a
/// `recv` future cannot discard bytes. A cancelled partial send poisons the
/// sender: an uncertain command is never restarted or silently retransmitted.
pub struct Framed {
    pub(crate) channel: Arc<dyn DataChannel>,
    pub(crate) packets: mpsc::Receiver<Vec<u8>>,
    pub(crate) stop: CancellationToken,
    pub(crate) decoder: Decoder,
    pub(crate) sending: bool,
}
impl Framed {
    pub async fn send(&mut self, frame: Frame) -> Result<(), Error> {
        if self.sending || self.stop.is_cancelled() {
            return Err(Error::Transport);
        }
        let (kind, bytes) = match frame {
            Frame::Text(text) => (0, text.into_bytes()),
            Frame::Ping => (1, vec![]),
            Frame::Pong => (2, vec![]),
            Frame::Close => (3, vec![]),
        };
        if bytes.len() > MAX_FRAME_BYTES {
            return Err(Error::Frame);
        }
        self.sending = true;
        // Even an empty control/text frame has one packet.
        let count = bytes.len().max(1).div_ceil(PAYLOAD);
        for part in 0..count {
            let offset = part * PAYLOAD;
            let end = (offset + PAYLOAD).min(bytes.len());
            let mut packet = BytesMut::with_capacity(HEADER + end - offset);
            packet.extend_from_slice(b"TK\x01");
            packet.extend_from_slice(&[kind]);
            packet.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
            packet.extend_from_slice(&(offset as u32).to_be_bytes());
            packet.extend_from_slice(&bytes[offset..end]);
            tokio::select! {
                biased;
                _=self.stop.cancelled()=>return Err(Error::Transport),
                sent=self.channel.send(packet)=>sent.map_err(|_|Error::Transport)?,
            }
        }
        self.sending = false;
        Ok(())
    }
    pub async fn recv(&mut self) -> Result<Frame, Error> {
        loop {
            let packet = tokio::select! {
                biased;
                _=self.stop.cancelled()=>return Err(Error::Transport),
                packet=self.packets.recv()=>packet.ok_or(Error::Transport)?,
            };
            match self.decoder.push(&packet) {
                Ok(Some(frame)) => return Ok(frame),
                Ok(None) => {}
                Err(error) => {
                    self.stop.cancel();
                    return Err(error);
                }
            }
        }
    }
}
impl Drop for Framed {
    fn drop(&mut self) {
        self.stop.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn packet(total: u32, offset: u32, payload: &[u8]) -> Vec<u8> {
        let mut bytes = b"TK\x01\0".to_vec();
        bytes.extend(total.to_be_bytes());
        bytes.extend(offset.to_be_bytes());
        bytes.extend(payload);
        bytes
    }
    #[test]
    fn partial_frames_survive_between_calls_and_require_contiguous_offsets() {
        let mut decoder = Decoder::default();
        assert!(decoder.push(&packet(4, 0, b"ab")).unwrap().is_none());
        assert!(
            matches!(decoder.push(&packet(4,2,b"cd")).unwrap(),Some(Frame::Text(s)) if s=="abcd")
        );
        assert!(decoder.push(&packet(4, 0, b"ab")).unwrap().is_none());
        assert!(decoder.push(&packet(4, 1, b"cd")).is_err());
    }
    #[test]
    fn rejects_malformed_oversized_and_invalid_utf8_before_delivery() {
        for bytes in [
            vec![],
            packet(MAX_FRAME_BYTES as u32 + 1, 0, b"a"),
            packet(1, 0, b""),
            packet(1, 0, &[255]),
            packet(1, 0, b"ab"),
            packet(20000, 0, &vec![0; MAX_PACKET_BYTES]),
        ] {
            assert!(Decoder::default().push(&bytes).is_err());
        }
    }
}
