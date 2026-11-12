//! Bounded UI batches with reliable-message delivery priority.
use super::*;

impl WebviewOutbox {
    /// Takes at most one UI tick's worth of messages: reliable first,
    /// then lossy fill. FIFO within each lane.
    #[cfg(test)]
    pub(crate) fn take_batch(&mut self) -> Vec<String> {
        self.take_prepared_batch()
            .into_iter()
            .map(|message| message.body.to_string())
            .collect()
    }

    pub(crate) fn take_prepared_batch(&mut self) -> Vec<QueuedWebviewMessage> {
        let mut batch = Vec::new();
        let mut bytes = 0;
        while batch.len() < MAX_BATCH_PER_TICK {
            if self.reliable.front().is_some_and(|body| {
                !batch.is_empty() && body.bytes() > MAX_BATCH_BYTES.saturating_sub(bytes)
            }) {
                break;
            }
            if let Some(body) = self.reliable.pop_front() {
                self.reliable_bytes = self.reliable_bytes.saturating_sub(body.bytes());
                bytes += body.bytes();
                batch.push(body);
                if bytes >= MAX_BATCH_BYTES {
                    break;
                }
            } else {
                break;
            }
        }
        while batch.len() < MAX_BATCH_PER_TICK {
            if self
                .lossy
                .front()
                .is_some_and(|item| item.bytes() > MAX_BATCH_BYTES.saturating_sub(bytes))
            {
                break;
            }
            if let Some(queued) = self.lossy.pop_front() {
                self.lossy_bytes -= queued.bytes();
                bytes += queued.bytes();
                batch.push(queued);
            } else {
                break;
            }
        }
        batch
    }
}
