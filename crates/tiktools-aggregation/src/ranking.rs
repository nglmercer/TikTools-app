//! Incremental ordered projection of authoritative scores.
//!
//! This index is not a counter: callers own the score and supply its old and
//! new values. Ties sort by stable UTF-8 identity, independent of arrival order.
use std::{cmp::Reverse, collections::BTreeSet};

#[derive(Default)]
pub struct RankingIndex {
    entries: BTreeSet<(Reverse<u64>, String)>,
}
impl RankingIndex {
    /// Replace a previously indexed score. `None` removes or creates an entry.
    pub fn update(&mut self, key: &str, previous: Option<u64>, next: Option<u64>) {
        if let Some(score) = previous {
            self.entries.remove(&(Reverse(score), key.to_owned()));
        }
        if let Some(score) = next {
            self.entries.insert((Reverse(score), key.to_owned()));
        }
    }
    pub fn clear(&mut self) {
        self.entries.clear();
    }
    pub fn top(&self, limit: usize) -> impl Iterator<Item = (&str, u64)> {
        self.entries
            .iter()
            .take(limit)
            .map(|(Reverse(score), key)| (key.as_str(), *score))
    }

    /// Membership in the visible top-N without materializing the snapshot.
    pub fn is_in_top(&self, key: &str, limit: usize) -> bool {
        self.entries
            .iter()
            .take(limit)
            .any(|(_, candidate)| candidate == key)
    }
}
