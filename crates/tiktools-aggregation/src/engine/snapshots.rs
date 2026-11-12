//! Bounded presentation metadata, change tokens and ranking projections.
use super::*;

impl Engine {
    /// Update presentation for an existing entry without creating a score.
    /// Unknown keys are ignored and changed labels advance the revision.
    pub fn set_entry_metadata(
        &mut self,
        id: &str,
        key: &str,
        metadata: &EntryMetadata,
    ) -> Result<(), Error> {
        let ranking = self.rankings.get_mut(id).ok_or(Error::NotFound)?;
        if !ranking.state.scores.contains_key(key) {
            return Ok(());
        }
        let mut next = ranking.state.metadata.get(key).cloned().unwrap_or_default();
        next.merge(metadata.bounded());
        if ranking.state.metadata.get(key) != Some(&next) {
            ranking.state.touch()?;
            ranking.state.metadata.insert(key.to_owned(), next);
        }
        Ok(())
    }

    /// Whether any ranking consumes `source`; lets adapters skip unused work.
    pub fn has_source(&self, source: &str) -> bool {
        self.by_source.contains_key(source)
    }

    /// Small change tokens used by coalesced presentation publishers.
    pub fn change_tokens(&self) -> Vec<(String, u64, u64)> {
        self.rankings
            .iter()
            .map(|(id, ranking)| (id.clone(), ranking.instance, ranking.state.revision()))
            .collect()
    }

    pub fn snapshot_unranked(&self, id: &str) -> Option<(Snapshot, usize)> {
        self.snapshot_unranked_capped(id, usize::MAX)
    }

    /// Bounded presentation snapshot; control retrieval retains the configured limit.
    pub fn snapshot_unranked_capped(&self, id: &str, cap: usize) -> Option<(Snapshot, usize)> {
        self.rankings.get(id).map(|r| {
            let limit = r.definition.limit.min(cap);
            let mut snapshot = r.state.snapshot_candidates(limit);
            snapshot.ranking_id = r.definition.id.clone();
            snapshot.instance_id = Some(r.instance_id.clone());
            snapshot.creator_id = r.definition.creator_id.clone();
            snapshot.room_id = r.definition.room_id.clone();
            snapshot.window = r.definition.window.clone();
            snapshot.rejected = r.rejected;
            snapshot.dropped = r.dropped;
            snapshot.limited = r.limited;
            snapshot.cooldown_rejected = r.cooldown_rejected;
            snapshot.paused = r.paused;
            if r.rejected > 0 || r.dropped > 0 {
                snapshot.integrity_status = "degraded".into();
            }
            (snapshot, limit)
        })
    }

    /// All eligible entries for a bounded ranking, independent of display limit.
    pub fn selection_snapshot(&self, id: &str) -> Option<Snapshot> {
        self.rankings.get(id).map(|ranking| {
            let mut snapshot = ranking
                .state
                .snapshot_candidates(ranking.definition.capacity);
            snapshot.ranking_id = ranking.definition.id.clone();
            snapshot.instance_id = Some(ranking.instance_id.clone());
            snapshot.creator_id = ranking.definition.creator_id.clone();
            snapshot.room_id = ranking.definition.room_id.clone();
            snapshot.window = ranking.definition.window.clone();
            snapshot.rejected = ranking.rejected;
            snapshot.dropped = ranking.dropped;
            snapshot.limited = ranking.limited;
            snapshot.cooldown_rejected = ranking.cooldown_rejected;
            snapshot.paused = ranking.paused;
            if ranking.rejected > 0 || ranking.dropped > 0 {
                snapshot.integrity_status = "degraded".into();
            }
            snapshot.rank(ranking.definition.capacity)
        })
    }

    pub fn snapshot(&self, id: &str) -> Option<Snapshot> {
        self.snapshot_unranked(id)
            .map(|(snapshot, limit)| snapshot.rank(limit))
    }
}
