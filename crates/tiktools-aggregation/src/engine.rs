use crate::model::{Contribution, Definition, GroupBy, Window};
use crate::{Aggregation, EntryMetadata, Error, Snapshot};
use std::collections::{HashMap, VecDeque};

/// Complete session state for persistence. The definition is included so a
/// checkpoint cannot be restored against changed scoring or scope settings.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionCheckpoint {
    pub schema_version: u32,
    pub definition: Definition,
    pub snapshot: Snapshot,
    #[serde(default)]
    window_events: Vec<(u64, String, u64)>,
    #[serde(default)]
    last_time: u64,
    #[serde(default)]
    users: HashMap<String, UserState>,
}

/// Bounded aggregate health counters; no participant identities are exposed.
#[derive(Clone, Debug, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostics {
    pub rankings: usize,
    pub paused: usize,
    pub degraded: usize,
    pub rejected: u64,
    pub dropped: u64,
    pub limited: u64,
    pub cooldown_rejected: u64,
}

/// Indexed definitions and bounded ranking state. Callers serialize updates
/// and supply only events already admitted and deduplicated by ingress.
#[derive(Default)]
pub struct Engine {
    rankings: HashMap<String, Ranking>,
    by_source: HashMap<String, Vec<String>>,
    next_instance: u64,
}
struct Ranking {
    definition: Definition,
    instance: u64,
    instance_id: String,
    state: Aggregation,
    paused: bool,
    rejected: u64,
    dropped: u64,
    window_events: VecDeque<(u64, String, u64)>,
    last_time: u64,
    users: HashMap<String, UserState>,
    limited: u64,
    cooldown_rejected: u64,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct UserState {
    count: u64,
    last_time: u64,
}
#[derive(Clone, Copy)]
enum UserDecision {
    Allowed,
    Limited,
    Cooldown,
}

#[derive(Debug, PartialEq, Eq)]
pub enum DefinitionError {
    Invalid,
    Duplicate,
    Capacity,
    NotFound,
    NotPaused,
    ResetRequired,
    RevisionOverflow,
    IdentityUnavailable,
}

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
    /// Export all bounded accounting state, including rolling expiry and limits.
    pub fn session_checkpoint(&self, id: &str) -> Result<SessionCheckpoint, Error> {
        let ranking = self.rankings.get(id).ok_or(Error::NotFound)?;
        Ok(SessionCheckpoint {
            schema_version: 1,
            definition: ranking.definition.clone(),
            snapshot: self.selection_snapshot(id).ok_or(Error::NotFound)?,
            window_events: ranking.window_events.iter().cloned().collect(),
            last_time: ranking.last_time,
            users: ranking.users.clone(),
        })
    }

