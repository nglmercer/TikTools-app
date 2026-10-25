//! Backend canonical message deduplication.
//!
//! TikTok re-sends recent messages as history when a connection drops and
//! rejoins, so the same event can arrive twice: once live, once as replay.
//! Without a backend guard the replay would award points, analytics,
//! automations, and UI effects a second time. The frontend keeps its own
//! bounded guard as defense in depth; this store is authoritative because
//! it sits at the single choke point (`handle_live_event`) that fans out
//! to points, analytics, automation, and UI together, so a replay is
//! either fully applied or fully skipped everywhere.
//!
//! Identity rules, per canonical event type:
//!
//! - Key: `(creator, room, method, event-type tag, TikTok msg_id)`. Types do not share one ID
//!   space: a chat and a gift that somehow carried the same numeric ID
//!   are still distinct events.
//! - `msg_id == 0` means missing/invalid identity: never suppressed.
//!   Distinct events with absent IDs always process.
//! - Gift streaks are safe: every streak message carries its own msg_id,
//!   so intermediates and the finalization all pass exactly once, while
//!   an identical replay of the finalization is suppressed (no
//!   double-counted diamonds). Totals never merge across streaks.
//! - Identical chat text from the same viewer passes when the msg_ids
//!   differ: only exact identity replays are suppressed.
//! - `is_history` replays with an unseen msg_id still process (genuinely
//!   missed events must not be lost); replays of seen IDs are suppressed.
//!
//! The store is bounded (default 4,096 entries, 5-minute TTL) and keyed by
//! message identity alone — deliberately *not* by connection generation —
//! so it survives reconnects: that is precisely when replays arrive.
//! Stale-connection *processing* is fenced separately at ingress (the
//! worker drops data from superseded generations), while this store
//! suppresses exact replays. TTL expiry plus FIFO eviction keep memory
//! flat across arbitrarily many sessions.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::*;
use tiktools_plugin_api::sync::recover_mutex;

/// Bounded identity memory for replay suppression.
pub(crate) const DEDUP_CAPACITY: usize = 4_096;
/// Identities older than this are forgotten; replays arrive within seconds
/// of a reconnect, so five minutes is generous without retaining sessions.
pub(crate) const DEDUP_TTL: Duration = Duration::from_secs(300);

pub(crate) struct LiveDedup {
    inner: Mutex<DedupInner>,
    capacity: usize,
    ttl: Duration,
    duplicates: AtomicU64,
}

struct DedupInner {
    scope: Option<(String, String)>,
    seen: HashMap<(String, String, String, u8, u64), Instant>,
    order: VecDeque<(String, String, String, u8, u64)>,
}

impl LiveDedup {
    pub(crate) fn new(capacity: usize, ttl: Duration) -> Self {
        Self {
            inner: Mutex::new(DedupInner {
                scope: None,
                seen: HashMap::new(),
                order: VecDeque::new(),
            }),
            capacity: capacity.max(1),
            ttl,
            duplicates: AtomicU64::new(0),
        }
    }

    pub(crate) fn default() -> Self {
        Self::new(DEDUP_CAPACITY, DEDUP_TTL)
    }

    pub(crate) fn set_scope(&self, creator: &str, room: &str) {
        let mut inner = recover_mutex(&self.inner, "live dedup");
        let scope = (creator.to_owned(), room.to_owned());
        if inner.scope.as_ref() != Some(&scope) {
            inner.scope = Some(scope);
        }
    }

    /// Reports whether `event` was already processed. `true` means the
    /// caller must skip *every* side effect (points, analytics,
    /// automation, UI) for this event.
    pub(crate) fn check_duplicate(&self, event: &NativeLiveEvent) -> bool {
        let msg_id = event.msg_id();
        if msg_id == 0 {
            // Missing or invalid identity: never suppress. Distinct
            // events without IDs always process.
            return false;
        }
        let mut inner = recover_mutex(&self.inner, "live dedup");
        let (creator, room) = inner.scope.clone().unwrap_or_default();
        let key = (
            creator,
            room,
            event.method().to_owned(),
            event_tag(&event.base),
            msg_id,
        );
        if let Some(seen_at) = inner.seen.get(&key).copied() {
            if seen_at.elapsed() < self.ttl {
                self.duplicates.fetch_add(1, Ordering::Relaxed);
                return true;
            }
            // TTL expired: forget the old occurrence and process this one
            // as new instead of suppressing it forever.
            inner.order.retain(|entry| entry != &key);
            inner.seen.insert(key.clone(), Instant::now());
            inner.order.push_back(key);
            return false;
        }
        if inner.seen.len() >= self.capacity {
            self.evict_expired(&mut inner);
            while inner.seen.len() >= self.capacity {
                match inner.order.pop_front() {
                    Some(oldest) => {
                        inner.seen.remove(&oldest);
                    }
                    None => break,
                }
            }
        }
        inner.seen.insert(key.clone(), Instant::now());
        inner.order.push_back(key);
        false
    }

