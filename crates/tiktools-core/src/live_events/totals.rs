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
const LIKE_WIDGET_LIMIT: usize = 500;

#[derive(Clone)]
struct LikeIdentity {
    unique_id: String,
    nickname: String,
    avatar_url: Option<String>,
}
type ComboKey = (String, String, u64, u64, u64);
#[derive(Default)]
pub(crate) struct LiveTotals(Mutex<State>);
#[derive(Default)]
struct State {
    combos: HashMap<ComboKey, (u64, u64, Instant)>,
    unknown_methods: std::collections::BTreeMap<String, u64>,
    // Global counters serve existing automation; per-user scores and order
    // are owned by the shared aggregation primitive.
    likes: HashMap<(String, String), u64>,
    like_rankings: HashMap<(String, String), tiktools_aggregation::Aggregation>,
    like_participants: usize,
    like_identities: HashMap<(String, String, String), LikeIdentity>,
    like_revision: u64,
    like_degraded_scopes: std::collections::HashSet<(String, String)>,
    like_untracked_scope_failure: bool,
    like_unknown_ingress_loss: bool,
    untracked: u64,
    like_overflow: u64,
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
                let untracked_before = state.untracked;
                let user = if like.user.id != 0 {
                    like.user.id.to_string()
                } else {
                    like.user.unique_id.clone()
                };
                let scope = (creator.clone(), room.clone());
                let global = if !state.likes.contains_key(&scope)
                    && state.likes.len() + state.like_participants >= CAPACITY
                {
                    state.untracked += 1;
                    Value::Null
                } else {
                    let total = state.likes.entry(scope.clone()).or_default();
                    let previous = *total;
                    match total.checked_add(like.count) {
                        Some(next) => {
                            *total = next;
                            json!({"previous": previous, "total": next})
                        }
                        None => {
                            state.untracked += 1;
                            state.like_overflow += 1;
                            Value::Null
                        }
                    }
                };
                let user_total = if user.is_empty() {
                    None
                } else {
                    let previous = state
                        .like_rankings
                        .get(&scope)
                        .and_then(|ranking| ranking.score(&user));
                    if previous.is_none() && state.likes.len() + state.like_participants >= CAPACITY
                    {
                        state.untracked += 1;
                        Some(Value::Null)
                    } else if like.count == 0 {
                        Some(
                            json!({"previous": previous.unwrap_or(0), "total": previous.unwrap_or(0)}),
                        )
                    } else {
                        let ranking = state
                            .like_rankings
                            .entry(scope.clone())
                            .or_insert_with(|| tiktools_aggregation::Aggregation::new(CAPACITY));
                        match ranking.apply(&user, like.count, 1) {
                            Ok(()) => {
                                let total = ranking.score(&user).expect("positive accepted score");
                                if previous.is_none() {
                                    state.like_participants += 1;
                                }
                                state.like_identities.insert(
                                    (creator, room, user.clone()),
                                    LikeIdentity {
                                        unique_id: clean_unique_id(&like.user.unique_id)
                                            .unwrap_or_else(|| user.clone()),
                                        nickname: like.user.nickname.clone(),
                                        avatar_url: like.user.avatar_url.clone(),
                                    },
                                );
                                state.like_revision = state.like_revision.saturating_add(1);
                                Some(json!({"previous": previous.unwrap_or(0), "total": total}))
                            }
                            Err(error) => {
                                state.untracked += 1;
                                if error == tiktools_aggregation::Error::Overflow {
                                    state.like_overflow += 1;
                                }
                                Some(Value::Null)
                            }
                        }
                    }
                };
                if state.untracked != untracked_before {
                    let changed = if state.likes.contains_key(&scope)
                        && (state.like_degraded_scopes.contains(&scope)
                            || state.like_degraded_scopes.len() < CAPACITY)
                    {
                        state.like_degraded_scopes.insert(scope)
                    } else if state.likes.contains_key(&scope) && !state.like_unknown_ingress_loss {
                        state.like_unknown_ingress_loss = true;
                        true
                    } else if !state.like_untracked_scope_failure {
                        state.like_untracked_scope_failure = true;
                        true
                    } else {
                        false
                    };
                    if changed {
                        state.like_revision = state.like_revision.saturating_add(1);
                    }
                }
                json!({"global": global, "user": user_total})
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
    /// A native ingress drop may contain likes even when its payload is not
    /// available. Keep the loss sticky for the affected scope.
    pub(crate) fn mark_like_ingress_loss(&self, context: Option<&LiveContext>) {
        let mut state = recover_mutex(&self.0, "LIVE like leaderboard");
        let changed = match context {
            Some(context)
                if state.like_degraded_scopes.len() < CAPACITY
                    || state
                        .like_degraded_scopes
                        .contains(&(context.unique_id.clone(), context.room_id.clone())) =>
            {
                state
                    .like_degraded_scopes
                    .insert((context.unique_id.clone(), context.room_id.clone()))
            }
            _ if !state.like_unknown_ingress_loss => {
                state.like_unknown_ingress_loss = true;
                true
            }
            _ => false,
        };
        if changed {
            state.like_revision = state.like_revision.saturating_add(1);
        }
    }