    /// Restore an entire checkpoint atomically into a fresh ranking instance.
    /// A caller must separately prove the checkpoint includes all accepted
    /// contributions before relying on its integrity for winner selection.
    pub fn restore_session_checkpoint(
        &mut self,
        checkpoint: &SessionCheckpoint,
    ) -> Result<(), Error> {
        let ranking = self
            .rankings
            .get_mut(&checkpoint.definition.id)
            .ok_or(Error::NotFound)?;
        let snapshot = &checkpoint.snapshot;
        if checkpoint.schema_version != 1
            || ranking.definition != checkpoint.definition
            || ranking.state.revision() != 0
            || snapshot.schema_version != 1
            || snapshot.ranking_id != ranking.definition.id
            || snapshot.creator_id != ranking.definition.creator_id
            || snapshot.room_id != ranking.definition.room_id
            || snapshot.window != ranking.definition.window
            || snapshot.instance_id.as_ref().is_some_and(|id| {
                id.len() != 32 || !id.bytes().all(|byte| byte.is_ascii_hexdigit())
            })
            || snapshot.entries.len() != snapshot.total_participants
            || snapshot.entries.len() > ranking.definition.capacity
            || !matches!(snapshot.integrity_status.as_str(), "unknown" | "degraded")
            || (snapshot.integrity_status == "unknown"
                && (snapshot.rejected != 0 || snapshot.dropped != 0))
            || (snapshot.integrity_status == "degraded"
                && snapshot.rejected == 0
                && snapshot.dropped == 0)
        {
            return Err(Error::IncompatibleDefinition);
        }
        let mut state = Aggregation::new(ranking.definition.capacity);
        let mut seen = std::collections::HashSet::new();
        for entry in &snapshot.entries {
            if entry.score == 0 || !seen.insert(&entry.key) {
                return Err(Error::InvalidKey);
            }
            state.apply(&entry.key, entry.score, 1)?;
            let mut metadata = entry.metadata.clone();
            metadata.label = Some(entry.label.clone());
            state.update_metadata(&entry.key, &metadata);
        }
        if state.total != snapshot.total_score || snapshot.revision < state.revision() {
            return Err(Error::IncompatibleDefinition);
        }
        match ranking.definition.window {
            Window::Session | Window::Daily { .. } if !checkpoint.window_events.is_empty() => {
                return Err(Error::IncompatibleDefinition)
            }
            Window::Rolling { max_events, .. } => {
                if checkpoint.window_events.len() > max_events {
                    return Err(Error::IncompatibleDefinition);
                }
                let mut totals = HashMap::<&str, u64>::new();
                let mut last = 0;
                for (time, key, score) in &checkpoint.window_events {
                    if *time < last || *time > checkpoint.last_time || *score == 0 {
                        return Err(Error::IncompatibleDefinition);
                    }
                    last = *time;
                    let total = totals.entry(key).or_default();
                    *total = total.checked_add(*score).ok_or(Error::Overflow)?;
                }
                if totals.len() != state.scores.len()
                    || state
                        .scores
                        .iter()
                        .any(|(key, score)| totals.get(key.as_str()) != Some(score))
                {
                    return Err(Error::IncompatibleDefinition);
                }
            }
            _ => {}
        }
        match &ranking.definition.user_limits {
            None if !checkpoint.users.is_empty() => return Err(Error::IncompatibleDefinition),
            Some(limits)
                if checkpoint.users.len() > limits.max_users
                    || (!state.scores.is_empty() && checkpoint.users.is_empty())
                    || checkpoint.users.iter().any(|(key, user)| {
                        key.is_empty()
                            || key.len() > 512
                            || user.count == 0
                            || user.last_time > checkpoint.last_time
                            || limits.max_contributions.is_some_and(|max| user.count > max)
                    }) =>
            {
                return Err(Error::IncompatibleDefinition)
            }
            _ => {}
        }
        state.revision = snapshot.revision;
        ranking.state = state;
        ranking.paused = snapshot.paused;
        ranking.rejected = snapshot.rejected;
        ranking.dropped = snapshot.dropped;
        ranking.limited = snapshot.limited;
        ranking.cooldown_rejected = snapshot.cooldown_rejected;
        ranking.window_events = checkpoint.window_events.iter().cloned().collect();
        ranking.last_time = checkpoint.last_time;
        ranking.users = checkpoint.users.clone();
        if let Some(instance_id) = &snapshot.instance_id {
            ranking.instance_id = instance_id.clone();
        }
        Ok(())
    }
    pub fn create(&mut self, definition: Definition) -> Result<(), DefinitionError> {
        if self.rankings.len() >= 128 {
            return Err(DefinitionError::Capacity);
        }
        if definition.schema_version != 1
            || definition.id.is_empty()
            || definition.creator_id.is_empty()
            || definition.room_id.is_empty()
            || definition.id.len() > 128
            || definition.creator_id.len() > 256
            || definition.room_id.len() > 256
            || definition.sources.len() > 64
            || definition.capacity > 100_000
            || definition.capacity == 0
            || definition.limit == 0
            || matches!(&definition.window, Window::Rolling { duration_ms, max_events } if *duration_ms == 0 || *max_events == 0 || *max_events > 100_000)
            || matches!(&definition.window, Window::Daily { utc_offset_minutes } if !(-840..=840).contains(utc_offset_minutes))
            || definition.user_limits.as_ref().is_some_and(|limits| {
                limits.max_users == 0
                    || limits.max_users > 100_000
                    || limits.max_contributions == Some(0)
                    || limits.cooldown_ms == Some(0)
                    || (limits.max_contributions.is_none() && limits.cooldown_ms.is_none())
            })
            || definition.sources.is_empty()
            || (definition
                .sources
                .iter()
                .any(|s| s.source == "competition.vote")
                && Self::assignment_weight(&definition).is_err())
            || definition.sources.iter().any(|s| {
                s.source.is_empty()
                    || s.weight > 1_000_000_000
                    || s.gift_multipliers.len() > 512
                    || s.gift_multipliers.iter().any(|(gift, multiplier)| {
                        gift.is_empty() || gift.len() > 128 || *multiplier > 1_000_000_000
                    })
                    || s.source.len() > 128
                    || s.metric.is_empty()
                    || s.metric.len() > 128
            })
        {
            return Err(DefinitionError::Invalid);
        }
        if self.rankings.contains_key(&definition.id) {
            return Err(DefinitionError::Duplicate);
        }
        let mut identity = [0u8; 16];
        getrandom::fill(&mut identity).map_err(|_| DefinitionError::IdentityUnavailable)?;
        let instance_id = identity
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        for source in &definition.sources {
            let ids = self.by_source.entry(source.source.clone()).or_default();
            if !ids.contains(&definition.id) {
                ids.push(definition.id.clone());
            }
        }
        let instance = self
            .next_instance
            .checked_add(1)
            .ok_or(DefinitionError::RevisionOverflow)?;
        self.next_instance = instance;
        let state = Aggregation::new(definition.capacity);
        self.rankings.insert(
            definition.id.clone(),
            Ranking {
                definition,
                instance,
                instance_id,
                state,
                paused: false,
                rejected: 0,
                dropped: 0,
                window_events: VecDeque::new(),
                last_time: 0,
                users: HashMap::new(),
                limited: 0,
                cooldown_rejected: 0,
            },
        );
        Ok(())
    }

