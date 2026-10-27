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
const LIKE_LEADERBOARD_LIMIT: usize = 10;

#[derive(Clone)]
struct LikeLeader {
    user_key: String,
    unique_id: String,
    nickname: String,
    avatar_url: Option<String>,
    score: u64,
}
type ComboKey = (String, String, u64, u64, u64);
#[derive(Default)]
pub(crate) struct LiveTotals(Mutex<State>);
#[derive(Default)]
struct State {
    combos: HashMap<ComboKey, (u64, u64, Instant)>,
    unknown_methods: std::collections::BTreeMap<String, u64>,
    likes: HashMap<(String, String, String), u64>,
    // Bounded top-ten projections keyed by creator + LIVE room. Counters in
    // likes remain authoritative; display names are not used as identities.
    like_leaders: HashMap<(String, String), Vec<LikeLeader>>,
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
                // Update only ten candidates (O(10), not a scan of every
                // viewer) on every accepted packet. No ranking is inferred
                // from the room-wide TikTok total or nicknames.
                if like.count > 0 {
                    if let Some(total) = totals.get(1).and_then(|value| value["total"].as_u64()) {
                        let user_key = if like.user.id != 0 {
                            like.user.id.to_string()
                        } else {
                            like.user.unique_id.clone()
                        };
                        if !user_key.is_empty() {
                            let leaders = state.like_leaders.entry((creator, room)).or_default();
                            let unique_id = clean_unique_id(&like.user.unique_id)
                                .unwrap_or_else(|| user_key.clone());
                            if let Some(leader) =
                                leaders.iter_mut().find(|leader| leader.user_key == user_key)
                            {
                                leader.score = total;
                                leader.unique_id = unique_id;
                                leader.nickname = like.user.nickname.clone();
                                leader.avatar_url = like.user.avatar_url.clone();
                            } else if leaders.len() < LIKE_LEADERBOARD_LIMIT
                                || leaders.last().is_some_and(|last| total > last.score)
                            {
                                leaders.push(LikeLeader {
                                    user_key,
                                    unique_id,
                                    nickname: like.user.nickname.clone(),
                                    avatar_url: like.user.avatar_url.clone(),
                                    score: total,
                                });
                            }
                            leaders.sort_unstable_by(|a, b| {
                                b.score.cmp(&a.score).then_with(|| a.user_key.cmp(&b.user_key))
                            });
                            leaders.truncate(LIKE_LEADERBOARD_LIMIT);
                        }
                    }
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
    /// Current authoritative per-viewer like ranking for one creator/room.
    /// This is independent of the lossy UI feed, points and TikTok snapshots.
    pub(crate) fn like_leaderboard(&self, context: &LiveContext) -> Vec<Value> {
        let state = recover_mutex(&self.0, "LIVE like leaderboard");
        let key = (context.unique_id.clone(), context.room_id.clone());
        state
            .like_leaders
            .get(&key)
            .into_iter()
            .flat_map(|leaders| leaders.iter())
            .enumerate()
            .map(|(index, leader)| {
                let mut item = json!({
                    "rank": index + 1,
                    "score": leader.score,
                    "uniqueId": leader.unique_id,
                    "nickname": leader.nickname,
                    "userId": leader.user_key
                });
                if let Some(url) = &leader.avatar_url {
                    item["avatarUrl"] = json!(url);
                }
                item
            })
            .collect()
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
    fn like_batches_contribute_packet_count_not_room_cumulative_total() {
        let totals = LiveTotals::default();
        let mut event = NativeLiveEvent {
            base: CanonicalLiveEvent::Like(LikeEvent {
                user: EventUser {
                    id: 11,
                    ..Default::default()
                },
                count: 15,
                total: 1000,
            }),
            metadata: EventMetadata {
                method: "WebcastLikeMessage".into(),
                msg_id: 101,
                is_history: false,
            },
            gift: None,
        };
        let first = totals.observe(&event, None);
        assert_eq!(first["global"], json!({"previous": 0, "total": 15}));
        assert_eq!(first["user"], json!({"previous": 0, "total": 15}));
        if let CanonicalLiveEvent::Like(like) = &mut event.base {
            like.user.id = 12;
            like.total = 1015;
            like.count = 15;
        }
        let second = totals.observe(&event, None);
        assert_eq!(second["global"], json!({"previous": 15, "total": 30}));
        assert_eq!(second["user"], json!({"previous": 0, "total": 15}));
    }

    #[test]
    fn like_leaderboard_ranks_only_observed_packet_deltas_per_room() {
        let totals = LiveTotals::default();
        let room_a = LiveContext {
            unique_id: "creator".into(),
            room_id: "room-a".into(),
            connection_id: "session-a".into(),
        };
        let room_b = LiveContext {
            unique_id: "creator".into(),
            room_id: "room-b".into(),
            connection_id: "session-b".into(),
        };
        let mut event = NativeLiveEvent {
            base: CanonicalLiveEvent::Like(LikeEvent {
                user: EventUser {
                    id: 1,
                    unique_id: "alice".into(),
                    nickname: "Alice".into(),
                    ..Default::default()
                },
                count: 15,
                total: 50_000,
            }),
            metadata: EventMetadata {
                method: "WebcastLikeMessage".into(),
                msg_id: 41,
                is_history: false,
            },
            gift: None,
        };
        totals.observe(&event, Some(&room_a));
        if let CanonicalLiveEvent::Like(like) = &mut event.base {
            like.count = 10;
            like.total = 60_000;
            like.user.id = 2;
            like.user.unique_id = "bob".into();
            like.user.nickname = "Bob".into();
        }
        totals.observe(&event, Some(&room_a));
        if let CanonicalLiveEvent::Like(like) = &mut event.base {
            like.count = 20;
            like.user.id = 2;
        }
        totals.observe(&event, Some(&room_a));

        let leaders = totals.like_leaderboard(&room_a);
        assert_eq!(leaders.len(), 2);
        assert_eq!(leaders[0]["rank"], 1);
        assert_eq!(leaders[0]["uniqueId"], "bob");
        assert_eq!(leaders[0]["score"], 30);
        assert_eq!(leaders[1]["rank"], 2);
        assert_eq!(leaders[1]["uniqueId"], "alice");
        assert_eq!(leaders[1]["score"], 15);
        assert!(totals.like_leaderboard(&room_b).is_empty());

        // Same viewer can independently rank within another room.
        totals.observe(&event, Some(&room_b));
        let other = totals.like_leaderboard(&room_b);
        assert_eq!(other.len(), 1);
        assert_eq!(other[0]["score"], 20);

        // A missing stable identity must never enter the ranked leaderboard.
        if let CanonicalLiveEvent::Like(like) = &mut event.base {
            like.user.id = 0;
            like.user.unique_id.clear();
            like.count = 9;
        }
        totals.observe(&event, Some(&room_a));
        assert_eq!(totals.like_leaderboard(&room_a).len(), 2);
    }

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
