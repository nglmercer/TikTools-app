//! Bounded cumulative counters, after replay admission. Originals are never
//! replaced. Combo identity is independent of message identity and delivery.
use crate::*;
use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};
use tiktools_tiktok::events::CanonicalLiveEvent;
const CAPACITY: usize = 4096;
const RETENTION: Duration = Duration::from_secs(300);
type ComboKey = (String, String, u64, u64, u64);
#[derive(Default)]
pub(crate) struct LiveTotals(Mutex<State>);
#[derive(Default)]
struct State {
    combos: HashMap<ComboKey, (u64, u64, Instant)>,
    unknown_methods: std::collections::BTreeMap<String, u64>,
    likes: HashMap<(String, String, String), u64>,
    untracked: u64,
    untracked_combos: u64,
}
impl LiveTotals {
    /// Account observed increments immediately. Finalization is a zero delta
    /// when all progress was received, so losing it never loses earned units.
    /// Keep completed combos as tombstones for the same replay retention window.
    pub(crate) fn observe(&self, event: &NativeLiveEvent, context: Option<&LiveContext>) -> Value {
        let mut state = recover_mutex(&self.0, "LIVE totals");
        let creator = context.map_or("", |c| c.unique_id.as_str()).to_owned();
        let room = context.map_or("", |c| c.room_id.as_str()).to_owned();
        match &event.base {
            CanonicalLiveEvent::Gift(gift) => {
                let count = gift.repeat_count.max(gift.combo_count).max(1);
                let mut delta = count;
                let mut cumulative = count;
                let price = event.gift_diamond_count().unwrap_or(gift.diamond_count);
                let mut diamonds = count.saturating_mul(price);
                // groupId plus stable sender/gift IDs is positive combo evidence,
                // including when the catalog has no streakability metadata.
                if gift.group_id != 0 && gift.user.id != 0 && gift.gift_id != 0 {
                    state
                        .combos
                        .retain(|_, (_, _, at)| at.elapsed() < RETENTION);
                    let key = (creator, room, gift.user.id, gift.gift_id, gift.group_id);
                    if !state.combos.contains_key(&key) && state.combos.len() >= CAPACITY {
                        state.untracked_combos += 1;
                        // Preserve the original packet, but never guess a delta
                        // after forgetting a still-retained combo's high water.
                        return Value::Null;
                    }
                    let entry = state.combos.entry(key).or_insert((0, 0, Instant::now()));
                    delta = count.saturating_sub(entry.0);
                    entry.0 = entry.0.max(count);
                    cumulative = entry.0;
                    let target = entry.0.saturating_mul(price);
                    diamonds = target.saturating_sub(entry.1);
                    entry.1 = entry.1.max(target);
                    entry.2 = Instant::now();
                }
                json!({"increment": delta, "cumulative": cumulative, "diamonds": diamonds})
            }
            CanonicalLiveEvent::Like(like) => {
                let user = if like.user.id != 0 {
                    like.user.id.to_string()
                } else {
                    like.user.unique_id.clone()
                };
                let mut totals = Vec::new();
                let users = if user.is_empty() {
                    vec![String::new()]
                } else {
                    vec![String::new(), user]
                };
                for user in users {
                    let key = (creator.clone(), room.clone(), user);
                    if !state.likes.contains_key(&key) && state.likes.len() >= CAPACITY {
                        // Refuse new counters instead of forgetting a seen user's
                        // total and firing the same goal again after reconnect.
                        state.untracked += 1;
                        totals.push(Value::Null);
                        continue;
                    }
                    let total = state.likes.entry(key).or_default();
                    let previous = *total;
                    *total = total.saturating_add(like.count);
                    totals.push(json!({"previous": previous, "total": *total}));
                }
                json!({"global": totals[0], "user": totals.get(1)})
            }
            CanonicalLiveEvent::Unknown { .. } => {
                let method: String = event
                    .method()
                    .chars()
                    .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .take(96)
                    .collect();
                if state.unknown_methods.contains_key(&method) || state.unknown_methods.len() < 64 {
                    *state.unknown_methods.entry(method).or_default() += 1;
                } else {
                    *state.unknown_methods.entry("other".into()).or_default() += 1;
                }
                Value::Null
            }
            _ => Value::Null,
        }
    }
    pub(crate) fn diagnostics(&self) -> Value {
        let state = recover_mutex(&self.0, "LIVE totals");
        json!({"untrackedCombos": state.untracked_combos, "untrackedLikeScopes": state.untracked, "unknownMethods": state.unknown_methods, "combos": state.combos.len(), "likeScopes": state.likes.len(), "capacity": CAPACITY, "retentionSeconds": RETENTION.as_secs()})
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use tiktools_tiktok::events::{EventMetadata, EventUser, GiftEvent};
    #[test]
    fn cumulative_progress_final_and_missing_groups() {
        let totals = LiveTotals::default();
        let mut event = NativeLiveEvent {
            base: CanonicalLiveEvent::Gift(GiftEvent {
                user: EventUser {
                    id: 1,
                    ..Default::default()
                },
                gift_id: 1,
                group_id: 2,
                repeat_count: 1,
                diamond_count: 2,
                ..Default::default()
            }),
            metadata: EventMetadata {
                method: "gift".into(),
                msg_id: 0,
                is_history: false,
            },
            gift: None,
        };
        assert_eq!(totals.observe(&event, None)["increment"], 1);
        if let CanonicalLiveEvent::Gift(g) = &mut event.base {
            g.repeat_count = 3;
        }
        assert_eq!(totals.observe(&event, None)["increment"], 2);
        if let CanonicalLiveEvent::Gift(g) = &mut event.base {
            g.repeat_end = true;
        }
        assert_eq!(totals.observe(&event, None)["increment"], 0);
        if let CanonicalLiveEvent::Gift(g) = &mut event.base {
            g.group_id = 0;
        }
        assert_eq!(totals.observe(&event, None)["increment"], 3);
        assert_eq!(totals.observe(&event, None)["increment"], 3);
        // An old incomplete combo can expire; observed units were already
        // accounted before expiry and no synthetic gift must be published.
        let mut state = recover_mutex(&totals.0, "test totals");
        for (_, _, touched) in state.combos.values_mut() {
            *touched = Instant::now() - RETENTION;
        }
        drop(state);
        if let CanonicalLiveEvent::Gift(g) = &mut event.base {
            g.group_id = 3;
        }
        assert_eq!(totals.observe(&event, None)["increment"], 3);
        assert_eq!(totals.diagnostics()["combos"], 1);
    }
}

#[cfg(test)]
mod like_tests {
    use super::*;
    use tiktools_tiktok::events::{EventMetadata, EventUser, LikeEvent};
    #[test]
    fn repeated_packets_accumulate_global_and_user_scopes_without_nickname_identity() {
        let totals = LiveTotals::default();
        let mut event = NativeLiveEvent {
            base: CanonicalLiveEvent::Like(LikeEvent {
                user: EventUser {
                    id: 1,
                    ..Default::default()
                },
                count: 10,
                ..Default::default()
            }),
            metadata: EventMetadata {
                method: "like".into(),
                msg_id: 0,
                is_history: false,
            },
            gift: None,
        };
        assert_eq!(totals.observe(&event, None)["global"]["total"], 10);
        let transition = totals.observe(&event, None);
        assert_eq!(transition["global"]["previous"], 10);
        assert_eq!(transition["global"]["total"], 20);
        assert_eq!(transition["user"]["total"], 20);
        if let CanonicalLiveEvent::Like(l) = &mut event.base {
            l.user.id = 2;
        }
        let transition = totals.observe(&event, None);
        assert_eq!(transition["global"]["total"], 30);
        assert_eq!(transition["user"]["total"], 10);
        if let CanonicalLiveEvent::Like(l) = &mut event.base {
            l.user.id = 0;
        }
        let transition = totals.observe(&event, None);
        assert_eq!(transition["global"]["total"], 40);
        assert!(transition["user"].is_null());
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use tiktools_tiktok::events::{EventMetadata, EventUser, GiftEvent};
    struct Emitter;
    impl HostEmitter for Emitter {
        fn emit(&self, _: HostMessage) {}
    }
    #[tokio::test]
    async fn gift_progress_finalization_and_replay_award_only_incremental_coins() {
        let core = Arc::new(AppCore::new(Arc::new(Emitter)));
        let coins = || {
            core.points
                .viewer("gift-delta-accounting-viewer")
                .and_then(|v| v["totalCoins"].as_i64())
                .unwrap_or(0)
        };
        let before = coins();
        let mut event = NativeLiveEvent {
            base: CanonicalLiveEvent::Gift(GiftEvent {
                user: EventUser {
                    id: 71,
                    unique_id: "gift-delta-accounting-viewer".into(),
                    ..Default::default()
                },
                gift_id: 1,
                group_id: 93,
                repeat_count: 1,
                diamond_count: 2,
                ..Default::default()
            }),
            metadata: EventMetadata {
                method: "WebcastGiftMessage".into(),
                msg_id: 8001,
                is_history: false,
            },
            gift: None,
        };
        core.handle_live_event(event.clone()).await;
        if let CanonicalLiveEvent::Gift(g) = &mut event.base {
            g.repeat_count = 3;
        }
        event.metadata.msg_id += 1;
        core.handle_live_event(event.clone()).await;
        if let CanonicalLiveEvent::Gift(g) = &mut event.base {
            g.repeat_end = true;
        }
        event.metadata.msg_id += 1;
        core.handle_live_event(event.clone()).await;
        event.metadata.is_history = true;
        core.handle_live_event(event).await;
        assert_eq!(coins() - before, 6);
        assert_eq!(core.live_ingress.ui_published.load(Ordering::Relaxed), 3);
        assert_eq!(core.live_dedup.duplicate_count(), 1);
        core.shutdown().await;
    }
}
