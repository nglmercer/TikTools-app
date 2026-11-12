//! Health counters and explicit accounting gaps for incomplete input.
use super::*;

impl Engine {
    pub fn diagnostics(&self) -> Diagnostics {
        let mut result = Diagnostics {
            rankings: self.rankings.len(),
            ..Default::default()
        };
        for ranking in self.rankings.values() {
            result.paused += usize::from(ranking.paused);
            result.degraded += usize::from(ranking.rejected > 0 || ranking.dropped > 0);
            result.rejected = result.rejected.saturating_add(ranking.rejected);
            result.dropped = result.dropped.saturating_add(ranking.dropped);
            result.limited = result.limited.saturating_add(ranking.limited);
            result.cooldown_rejected = result
                .cooldown_rejected
                .saturating_add(ranking.cooldown_rejected);
        }
        result
    }

    /// Mark one ranking whose routed contribution could not be accounted for.
    pub fn mark_ranking_gap(&mut self, id: &str) {
        if let Some(ranking) = self.rankings.get_mut(id) {
            if !ranking.paused {
                ranking.rejected = ranking.rejected.saturating_add(1);
                let _ = ranking.state.touch();
            }
        }
    }

    /// Mark rankings whose accepted input could not be accounted for.
    /// Recovery gaps apply even when the restored ranking is paused.
    pub fn mark_recovery_gap(&mut self, id: &str) -> Result<(), Error> {
        let ranking = self.rankings.get_mut(id).ok_or(Error::NotFound)?;
        ranking.rejected = ranking.rejected.saturating_add(1);
        ranking.state.touch()?;
        Ok(())
    }

    pub fn mark_gap(&mut self, creator: &str, room: &str, source: &str) {
        if let Some(ids) = self.by_source.get(source) {
            for id in ids {
                let ranking = self.rankings.get_mut(id).expect("indexed ranking");
                if !ranking.paused
                    && (source == "competition.vote"
                        || !ranking
                            .definition
                            .sources
                            .iter()
                            .any(|rule| rule.source == "competition.vote"))
                    && ranking.definition.creator_id == creator
                    && ranking.definition.room_id == room
                    && ranking
                        .definition
                        .sources
                        .iter()
                        .any(|rule| rule.enabled && rule.source == source)
                {
                    ranking.rejected = ranking.rejected.saturating_add(1);
                    let _ = ranking.state.touch();
                }
            }
        }
    }

    /// Mark only rankings depending on a metric that could not be measured.
    pub fn mark_metric_gap(&mut self, creator: &str, room: &str, source: &str, metric: &str) {
        if let Some(ids) = self.by_source.get(source) {
            for id in ids {
                let ranking = self.rankings.get_mut(id).expect("indexed ranking");
                if !ranking.paused
                    && (source == "competition.vote"
                        || !ranking
                            .definition
                            .sources
                            .iter()
                            .any(|rule| rule.source == "competition.vote"))
                    && ranking.definition.creator_id == creator
                    && ranking.definition.room_id == room
                    && ranking.definition.sources.iter().any(|rule| {
                        rule.enabled
                            && rule.weight > 0
                            && rule.source == source
                            && rule.metric == metric
                    })
                {
                    ranking.rejected = ranking.rejected.saturating_add(1);
                    let _ = ranking.state.touch();
                }
            }
        }
    }

    /// An ingress loss has unknown event identity and scope. Conservatively
    /// flag all running rankings rather than claim their totals are complete.
    pub fn mark_ingress_loss(&mut self, count: u64) {
        if count == 0 {
            return;
        }
        for ranking in self.rankings.values_mut().filter(|ranking| !ranking.paused) {
            ranking.dropped = ranking.dropped.saturating_add(count);
            let _ = ranking.state.touch();
        }
    }
}