    fn evict_expired(&self, inner: &mut DedupInner) {
        let now = Instant::now();
        while let Some(front) = inner.order.front().cloned() {
            match inner.seen.get(&front) {
                Some(seen_at) if now.duration_since(*seen_at) < self.ttl => break,
                _ => {
                    inner.order.pop_front();
                    inner.seen.remove(&front);
                }
            }
        }
    }

    pub(crate) fn duplicate_count(&self) -> u64 {
        self.duplicates.load(Ordering::Relaxed)
    }

    pub(crate) fn diagnostics(&self) -> Value {
        let tracked = recover_mutex(&self.inner, "live dedup").seen.len();
        json!({
            "duplicates": self.duplicate_count(),
            "tracked": tracked,
            "capacity": self.capacity,
        })
    }
}

#[cfg(all(test, feature = "native-tiktok"))]
mod tests {
    use super::*;
    use tiktools_tiktok::events::{
        CanonicalLiveEvent, ChatEvent, EventMetadata, EventUser, GiftEnrichment, GiftEvent,
    };

    fn user() -> EventUser {
        EventUser {
            id: 7,
            unique_id: "spectator".to_owned(),
            nickname: "Spectator".to_owned(),
            sec_uid: "sec-spectator".to_owned(),
            avatar_url: None,
        }
    }

    fn chat(msg_id: u64, comment: &str) -> NativeLiveEvent {
        NativeLiveEvent {
            base: CanonicalLiveEvent::Chat(ChatEvent {
                user: user(),
                comment: comment.to_owned(),
                ..Default::default()
            }),
            metadata: EventMetadata {
                method: "WebcastTest".to_owned(),
                msg_id,
                is_history: false,
            },
            gift: None,
        }
    }

    fn gift(msg_id: u64, repeat_end: bool) -> NativeLiveEvent {
        NativeLiveEvent {
            base: CanonicalLiveEvent::Gift(GiftEvent {
                user: user(),
                gift_id: 1,
                gift_name: "Rose".to_owned(),
                gift_image_url: None,
                diamond_count: 1,
                repeat_count: 3,
                combo_count: 3,
                group_id: 9,
                repeat_end,
            }),
            metadata: EventMetadata {
                method: "WebcastTest".to_owned(),
                msg_id,
                is_history: false,
            },
            gift: Some(GiftEnrichment {
                name: "Rose".to_owned(),
                diamond_count: 1,
                streakable: true,
                icon_url: None,
            }),
        }
    }

    #[test]
    fn same_message_replayed_twice_processes_once() {
        let dedup = LiveDedup::default();
        assert!(!dedup.check_duplicate(&chat(101, "hello")));
        assert!(dedup.check_duplicate(&chat(101, "hello")));
        assert_eq!(dedup.duplicate_count(), 1);
    }

    #[test]
    fn identical_text_with_distinct_ids_all_process() {
        let dedup = LiveDedup::default();
        assert!(!dedup.check_duplicate(&chat(201, "same")));
        assert!(!dedup.check_duplicate(&chat(202, "same")));
        assert!(!dedup.check_duplicate(&chat(203, "same")));
        assert_eq!(dedup.duplicate_count(), 0);
    }

    #[test]
    fn gift_streak_increments_preserved_but_final_replay_suppressed() {
        let dedup = LiveDedup::default();
        // Streak progress and finalization carry distinct identities: every
        // increment processes exactly once, preserving gift totals.
        assert!(!dedup.check_duplicate(&gift(301, false)));
        assert!(!dedup.check_duplicate(&gift(302, false)));
        assert!(!dedup.check_duplicate(&gift(303, true)));
        // An identical replay of the finalization must not double-count.
        assert!(dedup.check_duplicate(&gift(303, true)));
        assert_eq!(dedup.duplicate_count(), 1);
    }

    #[test]
    fn missing_message_ids_are_never_suppressed() {
        let dedup = LiveDedup::default();
        for _ in 0..10 {
            assert!(!dedup.check_duplicate(&chat(0, "no identity")));
        }
        assert_eq!(dedup.duplicate_count(), 0);
    }

    #[test]
    fn reconnect_replay_of_seen_ids_is_suppressed() {
        let dedup = LiveDedup::default();
        assert!(!dedup.check_duplicate(&chat(401, "before drop")));
        // Same identity arriving as history after a reconnect: replay.
        let mut replay = chat(401, "before drop");
        replay.metadata.is_history = true;
        assert!(dedup.check_duplicate(&replay));
        // ...while genuinely missed history still processes.
        let mut missed = chat(402, "missed live");
        missed.metadata.is_history = true;
        assert!(!dedup.check_duplicate(&missed));
    }

