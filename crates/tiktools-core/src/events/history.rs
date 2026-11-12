//! Bounded global replay and isolated creator-local stream recovery.
use super::{
    CreatorFeedPage, CreatorFeedStream, CreatorFeedUnavailable, DomainEvent, EventBus,
    FeedBackfill, FeedEntry, FeedHistoryState,
};

impl FeedHistoryState {
    pub(super) fn creator_stream<F>(
        &mut self,
        creator: &str,
        sequence: u64,
        fill: F,
    ) -> Option<&mut CreatorFeedStream>
    where
        F: FnOnce(&mut [u8; 16]) -> bool,
    {
        if self.scoped_unavailable {
            return None;
        }
        if !self.creators.contains_key(creator) {
            let mut random = [0u8; 16];
            if !fill(&mut random) {
                self.scoped_unavailable = true;
                return None;
            }
            if self.creators.len() >= 64 {
                if let Some(oldest) = self
                    .creators
                    .iter()
                    .min_by_key(|(_, stream)| stream.last_global)
                    .map(|(id, _)| id.clone())
                {
                    self.creators.remove(&oldest);
                }
            }
            let id = random.iter().map(|byte| format!("{byte:02x}")).collect();
            self.creators.insert(
                creator.to_owned(),
                CreatorFeedStream {
                    id,
                    next: 0,
                    last_global: sequence,
                    ingress_gap: false,
                    events: std::collections::VecDeque::new(),
                },
            );
        }
        self.creators.get_mut(creator)
    }
}

impl EventBus {
    /// Read-only bounded recovery shared by headless and browser clients.
    /// Recording occurs synchronously on the existing bus, without another task.
    pub fn feed_backfill(&self, after: u64, limit: usize) -> FeedBackfill {
        let history = self
            .feed_history
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let oldest = history
            .entries
            .front()
            .map(|entry| entry.cursor)
            .unwrap_or(0);
        let latest = history
            .entries
            .back()
            .map(|entry| entry.cursor)
            .unwrap_or(0);
        let events: Vec<FeedEntry> = history
            .entries
            .iter()
            .filter(|entry| entry.cursor > after)
            .take(limit.clamp(1, 512))
            .filter_map(|entry| {
                if let DomainEvent::LiveUiEvent { event } = entry.event.as_ref() {
                    Some(FeedEntry {
                        cursor: entry.cursor,
                        event: event.clone(),
                    })
                } else {
                    None
                }
            })
            .collect();
        let gap = after > latest
            || (after != 0
                && events
                    .first()
                    .is_some_and(|entry| entry.cursor > after.saturating_add(1)));
        let last = events.last().map(|entry| entry.cursor).unwrap_or(after);
        FeedBackfill {
            oldest_cursor: oldest,
            latest_cursor: latest,
            depth: history.entries.len(),
            gap,
            truncated: last < latest,
            events,
        }
    }

    /// Creator-local, bounded history. `stream` identifies an in-memory
    /// generation; a changed or evicted stream resets the cursor with a gap.
    /// The output deliberately excludes raw event JSON and global feed IDs.
    pub fn creator_feed(
        &self,
        creator: &str,
        stream: Option<&str>,
        after: u64,
        limit: usize,
        byte_limit: usize,
    ) -> Result<CreatorFeedPage, CreatorFeedUnavailable> {
        let history = self.feed_history.lock().unwrap_or_else(|e| e.into_inner());
        if history.scoped_unavailable {
            return Err(CreatorFeedUnavailable);
        }
        let current = history.creators.get(creator);
        let stream_id = current.map(|s| s.id.clone());
        let changed = stream.is_some() && stream_id.as_deref() != stream;
        let latest = current.map_or(0, |s| s.next);
        let reset = changed || after > latest;
        let after = if reset { 0 } else { after };
        let oldest = current
            .and_then(|s| s.events.front())
            .and_then(|e| e.cursor.parse().ok())
            .unwrap_or(0);
        let mut events = Vec::new();
        let mut used = 0usize;
        for item in current.into_iter().flat_map(|s| s.events.iter()) {
            if item
                .cursor
                .parse::<u64>()
                .map_or(true, |cursor| cursor <= after)
            {
                continue;
            }
            let size = serde_json::to_vec(&item).map_or(usize::MAX, |v| v.len());
            if events.len() >= limit.clamp(1, 64)
                || used.saturating_add(size) > byte_limit.saturating_sub(512)
            {
                break;
            }
            used += size;
            events.push(item.clone());
        }
        let exhausted = oldest == 0 && after < latest;
        let last = events
            .last()
            .and_then(|e| e.cursor.parse().ok())
            .unwrap_or(if exhausted { latest } else { after });
        Ok(CreatorFeedPage {
            creator_id: creator.to_owned(),
            stream_id,
            oldest_cursor: oldest.to_string(),
            latest_cursor: latest.to_string(),
            next_cursor: last.to_string(),
            gap: current.is_some_and(|s| s.ingress_gap)
                || reset
                || exhausted
                || (oldest > 0
                    && ((after == 0 && oldest > 1)
                        || (after > 0 && oldest > after.saturating_add(1)))),
            truncated: last < latest,
            events,
        })
    }
}
