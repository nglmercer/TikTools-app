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

    /// Whether one key is inside the ordered top `limit`, without copying
    /// labels, metadata, or snapshot entries.
    pub fn is_in_top(&self, key: &str, limit: usize) -> bool {
        self.index.is_in_top(key, limit)
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
        assert!(aggregation.is_in_top("user-0001", 10));
        assert!(!aggregation.is_in_top("user-0999", 1));
        let full = aggregation.snapshot(2000);
        assert_eq!(top.entries, full.entries[..10]);
        aggregation.expire("user-0001", 100).unwrap();
        let restored = aggregation.snapshot(10);
        assert_ne!(restored.entries[0].key, "user-0001");
        aggregation.reset().unwrap();
        assert!(aggregation.snapshot(10).entries.is_empty());
    }
}
