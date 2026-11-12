use super::*;

impl PointsService {
    /// Direct indexed lookup by identity: hot cache first, then one SQLite
    /// primary-key read that promotes the row into the cache. Returns `None`
    /// for an invalid/missing identity or a cold reload that cannot safely
    /// finish within the writer wait bound.
    pub fn viewer(&self, unique_id: &str) -> Option<Value> {
        let _mutation = recover_mutex(&self.mutation, "points mutation");
        let needle = clean_unique_id(unique_id)?;
        {
            let mut inner = recover_rwlock_write(&self.inner, "points viewers");
            if let Some(state) = inner.viewers.get(&needle).cloned() {
                inner.hits += 1;
                return Some(state.to_value());
            }
            inner.misses += 1;
        }
        #[cfg(feature = "persistence")]
        {
            // Never base a reload on state older than our own queued work.
            if !self.wait_for_viewer_resolved(&needle) {
                return None;
            }
            let loaded = self
                .database
                .as_ref()
                .and_then(|database| database.load_viewer_by_id(&needle).ok().flatten())
                .and_then(|value| ViewerState::from_value(&value));
            if let Some(state) = loaded {
                let resolved = self.resolved_seq();
                let mut inner = recover_rwlock_write(&self.inner, "points viewers");
                // A concurrent award may have created the row while the
                // database read was in flight: keep the newer resident.
                let resident = inner.viewers.entry(needle).or_insert(state).clone();
                inner.evict_cold(resolved);
                return Some(resident.to_value());
            }
        }
        None
    }

    /// Top-K leaderboard with deterministic tie-breaking. Selection is
    /// O(N) to find the K winners plus O(K log K) to order them — never a
    /// full O(N log N) sort — and it runs only when a caller asks for the
    /// board, never per awarded event.
    pub fn leaderboard(&self, limit: Option<i64>) -> Vec<Value> {
        let limit = limit.unwrap_or(100).clamp(0, 1_000) as usize;
        if limit == 0 {
            return Vec::new();
        }
        let inner = recover_rwlock_read(&self.inner, "points viewers");
        if inner.viewers.len() <= limit {
            let mut all: Vec<&ViewerState> = inner.viewers.values().collect();
            all.sort_by(|left, right| ViewerState::compare_rank(left, right));
            return all.into_iter().map(ViewerState::to_value).collect();
        }
        let mut states: Vec<&ViewerState> = inner.viewers.values().collect();
        states.select_nth_unstable_by(limit, |left, right| ViewerState::compare_rank(left, right));
        let mut top: Vec<&ViewerState> = states.into_iter().take(limit).collect();
        top.sort_by(|left, right| ViewerState::compare_rank(left, right));
        top.into_iter().map(ViewerState::to_value).collect()
    }
}