    #[test]
    fn duplicate_flood_stays_bounded() {
        let dedup = LiveDedup::new(64, Duration::from_secs(300));
        for id in 1..=1_000u64 {
            assert!(!dedup.check_duplicate(&chat(id, "flood")));
        }
        for id in 1..=1_000u64 {
            // Oldest entries were evicted FIFO, so these read as new; the
            // store itself never exceeds capacity.
            let _ = dedup.check_duplicate(&chat(id, "flood"));
        }
        let diagnostics = dedup.diagnostics();
        assert!(diagnostics["tracked"].as_u64().unwrap() <= 64);
        assert_eq!(diagnostics["capacity"], 64);
    }

    #[test]
    fn expired_identities_are_forgotten() {
        let dedup = LiveDedup::new(4_096, Duration::from_millis(20));
        assert!(!dedup.check_duplicate(&chat(501, "ephemeral")));
        std::thread::sleep(Duration::from_millis(40));
        // TTL expired: the identity processes again instead of leaking a
        // permanent suppression.
        assert!(!dedup.check_duplicate(&chat(501, "ephemeral")));
    }

    #[test]
    fn type_and_room_scope_are_independent_of_reconnect_generation() {
        let dedup = LiveDedup::default();
        dedup.set_scope("alice", "room1");
        assert!(!dedup.check_duplicate(&chat(42, "same")));
        assert!(!dedup.check_duplicate(&gift(42, true)));
        dedup.set_scope("alice", "room1");
        assert!(dedup.check_duplicate(&gift(42, true)));
        dedup.set_scope("alice", "room2");
        assert!(!dedup.check_duplicate(&gift(42, true)));
        dedup.set_scope("alice", "room1");
        assert!(dedup.check_duplicate(&gift(42, true)));
    }

    #[test]
    fn expired_reinsertion_keeps_fifo_bounded() {
        let dedup = LiveDedup::new(8, Duration::ZERO);
        for _ in 0..100 {
            assert!(!dedup.check_duplicate(&chat(42, "same")));
        }
        let inner = recover_mutex(&dedup.inner, "test dedup");
        assert_eq!(inner.seen.len(), 1);
        assert_eq!(inner.order.len(), 1);
    }

    struct NoopEmitter;
    impl HostEmitter for NoopEmitter {
        fn emit(&self, _: HostMessage) {}
    }

    /// Replays through the real handler award points exactly once: the
    /// choke point keeps UI, points, analytics, and automations
    /// consistent by construction.
    #[tokio::test]
    async fn replayed_chat_awards_points_exactly_once() {
        let core = Arc::new(AppCore::new(Arc::new(NoopEmitter)));
        let mut live = chat(601, "once only");
        // Unique author so other tests sharing the on-disk database cannot
        // interfere; delta assertions keep this hermetic across reruns too.
        if let CanonicalLiveEvent::Chat(chat) = &mut live.base {
            chat.user.unique_id = "dedup-e2e-viewer".to_owned();
        }
        let balance = || {
            core.points
                .viewer("dedup-e2e-viewer")
                .and_then(|viewer| viewer.get("points").and_then(Value::as_f64))
                .unwrap_or(0.0)
        };
        let before = balance();
        core.handle_live_event(live.clone()).await;
        let after_first = balance();
        // The reconnect replay carries the same identity as history.
        let mut replay = live;
        replay.metadata.is_history = true;
        core.handle_live_event(replay).await;
        let after_replay = balance();
        // The exact amount is config-dependent (other tests persist
        // config to the shared test database); what this test owns is
        // once-only delivery: first awards, replay adds nothing.
        assert!(after_first > before, "first delivery awards points");
        assert_eq!(after_replay, after_first, "replay awards nothing");
        assert_eq!(core.live_dedup.duplicate_count(), 1);
        core.points.shutdown();
    }
}

/// Per-type identity namespace. TikTok message-ID semantics are not
/// assumed uniform across event kinds.
fn event_tag(base: &tiktools_tiktok::events::CanonicalLiveEvent) -> u8 {
    use tiktools_tiktok::events::CanonicalLiveEvent;
    match base {
        CanonicalLiveEvent::Chat(_) => 1,
        CanonicalLiveEvent::EmoteChat(_) => 8,
        CanonicalLiveEvent::Gift(_) => 2,
        CanonicalLiveEvent::Like(_) => 3,
        CanonicalLiveEvent::Member(_) => 4,
        CanonicalLiveEvent::Social(_) => 5,
        CanonicalLiveEvent::RoomUser(_) => 6,
        CanonicalLiveEvent::Unknown { .. } => 7,
    }
}