    /// Configuration changes require a paused ranking; changing accounting
    /// semantics also requires an explicit reset of the accumulated state.
    pub fn validate_update(
        &self,
        definition: &Definition,
        reset: bool,
    ) -> Result<(), DefinitionError> {
        let mut validator = Engine::default();
        validator.create(definition.clone())?;
        let current = self
            .rankings
            .get(&definition.id)
            .ok_or(DefinitionError::NotFound)?;
        if !current.paused {
            return Err(DefinitionError::NotPaused);
        }
        let mut comparable = current.definition.clone();
        comparable.limit = definition.limit;
        if comparable != *definition && !reset {
            return Err(DefinitionError::ResetRequired);
        }
        if current.state.revision() == u64::MAX {
            return Err(DefinitionError::RevisionOverflow);
        }
        Ok(())
    }
    pub fn update(&mut self, definition: Definition, reset: bool) -> Result<(), DefinitionError> {
        self.validate_update(&definition, reset)?;
        self.by_source.retain(|_, ids| {
            ids.retain(|id| id != &definition.id);
            !ids.is_empty()
        });
        for source in &definition.sources {
            let ids = self.by_source.entry(source.source.clone()).or_default();
            if !ids.contains(&definition.id) {
                ids.push(definition.id.clone());
            }
        }
        let ranking = self
            .rankings
            .get_mut(&definition.id)
            .expect("validated ranking");
        if reset {
            ranking
                .state
                .reset()
                .map_err(|_| DefinitionError::RevisionOverflow)?;
            ranking.state.set_capacity(definition.capacity);
            ranking.rejected = 0;
            ranking.dropped = 0;
            ranking.window_events.clear();
            ranking.last_time = 0;
            ranking.users.clear();
            ranking.limited = 0;
            ranking.cooldown_rejected = 0;
        } else {
            ranking
                .state
                .touch()
                .map_err(|_| DefinitionError::RevisionOverflow)?;
        }
        ranking.definition = definition;
        Ok(())
    }

    /// Weight for the durable command-vote projection. Routed like and share
    /// scores are recovered separately; other sources remain unsupported.
    fn assignment_weight(definition: &Definition) -> Result<u64, Error> {
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
    fn expire_ranking(ranking: &mut Ranking, now: u64) -> Result<(), Error> {
        if let Window::Daily { utc_offset_minutes } = ranking.definition.window {
            let day = |time: u64| {
                (time.min(i64::MAX as u64) as i64)
                    .saturating_add(i64::from(utc_offset_minutes) * 60_000)
                    .div_euclid(86_400_000)
            };
            let previous = ranking.last_time;
            ranking.last_time = previous.max(now);
            if previous != 0 && day(ranking.last_time) != day(previous) {
                ranking.state.reset()?;
                ranking.users.clear();
                ranking.rejected = 0;
                ranking.dropped = 0;
                ranking.limited = 0;
                ranking.cooldown_rejected = 0;
            }
            return Ok(());
        }
        ranking.last_time = ranking.last_time.max(now);
        let Window::Rolling { duration_ms, .. } = ranking.definition.window else {
            return Ok(());
        };
        while let Some((at, key, amount)) = ranking.window_events.front() {
            if ranking.last_time.saturating_sub(*at) < duration_ms {
                break;
            }
            ranking.state.expire(key, *amount)?;
            ranking.window_events.pop_front();
        }
        Ok(())
    }
    /// Expires contributions even when no new events arrive.
    pub fn expire_at(&mut self, now: u64) {
        for ranking in self.rankings.values_mut() {
            if Self::expire_ranking(ranking, now).is_err() {
                ranking.rejected = ranking.rejected.saturating_add(1);
            }
        }
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
    pub fn definitions(&self) -> Vec<Definition> {
        let mut definitions: Vec<_> = self
            .rankings
            .values()
            .map(|r| r.definition.clone())
            .collect();
        definitions.sort_unstable_by(|a, b| a.id.cmp(&b.id));
        definitions
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
    pub fn reset(&mut self, id: &str) -> Option<Result<(), Error>> {
        self.rankings.get_mut(id).map(|r| {
            let result = r.state.reset();
            if result.is_ok() {
                r.rejected = 0;
                r.dropped = 0;
                r.window_events.clear();
                r.last_time = 0;
                r.users.clear();
                r.limited = 0;
                r.cooldown_rejected = 0;
            }
            result
        })
    }
    pub fn set_paused(&mut self, id: &str, paused: bool) -> Option<Result<(), Error>> {
        self.rankings.get_mut(id).map(|r| {
            if r.paused != paused {
                r.state.touch()?;
                r.paused = paused;
            }
            Ok(())
        })
    }
    pub fn remove(&mut self, id: &str) -> bool {
        if self.rankings.remove(id).is_none() {
            return false;
        }
        self.by_source.retain(|_, ids| {
            ids.retain(|item| item != id);
            !ids.is_empty()
        });
        true
    }
}
