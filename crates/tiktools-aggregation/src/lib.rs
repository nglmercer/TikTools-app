//! Bounded, incremental aggregation primitives.
//!
//! Contributions must already be normalized and deduplicated at ingress.
//! Scores use integer units; multiplication and addition are checked and an
//! overflowing contribution is rejected atomically.
pub mod engine;
pub mod metrics;
pub mod model;
pub mod ranking;
mod scoring;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Optional display data supplied by a source adapter. Identity and scores
/// remain the ranking key and integer score, never these presentation fields.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EntryMetadata {
    #[serde(skip)]
    #[schemars(skip)]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unique_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nickname: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gift_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
}
impl EntryMetadata {
    fn bounded(&self) -> Self {
        fn value(value: &Option<String>, max: usize) -> Option<String> {
            value
                .as_ref()
                .filter(|value| !value.is_empty() && value.len() <= max)
                .cloned()
        }
        Self {
            label: value(&self.label, 512),
            user_id: value(&self.user_id, 512),
            unique_id: value(&self.unique_id, 512),
            nickname: value(&self.nickname, 512),
            avatar_url: value(&self.avatar_url, 2048),
            gift_id: value(&self.gift_id, 512),
            icon_url: value(&self.icon_url, 2048),
        }
    }
    fn merge(&mut self, update: Self) {
        if update.label.is_some() {
            self.label = update.label;
        }
        if update.user_id.is_some() {
            self.user_id = update.user_id;
        }
        if update.unique_id.is_some() {
            self.unique_id = update.unique_id;
        }
        if update.nickname.is_some() {
            self.nickname = update.nickname;
        }
        if update.avatar_url.is_some() {
            self.avatar_url = update.avatar_url;
        }
        if update.gift_id.is_some() {
            self.gift_id = update.gift_id;
        }
        if update.icon_url.is_some() {
            self.icon_url = update.icon_url;
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub key: String,
    pub rank: usize,
    pub score: u64,
    #[serde(default)]
    pub label: String,
    #[serde(flatten)]
    pub metadata: EntryMetadata,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub ranking_id: String,
    pub schema_version: u32,
    pub creator_id: String,
    pub room_id: String,
    pub window: model::Window,
    pub revision: u64,
    /// Opaque identity of this ranking instance, retained by clean checkpoints.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    /// Accounting completeness cannot be inferred from local state alone.
    pub integrity_status: String,
    pub rejected: u64,
    pub dropped: u64,
    #[serde(default)]
    pub limited: u64,
    #[serde(default)]
    pub cooldown_rejected: u64,
    pub paused: bool,
    pub entries: Vec<Entry>,
    pub total_score: u64,
    pub total_participants: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Capacity,
    Overflow,
    InvalidKey,
    InsufficientScore,
    IncompatibleDefinition,
    NotFound,
    WindowCapacity,
}

/// In-memory state for a single scope. The caller owns event identity,
/// persistence, window expiry, and scope changes.
pub struct Aggregation {
    scores: HashMap<String, u64>,
    metadata: HashMap<String, EntryMetadata>,
    index: ranking::RankingIndex,
    capacity: usize,
    total: u64,
    revision: u64,
}

impl Aggregation {
    pub fn new(capacity: usize) -> Self {
        Self {
            scores: HashMap::new(),
            metadata: HashMap::new(),
            index: ranking::RankingIndex::default(),
            capacity,
            total: 0,
            revision: 0,
        }
    }

    pub fn apply(&mut self, key: &str, delta: u64, weight: u64) -> Result<(), Error> {
        if key.is_empty() || key.len() > 512 {
            return Err(Error::InvalidKey);
        }
        let value = delta.checked_mul(weight).ok_or(Error::Overflow)?;
        if value == 0 {
            return Ok(());
        }
        if !self.scores.contains_key(key) && self.scores.len() >= self.capacity {
            return Err(Error::Capacity);
        }
        let score = self
            .scores
            .get(key)
            .copied()
            .unwrap_or(0)
            .checked_add(value)
            .ok_or(Error::Overflow)?;
        let total = self.total.checked_add(value).ok_or(Error::Overflow)?;
        let revision = self.revision.checked_add(1).ok_or(Error::Overflow)?;
        let previous = self.scores.insert(key.to_owned(), score);
        self.index.update(key, previous, Some(score));
        self.total = total;
        self.revision = revision;
        Ok(())
    }

    fn update_metadata(&mut self, key: &str, metadata: &EntryMetadata) {
        if self.scores.contains_key(key) {
            self.metadata
                .entry(key.to_owned())
                .or_default()
                .merge(metadata.bounded());
        }
    }

    /// Atomically move previously accounted score to another group and add
    /// an optional new score. A failed move leaves scores and revision intact.
    /// This is a primitive for durable assignment changes; callers must ensure
    /// that their event identity and window policies are applied separately.
    pub fn reassign(
        &mut self,
        from: &str,
        to: &str,
        amount: u64,
        additional: u64,
    ) -> Result<(), Error> {
        if from.is_empty() || to.is_empty() || from.len() > 512 || to.len() > 512 || from == to {
            return Err(Error::InvalidKey);
        }
        let old_source = self
            .scores
            .get(from)
            .copied()
            .ok_or(Error::InsufficientScore)?;
        let new_source = old_source
            .checked_sub(amount)
            .ok_or(Error::InsufficientScore)?;
        let old_target = self.scores.get(to).copied();
        let new_target = old_target
            .unwrap_or(0)
            .checked_add(amount)
            .and_then(|n| n.checked_add(additional))
            .ok_or(Error::Overflow)?;
        if new_target > 0
            && old_target.is_none()
            && new_source > 0
            && self.scores.len() >= self.capacity
        {
            return Err(Error::Capacity);
        }
        let total = self.total.checked_add(additional).ok_or(Error::Overflow)?;
        let revision = self.revision.checked_add(1).ok_or(Error::Overflow)?;
        if new_source == 0 {
            self.scores.remove(from);
            self.metadata.remove(from);
        } else {
            self.scores.insert(from.to_owned(), new_source);
        }
        if new_target > 0 {
            self.scores.insert(to.to_owned(), new_target);
        }
        self.index.update(
            from,
            Some(old_source),
            (new_source > 0).then_some(new_source),
        );
        self.index
            .update(to, old_target, (new_target > 0).then_some(new_target));
        self.total = total;
        self.revision = revision;
        Ok(())
    }

    fn expire(&mut self, key: &str, amount: u64) -> Result<(), Error> {
        let revision = self.revision.checked_add(1).ok_or(Error::Overflow)?;
        let previous = *self.scores.get(key).expect("window contribution exists");
        let score = previous - amount;
        self.total -= amount;
        if score == 0 {
            self.scores.remove(key);
            self.metadata.remove(key);
        } else {
            self.scores.insert(key.to_owned(), score);
        }
        self.index
            .update(key, Some(previous), (score > 0).then_some(score));
        self.revision = revision;
        Ok(())
    }

    /// Read one score without materializing a ranking snapshot.
    pub fn score(&self, key: &str) -> Option<u64> {
        self.scores.get(key).copied()
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn set_capacity(&mut self, capacity: usize) {
        self.capacity = capacity;
    }

    pub fn touch(&mut self) -> Result<(), Error> {
        self.revision = self.revision.checked_add(1).ok_or(Error::Overflow)?;
        Ok(())
    }

    pub fn reset(&mut self) -> Result<(), Error> {
        let revision = self.revision.checked_add(1).ok_or(Error::Overflow)?;
        self.scores.clear();
        self.metadata.clear();
        self.index.clear();
        self.total = 0;
        self.revision = revision;
        Ok(())
    }

    /// Copies at most `limit` ordered candidates while the caller holds its
    /// lock; the ordered index avoids scanning every participant per update.
    pub fn snapshot_candidates(&self, limit: usize) -> Snapshot {
        let entries = self
            .index
            .top(limit)
            .map(|(key, score)| {
                let mut metadata = self.metadata.get(key).cloned().unwrap_or_default();
                let label = metadata.label.take().unwrap_or_else(|| key.to_owned());
                Entry {
                    key: key.to_owned(),
                    rank: 0,
                    score,
                    label,
                    metadata,
                }
            })
            .collect();
        Snapshot {
            ranking_id: String::new(),
            schema_version: 1,
            creator_id: String::new(),
            room_id: String::new(),
            window: model::Window::Session,
            revision: self.revision,
            instance_id: None,
            integrity_status: "unknown".into(),
            rejected: 0,
            dropped: 0,
            limited: 0,
            cooldown_rejected: 0,
            paused: false,
            entries,
            total_score: self.total,
            total_participants: self.scores.len(),
        }
    }
    pub fn snapshot_unranked(&self) -> Snapshot {
        self.snapshot_candidates(self.scores.len())
    }
    pub fn snapshot(&self, limit: usize) -> Snapshot {
        self.snapshot_candidates(limit).rank(limit)
    }
}

impl Snapshot {
    pub fn rank(mut self, limit: usize) -> Self {
        // Select only the requested prefix before sorting, keeping large
        // rankings O(n + k log k) rather than sorting every participant.
        let compare = |a: &Entry, b: &Entry| b.score.cmp(&a.score).then_with(|| a.key.cmp(&b.key));
        if limit < self.entries.len() {
            self.entries.select_nth_unstable_by(limit, compare);
            self.entries.truncate(limit);
        }
        self.entries.sort_unstable_by(compare);
        for (index, entry) in self.entries.iter_mut().enumerate() {
            entry.rank = index + 1;
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn indexed_top_matches_full_order_and_tracks_updates() {
        let mut aggregation = Aggregation::new(2000);
        for id in (0..1000).rev() {
            aggregation
                .apply(&format!("user-{id:04}"), (id % 13 + 1) as u64, 1)
                .unwrap();
        }
        aggregation.apply("user-0001", 100, 1).unwrap();
        let candidates = aggregation.snapshot_candidates(10);
        assert_eq!(candidates.entries.len(), 10);
        assert_eq!(candidates.total_participants, 1000);
        let top = candidates.rank(10);
        assert_eq!(top.entries[0].key, "user-0001");
        let full = aggregation.snapshot(2000);
        assert_eq!(top.entries, full.entries[..10]);
        aggregation.expire("user-0001", 100).unwrap();
        let restored = aggregation.snapshot(10);
        assert_ne!(restored.entries[0].key, "user-0001");
        aggregation.reset().unwrap();
        assert!(aggregation.snapshot(10).entries.is_empty());
    }
    #[test]
    fn score_reassignment_is_atomic_and_keeps_index_consistent() {
        let mut a = Aggregation::new(2);
        a.apply("a", 10, 1).unwrap();
        a.apply("b", 3, 1).unwrap();
        let before = a.snapshot(10);
        assert_eq!(a.reassign("a", "b", 11, 0), Err(Error::InsufficientScore));
        assert_eq!(a.reassign("a", "c", 5, 0), Err(Error::Capacity));
        assert_eq!(a.reassign("a", "b", 1, u64::MAX), Err(Error::Overflow));
        assert_eq!(a.snapshot(10), before);
        a.reassign("a", "b", 4, 1).unwrap();
        let snapshot = a.snapshot(10);
        assert_eq!(snapshot.total_score, 14);
        assert_eq!(
            (snapshot.entries[0].key.as_str(), snapshot.entries[0].score),
            ("b", 8)
        );
        assert_eq!(
            (snapshot.entries[1].key.as_str(), snapshot.entries[1].score),
            ("a", 6)
        );
        a.reassign("a", "c", 6, 0).unwrap();
        assert_eq!(a.snapshot(10).entries[1].key, "c");
    }
    #[test]
    fn ordering_and_atomic_rejection() {
        let mut a = Aggregation::new(2);
        a.apply("b", 2, 3).unwrap();
        a.apply("a", 6, 1).unwrap();
        let before = a.snapshot(10);
        assert_eq!(before.entries[0].key, "a");
        assert_eq!(a.apply("c", 1, 1), Err(Error::Capacity));
        assert_eq!(a.apply("a", u64::MAX, 2), Err(Error::Overflow));
        assert_eq!(a.snapshot(10), before);
    }
}

#[cfg(test)]
mod engine_tests {
    use crate::engine::Engine;
    use crate::model::{Contribution, Definition, SourceRule};
    #[test]
    fn daily_window_resets_at_local_midnight() {
        use crate::model::Window;
        const DAY: u64 = 86_400_000;
        let mut engine = Engine::default();
        engine
            .create(Definition {
                schema_version: 1,
                id: "daily".into(),
                creator_id: "c".into(),
                room_id: "r".into(),
                group_by: Default::default(),
                // UTC-5: local midnight is 05:00 UTC.
                window: Window::Daily {
                    utc_offset_minutes: -300,
                },
                user_limits: None,
                capacity: 3,
                limit: 3,
                sources: vec![SourceRule {
                    source: "like".into(),
                    metric: "count".into(),
                    weight: 1,
                    enabled: true,
                    gift_multipliers: Default::default(),
                }],
            })
            .unwrap();
        let event = Contribution {
            creator_id: "c",
            room_id: "r",
            source: "like",
            user: Some("u"),
            gift: None,
            option: None,
            team: None,
            custom_key: None,
            metrics: &[("count", 3)],
        };
        let base = 10 * DAY + 3_600_000; // 01:00 UTC = 20:00 local, previous day.
        engine.apply_at(&event, base);
        engine.apply_at(&event, base + 3_000_000); // 01:50 UTC, same local day.
        assert_eq!(engine.snapshot("daily").unwrap().total_score, 6);
        engine.expire_at(base + 4 * 3_600_000 + 1); // 05:00 UTC: local midnight.
        assert_eq!(engine.snapshot("daily").unwrap().total_score, 0);
        engine.apply_at(&event, base + 5 * 3_600_000);
        assert_eq!(engine.snapshot("daily").unwrap().total_score, 3);
    }
    #[test]
    fn checkpoint_preserves_rolling_expiry_and_user_limits() {
        use crate::model::{UserLimits, Window};
        let definition = Definition {
            schema_version: 1,
            id: "rolling".into(),
            creator_id: "c".into(),
            room_id: "r".into(),
            group_by: Default::default(),
            window: Window::Rolling {
                duration_ms: 100,
                max_events: 4,
            },
            user_limits: Some(UserLimits {
                max_contributions: Some(2),
                cooldown_ms: Some(20),
                max_users: 3,
            }),
            capacity: 3,
            limit: 3,
            sources: vec![SourceRule {
                source: "like".into(),
                metric: "count".into(),
                weight: 1,
                enabled: true,
                gift_multipliers: Default::default(),
            }],
        };
        let mut source = Engine::default();
        source.create(definition.clone()).unwrap();
        let event = Contribution {
            creator_id: "c",
            room_id: "r",
            source: "like",
            user: Some("u"),
            gift: None,
            option: None,
            team: None,
            custom_key: None,
            metrics: &[("count", 3)],
        };
        source.apply_at(&event, 100);
        let checkpoint = source.session_checkpoint("rolling").unwrap();
        let mut restored = Engine::default();
        restored.create(definition).unwrap();
        restored.restore_session_checkpoint(&checkpoint).unwrap();
        restored.apply_at(&event, 110);
        assert_eq!(restored.snapshot("rolling").unwrap().cooldown_rejected, 1);
        restored.apply_at(&event, 120);
        assert_eq!(restored.snapshot("rolling").unwrap().total_score, 6);
        restored.expire_at(200);
        assert_eq!(restored.snapshot("rolling").unwrap().total_score, 3);
        restored.apply_at(&event, 220);
        assert_eq!(restored.snapshot("rolling").unwrap().limited, 1);
        assert_eq!(restored.snapshot("rolling").unwrap().total_score, 0);
    }
    #[test]
    fn session_checkpoint_restores_full_ranking_and_rejects_corruption_atomically() {
        let definition = Definition {
            schema_version: 1,
            id: "rank".into(),
            creator_id: "c".into(),
            room_id: "r".into(),
            group_by: Default::default(),
            window: Default::default(),
            user_limits: None,
            capacity: 3,
            limit: 1,
            sources: vec![SourceRule {
                source: "like".into(),
                metric: "count".into(),
                weight: 1,
                enabled: true,
                gift_multipliers: Default::default(),
            }],
        };
        let mut source = Engine::default();
        source.create(definition.clone()).unwrap();
        for user in ["a", "b"] {
            assert!(source
                .apply(&Contribution {
                    creator_id: "c",
                    room_id: "r",
                    source: "like",
                    user: Some(user),
                    gift: None,
                    option: None,
                    team: None,
                    custom_key: None,
                    metrics: &[("count", 3)]
                })
                .is_empty());
        }
        let checkpoint = source.session_checkpoint("rank").unwrap();
        assert_eq!(checkpoint.snapshot.entries.len(), 2);
        let (presentation, limit) = source.snapshot_unranked_capped("rank", 1).unwrap();
        assert_eq!(presentation.rank(limit).entries.len(), 1);
        assert_eq!(source.selection_snapshot("rank").unwrap().entries.len(), 2);
        let mut target = Engine::default();
        target.create(definition.clone()).unwrap();
        let fresh_identity = target.snapshot("rank").unwrap().instance_id.unwrap();
        assert_ne!(
            Some(&fresh_identity),
            checkpoint.snapshot.instance_id.as_ref()
        );
        let mut corrupt = checkpoint.clone();
        corrupt.snapshot.total_score += 1;
        assert_eq!(
            target.restore_session_checkpoint(&corrupt),
            Err(crate::Error::IncompatibleDefinition)
        );
        assert_eq!(target.snapshot("rank").unwrap().total_score, 0);
        let mut legacy = checkpoint.clone();
        legacy.snapshot.instance_id = None;
        let mut legacy_target = Engine::default();
        legacy_target.create(definition).unwrap();
        let legacy_identity = legacy_target.snapshot("rank").unwrap().instance_id;
        legacy_target.restore_session_checkpoint(&legacy).unwrap();
        assert_eq!(
            legacy_target.snapshot("rank").unwrap().instance_id,
            legacy_identity
        );
        target.restore_session_checkpoint(&checkpoint).unwrap();
        assert_eq!(
            target.selection_snapshot("rank"),
            source.selection_snapshot("rank")
        );
        target.set_paused("rank", true).unwrap().unwrap();
        target.mark_recovery_gap("rank").unwrap();
        assert_eq!(
            target.snapshot("rank").unwrap().integrity_status,
            "degraded"
        );
        assert_eq!(
            target.restore_session_checkpoint(&checkpoint),
            Err(crate::Error::IncompatibleDefinition)
        );
    }
    #[test]
    fn presentation_metadata_follows_accepted_scores_and_expires() {
        use crate::{model::Window, EntryMetadata};
        let mut engine = Engine::default();
        engine
            .create(Definition {
                schema_version: 1,
                id: "likes".into(),
                creator_id: "c".into(),
                room_id: "r".into(),
                group_by: Default::default(),
                window: Window::Rolling {
                    duration_ms: 100,
                    max_events: 2,
                },
                user_limits: None,
                capacity: 2,
                limit: 2,
                sources: vec![SourceRule {
                    source: "like".into(),
                    metric: "count".into(),
                    weight: 1,
                    enabled: true,
                    gift_multipliers: Default::default(),
                }],
            })
            .unwrap();
        let mut event = Contribution {
            creator_id: "c",
            room_id: "r",
            source: "like",
            user: Some("42"),
            gift: None,
            option: None,
            team: None,
            custom_key: None,
            metrics: &[("count", 2)],
        };
        let metadata = EntryMetadata {
            label: Some("Alice".into()),
            user_id: Some("42".into()),
            nickname: Some("Alice".into()),
            avatar_url: Some("x".repeat(2049)),
            ..Default::default()
        };
        assert!(engine
            .apply_at_with_metadata(&event, 10, Some(&metadata), None)
            .is_empty());
        let entry = &engine.snapshot("likes").unwrap().entries[0];
        assert_eq!(
            (
                entry.key.as_str(),
                entry.label.as_str(),
                entry.metadata.user_id.as_deref()
            ),
            ("42", "Alice", Some("42"))
        );
        assert!(entry.metadata.avatar_url.is_none());
        event.metrics = &[("count", 0)];
        let other = EntryMetadata {
            label: Some("Ignored".into()),
            ..Default::default()
        };
        engine.apply_at_with_metadata(&event, 11, Some(&other), None);
        assert_eq!(engine.snapshot("likes").unwrap().entries[0].label, "Alice");
        engine.expire_at(110);
        assert!(engine.snapshot("likes").unwrap().entries.is_empty());
        event.metrics = &[("count", 1)];
        engine.apply_at(&event, 111);
        assert_eq!(engine.snapshot("likes").unwrap().entries[0].label, "42");
    }
    #[test]
    fn durable_assignment_rebuild_and_switch_match_incremental_projection() {
        use crate::model::GroupBy;
        use crate::Error;
        let mut engine = Engine::default();
        let definition = Definition {
            schema_version: 1,
            id: "poll".into(),
            creator_id: "c".into(),
            room_id: "r".into(),
            group_by: GroupBy::Option,
            window: Default::default(),
            user_limits: None,
            sources: vec![SourceRule {
                source: "competition.vote".into(),
                metric: "event.count".into(),
                weight: 3,
                enabled: true,
                gift_multipliers: Default::default(),
            }],
            limit: 10,
            capacity: 2,
        };
        engine.create(definition).unwrap();
        engine.apply_assignment("poll", None, ("a", 1)).unwrap();
        engine
            .apply_assignment("poll", Some(("a", 1)), ("b", 2))
            .unwrap();
        let before = engine.snapshot("poll").unwrap();
        assert_eq!(
            (before.entries[0].key.as_str(), before.total_score),
            ("b", 6)
        );
        assert_eq!(
            engine.restore_assignments("poll", &[("a".into(), u64::MAX)]),
            Err(Error::Overflow)
        );
        assert_eq!(engine.snapshot("poll").unwrap(), before);
        engine
            .restore_assignments("poll", &[("b".into(), 2)])
            .unwrap();
        let restored = engine.snapshot("poll").unwrap();
        assert_eq!(restored.entries, before.entries);
        assert_eq!(restored.total_score, before.total_score);
        assert!(restored.revision > before.revision);
        assert_eq!(
            engine.restore_assignments("poll", &[("a".into(), 1), ("a".into(), 2)]),
            Err(Error::InvalidKey)
        );
        assert_eq!(engine.snapshot("poll").unwrap(), restored);
        engine
            .restore_assignments_with_scores(
                "poll",
                &[("b".into(), 2)],
                &[("a".into(), 5), ("b".into(), 4)],
            )
            .unwrap();
        let mixed = engine.snapshot("poll").unwrap();
        assert_eq!(mixed.total_score, 15);
        assert_eq!(
            (mixed.entries[0].key.as_str(), mixed.entries[0].score),
            ("b", 10)
        );
        assert_eq!(
            engine.restore_assignments_with_scores(
                "poll",
                &[("b".into(), 2)],
                &[("a".into(), u64::MAX)]
            ),
            Err(Error::Overflow)
        );
        assert_eq!(engine.snapshot("poll").unwrap(), mixed);
        assert_eq!(
            engine.restore_assignments_with_scores(
                "poll",
                &[],
                &[("a".into(), 1), ("a".into(), 2)]
            ),
            Err(Error::InvalidKey)
        );
        assert_eq!(engine.snapshot("poll").unwrap(), mixed);
    }
    #[test]
    fn user_limits_are_bounded_and_count_only_accepted_scored_packets() {
        use crate::model::{GroupBy, UserLimits};
        use crate::Error;
        let mut engine = Engine::default();
        engine
            .create(Definition {
                schema_version: 1,
                id: "limits".into(),
                creator_id: "c".into(),
                room_id: "r".into(),
                group_by: GroupBy::Gift,
                window: Default::default(),
                user_limits: Some(UserLimits {
                    max_contributions: Some(2),
                    cooldown_ms: Some(100),
                    max_users: 1,
                }),
                sources: vec![SourceRule {
                    source: "gift".into(),
                    metric: "count".into(),
                    weight: 1,
                    enabled: true,
                    gift_multipliers: Default::default(),
                }],
                limit: 10,
                capacity: 10,
            })
            .unwrap();
        let mut event = Contribution {
            creator_id: "c",
            room_id: "r",
            source: "gift",
            user: Some("u"),
            gift: Some("a"),
            option: None,
            team: None,
            custom_key: None,
            metrics: &[("count", 3)],
        };
        assert!(engine.apply_at(&event, 1000).is_empty());
        event.gift = Some("b");
        assert!(engine.apply_at(&event, 1099).is_empty());
        event.metrics = &[("count", 0)];
        assert!(engine.apply_at(&event, 1099).is_empty());
        event.metrics = &[("count", 3)];
        assert!(engine.apply_at(&event, 1100).is_empty());
        assert!(engine.apply_at(&event, 1200).is_empty());
        let snapshot = engine.snapshot("limits").unwrap();
        assert_eq!(
            (
                snapshot.total_score,
                snapshot.limited,
                snapshot.cooldown_rejected
            ),
            (6, 1, 1)
        );
        assert_eq!(
            (snapshot.rejected, snapshot.integrity_status.as_str()),
            (0, "unknown")
        );
        event.user = Some("other");
        assert_eq!(
            engine.apply_at(&event, 1300),
            vec![("limits".into(), Error::Capacity)]
        );
        event.user = None;
        assert_eq!(
            engine.apply_at(&event, 1301),
            vec![("limits".into(), Error::InvalidKey)]
        );
        engine.reset("limits").unwrap().unwrap();
        event.user = Some("other");
        assert!(engine.apply_at(&event, 1302).is_empty());
        let reset = engine.snapshot("limits").unwrap();
        assert_eq!(
            (
                reset.total_score,
                reset.limited,
                reset.cooldown_rejected,
                reset.rejected
            ),
            (3, 0, 0, 0)
        );
    }
    #[test]
    fn gift_multiplier_is_integer_scoped_and_rejects_ambiguous_or_overflowing_input() {
        use crate::Error;
        use std::collections::BTreeMap;
        let mut engine = Engine::default();
        let rule = SourceRule {
            source: "tiktok.gift".into(),
            metric: "gift.diamonds".into(),
            weight: 2,
            enabled: true,
            gift_multipliers: BTreeMap::from([("rose".into(), 3), ("disabled".into(), 0)]),
        };
        let definition = Definition {
            schema_version: 1,
            id: "gift".into(),
            creator_id: "c".into(),
            room_id: "r".into(),
            group_by: Default::default(),
            window: Default::default(),
            user_limits: None,
            capacity: 10,
            limit: 10,
            sources: vec![rule],
        };
        engine.create(definition.clone()).unwrap();
        let mut event = Contribution {
            creator_id: "c",
            room_id: "r",
            source: "tiktok.gift",
            user: Some("u"),
            gift: Some("rose"),
            option: None,
            team: None,
            custom_key: None,
            metrics: &[("gift.diamonds", 5)],
        };
        assert!(engine.apply_at(&event, 1).is_empty());
        assert_eq!(engine.snapshot("gift").unwrap().total_score, 30);
        event.gift = Some("disabled");
        event.metrics = &[("gift.diamonds", u64::MAX)];
        assert!(engine.apply_at(&event, 2).is_empty());
        event.metrics = &[("gift.diamonds", 5)];
        event.gift = Some("unknown");
        assert!(engine.apply_at(&event, 3).is_empty());
        assert_eq!(engine.snapshot("gift").unwrap().total_score, 40);
        event.gift = None;
        assert_eq!(
            engine.apply_at(&event, 4),
            vec![("gift".into(), Error::InvalidKey)]
        );
        event.metrics = &[("gift.diamonds", 0)];
        assert!(engine.apply_at(&event, 5).is_empty());
        event.gift = Some("rose");
        event.metrics = &[("gift.diamonds", u64::MAX)];
        assert_eq!(
            engine.apply_at(&event, 6),
            vec![("gift".into(), Error::Overflow)]
        );
        assert_eq!(engine.snapshot("gift").unwrap().total_score, 40);
        let mut invalid = definition;
        invalid.id = "invalid".into();
        invalid.sources[0].weight = 1_000_000_001;
        assert!(engine.create(invalid).is_err());
    }
    #[test]
    fn rolling_window_expires_at_boundary_and_rejects_excess() {
        use crate::model::Window;
        use crate::Error;
        let mut engine = Engine::default();
        engine
            .create(Definition {
                schema_version: 1,
                id: "r".into(),
                creator_id: "c".into(),
                room_id: "room".into(),
                group_by: Default::default(),
                window: Window::Rolling {
                    duration_ms: 100,
                    max_events: 2,
                },
                user_limits: None,
                limit: 10,
                capacity: 4,
                sources: vec![SourceRule {
                    source: "like".into(),
                    metric: "count".into(),
                    weight: 1,
                    enabled: true,
                    gift_multipliers: Default::default(),
                }],
            })
            .unwrap();
        let event = Contribution {
            creator_id: "c",
            room_id: "room",
            source: "like",
            user: Some("u"),
            gift: None,
            option: None,
            team: None,
            custom_key: None,
            metrics: &[("count", 5)],
        };
        assert!(engine.apply_at(&event, 1000).is_empty());
        assert!(engine.apply_at(&event, 1010).is_empty());
        assert_eq!(
            engine.apply_at(&event, 1020),
            vec![("r".into(), Error::WindowCapacity)]
        );
        assert_eq!(engine.snapshot("r").unwrap().total_score, 10);
        engine.expire_at(1099);
        assert_eq!(engine.snapshot("r").unwrap().total_score, 10);
        engine.expire_at(1100);
        assert_eq!(engine.snapshot("r").unwrap().total_score, 5);
        // Regressing clocks never bring expired contributions back.
        engine.expire_at(900);
        assert_eq!(engine.snapshot("r").unwrap().total_score, 5);
        engine.expire_at(1110);
        let snapshot = engine.snapshot("r").unwrap();
        assert_eq!(snapshot.total_score, 0);
        assert_eq!(snapshot.total_participants, 0);
        assert_eq!(snapshot.integrity_status, "degraded");
    }
    #[test]
    fn definition_update_requires_pause_and_explicit_reset() {
        use crate::engine::DefinitionError;
        let mut engine = Engine::default();
        let mut definition = Definition {
            schema_version: 1,
            id: "r".into(),
            creator_id: "c".into(),
            room_id: "room".into(),
            group_by: Default::default(),
            window: Default::default(),
            user_limits: None,
            limit: 2,
            capacity: 4,
            sources: vec![SourceRule {
                source: "tiktok.like".into(),
                metric: "like.count".into(),
                weight: 1,
                enabled: true,
                gift_multipliers: Default::default(),
            }],
        };
        engine.create(definition.clone()).unwrap();
        assert_eq!(
            engine.update(definition.clone(), false),
            Err(DefinitionError::NotPaused)
        );
        let event = Contribution {
            creator_id: "c",
            room_id: "room",
            source: "tiktok.like",
            user: Some("u"),
            gift: None,
            option: None,
            team: None,
            custom_key: None,
            metrics: &[("like.count", 5)],
        };
        engine.apply(&event);
        engine.set_paused("r", true).unwrap().unwrap();
        definition.limit = 1;
        engine.update(definition.clone(), false).unwrap();
        assert_eq!(engine.snapshot("r").unwrap().total_score, 5);
        definition.sources[0].weight = 2;
        assert_eq!(
            engine.update(definition.clone(), false),
            Err(DefinitionError::ResetRequired)
        );
        engine.update(definition, true).unwrap();
        assert_eq!(engine.snapshot("r").unwrap().total_score, 0);
        engine.set_paused("r", false).unwrap().unwrap();
        engine.apply(&event);
        assert_eq!(engine.snapshot("r").unwrap().total_score, 10);
    }
    #[test]
    fn ingress_loss_degrades_running_rankings_and_reset_recovers_local_status() {
        let mut engine = Engine::default();
        engine
            .create(Definition {
                schema_version: 1,
                id: "r".into(),
                creator_id: "c".into(),
                room_id: "room".into(),
                group_by: Default::default(),
                window: Default::default(),
                user_limits: None,
                limit: 1,
                capacity: 4,
                sources: vec![SourceRule {
                    source: "tiktok.like".into(),
                    metric: "like.count".into(),
                    weight: 1,
                    enabled: true,
                    gift_multipliers: Default::default(),
                }],
            })
            .unwrap();
        engine.mark_ingress_loss(3);
        let degraded = engine.snapshot("r").unwrap();
        assert_eq!(degraded.dropped, 3);
        assert_eq!(degraded.integrity_status, "degraded");
        assert_eq!(degraded.revision, 1);
        engine.reset("r").unwrap().unwrap();
        let reset = engine.snapshot("r").unwrap();
        assert_eq!(reset.dropped, 0);
        assert_eq!(reset.integrity_status, "unknown");
        assert!(reset.revision > degraded.revision);
    }
    #[test]
    fn packet_overflow_is_atomic_and_marks_integrity() {
        use crate::model::GroupBy;
        let mut engine = Engine::default();
        engine
            .create(Definition {
                schema_version: 1,
                id: "gift".into(),
                creator_id: "c".into(),
                room_id: "r".into(),
                group_by: GroupBy::Gift,
                window: Default::default(),
                user_limits: None,
                limit: 10,
                capacity: 10,
                sources: vec![
                    SourceRule {
                        source: "gift".into(),
                        metric: "count".into(),
                        weight: 1,
                        enabled: true,
                        gift_multipliers: Default::default(),
                    },
                    SourceRule {
                        source: "gift".into(),
                        metric: "diamonds".into(),
                        weight: 2,
                        enabled: true,
                        gift_multipliers: Default::default(),
                    },
                ],
            })
            .unwrap();
        let event = Contribution {
            creator_id: "c",
            room_id: "r",
            source: "gift",
            user: Some("u"),
            gift: Some("9"),
            option: None,
            team: None,
            custom_key: None,
            metrics: &[("count", 1), ("diamonds", u64::MAX)],
        };
        assert_eq!(engine.apply(&event).len(), 1);
        let snapshot = engine.snapshot("gift").unwrap();
        assert_eq!(snapshot.ranking_id, "gift");
        assert_eq!(snapshot.creator_id, "c");
        assert_eq!(snapshot.total_score, 0);
        assert_eq!(snapshot.rejected, 1);
        assert_eq!(snapshot.integrity_status, "degraded");
    }
    #[test]
    fn sources_scopes_and_pause_are_independent() {
        let mut engine = Engine::default();
        engine
            .create(Definition {
                schema_version: 1,
                id: "rank".into(),
                creator_id: "creator".into(),
                room_id: "room".into(),
                group_by: Default::default(),
                window: Default::default(),
                user_limits: None,
                limit: 10,
                capacity: 4,
                sources: vec![
                    SourceRule {
                        source: "tiktok.like".into(),
                        metric: "like.count".into(),
                        weight: 2,
                        enabled: true,
                        gift_multipliers: Default::default(),
                    },
                    SourceRule {
                        source: "tiktok.gift".into(),
                        metric: "gift.diamonds".into(),
                        weight: 3,
                        enabled: false,
                        gift_multipliers: Default::default(),
                    },
                ],
            })
            .unwrap();
        let mut c = Contribution {
            creator_id: "creator",
            room_id: "room",
            source: "tiktok.like",
            user: Some("user"),
            gift: None,
            option: None,
            team: None,
            custom_key: None,
            metrics: &[("like.count", 5)],
        };
        assert!(engine.apply(&c).is_empty());
        c.room_id = "other";
        engine.apply(&c);
        c.room_id = "room";
        c.source = "tiktok.gift";
        c.metrics = &[("gift.diamonds", 5)];
        engine.apply(&c);
        engine.set_paused("rank", true).unwrap().unwrap();
        c.source = "tiktok.like";
        c.metrics = &[("like.count", 5)];
        engine.apply(&c);
        assert_eq!(engine.snapshot("rank").unwrap().total_score, 10);
        assert!(engine.remove("rank"));
        assert!(engine.snapshot("rank").is_none());
    }
}
