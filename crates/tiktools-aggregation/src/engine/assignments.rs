//! Durable option assignments and separately committed routed score projections.
use super::*;

impl Engine {
    /// Weight for the durable command-vote projection. Routed like and share
    /// scores are recovered separately; other sources remain unsupported.
    pub(super) fn assignment_weight(definition: &Definition) -> Result<u64, Error> {
        if definition.group_by != GroupBy::Option
            || definition.window != Window::Session
            || definition.user_limits.is_some()
            || definition.sources.iter().any(|rule| {
                !matches!(
                    (rule.source.as_str(), rule.metric.as_str()),
                    ("competition.vote", "event.count")
                        | ("tiktok.like", "like.count")
                        | ("tiktok.like", "event.count")
                        | ("tiktok.share", "share.count")
                        | ("tiktok.share", "event.count")
                        | ("tiktok.gift", "gift.count")
                        | ("tiktok.gift", "gift.diamonds")
                        | ("tiktok.gift", "event.count")
                ) || (rule.source != "tiktok.gift" && !rule.gift_multipliers.is_empty())
            })
        {
            return Err(Error::IncompatibleDefinition);
        }
        definition
            .sources
            .iter()
            .filter(|rule| rule.enabled && rule.source == "competition.vote")
            .try_fold(0u64, |sum, rule| {
                sum.checked_add(rule.weight).ok_or(Error::Overflow)
            })
    }

    /// Rebuild a dedicated ranking from durable per-option vote totals. Build
    /// off to the side so invalid or overflowing data never partially replaces
    /// an existing snapshot. Callers serialize this with new vote admission.
    pub fn restore_assignments(&mut self, id: &str, totals: &[(String, u64)]) -> Result<(), Error> {
        self.restore_assignments_with_scores(id, totals, &[])
    }

    /// Restore vote totals and separately committed routed scores atomically.
    pub fn restore_assignments_with_scores(
        &mut self,
        id: &str,
        totals: &[(String, u64)],
        scored: &[(String, u64)],
    ) -> Result<(), Error> {
        let ranking = self.rankings.get_mut(id).ok_or(Error::NotFound)?;
        let weight = Self::assignment_weight(&ranking.definition)?;
        let revision = ranking
            .state
            .revision()
            .checked_add(1)
            .ok_or(Error::Overflow)?;
        let mut candidate = Aggregation::new(ranking.definition.capacity);
        let mut seen = std::collections::HashSet::new();
        for (option, votes) in totals {
            if !seen.insert(option) {
                return Err(Error::InvalidKey);
            }
            candidate.apply(option, *votes, weight)?;
        }
        let mut score_keys = std::collections::HashSet::new();
        for (option, score) in scored {
            if !score_keys.insert(option) {
                return Err(Error::InvalidKey);
            }
            candidate.apply(option, *score, 1)?;
        }
        candidate.revision = revision;
        ranking.state = candidate;
        Ok(())
    }

    /// Apply one committed participant assignment transition. `previous` and
    /// `next` are durable vote counts, not packet cumulative counters.
    pub fn apply_assignment(
        &mut self,
        id: &str,
        previous: Option<(&str, u64)>,
        next: (&str, u64),
    ) -> Result<(), Error> {
        let ranking = self.rankings.get_mut(id).ok_or(Error::NotFound)?;
        let weight = Self::assignment_weight(&ranking.definition)?;
        if ranking.paused {
            return Err(Error::IncompatibleDefinition);
        }
        let (option, votes) = next;
        let (old_option, old_votes) = previous.unwrap_or((option, 0));
        let delta = votes
            .checked_sub(old_votes)
            .ok_or(Error::InsufficientScore)?;
        let result = if old_option == option {
            ranking.state.apply(option, delta, weight)
        } else if weight == 0 {
            Ok(())
        } else {
            ranking.state.reassign(
                old_option,
                option,
                old_votes.checked_mul(weight).ok_or(Error::Overflow)?,
                delta.checked_mul(weight).ok_or(Error::Overflow)?,
            )
        };
        if result.is_err() {
            ranking.rejected = ranking.rejected.saturating_add(1);
            let _ = ranking.state.touch();
        }
        result
    }

    /// Score a canonical routed delta without changing state. The host must
    /// commit its identity and assignment before calling apply_committed_score.
    pub fn score_routed(&self, id: &str, contribution: &Contribution<'_>) -> Result<u64, Error> {
        let ranking = self.rankings.get(id).ok_or(Error::NotFound)?;
        Self::assignment_weight(&ranking.definition)?;
        if !ranking
            .definition
            .sources
            .iter()
            .any(|r| r.source == "competition.vote")
            || ranking.definition.creator_id != contribution.creator_id
            || ranking.definition.room_id != contribution.room_id
            || !matches!(
                contribution.source,
                "tiktok.like" | "tiktok.share" | "tiktok.gift"
            )
        {
            return Err(Error::IncompatibleDefinition);
        }
        crate::scoring::score(&ranking.definition.sources, contribution)
    }

    pub fn apply_committed_score(
        &mut self,
        id: &str,
        option: &str,
        score: u64,
    ) -> Result<(), Error> {
        let ranking = self.rankings.get_mut(id).ok_or(Error::NotFound)?;
        Self::assignment_weight(&ranking.definition)?;
        if ranking.paused {
            return Err(Error::IncompatibleDefinition);
        }
        let result = ranking.state.apply(option, score, 1);
        if result.is_err() {
            ranking.rejected = ranking.rejected.saturating_add(1);
            let _ = ranking.state.touch();
        }
        result
    }
}