    /// A local failure is scoped to a tracked room. If capacity prevented a
    /// room from being tracked, unknown rooms are conservatively incomplete.
    pub(crate) fn like_integrity_status(&self, context: &LiveContext) -> &'static str {
        let state = recover_mutex(&self.0, "LIVE like leaderboard");
        let scope = (context.unique_id.clone(), context.room_id.clone());
        if state.like_unknown_ingress_loss
            || state.like_degraded_scopes.contains(&scope)
            || (state.like_untracked_scope_failure && !state.likes.contains_key(&scope))
        {
            "degraded"
        } else {
            "unknown"
        }
    }
    pub(crate) fn like_revision(&self) -> u64 {
        recover_mutex(&self.0, "LIVE like leaderboard").like_revision
    }
    pub(crate) fn like_leaderboard(&self, context: &LiveContext) -> Vec<Value> {
        self.like_leaderboard_with_limit(context, LIKE_LEADERBOARD_LIMIT)
            .1
    }

    /// Bounded wider projection for widgets. Revision is a process-local
    /// presentation token, not durable accounting identity.
    pub(crate) fn like_leaderboard_with_limit(
        &self,
        context: &LiveContext,
        limit: usize,
    ) -> (u64, Vec<Value>) {
        let state = recover_mutex(&self.0, "LIVE like leaderboard");
        let key = (context.unique_id.clone(), context.room_id.clone());
        let leaders = state.like_rankings.get(&key).into_iter()
            .flat_map(|ranking| ranking.snapshot(limit.min(LIKE_WIDGET_LIMIT)).entries)
            .filter_map(|entry| {
                let identity = state.like_identities.get(&(key.0.clone(), key.1.clone(), entry.key.clone()))?;
                let mut item = json!({"rank": entry.rank, "score": entry.score, "uniqueId": identity.unique_id, "nickname": identity.nickname, "userId": entry.key});
                if let Some(url) = &identity.avatar_url { item["avatarUrl"] = json!(url); }
                Some(item)
            }).collect();
        (state.like_revision, leaders)
    }

    pub(crate) fn diagnostics(&self) -> Value {
        let state = recover_mutex(&self.0, "LIVE totals");
        json!({"untrackedCombos": state.untracked_combos, "untrackedLikeScopes": state.untracked, "likeOverflow": state.like_overflow, "unknownMethods": state.unknown_methods, "combos": state.combos.len(), "likeScopes": state.likes.len() + state.like_participants, "capacity": CAPACITY, "retentionSeconds": RETENTION.as_secs()})
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
    fn likes_ingress_loss_is_sticky_scoped_and_bounded() {
        let totals = LiveTotals::default();
        let room = LiveContext {
            unique_id: "creator".into(),
            room_id: "a".into(),
            connection_id: "session".into(),
        };
        let other = LiveContext {
            room_id: "b".into(),
            ..room.clone()
        };
        totals.mark_like_ingress_loss(Some(&room));
        assert_eq!(totals.like_integrity_status(&room), "degraded");
        assert_eq!(totals.like_integrity_status(&other), "unknown");
        let revision = totals.like_revision();
        totals.mark_like_ingress_loss(Some(&room));
        assert_eq!(totals.like_revision(), revision);
        totals.mark_like_ingress_loss(None);
        assert_eq!(totals.like_integrity_status(&other), "degraded");
    }

    #[test]
    fn like_projection_reports_overflow_and_preserves_existing_scores() {
        let totals = LiveTotals::default();
        let context = LiveContext {
            unique_id: "creator".into(),
            room_id: "room".into(),
            connection_id: "session".into(),
        };
        let mut event = NativeLiveEvent {
            base: CanonicalLiveEvent::Like(LikeEvent {
                user: EventUser {
                    id: 1,
                    ..Default::default()
                },
                count: u64::MAX,
                total: 0,
            }),
            metadata: EventMetadata {
                method: "like".into(),
                msg_id: 1,
                is_history: false,
            },
            gift: None,
        };
        assert_eq!(
            totals.observe(&event, Some(&context))["user"]["total"],
            u64::MAX
        );
        if let CanonicalLiveEvent::Like(like) = &mut event.base {
            like.count = 1;
        }
        let rejected = totals.observe(&event, Some(&context));
        assert!(rejected["user"].is_null());
        assert!(rejected["global"].is_null());
        assert_eq!(totals.like_leaderboard(&context)[0]["score"], u64::MAX);
        assert_eq!(totals.diagnostics()["untrackedLikeScopes"], 2);
        assert_eq!(totals.diagnostics()["likeOverflow"], 2);
        assert_eq!(totals.like_integrity_status(&context), "degraded");
        let other = LiveContext {
            unique_id: "creator".into(),
            room_id: "other".into(),
            connection_id: "session".into(),
        };
        assert_eq!(totals.like_integrity_status(&other), "unknown");
        assert!(totals.like_revision() > 1);
    }

    #[test]
    fn like_projection_bounds_new_users_without_dropping_existing_users() {
        let totals = LiveTotals::default();
        let context = LiveContext {
            unique_id: "creator".into(),
            room_id: "room".into(),
            connection_id: "session".into(),
        };
        let mut event = NativeLiveEvent {
            base: CanonicalLiveEvent::Like(LikeEvent {
                user: EventUser {
                    id: 1,
                    ..Default::default()
                },
                count: 1,
                total: 0,
            }),
            metadata: EventMetadata {
                method: "like".into(),
                msg_id: 1,
                is_history: false,
            },
            gift: None,
        };
        for id in 1..CAPACITY as u64 {
            if let CanonicalLiveEvent::Like(like) = &mut event.base {
                like.user.id = id;
            }
            totals.observe(&event, Some(&context));
        }
        if let CanonicalLiveEvent::Like(like) = &mut event.base {
            like.user.id = CAPACITY as u64;
        }
        assert!(totals.observe(&event, Some(&context))["user"].is_null());
        if let CanonicalLiveEvent::Like(like) = &mut event.base {
            like.user.id = 1;
        }
        assert_eq!(totals.observe(&event, Some(&context))["user"]["total"], 2);
        assert_eq!(totals.diagnostics()["likeScopes"], CAPACITY);
        assert_eq!(totals.diagnostics()["untrackedLikeScopes"], 1);
        assert_eq!(totals.like_integrity_status(&context), "degraded");
        let unseen = LiveContext {
            unique_id: "creator".into(),
            room_id: "unseen".into(),
            connection_id: "session".into(),
        };
        totals.observe(&event, Some(&unseen));
        assert_eq!(totals.like_integrity_status(&unseen), "degraded");
        assert!(recover_mutex(&totals.0, "test").like_degraded_scopes.len() <= CAPACITY);
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
    fn wider_like_projection_is_bounded_and_preserves_legacy_top_ten() {
        let totals = LiveTotals::default();
        let context = LiveContext {
            unique_id: "creator".into(),
            room_id: "room".into(),
            connection_id: "session".into(),
        };
        for id in 1..=600 {
            let event = NativeLiveEvent {
                base: CanonicalLiveEvent::Like(LikeEvent {
                    user: EventUser {
                        id,
                        unique_id: format!("u{id}"),
                        nickname: format!("User {id}"),
                        ..Default::default()
                    },
                    count: id,
                    total: 1_000_000,
                }),
                metadata: EventMetadata {
                    method: "like".into(),
                    msg_id: id,
                    is_history: false,
                },
                gift: None,
            };
            totals.observe(&event, Some(&context));
        }
        let (_, wider) = totals.like_leaderboard_with_limit(&context, 9999);
        let top = totals.like_leaderboard(&context);
        assert_eq!(wider.len(), 500);
        assert_eq!(top, wider[..10]);
        assert_eq!(wider[0]["userId"], "600");
        assert_eq!(wider[499]["userId"], "101");
        let update = NativeLiveEvent {
            base: CanonicalLiveEvent::Like(LikeEvent {
                user: EventUser {
                    id: 1,
                    unique_id: "u1".into(),
                    nickname: "Updated".into(),
                    ..Default::default()
                },
                count: 1000,
                total: 1_001_000,
            }),
            metadata: EventMetadata {
                method: "like".into(),
                msg_id: 1001,
                is_history: false,
            },
            gift: None,
        };
        totals.observe(&update, Some(&context));
        let (_, updated) = totals.like_leaderboard_with_limit(&context, 500);
        assert_eq!(updated[0]["userId"], "1");
        assert_eq!(updated[0]["score"], 1001);
        assert_eq!(updated[0]["nickname"], "Updated");
        assert_eq!(
            updated
                .iter()
                .filter(|entry| entry["userId"] == "1")
                .count(),
            1
        );
    }

    #[test]
    fn observed_like_ranking_matches_generic_engine_for_scores_ties_and_rooms() {
        use tiktools_aggregation::{
            engine::Engine,
            model::{Contribution, Definition, SourceRule},
        };
        let totals = LiveTotals::default();
        let mut engine = Engine::default();
        let rooms = ["room-a", "room-b"];
        for room in rooms {
            engine
                .create(Definition {
                    schema_version: 1,
                    id: room.into(),
                    creator_id: "creator".into(),
                    room_id: room.into(),
                    group_by: Default::default(),
                    window: Default::default(),
                    user_limits: None,
                    capacity: 100,
                    limit: 10,
                    sources: vec![SourceRule {
                        source: "tiktok.like".into(),
                        metric: "like.count".into(),
                        weight: 1,
                        enabled: true,
                        gift_multipliers: Default::default(),
                    }],
                })
                .unwrap();
        }
        // Insert in reverse order so tied boundary candidates must be replaced
        // by the stable identity order, not arrival order.
        for (index, id) in (1..=14).rev().enumerate() {
            let room = rooms[index % 2];
            let context = LiveContext {
                unique_id: "creator".into(),
                room_id: room.into(),
                connection_id: "session".into(),
            };
            let key = id.to_string();
            let event = NativeLiveEvent {
                base: CanonicalLiveEvent::Like(LikeEvent {
                    user: EventUser {
                        id,
                        unique_id: format!("viewer{id}"),
                        nickname: format!("Viewer {id}"),
                        ..Default::default()
                    },
                    count: 10,
                    total: 100_000,
                }),
                metadata: EventMetadata {
                    method: "like".into(),
                    msg_id: id,
                    is_history: false,
                },
                gift: None,
            };
            totals.observe(&event, Some(&context));
            assert!(engine
                .apply_at(
                    &Contribution {
                        creator_id: "creator",
                        room_id: room,
                        source: "tiktok.like",
                        user: Some(&key),
                        gift: None,
                        option: None,
                        team: None,
                        custom_key: None,
                        metrics: &[("like.count", 10)]
                    },
                    100
                )
                .is_empty());
        }
        for id in (1..=14).rev() {
            let context = LiveContext {
                unique_id: "creator".into(),
                room_id: "room-a".into(),
                connection_id: "session".into(),
            };
            let key = id.to_string();
            let event = NativeLiveEvent {
                base: CanonicalLiveEvent::Like(LikeEvent {
                    user: EventUser {
                        id,
                        unique_id: format!("viewer{id}"),
                        nickname: format!("Viewer {id}"),
                        ..Default::default()
                    },
                    count: 10,
                    total: 200_000,
                }),
                metadata: EventMetadata {
                    method: "like".into(),
                    msg_id: id + 100,
                    is_history: false,
                },
                gift: None,
            };
            totals.observe(&event, Some(&context));
            engine.apply_at(
                &Contribution {
                    creator_id: "creator",
                    room_id: "room-a",
                    source: "tiktok.like",
                    user: Some(&key),
                    gift: None,
                    option: None,
                    team: None,
                    custom_key: None,
                    metrics: &[("like.count", 10)],
                },
                101,
            );
        }
        for room in rooms {
            let context = LiveContext {
                unique_id: "creator".into(),
                room_id: room.into(),
                connection_id: "session".into(),
            };
            let legacy = totals.like_leaderboard(&context);
            let snapshot = engine.snapshot(room).unwrap();
            assert_eq!(legacy.len(), snapshot.entries.len());
            for (old, entry) in legacy.iter().zip(snapshot.entries.iter()) {
                assert_eq!(old["userId"].as_str(), Some(entry.key.as_str()));
                assert_eq!(old["score"].as_u64(), Some(entry.score));
                assert_eq!(old["rank"].as_u64(), Some(entry.rank as u64));
                assert_eq!(old["nickname"], format!("Viewer {}", entry.key));
            }
        }
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
    #[cfg(all(feature = "persistence", feature = "native-tiktok"))]
    #[tokio::test]
    async fn mixed_poll_scores_canonical_likes_and_recovers_them() {
        use tiktools_aggregation::model::{Definition as Ranking, GroupBy, SourceRule};
        use tiktools_competitions::{
            CompetitionKind, Definition, OptionDefinition, SelectionMode, TiePolicy, Transition,
        };
        use tiktools_tiktok::events::{ChatEvent, LikeEvent, SocialEvent};
        let root = std::env::temp_dir().join(format!(
            "tiktools-mixed-poll-{}-{}",
            std::process::id(),
            crate::now_millis()
        ));
        let paths = crate::paths::AppPaths {
            data: root.join("data"),
            plugins: root.join("plugins"),
            plugin_data: root.join("plugin-data"),
            builtin_plugins: root.join("builtin"),
            development_plugins: None,
            logs: root.join("logs"),
            temp: root.join("temp"),
            root: root.clone(),
        };
        let core = Arc::new(AppCore::with_paths(
            Arc::new(Emitter),
            Arc::new(crate::NoopMediaHost),
            paths.clone(),
        ));
        *core.connection_context.write().unwrap() = Some(LiveContext {
            unique_id: "creator".into(),
            room_id: "room".into(),
            connection_id: "session".into(),
        });
        let rule = |source: &str, metric: &str, weight| SourceRule {
            source: source.into(),
            metric: metric.into(),
            weight,
            enabled: true,
            gift_multipliers: Default::default(),
        };
        core.ranking_create(Ranking {
            schema_version: 1,
            id: "rank".into(),
            creator_id: "creator".into(),
            room_id: "room".into(),
            group_by: GroupBy::Option,
            window: Default::default(),
            user_limits: None,
            sources: vec![
                rule("competition.vote", "event.count", 1),
                rule("tiktok.like", "like.count", 2),
                rule("tiktok.share", "share.count", 3),
                rule("tiktok.gift", "gift.diamonds", 1),
            ],
            limit: 10,
            capacity: 10,
        })
        .unwrap();
        let option = |id: &str| OptionDefinition {
            id: id.into(),
            label: id.into(),
            commands: vec![format!("!{id}")],
            gift_ids: if id == "a" { vec!["77".into()] } else { vec![] },
            image_url: None,
            on_win: None,
        };
        core.competition_create(Definition {
            schema_version: 1,
            id: "poll".into(),
            name: "Poll".into(),
            kind: CompetitionKind::TeamBattle,
            ranking_id: "rank".into(),
            options: vec![option("a"), option("b")],
            selection_mode: SelectionMode::HighestScore,
            tie_policy: TiePolicy::Stable,
            duration_ms: None,
            vote_limit: 1,
            allow_switch: true,
            minimum_participants: 2,
            target_score: None,
            on_user_win: None,
        })
        .unwrap();
        // A draft cannot be rebound to a ranking carrying old vote scores.
        let draft = core.competition_get("poll").unwrap();
        core.aggregations
            .engine
            .lock()
            .unwrap()
            .apply_assignment("rank", None, ("a", 1))
            .unwrap();
        assert!(core.competition_update(draft.clone()).is_err());
        core.ranking_reset("rank").unwrap();
        core.competition_update(draft).unwrap();
        let draft_command = NativeLiveEvent {
            base: CanonicalLiveEvent::Chat(ChatEvent {
                user: EventUser {
                    id: 42,
                    ..Default::default()
                },
                comment: "!a".into(),
                ..Default::default()
            }),
            metadata: EventMetadata {
                method: "test".into(),
                msg_id: 900,
                is_history: false,
            },
            gift: None,
        };
        core.handle_live_event(draft_command).await;
        let draft_like = NativeLiveEvent {
            base: CanonicalLiveEvent::Like(LikeEvent {
                user: EventUser {
                    id: 42,
                    ..Default::default()
                },
                count: 5,
                total: 5,
            }),
            metadata: EventMetadata {
                method: "test".into(),
                msg_id: 901,
                is_history: false,
            },
            gift: None,
        };
        core.handle_live_event(draft_like).await;
        let draft_unknown_gift = NativeLiveEvent {
            base: CanonicalLiveEvent::Gift(GiftEvent {
                user: EventUser {
                    id: 42,
                    ..Default::default()
                },
                gift_id: 9,
                group_id: 22,
                repeat_count: 1,
                diamond_count: 0,
                ..Default::default()
            }),
            metadata: EventMetadata {
                method: "gift".into(),
                msg_id: 902,
                is_history: false,
            },
            gift: None,
        };
        core.handle_live_event(draft_unknown_gift).await;
        assert_ne!(
            core.ranking_snapshot("rank").unwrap().integrity_status,
            "degraded"
        );
        assert_eq!(core.competitions.diagnostics()["failed"], 0);
        core.competition_transition("poll", Transition::Start)
            .unwrap();
        let event = |id, base| NativeLiveEvent {
            base,
            metadata: EventMetadata {
                method: "test".into(),
                msg_id: id,
                is_history: false,
            },
            gift: None,
        };
        let user = EventUser {
            id: 42,
            ..Default::default()
        };
        let like = event(
            1,
            CanonicalLiveEvent::Like(LikeEvent {
                user: user.clone(),
                count: 5,
                total: 900,
            }),
        );
        core.handle_live_event(like.clone()).await;
        core.handle_live_event(event(
            2,
            CanonicalLiveEvent::Chat(ChatEvent {
                user: user.clone(),
                comment: "!a".into(),
                ..Default::default()
            }),
        ))
        .await;
        core.handle_live_event(like).await;
        core.handle_live_event(event(
            3,
            CanonicalLiveEvent::Like(LikeEvent {
                user: user.clone(),
                count: 5,
                total: 905,
            }),
        ))
        .await;
        core.handle_live_event(event(
            4,
            CanonicalLiveEvent::Chat(ChatEvent {
                user: user.clone(),
                comment: "!b".into(),
                ..Default::default()
            }),
        ))
        .await;
        // share_count is an upstream cumulative counter, not the packet delta.
        let share = event(
            5,
            CanonicalLiveEvent::Social(SocialEvent {
                user: user.clone(),
                action: 3,
                share_count: 9000,
                ..Default::default()
            }),
        );
        core.handle_live_event(share.clone()).await;
        core.handle_live_event(share.clone()).await;
        let gift = event(
            10,
            CanonicalLiveEvent::Gift(GiftEvent {
                user: EventUser {
                    id: 42,
                    ..Default::default()
                },
                gift_id: 9,
                group_id: 22,
                repeat_count: 2,
                diamond_count: 3,
                ..Default::default()
            }),
        );
        let mut gift_start = gift.clone();
        gift_start.metadata.msg_id = 9;
        if let CanonicalLiveEvent::Gift(item) = &mut gift_start.base {
            item.repeat_count = 1;
        }
        core.handle_live_event(gift_start).await;
        core.handle_live_event(gift.clone()).await;
        core.handle_live_event(gift.clone()).await;
        let gift_final = event(
            11,
            CanonicalLiveEvent::Gift(GiftEvent {
                user: EventUser {
                    id: 42,
                    ..Default::default()
                },
                gift_id: 9,
                group_id: 22,
                repeat_count: 2,
                diamond_count: 3,
                repeat_end: true,
                ..Default::default()
            }),
        );
        core.handle_live_event(gift_final).await;
        // A sender without a command selects team A through the gift binding.
        let direct_gift = event(
            12,
            CanonicalLiveEvent::Gift(GiftEvent {
                user: EventUser {
                    id: 43,
                    ..Default::default()
                },
                gift_id: 77,
                group_id: 99,
                repeat_count: 1,
                diamond_count: 3,
                ..Default::default()
            }),
        );
        core.handle_live_event(direct_gift.clone()).await;
        core.handle_live_event(event(
            13,
            CanonicalLiveEvent::Like(LikeEvent {
                user: EventUser {
                    id: 43,
                    ..Default::default()
                },
                count: 100,
                total: 100,
            }),
        ))
        .await;
        assert_eq!(core.db.competition_participant_count("poll", 1).unwrap(), 2);
        core.competition_transition("poll", Transition::Pause)
            .unwrap();
        let paused_share = event(
            6,
            CanonicalLiveEvent::Social(SocialEvent {
                user,
                action: 3,
                ..Default::default()
            }),
        );
        core.handle_live_event(paused_share.clone()).await;
        core.competition_transition("poll", Transition::Resume)
            .unwrap();
        let snapshot = core.ranking_snapshot("rank").unwrap();
        assert_eq!(snapshot.total_score, 23);
        assert_eq!(
            snapshot
                .entries
                .iter()
                .map(|e| (e.key.as_str(), e.score))
                .collect::<Vec<_>>(),
            vec![("a", 13), ("b", 10)]
        );
        let scores = core.db.competition_scored_totals("poll", 1).unwrap();
        assert_eq!(scores, vec![("a".into(), 13), ("b".into(), 9)]);
        core.shutdown().await;
        drop(core);
        let restarted = Arc::new(AppCore::with_paths(
            Arc::new(Emitter),
            Arc::new(crate::NoopMediaHost),
            paths.clone(),
        ));
        *restarted.connection_context.write().unwrap() = Some(LiveContext {
            unique_id: "creator".into(),
            room_id: "room".into(),
            connection_id: "reconnected".into(),
        });
        assert_eq!(restarted.ranking_snapshot("rank").unwrap().total_score, 23);
        restarted.handle_live_event(direct_gift).await;
        restarted.handle_live_event(gift).await;
        restarted.handle_live_event(share).await;
        restarted.handle_live_event(paused_share).await;
        assert_eq!(restarted.ranking_snapshot("rank").unwrap().total_score, 23);
        assert_eq!(
            restarted
                .competition_finish("poll")
                .unwrap()
                .selection
                .winner
                .as_deref(),
            Some("a")
        );
        restarted.shutdown().await;
        drop(restarted);
        let finished = Arc::new(AppCore::with_paths(
            Arc::new(Emitter),
            Arc::new(crate::NoopMediaHost),
            paths.clone(),
        ));
        *finished.connection_context.write().unwrap() = Some(LiveContext {
            unique_id: "creator".into(),
            room_id: "room".into(),
            connection_id: "after-result".into(),
        });
        finished
            .handle_live_event(event(
                7,
                CanonicalLiveEvent::Chat(ChatEvent {
                    user: EventUser {
                        id: 42,
                        ..Default::default()
                    },
                    comment: "!a".into(),
                    ..Default::default()
                }),
            ))
            .await;
        finished
            .handle_live_event(event(
                8,
                CanonicalLiveEvent::Like(LikeEvent {
                    user: EventUser {
                        id: 42,
                        ..Default::default()
                    },
                    count: 1,
                    total: 1,
                }),
            ))
            .await;
        assert_eq!(finished.competitions.diagnostics()["failed"], 0);
        assert_ne!(
            finished.ranking_snapshot("rank").unwrap().integrity_status,
            "degraded"
        );
        assert_eq!(finished.ranking_snapshot("rank").unwrap().total_score, 23);
        for (id, rank) in [("active", "active-rank"), ("draft", "draft-rank")] {
            let mut ranking = finished.ranking_get("rank").unwrap();
            ranking.id = rank.into();
            finished.ranking_create(ranking).unwrap();
            let mut definition = finished.competition_get("poll").unwrap();
            definition.id = id.into();
            definition.ranking_id = rank.into();
            finished.competition_create(definition).unwrap();
        }
        finished
            .competition_transition("active", Transition::Start)
            .unwrap();
        finished
            .handle_live_event(event(
                14,
                CanonicalLiveEvent::Gift(GiftEvent {
                    user: EventUser {
                        id: 43,
                        ..Default::default()
                    },
                    gift_id: 77,
                    group_id: 100,
                    repeat_count: 1,
                    diamond_count: 0,
                    ..Default::default()
                }),
            ))
            .await;
        assert_eq!(
            finished
                .ranking_snapshot("active-rank")
                .unwrap()
                .integrity_status,
            "degraded"
        );
        assert_eq!(
            finished
                .ranking_snapshot("draft-rank")
                .unwrap()
                .integrity_status,
            "unknown"
        );
        assert_eq!(
            finished.ranking_snapshot("rank").unwrap().integrity_status,
            "unknown"
        );
        for (id, rank) in [("broken", "broken-rank"), ("healthy", "healthy-rank")] {
            let mut ranking = finished.ranking_get("rank").unwrap();
            ranking.id = rank.into();
            finished.ranking_create(ranking).unwrap();
            let mut definition = finished.competition_get("poll").unwrap();
            definition.id = id.into();
            definition.ranking_id = rank.into();
            finished.competition_create(definition).unwrap();
            finished
                .competition_transition(id, Transition::Start)
                .unwrap();
        }
        let connection = rusqlite::Connection::open(finished.db.automation_path()).unwrap();
        connection.execute_batch("CREATE TRIGGER reject_broken_gift BEFORE INSERT ON competition_gift_receipts WHEN NEW.competition_id='broken' BEGIN SELECT RAISE(ABORT, 'injected failure'); END;").unwrap();
        finished
            .handle_live_event(event(
                15,
                CanonicalLiveEvent::Gift(GiftEvent {
                    user: EventUser {
                        id: 43,
                        ..Default::default()
                    },
                    gift_id: 77,
                    group_id: 101,
                    repeat_count: 1,
                    diamond_count: 3,
                    ..Default::default()
                }),
            ))
            .await;
        assert_eq!(
            finished
                .ranking_snapshot("broken-rank")
                .unwrap()
                .integrity_status,
            "degraded"
        );
        assert_eq!(
            finished
                .ranking_snapshot("broken-rank")
                .unwrap()
                .total_score,
            0
        );
        assert_eq!(
            finished
                .ranking_snapshot("healthy-rank")
                .unwrap()
                .integrity_status,
            "unknown"
        );
        assert_eq!(
            finished
                .ranking_snapshot("healthy-rank")
                .unwrap()
                .total_score,
            3
        );
        let rolled_back: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM competition_gift_combos WHERE competition_id='broken'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(rolled_back, 0);
        connection
            .execute_batch("DROP TRIGGER reject_broken_gift")
            .unwrap();
        drop(connection);
        let mut draw_ranking = finished.ranking_get("rank").unwrap();
        draw_ranking.id = "draw-rank".into();
        finished.ranking_create(draw_ranking).unwrap();
        let mut draw = finished.competition_get("poll").unwrap();
        draw.id = "draw".into();
        draw.ranking_id = "draw-rank".into();
        draw.kind = CompetitionKind::RandomDraw;
        draw.selection_mode = SelectionMode::UniformRandom;
        draw.minimum_participants = 2;
        draw.options[0].id = "42".into();
        draw.options[0].on_win = Some(tiktools_competitions::WinnerAction {
            action_id: "must-not-run".into(),
            inputs: json!({}),
        });
        finished.competition_create(draw).unwrap();
        finished
            .competition_transition("draw", Transition::Start)
            .unwrap();
        for (message_id, user_id) in [(21, 42), (22, 43)] {
            finished
                .handle_live_event(event(
                    message_id,
                    CanonicalLiveEvent::Chat(ChatEvent {
                        user: EventUser {
                            id: user_id,
                            ..Default::default()
                        },
                        comment: "!a".into(),
                        ..Default::default()
                    }),
                ))
                .await;
        }
        let draw_result = finished.competition_finish("draw").unwrap();
        assert_eq!(
            draw_result
                .selection
                .eligible
                .iter()
                .map(|entry| (entry.key.as_str(), entry.weight))
                .collect::<Vec<_>>(),
            vec![("42", 1), ("43", 1)]
        );
        assert!(matches!(
            draw_result.selection.winner.as_deref(),
            Some("42" | "43")
        ));
        assert_eq!(finished.competition_finish("draw").unwrap(), draw_result);
        assert!(finished.competition_action("draw", 1).unwrap().is_none());
        let mut weighted_ranking = finished.ranking_get("rank").unwrap();
        weighted_ranking.id = "weighted-rank".into();
        finished.ranking_create(weighted_ranking).unwrap();
        let mut weighted = finished.competition_get("poll").unwrap();
        weighted.id = "weighted".into();
        weighted.ranking_id = "weighted-rank".into();
        weighted.kind = CompetitionKind::WeightedDraw;
        weighted.selection_mode = SelectionMode::WeightedRandom;
        weighted.minimum_participants = 2;
        finished.competition_create(weighted).unwrap();
        finished
            .competition_transition("weighted", Transition::Start)
            .unwrap();
        finished
            .handle_live_event(event(
                31,
                CanonicalLiveEvent::Chat(ChatEvent {
                    user: EventUser {
                        id: 42,
                        ..Default::default()
                    },
                    comment: "!a".into(),
                    ..Default::default()
                }),
            ))
            .await;
        finished
            .handle_live_event(event(
                32,
                CanonicalLiveEvent::Like(LikeEvent {
                    user: EventUser {
                        id: 42,
                        ..Default::default()
                    },
                    count: 5,
                    total: 999,
                }),
            ))
            .await;
        finished
            .handle_live_event(event(
                33,
                CanonicalLiveEvent::Gift(GiftEvent {
                    user: EventUser {
                        id: 43,
                        ..Default::default()
                    },
                    gift_id: 77,
                    group_id: 222,
                    repeat_count: 1,
                    diamond_count: 3,
                    ..Default::default()
                }),
            ))
            .await;
        let weighted_result = finished.competition_finish("weighted").unwrap();
        assert_eq!(
            weighted_result
                .selection
                .eligible
                .iter()
                .map(|candidate| (candidate.key.as_str(), candidate.weight))
                .collect::<Vec<_>>(),
            vec![("42", 11), ("43", 3)]
        );
        assert_eq!(
            finished.competition_finish("weighted").unwrap(),
            weighted_result
        );
        assert!(finished
            .competition_action("weighted", 1)
            .unwrap()
            .is_none());
        finished.shutdown().await;
        drop(finished);
        let _ = std::fs::remove_dir_all(root);
    }

    #[cfg(feature = "persistence")]
    #[tokio::test]
    async fn matching_live_commands_are_persisted_once_after_native_dedup() {
        use tiktools_aggregation::model::{Definition as Ranking, SourceRule};
        use tiktools_competitions::{
            CommandParticipation, CompetitionKind, Definition, OptionDefinition,
            ParticipationOutcome, SelectionMode, TiePolicy, Transition,
        };
        use tiktools_tiktok::events::ChatEvent;
        let root = std::env::temp_dir().join(format!(
            "tiktools-command-ingress-{}-{}",
            std::process::id(),
            crate::now_millis()
        ));
        let paths = crate::paths::AppPaths {
            data: root.join("data"),
            plugins: root.join("plugins"),
            plugin_data: root.join("plugin-data"),
            builtin_plugins: root.join("builtin"),
            development_plugins: None,
            logs: root.join("logs"),
            temp: root.join("temp"),
            root: root.clone(),
        };
        let core = Arc::new(AppCore::with_paths(
            Arc::new(Emitter),
            Arc::new(crate::NoopMediaHost),
            paths.clone(),
        ));
        *core.connection_context.write().unwrap() = Some(LiveContext {
            unique_id: "creator".into(),
            room_id: "room".into(),
            connection_id: "session".into(),
        });
        core.ranking_create(Ranking {
            schema_version: 1,
            id: "rank".into(),
            creator_id: "creator".into(),
            room_id: "room".into(),
            group_by: tiktools_aggregation::model::GroupBy::Option,
            window: Default::default(),
            user_limits: None,
            sources: vec![SourceRule {
                source: "competition.vote".into(),
                metric: "event.count".into(),
                weight: 1,
                enabled: true,
                gift_multipliers: Default::default(),
            }],
            limit: 1,
            capacity: 100,
        })
        .unwrap();
        let mut domain_events = core.events.subscribe_domain();
        core.automation_create(crate::control::AutomationKind::Action, json!({"id":"workflow-log", "name":"Workflow", "enabled":true, "typeId":"core.log", "config":{"message":"{{data.winnerId}}"}})).unwrap();
        core.automation_create(crate::control::AutomationKind::Event, json!({"id":"winner-workflow", "name":"Winner event", "enabled":true, "trigger":"competition.winner.selected", "actionIds":["workflow-log"]})).unwrap();
        core.automation_create(crate::control::AutomationKind::Action, json!({"id":"winner-log", "name":"Winner", "enabled":true, "typeId":"core.log", "config":{"message":"winner {{data.winnerId}}"}})).unwrap();
        core.competition_create(Definition {
            schema_version: 1,
            id: "poll".into(),
            name: "Poll".into(),
            kind: CompetitionKind::Roulette,
            ranking_id: "rank".into(),
            options: vec![
                OptionDefinition {
                    id: "a".into(),
                    label: "A".into(),
                    commands: vec!["!a".into()],
                    gift_ids: vec![],
                    image_url: None,
                    on_win: Some(tiktools_competitions::WinnerAction {
                        action_id: "winner-log".into(),
                        inputs: json!({}),
                    }),
                },
                OptionDefinition {
                    id: "b".into(),
                    label: "B".into(),
                    commands: vec!["!b".into()],
                    gift_ids: vec![],
                    image_url: None,
                    on_win: None,
                },
            ],
            selection_mode: SelectionMode::HighestScore,
            tie_policy: TiePolicy::Manual,
            duration_ms: None,
            vote_limit: 1,
            allow_switch: true,
            minimum_participants: 1,
            target_score: None,
            on_user_win: None,
        })
        .unwrap();
        core.competition_transition("poll", Transition::Start)
            .unwrap();
        let event = NativeLiveEvent {
            base: CanonicalLiveEvent::Chat(ChatEvent {
                user: EventUser {
                    id: 42,
                    ..Default::default()
                },
                comment: "!a".into(),
                ..Default::default()
            }),
            metadata: EventMetadata {
                method: "WebcastChatMessage".into(),
                msg_id: 12345,
                is_history: false,
            },
            gift: None,
        };
        core.handle_live_event(event.clone()).await;
        core.handle_live_event(event.clone()).await;
        assert_eq!(core.competitions.diagnostics()["accepted"], 1);
        assert_eq!(core.ranking_snapshot("rank").unwrap().entries[0].key, "a");
        assert_eq!(core.ranking_snapshot("rank").unwrap().entries[0].label, "A");
        let mut switched = event;
        switched.metadata.msg_id = 12346;
        if let CanonicalLiveEvent::Chat(chat) = &mut switched.base {
            chat.comment = "!b".into();
        }
        core.handle_live_event(switched).await;
        let snapshot = core.ranking_snapshot("rank").unwrap();
        assert_eq!(
            (snapshot.entries[0].key.as_str(), snapshot.total_score),
            ("b", 1)
        );
        let second = NativeLiveEvent {
            base: CanonicalLiveEvent::Chat(ChatEvent {
                user: EventUser {
                    id: 43,
                    ..Default::default()
                },
                comment: "!a".into(),
                ..Default::default()
            }),
            metadata: EventMetadata {
                method: "WebcastChatMessage".into(),
                msg_id: 12347,
                is_history: false,
            },
            gift: None,
        };
        core.handle_live_event(second).await;
        assert_eq!(core.ranking_snapshot("rank").unwrap().entries.len(), 1);
        let stored = core
            .db
            .route_competition_command(
                "poll",
                &CommandParticipation {
                    creator_id: "creator",
                    room_id: "room",
                    participant_id: "42",
                    event_identity: "tiktok.chat:12345",
                    command: "!a",
                    received_at: crate::now_millis(),
                },
            )
            .unwrap();
        assert_eq!(
            stored,
            Some(ParticipationOutcome::Duplicate {
                option_id: "a".into()
            })
        );
        assert!(core.competition_finish("poll").is_err());
        let (_, candidates) = core.competition_manual_candidates("poll").unwrap();
        assert_eq!(
            candidates.len(),
            2,
            "display limit must not hide a tied candidate"
        );
        assert!(core
            .competition_select_winner("poll", "not-a-leader", 1)
            .is_err());
        assert!(core.competition_select_winner("poll", "a", 0).is_err());
        let result = core.competition_select_winner("poll", "a", 1).unwrap();
        assert_eq!(
            core.competition_select_winner("poll", "b", 1).unwrap(),
            result
        );
        assert_eq!(result.selection.winner.as_deref(), Some("a"));
        assert_eq!(result.selection.eligible.len(), 2);
        assert_eq!(core.competition_finish("poll").unwrap(), result);
        let mut started = 0;
        let mut selected = 0;
        let mut finished = 0;
        while let Ok(event) = domain_events.try_recv() {
            match event {
                crate::events::DomainEvent::CompetitionStarted(data)
                    if data.competition_id == "poll" =>
                {
                    started += 1
                }
                crate::events::DomainEvent::CompetitionFinished(data)
                    if data.competition_id == "poll" =>
                {
                    assert_eq!(data.winner_id.as_deref(), Some("a"));
                    finished += 1;
                }
                crate::events::DomainEvent::CompetitionWinnerSelected(data)
                    if data.competition_id == "poll" =>
                {
                    assert_eq!(data.winner_score, Some(1));
                    selected += 1;
                }
                _ => {}
            }
        }
        assert_eq!((started, finished, selected), (1, 1, 1));
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while !core
                .automation
                .recent_runs()
                .iter()
                .any(|run| run["actionId"] == "workflow-log")
            {
                tokio::time::sleep(std::time::Duration::from_millis(25)).await;
            }
        })
        .await
        .expect("winner event reaches automation");
        core.start_competition_timer();
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                if core
                    .competition_action("poll", 1)
                    .unwrap()
                    .is_some_and(|action| {
                        action.status == tiktools_competitions::ActionStatus::Completed
                    })
                {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(25)).await;
            }
        })
        .await
        .expect("winner action completes");
        assert_eq!(
            core.automation
                .recent_runs()
                .iter()
                .filter(|run| run["actionId"] == "winner-log")
                .count(),
            1
        );
        assert_eq!(
            core.competition_state("poll").unwrap().lifecycle,
            tiktools_competitions::Lifecycle::Finished
        );
        core.ranking_create(Ranking {
            schema_version: 1,
            id: "timed-rank".into(),
            creator_id: "creator".into(),
            room_id: "room".into(),
            group_by: tiktools_aggregation::model::GroupBy::Option,
            window: Default::default(),
            user_limits: None,
            sources: vec![SourceRule {
                source: "competition.vote".into(),
                metric: "event.count".into(),
                weight: 1,
                enabled: true,
                gift_multipliers: Default::default(),
            }],
            limit: 10,
            capacity: 10,
        })
        .unwrap();
        core.automation_create(crate::control::AutomationKind::Action, json!({"id":"invalid-winner", "name":"Invalid", "enabled":true, "typeId":"core.points", "config":{}})).unwrap();
        core.competition_create(Definition {
            schema_version: 1,
            id: "timed-poll".into(),
            name: "Timed".into(),
            kind: CompetitionKind::Poll,
            ranking_id: "timed-rank".into(),
            options: vec![OptionDefinition {
                id: "yes".into(),
                label: "Yes".into(),
                commands: vec!["!yes".into()],
                gift_ids: vec![],
                image_url: None,
                on_win: Some(tiktools_competitions::WinnerAction {
                    action_id: "invalid-winner".into(),
                    inputs: json!({}),
                }),
            }],
            selection_mode: SelectionMode::HighestScore,
            tie_policy: TiePolicy::Stable,
            duration_ms: Some(500),
            vote_limit: 1,
            allow_switch: false,
            minimum_participants: 1,
            target_score: None,
            on_user_win: None,
        })
        .unwrap();
        core.competition_transition("timed-poll", Transition::Start)
            .unwrap();
        core.handle_live_event(NativeLiveEvent {
            base: CanonicalLiveEvent::Chat(ChatEvent {
                user: EventUser {
                    id: 44,
                    ..Default::default()
                },
                comment: "!yes".into(),
                ..Default::default()
            }),
            metadata: EventMetadata {
                method: "WebcastChatMessage".into(),
                msg_id: 12348,
                is_history: false,
            },
            gift: None,
        })
        .await;
        // Corrupt a different finished result and force a refresh in the same
        // tick as finalization. Presentation failure must not block the poll.
        let connection = rusqlite::Connection::open(core.db.automation_path()).unwrap();
        let original: String = connection.query_row("SELECT result_json FROM competition_results WHERE competition_id='poll' AND generation=1", [], |row| row.get(0)).unwrap();
        connection.execute("UPDATE competition_results SET result_json='invalid' WHERE competition_id='poll' AND generation=1", []).unwrap();
        assert!(core.db.latest_competition_results().is_err());
        let mut reveals = core.events.subscribe_domain();
        tokio::time::sleep(std::time::Duration::from_millis(550)).await;
        core.competition_timer_tick(true).unwrap();
        assert_eq!(
            core.competition_state("timed-poll").unwrap().lifecycle,
            tiktools_competitions::Lifecycle::Finished
        );
        let mut revealed = false;
        while let Ok(event) = reveals.try_recv() {
            if let crate::events::DomainEvent::CompetitionResult { result } = event {
                assert_ne!(result.competition_id, "poll");
                revealed |= result.competition_id == "timed-poll";
            }
        }
        assert!(revealed, "a corrupt result must not block other reveals");
        connection.execute("UPDATE competition_results SET result_json=?1 WHERE competition_id='poll' AND generation=1", [&original]).unwrap();
        drop(connection);
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                if core.competition_state("timed-poll").unwrap().lifecycle
                    == tiktools_competitions::Lifecycle::Finished
                {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(25)).await;
            }
        })
        .await
        .expect("deadline completes without a UI");
        assert_eq!(
            core.competition_results("timed-poll", None, 10).unwrap().0[0]
                .selection
                .winner
                .as_deref(),
            Some("yes")
        );
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                if core
                    .competition_action("timed-poll", 1)
                    .unwrap()
                    .is_some_and(|action| {
                        action.status == tiktools_competitions::ActionStatus::Uncertain
                    })
                {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(25)).await;
            }
        })
        .await
        .expect("failed execution is uncertain");
        let action_events = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            let mut seen = Vec::new();
            while seen.len() < 2 {
                match domain_events.recv().await.unwrap() {
                    crate::events::DomainEvent::CompetitionActionCompleted(data) => {
                        seen.push((data.competition_id, data.action_status))
                    }
                    crate::events::DomainEvent::CompetitionActionFailed(data) => {
                        seen.push((data.competition_id, data.action_status))
                    }
                    _ => {}
                }
            }
            seen
        })
        .await
        .expect("action status events arrive");
        assert!(action_events.contains(&(
            "poll".to_owned(),
            Some(tiktools_competitions::ActionStatus::Completed)
        )));
        assert!(action_events.contains(&(
            "timed-poll".to_owned(),
            Some(tiktools_competitions::ActionStatus::Uncertain)
        )));
        *core.competitions.action_inflight.lock().unwrap() = Some(("timed-poll".into(), 1));
        let error = core
            .competition_mark_action_uncertain("timed-poll", 1)
            .unwrap_err();
        assert_eq!(error.message, "competition action is still executing");
        *core.competitions.action_inflight.lock().unwrap() = None;
        assert_eq!(
            core.competition_action("timed-poll", 1)
                .unwrap()
                .unwrap()
                .reason
                .as_deref(),
            Some("execution_failed")
        );
        core.competition_resolve_action(
            "timed-poll",
            1,
            tiktools_competitions::ActionResolution::Failed,
        )
        .unwrap();
        core.ranking_create(Ranking {
            schema_version: 1,
            id: "clean-rank".into(),
            creator_id: "creator".into(),
            room_id: "room".into(),
            group_by: tiktools_aggregation::model::GroupBy::Option,
            window: Default::default(),
            user_limits: None,
            sources: vec![SourceRule {
                source: "competition.vote".into(),
                metric: "event.count".into(),
                weight: 1,
                enabled: true,
                gift_multipliers: Default::default(),
            }],
            limit: 10,
            capacity: 10,
        })
        .unwrap();
        core.competition_create(Definition {
            schema_version: 1,
            id: "clean-poll".into(),
            name: "Clean".into(),
            kind: CompetitionKind::Poll,
            ranking_id: "clean-rank".into(),
            options: vec![OptionDefinition {
                id: "x".into(),
                label: "X".into(),
                commands: vec!["!x".into()],
                gift_ids: vec![],
                image_url: None,
                on_win: None,
            }],
            selection_mode: SelectionMode::HighestScore,
            tie_policy: TiePolicy::Stable,
            duration_ms: None,
            vote_limit: 1,
            allow_switch: false,
            minimum_participants: 1,
            target_score: None,
            on_user_win: None,
        })
        .unwrap();
        core.competition_transition("clean-poll", Transition::Start)
            .unwrap();
        core.shutdown().await;
        drop(core);
        let restarted = Arc::new(AppCore::with_paths(
            Arc::new(Emitter),
            Arc::new(crate::NoopMediaHost),
            paths.clone(),
        ));
        assert_eq!(
            restarted
                .ranking_snapshot("clean-rank")
                .unwrap()
                .integrity_status,
            "unknown"
        );
        assert_eq!(restarted.competition_finish("poll").unwrap(), result);
        let failed = restarted
            .competition_action("timed-poll", 1)
            .unwrap()
            .unwrap();
        assert_eq!(failed.status, tiktools_competitions::ActionStatus::Failed);
        assert_eq!(failed.reason.as_deref(), Some("execution_failed"));
        restarted.start_competition_timer();
        tokio::time::sleep(std::time::Duration::from_millis(350)).await;
        assert!(restarted.automation.recent_runs().is_empty());
        assert_eq!(
            restarted
                .competition_action("poll", 1)
                .unwrap()
                .unwrap()
                .status,
            tiktools_competitions::ActionStatus::Completed
        );
        let restored = restarted.ranking_snapshot("rank").unwrap();
        assert_eq!(restored.entries[0].label, "A");
        assert_eq!(
            (restored.entries[0].key.as_str(), restored.total_score),
            ("a", 2)
        );
        restarted
            .live_ingress
            .admission_cancelled
            .fetch_add(1, Ordering::Relaxed);
        restarted.shutdown().await;
        drop(restarted);
        let after_loss = Arc::new(AppCore::with_paths(
            Arc::new(Emitter),
            Arc::new(crate::NoopMediaHost),
            paths,
        ));
        assert_eq!(
            after_loss
                .ranking_snapshot("clean-rank")
                .unwrap()
                .integrity_status,
            "degraded"
        );
        after_loss.shutdown().await;
        drop(after_loss);
        let _ = std::fs::remove_dir_all(root);
    }
    #[cfg(feature = "persistence")]
    #[tokio::test]
    async fn runtime_ranking_checkpoint_survives_crash_and_reset() {
        use tiktools_aggregation::model::{Definition, SourceRule};
        use tiktools_tiktok::events::LikeEvent;
        let root = std::env::temp_dir().join(format!(
            "tiktools-runtime-checkpoint-{}-{}",
            std::process::id(),
            crate::now_millis()
        ));
        let paths = crate::paths::AppPaths {
            data: root.join("data"),
            plugins: root.join("plugins"),
            plugin_data: root.join("plugin-data"),
            builtin_plugins: root.join("builtin"),
            development_plugins: None,
            logs: root.join("logs"),
            temp: root.join("temp"),
            root: root.clone(),
        };
        let open = || {
            Arc::new(AppCore::with_paths(
                Arc::new(Emitter),
                Arc::new(crate::NoopMediaHost),
                paths.clone(),
            ))
        };
        let core = open();
        *core.connection_context.write().unwrap() = Some(LiveContext {
            unique_id: "creator".into(),
            room_id: "room".into(),
            connection_id: "session".into(),
        });
        core.ranking_create(Definition {
            schema_version: 1,
            id: "likes".into(),
            creator_id: "creator".into(),
            room_id: "room".into(),
            group_by: Default::default(),
            window: Default::default(),
            user_limits: None,
            capacity: 20,
            limit: 5,
            sources: vec![SourceRule {
                source: "tiktok.like".into(),
                metric: "like.count".into(),
                weight: 1,
                enabled: true,
                gift_multipliers: Default::default(),
            }],
        })
        .unwrap();
        let like = |msg_id, count| NativeLiveEvent {
            base: CanonicalLiveEvent::Like(LikeEvent {
                user: EventUser {
                    id: 42,
                    nickname: "Alice".into(),
                    ..Default::default()
                },
                count,
                total: 9000,
            }),
            metadata: EventMetadata {
                method: "WebcastLikeMessage".into(),
                msg_id,
                is_history: false,
            },
            gift: None,
        };
        core.handle_live_event(like(1, 7)).await;
        use crate::services::aggregations::AggregationService;
        assert_eq!(AggregationService::checkpoint_changed(&core), 1);
        // Unchanged rankings are not rewritten.
        assert_eq!(AggregationService::checkpoint_changed(&core), 0);
        // Accepted after the last runtime write: lost by the crash below.
        core.handle_live_event(like(2, 5)).await;
        assert_eq!(core.ranking_snapshot("likes").unwrap().total_score, 12);
        drop(core); // No shutdown: simulates a crash.

        let recovered = open();
        let snapshot = recovered.ranking_snapshot("likes").unwrap();
        assert_eq!(snapshot.total_score, 7);
        assert_eq!(snapshot.integrity_status, "degraded");
        assert_eq!(snapshot.entries[0].label, "Alice");
        recovered.ranking_reset("likes").unwrap();
        drop(recovered); // Crash again before any runtime write.

        let after_reset = open();
        assert_eq!(
            after_reset.ranking_snapshot("likes").unwrap().total_score,
            0
        );
        after_reset.shutdown().await;
        drop(after_reset);
        let _ = std::fs::remove_dir_all(root);
    }
    #[cfg(feature = "persistence")]
    #[tokio::test]
    async fn session_ranking_recovers_after_drained_shutdown() {
        use tiktools_aggregation::model::{Definition, SourceRule};
        use tiktools_tiktok::events::LikeEvent;
        let root = std::env::temp_dir().join(format!(
            "tiktools-session-checkpoint-{}-{}",
            std::process::id(),
            crate::now_millis()
        ));
        let paths = crate::paths::AppPaths {
            data: root.join("data"),
            plugins: root.join("plugins"),
            plugin_data: root.join("plugin-data"),
            builtin_plugins: root.join("builtin"),
            development_plugins: None,
            logs: root.join("logs"),
            temp: root.join("temp"),
            root: root.clone(),
        };
        let core = Arc::new(AppCore::with_paths(
            Arc::new(Emitter),
            Arc::new(crate::NoopMediaHost),
            paths.clone(),
        ));
        *core.connection_context.write().unwrap() = Some(LiveContext {
            unique_id: "creator".into(),
            room_id: "room".into(),
            connection_id: "session".into(),
        });
        core.ranking_create(Definition {
            schema_version: 1,
            id: "likes".into(),
            creator_id: "creator".into(),
            room_id: "room".into(),
            group_by: Default::default(),
            window: Default::default(),
            user_limits: None,
            capacity: 20,
            limit: 1,
            sources: vec![SourceRule {
                source: "tiktok.like".into(),
                metric: "like.count".into(),
                weight: 1,
                enabled: true,
                gift_multipliers: Default::default(),
            }],
        })
        .unwrap();
        core.handle_live_event(NativeLiveEvent {
            base: CanonicalLiveEvent::Like(LikeEvent {
                user: EventUser {
                    id: 42,
                    nickname: "Alice".into(),
                    ..Default::default()
                },
                count: 7,
                total: 9000,
            }),
            metadata: EventMetadata {
                method: "WebcastLikeMessage".into(),
                msg_id: 123,
                is_history: false,
            },
            gift: None,
        })
        .await;
        assert_eq!(core.ranking_snapshot("likes").unwrap().total_score, 7);
        core.shutdown().await;
        drop(core);
        let restored = Arc::new(AppCore::with_paths(
            Arc::new(Emitter),
            Arc::new(crate::NoopMediaHost),
            paths.clone(),
        ));
        let snapshot = restored.ranking_snapshot("likes").unwrap();
        assert_eq!(snapshot.total_score, 7);
        assert_eq!(snapshot.entries[0].label, "Alice");
        drop(restored);
        let after_unclean = Arc::new(AppCore::with_paths(
            Arc::new(Emitter),
            Arc::new(crate::NoopMediaHost),
            paths,
        ));
        // The clean checkpoint survives the unclean exit, marked degraded.
        let snapshot = after_unclean.ranking_snapshot("likes").unwrap();
        assert_eq!(snapshot.integrity_status, "degraded");
        assert_eq!(snapshot.total_score, 7);
        let health = after_unclean.system_health();
        assert_eq!(health["aggregations"]["degraded"], 1);
        assert_eq!(health["status"], "degraded");
        after_unclean.shutdown().await;
        drop(after_unclean);
        let _ = std::fs::remove_dir_all(root);
    }
    #[cfg(not(feature = "persistence"))]
    #[tokio::test]
    async fn ranking_uses_deduplicated_like_packet_deltas() {
        use tiktools_aggregation::model::{Definition, SourceRule};
        use tiktools_tiktok::events::LikeEvent;
        let core = Arc::new(AppCore::new(Arc::new(Emitter)));
        *core.connection_context.write().unwrap() = Some(LiveContext {
            unique_id: "creator".into(),
            room_id: "room".into(),
            connection_id: "session".into(),
        });
        core.ranking_create(Definition {
            schema_version: 1,
            id: "likes".into(),
            creator_id: "creator".into(),
            room_id: "room".into(),
            group_by: Default::default(),
            window: Default::default(),
            user_limits: None,
            capacity: 20,
            limit: 10,
            sources: vec![SourceRule {
                source: "tiktok.like".into(),
                metric: "like.count".into(),
                weight: 1,
                enabled: true,
                gift_multipliers: Default::default(),
            }],
        })
        .unwrap();
        let event = NativeLiveEvent {
            base: CanonicalLiveEvent::Like(LikeEvent {
                user: EventUser {
                    id: 42,
                    unique_id: "alice".into(),
                    nickname: "Alice".into(),
                    avatar_url: Some("https://cdn.example/avatar.png".into()),
                    ..Default::default()
                },
                count: 7,
                total: 9000,
            }),
            metadata: EventMetadata {
                method: "WebcastLikeMessage".into(),
                msg_id: 112233,
                is_history: false,
            },
            gift: None,
        };
        core.handle_live_event(event.clone()).await;
        core.handle_live_event(event).await;
        let snapshot = core.ranking_snapshot("likes").unwrap();
        assert_eq!(snapshot.total_score, 7);
        assert_eq!(snapshot.entries[0].key, "42");
        assert_eq!(snapshot.entries[0].label, "Alice");
        let wire = serde_json::to_value(&snapshot.entries[0]).unwrap();
        assert_eq!(wire["label"], "Alice");
        assert_eq!(wire["uniqueId"], "alice");
        assert_eq!(wire["userId"], "42");
        assert_eq!(
            snapshot.entries[0].metadata.unique_id.as_deref(),
            Some("alice")
        );
        assert_eq!(
            snapshot.entries[0].metadata.avatar_url.as_deref(),
            Some("https://cdn.example/avatar.png")
        );
        core.shutdown().await;
    }
    #[cfg(not(feature = "persistence"))]
    #[tokio::test]
    async fn wider_likes_snapshot_publishes_complete_bounded_projection() {
        use tiktools_tiktok::events::LikeEvent;
        let core = Arc::new(AppCore::new(Arc::new(Emitter)));
        *core.connection_context.write().unwrap() = Some(LiveContext {
            unique_id: "creator".into(),
            room_id: "room".into(),
            connection_id: "session".into(),
        });
        for id in 1..=20 {
            let event = NativeLiveEvent {
                base: CanonicalLiveEvent::Like(LikeEvent {
                    user: EventUser {
                        id,
                        unique_id: format!("u{id}"),
                        ..Default::default()
                    },
                    count: id,
                    total: 1000,
                }),
                metadata: EventMetadata {
                    method: "like".into(),
                    msg_id: id,
                    is_history: false,
                },
                gift: None,
            };
            core.handle_live_event(event).await;
        }
        let mut receiver = core.events.subscribe_domain();
        core.start_competition_timer();
        let leaders = tokio::time::timeout(std::time::Duration::from_secs(3), async {
            loop {
                if let crate::events::DomainEvent::LiveLikesRankingFull {
                    room_id, leaders, ..
                } = receiver.recv().await.unwrap()
                {
                    if room_id == "room" {
                        break leaders;
                    }
                }
            }
        })
        .await
        .expect("wider likes snapshot");
        assert_eq!(leaders.len(), 20);
        assert_eq!(leaders[0]["score"], 20);
        assert_eq!(leaders[19]["score"], 1);
        core.shutdown().await;
    }

    #[cfg(not(feature = "persistence"))]
    #[tokio::test]
    async fn likes_snapshot_refresh_restores_room_without_replaying_events() {
        use tiktools_tiktok::events::LikeEvent;
        let core = Arc::new(AppCore::new(Arc::new(Emitter)));
        *core.connection_context.write().unwrap() = Some(LiveContext {
            unique_id: "creator".into(),
            room_id: "room".into(),
            connection_id: "session".into(),
        });
        let event = NativeLiveEvent {
            base: CanonicalLiveEvent::Like(LikeEvent {
                user: EventUser {
                    id: 1,
                    unique_id: "ana".into(),
                    nickname: "Ana".into(),
                    ..Default::default()
                },
                count: 15,
                total: 1000,
            }),
            metadata: EventMetadata {
                method: "like".into(),
                msg_id: 501,
                is_history: false,
            },
            gift: None,
        };
        core.handle_live_event(event).await;
        let mut receiver = core.events.subscribe_domain();
        core.start_competition_timer();
        let leaders = tokio::time::timeout(std::time::Duration::from_secs(7), async {
            loop {
                if let crate::events::DomainEvent::LiveLikesRanking {
                    room_id, leaders, ..
                } = receiver.recv().await.unwrap()
                {
                    if room_id == "room" {
                        break leaders;
                    }
                }
            }
        })
        .await
        .expect("periodic likes snapshot");
        assert_eq!(leaders[0]["score"], 15);
        assert_eq!(leaders[0]["uniqueId"], "ana");
        core.shutdown().await;
    }
    #[cfg(not(feature = "persistence"))]
    #[tokio::test]
    async fn ranking_updates_publish_snapshot_and_reconnect_can_read_it() {
        use tiktools_aggregation::model::{Definition, SourceRule};
        use tiktools_tiktok::events::LikeEvent;
        let core = Arc::new(AppCore::new(Arc::new(Emitter)));
        *core.connection_context.write().unwrap() = Some(LiveContext {
            unique_id: "creator".into(),
            room_id: "room".into(),
            connection_id: "session".into(),
        });
        core.ranking_create(Definition {
            schema_version: 1,
            id: "likes".into(),
            creator_id: "creator".into(),
            room_id: "room".into(),
            group_by: Default::default(),
            window: Default::default(),
            user_limits: None,
            capacity: 10,
            limit: 10,
            sources: vec![SourceRule {
                source: "tiktok.like".into(),
                metric: "like.count".into(),
                weight: 1,
                enabled: true,
                gift_multipliers: Default::default(),
            }],
        })
        .unwrap();
        let mut receiver = core.events.subscribe_domain();
        core.start_competition_timer();
        for id in 1..=10 {
            let event = NativeLiveEvent {
                base: CanonicalLiveEvent::Like(LikeEvent {
                    user: EventUser {
                        id: 1,
                        ..Default::default()
                    },
                    count: 1,
                    total: 1000,
                }),
                metadata: EventMetadata {
                    method: "like".into(),
                    msg_id: id,
                    is_history: false,
                },
                gift: None,
            };
            core.handle_live_event(event).await;
        }
        let snapshot = tokio::time::timeout(std::time::Duration::from_secs(3), async {
            loop {
                if let crate::events::DomainEvent::RankingUpdated { snapshot } =
                    receiver.recv().await.unwrap()
                {
                    if snapshot.ranking_id == "likes" && snapshot.total_score == 10 {
                        break snapshot;
                    }
                }
            }
        })
        .await
        .expect("coalesced ranking snapshot");
        assert_eq!(snapshot, core.ranking_snapshot("likes").unwrap());
        assert!(crate::events::DomainEvent::RankingUpdated {
            snapshot: snapshot.clone()
        }
        .is_lossy());
        // A new read-only gateway client can recover even when scores stay idle.
        let mut reconnect = core.events.subscribe_domain();
        let refresh = tokio::time::timeout(std::time::Duration::from_secs(7), async {
            loop {
                if let crate::events::DomainEvent::RankingUpdated { snapshot } =
                    reconnect.recv().await.unwrap()
                {
                    if snapshot.ranking_id == "likes" {
                        break snapshot;
                    }
                }
            }
        })
        .await
        .expect("periodic refresh");
        assert_eq!(refresh, snapshot);
        core.shutdown().await;
    }
    #[cfg(not(feature = "persistence"))]
    #[tokio::test]
    async fn ranking_rejects_unidentified_events_and_marks_missing_price() {
        use tiktools_aggregation::model::{Definition, SourceRule};
        use tiktools_tiktok::events::LikeEvent;
        let core = Arc::new(AppCore::new(Arc::new(Emitter)));
        *core.connection_context.write().unwrap() = Some(LiveContext {
            unique_id: "creator".into(),
            room_id: "room".into(),
            connection_id: "session".into(),
        });
        for (id, source, metric) in [
            ("likes", "tiktok.like", "like.count"),
            ("diamonds", "tiktok.gift", "gift.diamonds"),
            ("gifts", "tiktok.gift", "gift.count"),
            ("shares", "tiktok.share", "share.count"),
        ] {
            core.ranking_create(Definition {
                schema_version: 1,
                id: id.into(),
                creator_id: "creator".into(),
                room_id: "room".into(),
                group_by: Default::default(),
                window: Default::default(),
                user_limits: None,
                capacity: 10,
                limit: 10,
                sources: vec![SourceRule {
                    source: source.into(),
                    metric: metric.into(),
                    weight: 1,
                    enabled: true,
                    gift_multipliers: Default::default(),
                }],
            })
            .unwrap();
        }
        let like = NativeLiveEvent {
            base: CanonicalLiveEvent::Like(LikeEvent {
                user: EventUser {
                    id: 1,
                    ..Default::default()
                },
                count: 5,
                total: 500,
            }),
            metadata: EventMetadata {
                method: "like".into(),
                msg_id: 0,
                is_history: false,
            },
            gift: None,
        };
        core.handle_live_event(like).await;
        assert_eq!(core.ranking_snapshot("likes").unwrap().total_score, 0);
        assert_eq!(
            core.ranking_snapshot("likes").unwrap().integrity_status,
            "degraded"
        );
        let history = NativeLiveEvent {
            base: CanonicalLiveEvent::Like(LikeEvent {
                user: EventUser {
                    id: 1,
                    ..Default::default()
                },
                count: 8,
                total: 508,
            }),
            metadata: EventMetadata {
                method: "like".into(),
                msg_id: 777,
                is_history: true,
            },
            gift: None,
        };
        core.handle_live_event(history).await;
        assert_eq!(core.ranking_snapshot("likes").unwrap().total_score, 0);
        let gift = NativeLiveEvent {
            base: CanonicalLiveEvent::Gift(GiftEvent {
                user: EventUser {
                    id: 1,
                    ..Default::default()
                },
                gift_id: 5,
                repeat_count: 1,
                diamond_count: 0,
                ..Default::default()
            }),
            metadata: EventMetadata {
                method: "gift".into(),
                msg_id: 999,
                is_history: false,
            },
            gift: None,
        };
        core.handle_live_event(gift).await;
        assert_eq!(core.ranking_snapshot("gifts").unwrap().total_score, 1);
        let diamonds = core.ranking_snapshot("diamonds").unwrap();
        assert_eq!(diamonds.total_score, 0);
        assert_eq!(diamonds.integrity_status, "degraded");
        assert_eq!(
            core.ranking_snapshot("gifts").unwrap().integrity_status,
            "unknown"
        );
        let mut share = NativeLiveEvent {
            base: CanonicalLiveEvent::Social(tiktools_tiktok::events::SocialEvent {
                user: EventUser {
                    id: 1,
                    ..Default::default()
                },
                action: 3,
                share_count: 9000,
                ..Default::default()
            }),
            metadata: EventMetadata {
                method: "social".into(),
                msg_id: 1001,
                is_history: false,
            },
            gift: None,
        };
        core.handle_live_event(share.clone()).await;
        core.handle_live_event(share.clone()).await;
        if let CanonicalLiveEvent::Social(social) = &mut share.base {
            social.action = 0;
            social.share_count = 9001;
        }
        share.metadata.msg_id += 1;
        core.handle_live_event(share.clone()).await;
        if let CanonicalLiveEvent::Social(social) = &mut share.base {
            social.action = 3;
        }
        share.metadata.msg_id += 1;
        share.metadata.is_history = true;
        core.handle_live_event(share).await;
        let shares = core.ranking_snapshot("shares").unwrap();
        assert_eq!(
            shares.total_score, 1,
            "only one classified, non-replayed share counts"
        );
        assert_eq!(
            shares.integrity_status, "degraded",
            "unseen historical packets must report a gap"
        );
        core.shutdown().await;
    }
    #[cfg(not(feature = "persistence"))]
    #[tokio::test]
    async fn ranking_gifts_use_combo_deltas_and_atomic_metrics() {
        use tiktools_aggregation::model::{Definition, GroupBy, SourceRule};
        let core = Arc::new(AppCore::new(Arc::new(Emitter)));
        *core.connection_context.write().unwrap() = Some(LiveContext {
            unique_id: "creator".into(),
            room_id: "room".into(),
            connection_id: "session".into(),
        });
        core.ranking_create(Definition {
            schema_version: 1,
            id: "gifts".into(),
            creator_id: "creator".into(),
            room_id: "room".into(),
            group_by: GroupBy::Gift,
            window: Default::default(),
            user_limits: None,
            capacity: 10,
            limit: 10,
            sources: vec![
                SourceRule {
                    source: "tiktok.gift".into(),
                    metric: "gift.count".into(),
                    weight: 1,
                    enabled: true,
                    gift_multipliers: Default::default(),
                },
                SourceRule {
                    source: "tiktok.gift".into(),
                    metric: "gift.diamonds".into(),
                    weight: 2,
                    enabled: true,
                    gift_multipliers: Default::default(),
                },
            ],
        })
        .unwrap();
        core.ranking_create(Definition {
            schema_version: 1,
            id: "bonus".into(),
            creator_id: "creator".into(),
            room_id: "room".into(),
            group_by: GroupBy::User,
            window: Default::default(),
            user_limits: None,
            capacity: 10,
            limit: 10,
            sources: vec![SourceRule {
                source: "tiktok.gift".into(),
                metric: "gift.diamonds".into(),
                weight: 2,
                enabled: true,
                gift_multipliers: std::collections::BTreeMap::from([("9".into(), 3)]),
            }],
        })
        .unwrap();
        let mut event = NativeLiveEvent {
            base: CanonicalLiveEvent::Gift(GiftEvent {
                user: EventUser {
                    id: 71,
                    ..Default::default()
                },
                gift_id: 9,
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
        if let CanonicalLiveEvent::Gift(gift) = &mut event.base {
            gift.repeat_count = 3;
        }
        event.metadata.msg_id += 1;
        core.handle_live_event(event.clone()).await;
        if let CanonicalLiveEvent::Gift(gift) = &mut event.base {
            gift.repeat_end = true;
        }
        event.metadata.msg_id += 1;
        core.handle_live_event(event).await;
        let snapshot = core.ranking_snapshot("gifts").unwrap();
        assert_eq!(snapshot.total_score, 15);
        assert_eq!(snapshot.entries[0].key, "9");
        assert_eq!(snapshot.revision, 2);
        let bonus = core.ranking_snapshot("bonus").unwrap();
        assert_eq!(bonus.total_score, 36);
        assert_eq!(bonus.entries[0].key, "71");
        assert_eq!(bonus.revision, 2);
        let mut late_price = NativeLiveEvent {
            base: CanonicalLiveEvent::Gift(GiftEvent {
                user: EventUser {
                    id: 71,
                    ..Default::default()
                },
                gift_id: 9,
                group_id: 94,
                repeat_count: 1,
                diamond_count: 0,
                ..Default::default()
            }),
            metadata: EventMetadata {
                method: "WebcastGiftMessage".into(),
                msg_id: 8100,
                is_history: false,
            },
            gift: None,
        };
        core.handle_live_event(late_price.clone()).await;
        if let CanonicalLiveEvent::Gift(gift) = &mut late_price.base {
            gift.diamond_count = 2;
            gift.repeat_end = true;
        }
        late_price.metadata.msg_id += 1;
        core.handle_live_event(late_price).await;
        let bonus = core.ranking_snapshot("bonus").unwrap();
        assert_eq!(
            bonus.total_score, 36,
            "zero-delta final cannot add retroactive diamonds"
        );
        assert_eq!(bonus.integrity_status, "degraded");
        // A price arriving with a larger repeat count can include both new
        // gifts and retroactive value. Reject that ambiguous packet, then
        // accept the next provably incremental contribution.
        let mut price_with_progress = NativeLiveEvent {
            base: CanonicalLiveEvent::Gift(GiftEvent {
                user: EventUser {
                    id: 71,
                    ..Default::default()
                },
                gift_id: 9,
                group_id: 95,
                repeat_count: 1,
                diamond_count: 0,
                ..Default::default()
            }),
            metadata: EventMetadata {
                method: "WebcastGiftMessage".into(),
                msg_id: 8200,
                is_history: false,
            },
            gift: None,
        };
        core.handle_live_event(price_with_progress.clone()).await;
        if let CanonicalLiveEvent::Gift(gift) = &mut price_with_progress.base {
            gift.diamond_count = 2;
            gift.repeat_count = 2;
        }
        price_with_progress.metadata.msg_id += 1;
        core.handle_live_event(price_with_progress.clone()).await;
        assert_eq!(core.ranking_snapshot("bonus").unwrap().total_score, 36);
        if let CanonicalLiveEvent::Gift(gift) = &mut price_with_progress.base {
            gift.repeat_count = 3;
        }
        price_with_progress.metadata.msg_id += 1;
        core.handle_live_event(price_with_progress).await;
        assert_eq!(core.ranking_snapshot("bonus").unwrap().total_score, 48);
        let ambiguous = NativeLiveEvent {
            base: CanonicalLiveEvent::Gift(GiftEvent {
                user: EventUser {
                    id: 71,
                    ..Default::default()
                },
                gift_id: 9,
                group_id: 0,
                repeat_count: 3,
                diamond_count: 2,
                ..Default::default()
            }),
            metadata: EventMetadata {
                method: "WebcastGiftMessage".into(),
                msg_id: 8300,
                is_history: false,
            },
            gift: None,
        };
        let before = core.ranking_snapshot("gifts").unwrap().total_score;
        core.handle_live_event(ambiguous).await;
        let gifts = core.ranking_snapshot("gifts").unwrap();
        assert_eq!(gifts.total_score, before);
        assert_eq!(gifts.integrity_status, "degraded");
        assert_eq!(core.ranking_snapshot("bonus").unwrap().total_score, 48);
        core.shutdown().await;
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
