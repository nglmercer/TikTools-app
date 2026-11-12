//! Canonical event admission, bounded user decisions and atomic score accounting.
use super::*;

impl Engine {
    /// Returns failures per ranking so overload and arithmetic errors cannot
    /// be mistaken for successful accounting.
    pub fn apply(&mut self, contribution: &Contribution<'_>) -> Vec<(String, Error)> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .min(u64::MAX as u128) as u64;
        self.apply_at(contribution, now)
    }

    /// `now` is a host timestamp in milliseconds. Clock regressions are
    /// clamped to the last observed time within each ranking.
    pub fn apply_at(&mut self, contribution: &Contribution<'_>, now: u64) -> Vec<(String, Error)> {
        self.apply_at_with_metadata(contribution, now, None, None)
    }

    /// Metadata is selected by grouping and updated only with an accepted,
    /// positive score; it never admits a contribution or affects ordering.
    pub fn apply_at_with_metadata(
        &mut self,
        contribution: &Contribution<'_>,
        now: u64,
        user: Option<&EntryMetadata>,
        gift: Option<&EntryMetadata>,
    ) -> Vec<(String, Error)> {
        let mut failures = Vec::new();
        if let Some(ids) = self.by_source.get(contribution.source) {
            for id in ids {
                let ranking = self.rankings.get_mut(id).expect("indexed ranking");
                if ranking.paused
                    || ranking.definition.creator_id != contribution.creator_id
                    || ranking.definition.room_id != contribution.room_id
                {
                    continue;
                }
                if let Err(error) = Self::expire_ranking(ranking, now) {
                    ranking.rejected = ranking.rejected.saturating_add(1);
                    failures.push((id.clone(), error));
                    continue;
                }
                // Routed competition sources require a durable participant
                // assignment; the native adapter handles them separately.
                if contribution.option.is_none()
                    && ranking
                        .definition
                        .sources
                        .iter()
                        .any(|rule| rule.source == "competition.vote")
                {
                    continue;
                }
                // Sum all metrics first; apply a packet atomically to each ranking.
                let score = crate::scoring::score(&ranking.definition.sources, contribution);
                if score == Ok(0) {
                    continue;
                }
                let key = match ranking.definition.group_by {
                    GroupBy::User => contribution.user,
                    GroupBy::Gift => contribution.gift,
                    GroupBy::EventType => Some(contribution.source),
                    GroupBy::Option => contribution.option,
                    GroupBy::Team => contribution.team,
                    GroupBy::CustomKey => contribution.custom_key,
                };
                let result = match (score, key) {
                    (Err(error), _) => Err(error),
                    (_, None) => Err(Error::InvalidKey),
                    (Ok(score), Some(key)) => match ranking.definition.window {
                        Window::Rolling { max_events, .. }
                            if ranking.window_events.len() >= max_events =>
                        {
                            Err(Error::WindowCapacity)
                        }
                        _ => {
                            let decision = Self::user_decision(ranking, contribution.user);
                            match decision {
                                Err(error) => Err(error),
                                Ok(UserDecision::Limited | UserDecision::Cooldown) => {
                                    let decision = decision.expect("matched successful decision");
                                    match ranking.state.touch() {
                                        Ok(()) => {
                                            match decision {
                                                UserDecision::Limited => {
                                                    ranking.limited =
                                                        ranking.limited.saturating_add(1)
                                                }
                                                UserDecision::Cooldown => {
                                                    ranking.cooldown_rejected =
                                                        ranking.cooldown_rejected.saturating_add(1)
                                                }
                                                UserDecision::Allowed => unreachable!(),
                                            }
                                            Ok(())
                                        }
                                        Err(error) => Err(error),
                                    }
                                }
                                Ok(UserDecision::Allowed) => {
                                    let result = ranking.state.apply(key, score, 1);
                                    if result.is_ok() {
                                        let metadata = match ranking.definition.group_by {
                                            GroupBy::User => user,
                                            GroupBy::Gift => gift,
                                            _ => None,
                                        };
                                        if let Some(metadata) = metadata {
                                            ranking.state.update_metadata(key, metadata);
                                        }
                                        if ranking.definition.user_limits.is_some() {
                                            let user = contribution.user.expect("validated user");
                                            let state = ranking
                                                .users
                                                .entry(user.to_owned())
                                                .or_insert(UserState {
                                                    count: 0,
                                                    last_time: ranking.last_time,
                                                });
                                            state.count += 1;
                                            state.last_time = ranking.last_time;
                                        }
                                        if matches!(
                                            ranking.definition.window,
                                            Window::Rolling { .. }
                                        ) {
                                            ranking.window_events.push_back((
                                                ranking.last_time,
                                                key.to_owned(),
                                                score,
                                            ));
                                        }
                                    }
                                    result
                                }
                            }
                        }
                    },
                };
                if let Err(error) = result {
                    ranking.rejected = ranking.rejected.saturating_add(1);
                    let _ = ranking.state.touch();
                    failures.push((id.clone(), error));
                }
            }
        }
        failures
    }

    fn user_decision(ranking: &Ranking, user: Option<&str>) -> Result<UserDecision, Error> {
        let Some(limits) = &ranking.definition.user_limits else {
            return Ok(UserDecision::Allowed);
        };
        let user = user
            .filter(|user| !user.is_empty() && user.len() <= 512)
            .ok_or(Error::InvalidKey)?;
        if let Some(state) = ranking.users.get(user) {
            if limits
                .max_contributions
                .is_some_and(|max| state.count >= max)
            {
                return Ok(UserDecision::Limited);
            }
            if limits.cooldown_ms.is_some_and(|cooldown| {
                ranking.last_time.saturating_sub(state.last_time) < cooldown
            }) {
                return Ok(UserDecision::Cooldown);
            }
            state.count.checked_add(1).ok_or(Error::Overflow)?;
        } else if ranking.users.len() >= limits.max_users {
            return Err(Error::Capacity);
        }
        Ok(UserDecision::Allowed)
    }
}
