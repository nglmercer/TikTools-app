//! Main-page presentation policy. Domain/plugin/control subscribers are unchanged.
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tiktools_core::events::DomainEvent;

/// Bounded diagnostic mirror of events observed by the desktop subscriber.
/// Recovery reads the core EventBus journal through the Control API; this
/// mirror can contain subscriber gaps and is not authoritative. Memory is constant regardless of viewer count or
/// burst size: oldest entries are evicted first. Replay is read-only — the
/// journal performs no TTS/points/automation side effects.
pub(crate) const FEED_JOURNAL_CAPACITY: usize = 512;

/// One retained render event keyed by its monotonic bus cursor. The event
/// is stored un-serialized (an [`Arc`]) so hidden bursts cost a cheap clone,
/// never a JSON encode; only the bounded backfill pays serialization.
struct FeedJournal {
    entries: VecDeque<(u64, Arc<DomainEvent>)>,
    capacity: usize,
}

impl FeedJournal {
    fn new(capacity: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            capacity: capacity.max(1),
        }
    }

    fn record(&mut self, cursor: u64, event: Arc<DomainEvent>) {
        self.entries.push_back((cursor, event));
        while self.entries.len() > self.capacity {
            self.entries.pop_front();
        }
    }

    fn oldest(&self) -> u64 {
        self.entries.front().map(|(cursor, _)| *cursor).unwrap_or(0)
    }

    fn latest(&self) -> u64 {
        self.entries.back().map(|(cursor, _)| *cursor).unwrap_or(0)
    }

    fn depth(&self) -> usize {
        self.entries.len()
    }

    /// Entries strictly newer than `after`, oldest first, at most `limit`.
    #[cfg(test)]
    fn select(&self, after: u64, limit: usize) -> Vec<(u64, Arc<DomainEvent>)> {
        self.entries
            .iter()
            .filter(|(cursor, _)| *cursor > after)
            .take(limit)
            .map(|(cursor, event)| (*cursor, Arc::clone(event)))
            .collect()
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct PageView {
    pub generation: u64,
    pub revision: u64,
    pub feed: bool,
    pub behavior: bool,
    pub points: bool,
    pub visible: bool,
}

pub(crate) struct MainPageProjection {
    view: Mutex<PageView>,
    host_visible: std::sync::atomic::AtomicBool,
    feed_journal: Mutex<FeedJournal>,
    pub source: AtomicU64,
    pub filtered: AtomicU64,
    pub feed_suspended: AtomicU64,
    pub points_coalesced: AtomicU64,
    pub runs_coalesced: AtomicU64,
    pub snapshots: AtomicU64,
    pub lossy_skipped: AtomicU64,
    pub feed_bus_dropped: AtomicU64,
    pub feed_forwarded: AtomicU64,
    #[cfg(feature = "live-benchmark")]
    pub disable_points: std::sync::atomic::AtomicBool,
}

impl Default for MainPageProjection {
    fn default() -> Self {
        Self {
            view: Mutex::new(PageView {
                generation: 0,
                revision: 0,
                feed: false,
                behavior: false,
                points: false,
                visible: false,
            }),
            host_visible: std::sync::atomic::AtomicBool::new(true),
            feed_journal: Mutex::new(FeedJournal::new(FEED_JOURNAL_CAPACITY)),
            source: AtomicU64::new(0),
            filtered: AtomicU64::new(0),
            feed_suspended: AtomicU64::new(0),
            points_coalesced: AtomicU64::new(0),
            runs_coalesced: AtomicU64::new(0),
            snapshots: AtomicU64::new(0),
            lossy_skipped: AtomicU64::new(0),
            feed_bus_dropped: AtomicU64::new(0),
            feed_forwarded: AtomicU64::new(0),
            #[cfg(feature = "live-benchmark")]
            disable_points: std::sync::atomic::AtomicBool::new(false),
        }
    }
}

impl MainPageProjection {
    #[cfg(test)]
    pub(crate) fn with_feed_capacity(capacity: usize) -> Self {
        let projection = Self::default();
        *projection
            .feed_journal
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = FeedJournal::new(capacity);
        projection
    }

    pub fn view(&self) -> PageView {
        let mut view = *self.view.lock().unwrap_or_else(|e| e.into_inner());
        view.visible &= self.host_visible.load(Ordering::Acquire);
        #[cfg(feature = "live-benchmark")]
        {
            view.points &= !self.disable_points.load(Ordering::Relaxed);
        }
        view
    }
    pub fn retire(&self, generation: u64) {
        *self.view.lock().unwrap_or_else(|e| e.into_inner()) = PageView {
            generation,
            revision: 0,
            feed: false,
            behavior: false,
            points: false,
            visible: false,
        };
    }
    pub fn set_host_visible(&self, visible: bool) {
        self.host_visible.store(visible, Ordering::Release);
    }

    /// Retains one render event under its monotonic bus cursor for later
    /// read-only backfill. Called for every feed event the forwarder observes,
    /// hidden or visible, before the visibility filter.
    pub fn record_feed(&self, cursor: u64, event: Arc<DomainEvent>) {
        let mut journal = self
            .feed_journal
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        journal.record(cursor, event);
    }

    /// Bounded read-only replay of retained feed events with `cursor > after`.
    /// `gap` reports that events between the requested cursor and the oldest
    /// retained entry are no longer available (evicted or bus-dropped);
    /// `truncated` reports that more entries remain beyond `limit`. Neither
    /// key pretends the returned set is complete.
    #[cfg(test)]
    pub fn feed_backfill(&self, after: u64, limit: usize) -> serde_json::Value {
        let journal = self
            .feed_journal
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let oldest = journal.oldest();
        let latest = journal.latest();
        let selected = journal.select(after, limit.max(1));
        let first = selected.first().map(|(cursor, _)| *cursor);
        let last = selected.last().map(|(cursor, _)| *cursor).unwrap_or(after);
        let mut gap = after != 0 && first.is_some_and(|cursor| cursor > after.saturating_add(1));
        let mut events = Vec::with_capacity(selected.len());
        for (cursor, event) in selected {
            // The journal only ever holds render feed events; expose exactly
            // the payload the live wire carries as `data.event`.
            match event.as_ref() {
                DomainEvent::LiveUiEvent { event: render } => {
                    events.push(serde_json::json!({"cursor": cursor, "event": render}));
                }
                _ => gap = true,
            }
        }
        serde_json::json!({
            "oldestCursor": oldest,
            "latestCursor": latest,
            "depth": journal.depth(),
            "gap": gap,
            "truncated": last < latest,
            "events": events,
        })
    }

    pub fn update(
        &self,
        generation: u64,
        revision: u64,
        feed: bool,
        behavior: bool,
        points: bool,
        visible: bool,
    ) -> bool {
        let mut view = self.view.lock().unwrap_or_else(|e| e.into_inner());
        if generation != view.generation || revision <= view.revision {
            return false;
        }
        *view = PageView {
            generation,
            revision,
            feed,
            behavior,
            points,
            visible,
        };
        true
    }
    pub fn diagnostics(&self) -> serde_json::Value {
        let view = self.view();
        let (oldest_cursor, latest_cursor, journal_depth) = {
            let journal = self
                .feed_journal
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            (journal.oldest(), journal.latest(), journal.depth())
        };
        serde_json::json!({"generation": view.generation, "revision": view.revision,
            "feed": view.feed, "behavior": view.behavior, "points": view.points, "visible": view.visible,
            "sourceEvents": self.source.load(Ordering::Relaxed),
            "filteredEvents": self.filtered.load(Ordering::Relaxed),
            "feedSuspended": self.feed_suspended.load(Ordering::Relaxed),
            "pointsCoalesced": self.points_coalesced.load(Ordering::Relaxed),
            "runsCoalesced": self.runs_coalesced.load(Ordering::Relaxed),
            "lossyDomainSkipped": self.lossy_skipped.load(Ordering::Relaxed),
            "feedBusDropped": self.feed_bus_dropped.load(Ordering::Relaxed),
            "feedForwarded": self.feed_forwarded.load(Ordering::Relaxed),
            "feedOldestCursor": oldest_cursor,
            "feedLatestCursor": latest_cursor,
            "feedJournalDepth": journal_depth,
            "snapshots": self.snapshots.load(Ordering::Relaxed)})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn late_view_requests_and_retired_pages_cannot_restore_delivery() {
        let policy = MainPageProjection::default();
        assert!(policy.update(0, 2, true, false, true, true));
        assert!(!policy.update(0, 1, false, true, false, true));
        policy.retire(1);
        assert!(!policy.update(0, 3, true, true, true, true));
        assert!(!policy.view().visible);
        assert!(policy.update(1, 1, true, false, true, true));
        policy.set_host_visible(false);
        assert!(!policy.view().visible);
        policy.set_host_visible(true);
        assert!(policy.view().feed);
    }

    fn render(text: &str) -> Arc<DomainEvent> {
        Arc::new(DomainEvent::LiveUiEvent {
            event: serde_json::json!({"kind": "chat", "text": text}),
        })
    }

    #[test]
    fn feed_journal_evicts_oldest_and_reports_the_retained_window() {
        let mut journal = FeedJournal::new(3);
        assert_eq!(journal.latest(), 0);
        for cursor in 1..=5 {
            journal.record(cursor, render(&cursor.to_string()));
        }
        assert_eq!(journal.depth(), 3);
        assert_eq!(journal.oldest(), 3);
        assert_eq!(journal.latest(), 5);
    }

    #[test]
    fn feed_backfill_filters_by_cursor_oldest_first_and_bounds_the_page() {
        let projection = MainPageProjection::default();
        for cursor in 1..=5 {
            projection.record_feed(cursor, render(&cursor.to_string()));
        }
        let all = projection.feed_backfill(0, FEED_JOURNAL_CAPACITY);
        assert_eq!(all["latestCursor"], 5);
        assert_eq!(all["depth"], 5);
        assert_eq!(all["gap"], false);
        assert_eq!(all["truncated"], false);
        let events = all["events"].as_array().unwrap();
        assert_eq!(events.len(), 5);
        assert_eq!(events[0]["cursor"], 1);
        assert_eq!(events[4]["cursor"], 5);
        assert_eq!(events[0]["event"]["text"], "1");

        let tail = projection.feed_backfill(3, FEED_JOURNAL_CAPACITY);
        let tail_events = tail["events"].as_array().unwrap();
        assert_eq!(tail_events.len(), 2);
        assert_eq!(tail_events[0]["cursor"], 4);
        assert_eq!(tail["gap"], false);

        let empty = projection.feed_backfill(5, FEED_JOURNAL_CAPACITY);
        assert!(empty["events"].as_array().unwrap().is_empty());
        assert_eq!(empty["truncated"], false);
    }

    #[test]
    fn feed_backfill_flags_truncation_and_unrecoverable_gaps() {
        let projection = MainPageProjection::default();
        for cursor in 1..=5 {
            projection.record_feed(cursor, render(&cursor.to_string()));
        }
        // A bounded page leaves newer entries behind: truncated, not complete.
        let page = projection.feed_backfill(0, 2);
        let page_events = page["events"].as_array().unwrap();
        assert_eq!(page_events.len(), 2);
        assert_eq!(page["truncated"], true);
        assert_eq!(page["gap"], false);
        // A small journal with eviction: asking behind the retained window
        // must report a gap instead of silently skipping lost cursors.
        let evicting = MainPageProjection::with_feed_capacity(2);
        for cursor in 1..=4 {
            evicting.record_feed(cursor, render(&cursor.to_string()));
        }
        let gapped = evicting.feed_backfill(1, FEED_JOURNAL_CAPACITY);
        assert_eq!(gapped["oldestCursor"], 3);
        assert_eq!(gapped["gap"], true);
        assert_eq!(gapped["events"].as_array().unwrap()[0]["cursor"], 3);
        // Asking at the retained edge stays contiguous.
        assert_eq!(
            evicting.feed_backfill(3, FEED_JOURNAL_CAPACITY)["gap"],
            false
        );
    }

    #[test]
    fn record_feed_is_bounded_even_under_a_large_burst() {
        let projection = MainPageProjection::default();
        for cursor in 1..=10_000 {
            projection.record_feed(cursor, render(&cursor.to_string()));
        }
        let diagnostics = projection.diagnostics();
        assert_eq!(diagnostics["feedJournalDepth"], FEED_JOURNAL_CAPACITY);
        assert_eq!(diagnostics["feedLatestCursor"], 10_000u64);
        assert_eq!(
            diagnostics["feedOldestCursor"],
            10_000u64 - FEED_JOURNAL_CAPACITY as u64 + 1
        );
    }
}
