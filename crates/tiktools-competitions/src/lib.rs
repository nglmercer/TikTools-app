//! Headless competition state machine. The host owns clocks and persistence;
//! callers must persist a frozen result before publishing or dispatching it.
pub mod participants;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Lifecycle {
    Draft,
    Active,
    Paused,
    Finalizing,
    Finished,
    Cancelled,
    Failed,
}
/// Durable lifecycle; absolute deadlines are supplied by the host clock.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleState {
    pub lifecycle: Lifecycle,
    pub generation: u64,
    pub deadline_ms: Option<u64>,
    pub remaining_ms: Option<u64>,
}
impl Default for LifecycleState {
    fn default() -> Self {
        Self {
            lifecycle: Lifecycle::Draft,
            generation: 0,
            deadline_ms: None,
            remaining_ms: None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transition {
    Start,
    Pause,
    Resume,
    Cancel,
    Finish,
    Expire,
}
impl LifecycleState {
    pub fn transition(
        &mut self,
        transition: Transition,
        now: u64,
        duration: Option<u64>,
    ) -> Result<(), Error> {
        match transition {
            Transition::Start if self.lifecycle == Lifecycle::Draft => {
                let deadline = duration
                    .map(|d| now.checked_add(d).ok_or(Error::Overflow))
                    .transpose()?;
                let generation = self.generation.checked_add(1).ok_or(Error::Overflow)?;
                self.deadline_ms = deadline;
                self.generation = generation;
                self.lifecycle = Lifecycle::Active;
            }
            Transition::Pause if self.lifecycle == Lifecycle::Active && !self.is_due(now) => {
                self.remaining_ms = self.deadline_ms.map(|deadline| deadline - now);
                self.deadline_ms = None;
                self.lifecycle = Lifecycle::Paused;
            }
            Transition::Resume if self.lifecycle == Lifecycle::Paused => {
                self.deadline_ms = self
                    .remaining_ms
                    .map(|d| now.checked_add(d).ok_or(Error::Overflow))
                    .transpose()?;
                self.remaining_ms = None;
                self.lifecycle = Lifecycle::Active;
            }
            Transition::Cancel
                if matches!(
                    self.lifecycle,
                    Lifecycle::Draft | Lifecycle::Active | Lifecycle::Paused
                ) =>
            {
                self.lifecycle = Lifecycle::Cancelled;
                self.deadline_ms = None;
                self.remaining_ms = None;
            }
            Transition::Finish
                if matches!(self.lifecycle, Lifecycle::Active | Lifecycle::Paused) =>
            {
                self.lifecycle = Lifecycle::Finalizing;
                self.deadline_ms = None;
                self.remaining_ms = None;
            }
            Transition::Expire if self.is_due(now) => {
                self.lifecycle = Lifecycle::Finalizing;
                self.deadline_ms = None;
            }
            _ => return Err(Error::InvalidTransition),
        }
        Ok(())
    }
    pub fn is_due(&self, now: u64) -> bool {
        self.lifecycle == Lifecycle::Active && self.deadline_ms.is_some_and(|d| now >= d)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SelectionMode {
    HighestScore,
    UniformRandom,
    WeightedRandom,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TiePolicy {
    Manual,
    Stable,
    Random,
    NoWinner,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub key: String,
    pub weight: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Selection {
    pub mode: SelectionMode,
    pub tie_policy: TiePolicy,
    pub eligible: Vec<Candidate>,
    pub winner: Option<String>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CompetitionKind {
    Poll,
    WeightedPoll,
    LeaderboardChallenge,
    TeamBattle,
    RandomDraw,
    WeightedDraw,
    Roulette,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OptionDefinition {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub commands: Vec<String>,
    #[serde(default)]
    pub gift_ids: Vec<String>,
    pub image_url: Option<String>,
    pub on_win: Option<WinnerAction>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Definition {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub kind: CompetitionKind,
    pub ranking_id: String,
    pub options: Vec<OptionDefinition>,
    pub selection_mode: SelectionMode,
    pub tie_policy: TiePolicy,
    pub duration_ms: Option<u64>,
    pub vote_limit: u64,
    pub allow_switch: bool,
    pub minimum_participants: u64,
    /// Leaderboard challenges: the leader must reach this score to win.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_score: Option<u64>,
    /// Action for winners that are users rather than options (participant
    /// draws and leaderboard challenges).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_user_win: Option<WinnerAction>,
}
impl Definition {
    /// Reject ambiguous command and gift routing rather than choosing an
    /// arbitrary option. Commands are compared case-insensitively.
    pub fn validate(&self) -> Result<(), Error> {
        if self.schema_version != 1
            || self.id.is_empty()
            || self.id.len() > 128
            || self.name.trim().is_empty()
            || self.name.len() > 256
            || self.ranking_id.is_empty()
            || self.ranking_id.len() > 128
            || (self.options.is_empty() != (self.kind == CompetitionKind::LeaderboardChallenge))
            || self.options.len() > 256
            || (self.kind == CompetitionKind::LeaderboardChallenge
                && self.selection_mode != SelectionMode::HighestScore)
            || (self.target_score.is_some() && self.kind != CompetitionKind::LeaderboardChallenge)
            || self.target_score == Some(0)
            || (self.on_user_win.is_some()
                && !matches!(
                    self.kind,
                    CompetitionKind::LeaderboardChallenge
                        | CompetitionKind::RandomDraw
                        | CompetitionKind::WeightedDraw
                ))
            || self
                .on_user_win
                .as_ref()
                .is_some_and(|action| action.action_id.is_empty() || action.action_id.len() > 128)
            || (self.kind == CompetitionKind::RandomDraw
                && self.selection_mode != SelectionMode::UniformRandom)
            || (self.kind == CompetitionKind::WeightedDraw
                && self.selection_mode != SelectionMode::WeightedRandom)
            || self.vote_limit == 0
            || self.duration_ms == Some(0)
        {
            return Err(Error::InvalidCandidate);
        }
        let mut ids = BTreeSet::new();
        let mut commands = BTreeSet::new();
        let mut gifts = BTreeSet::new();
        for option in &self.options {
            if option.id.is_empty()
                || option.id.len() > 128
                || option.label.trim().is_empty()
                || option.label.len() > 256
                || !ids.insert(&option.id)
                || option.commands.len() > 64
                || option.gift_ids.len() > 256
            {
                return Err(Error::InvalidCandidate);
            }
            if let Some(action) = &option.on_win {
                if action.action_id.is_empty() || action.action_id.len() > 128 {
                    return Err(Error::InvalidCandidate);
                }
            }
            for command in &option.commands {
                if command.len() > 128
                    || !command.starts_with('!')
                    || command.trim() != command
                    || !commands.insert(command.to_lowercase())
                {
                    return Err(Error::InvalidCandidate);
                }
            }
            for gift in &option.gift_ids {
                if gift.is_empty() || gift.len() > 128 || !gifts.insert(gift) {
                    return Err(Error::InvalidCandidate);
                }
            }
        }
        Ok(())
    }
}

/// Canonical, already-admitted command with explicit replay and room scope.
/// `received_at` is a host timestamp, never a UI clock.
#[derive(Clone, Debug)]
pub struct CommandParticipation<'a> {
    pub creator_id: &'a str,
    pub room_id: &'a str,
    pub participant_id: &'a str,
    pub event_identity: &'a str,
    pub command: &'a str,
    pub received_at: u64,
}

/// Result of routing one command. Rejected or ambiguous input never chooses
/// an arbitrary option. A duplicate is scoped to a competition generation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "status",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum ParticipationOutcome {
    Accepted {
        option_id: String,
        votes: u64,
        switched: bool,
    },
    Duplicate {
        option_id: String,
    },
    IdentityConflict,
    Closed,
    UnknownCommand,
    SwitchForbidden,
    VoteLimit,
    Capacity,
}
impl Definition {
    /// Match a whole comment command, case-insensitively; messages with
    /// extra text are not implicitly accepted as votes.
    pub fn option_for_command(&self, comment: &str) -> Option<&str> {
        if comment.len() > 128 {
            return None;
        }
        let command = comment.trim().to_lowercase();
        self.options
            .iter()
            .find(|option| {
                option
                    .commands
                    .iter()
                    .any(|candidate| candidate.to_lowercase() == command)
            })
            .map(|option| option.id.as_str())
    }
}

/// A reference to an existing automation action; execution remains the
/// responsibility of the host automation runtime and its capability checks.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WinnerAction {
    pub action_id: String,
    pub inputs: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ActionClaim {
    pub competition_id: String,
    pub generation: u64,
    pub idempotency_key: String,
    pub action: WinnerAction,
}

/// Public execution status omits action inputs and idempotency keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Uncertain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActionResolution {
    Completed,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ActionExecution {
    pub competition_id: String,
    pub generation: u64,
    pub action_id: String,
    pub status: ActionStatus,
    /// Sanitized, machine-readable reason; never includes action inputs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

/// A finalized generation's immutable selection. The persisted eligible set
/// is the authority for replay and audit, not a widget animation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResultRecord {
    pub schema_version: u32,
    pub competition_id: String,
    pub generation: u64,
    pub ranking_id: String,
    pub ranking_revision: u64,
    pub finalized_at: u64,
    pub selection: Selection,
}

/// Bounded presentation projection of a committed result. The full frozen
/// eligible set remains in ResultRecord and is never altered by rendering.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RevealSnapshot {
    pub schema_version: u32,
    pub competition_id: String,
    pub ranking_id: String,
    pub generation: u64,
    pub selection: RevealSelection,
    pub eligible_count: usize,
    pub other_weight: u64,
}
/// Presentation-only subset; the complete selection remains in the result record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RevealSelection {
    pub mode: SelectionMode,
    pub eligible: Vec<Candidate>,
    pub winner: Option<String>,
}
impl ResultRecord {
    pub fn reveal_snapshot(&self) -> Result<RevealSnapshot, Error> {
        self.selection.validate()?;
        const LIMIT: usize = 256;
        let mut eligible = self
            .selection
            .eligible
            .iter()
            .take(LIMIT)
            .cloned()
            .collect::<Vec<_>>();
        if let Some(winner) = &self.selection.winner {
            if !eligible.iter().any(|entry| &entry.key == winner) {
                let winner = self
                    .selection
                    .eligible
                    .iter()
                    .find(|entry| &entry.key == winner)
                    .ok_or(Error::InvalidCandidate)?
                    .clone();
                eligible.pop();
                eligible.push(winner);
            }
        }
        let included: std::collections::HashSet<&str> =
            eligible.iter().map(|entry| entry.key.as_str()).collect();
        let other_weight = self
            .selection
            .eligible
            .iter()
            .filter(|entry| !included.contains(entry.key.as_str()))
            .try_fold(0u64, |sum, entry| {
                let weight = if self.selection.mode == SelectionMode::WeightedRandom {
                    entry.weight
                } else {
                    1
                };
                sum.checked_add(weight).ok_or(Error::Overflow)
            })?;
        Ok(RevealSnapshot {
            schema_version: self.schema_version,
            competition_id: self.competition_id.clone(),
            ranking_id: self.ranking_id.clone(),
            generation: self.generation,
            selection: RevealSelection {
                mode: self.selection.mode,
                eligible,
                winner: self.selection.winner.clone(),
            },
            eligible_count: self.selection.eligible.len(),
            other_weight,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidTransition,
    InvalidCandidate,
    DuplicateCandidate,
    Empty,
    ZeroWeight,
    Overflow,
    ManualTie,
    RandomUnavailable,
    UnknownOption,
    SwitchForbidden,
    Capacity,
    VoteLimit,
}

impl Selection {
    /// Validate a record before persistence or recovery; the eligible set is
    /// sorted, unique, bounded, and the winner must belong to it.
    pub fn validate(&self) -> Result<(), Error> {
        // An empty eligible set records an explicit no-winner outcome, such
        // as a leaderboard challenge whose target nobody reached.
        if self.eligible.is_empty() {
            return if self.winner.is_none() {
                Ok(())
            } else {
                Err(Error::Empty)
            };
        }
        if self.eligible.len() > 100_000 {
            return Err(Error::Empty);
        }
        if self
            .eligible
            .iter()
            .any(|c| c.key.is_empty() || c.key.len() > 512)
        {
            return Err(Error::InvalidCandidate);
        }
        if self
            .eligible
            .windows(2)
            .any(|pair| pair[0].key >= pair[1].key)
        {
            return Err(Error::DuplicateCandidate);
        }
        let chosen = match &self.winner {
            Some(key) => Some(
                self.eligible
                    .iter()
                    .find(|c| &c.key == key)
                    .ok_or(Error::InvalidCandidate)?,
            ),
            None => None,
        };
        match self.mode {
            SelectionMode::HighestScore => {
                let highest = self.eligible.iter().map(|c| c.weight).max().unwrap();
                let mut leaders = self.eligible.iter().filter(|c| c.weight == highest);
                let first = leaders.next().expect("nonempty candidates");
                let tied = leaders.next().is_some();
                if chosen.is_none() && (self.tie_policy != TiePolicy::NoWinner || !tied) {
                    return Err(Error::InvalidCandidate);
                }
                if let Some(chosen) = chosen {
                    if chosen.weight != highest
                        || (tied && self.tie_policy == TiePolicy::NoWinner)
                        || (tied && self.tie_policy == TiePolicy::Stable && chosen.key != first.key)
                    {
                        return Err(Error::InvalidCandidate);
                    }
                }
            }
            SelectionMode::UniformRandom => {
                if chosen.is_none() {
                    return Err(Error::InvalidCandidate);
                }
            }
            SelectionMode::WeightedRandom => {
                let total = self
                    .eligible
                    .iter()
                    .try_fold(0u64, |sum, c| sum.checked_add(c.weight))
                    .ok_or(Error::Overflow)?;
                if total == 0 || chosen.is_none_or(|c| c.weight == 0) {
                    return Err(Error::ZeroWeight);
                }
            }
        }
        Ok(())
    }
}

/// Resolve a manual highest-score tie against the frozen eligible set.
/// Only a tied leader can be chosen; no new random draw is performed.
pub fn select_manual(mut eligible: Vec<Candidate>, winner: &str) -> Result<Selection, Error> {
    if eligible.is_empty() || eligible.len() > 100_000 || winner.is_empty() {
        return Err(Error::InvalidCandidate);
    }
    eligible.sort_unstable_by(|a, b| a.key.cmp(&b.key));
    let highest = eligible
        .iter()
        .map(|candidate| candidate.weight)
        .max()
        .ok_or(Error::Empty)?;
    if eligible
        .iter()
        .filter(|candidate| candidate.weight == highest)
        .count()
        < 2
        || !eligible
            .iter()
            .any(|candidate| candidate.key == winner && candidate.weight == highest)
    {
        return Err(Error::InvalidCandidate);
    }
    let selection = Selection {
        mode: SelectionMode::HighestScore,
        tie_policy: TiePolicy::Manual,
        eligible,
        winner: Some(winner.to_owned()),
    };
    selection.validate()?;
    Ok(selection)
}

/// Cryptographically secure rejection sampling, with no modulo bias.
fn random_below(bound: u64) -> Result<u64, Error> {
    if bound == 0 {
        return Err(Error::ZeroWeight);
    }
    let threshold = bound.wrapping_neg() % bound;
    loop {
        let mut bytes = [0; 8];
        getrandom::fill(&mut bytes).map_err(|_| Error::RandomUnavailable)?;
        let value = u64::from_le_bytes(bytes);
        if value >= threshold {
            return Ok(value % bound);
        }
    }
}

/// Freezes a sorted candidate list and selects once. Store the returned value
/// durably before revealing it; this function does not execute any effects.
pub fn select(
    mut eligible: Vec<Candidate>,
    mode: SelectionMode,
    tie: TiePolicy,
) -> Result<Selection, Error> {
    if eligible.is_empty() || eligible.len() > 100_000 {
        return Err(Error::Empty);
    }
    if eligible
        .iter()
        .any(|c| c.key.is_empty() || c.key.len() > 512)
    {
        return Err(Error::InvalidCandidate);
    }
    eligible.sort_unstable_by(|a, b| a.key.cmp(&b.key));
    if eligible.windows(2).any(|pair| pair[0].key == pair[1].key) {
        return Err(Error::DuplicateCandidate);
    }
    let winner = match mode {
        SelectionMode::HighestScore => {
            let max = eligible.iter().map(|c| c.weight).max().unwrap();
            let tied: Vec<_> = eligible.iter().filter(|c| c.weight == max).collect();
            if tied.len() == 1 {
                Some(tied[0].key.clone())
            } else {
                match tie {
                    TiePolicy::Manual => return Err(Error::ManualTie),
                    TiePolicy::NoWinner => None,
                    TiePolicy::Stable => Some(tied[0].key.clone()),
                    TiePolicy::Random => {
                        Some(tied[random_below(tied.len() as u64)? as usize].key.clone())
                    }
                }
            }
        }
        SelectionMode::UniformRandom => Some(
            eligible[random_below(eligible.len() as u64)? as usize]
                .key
                .clone(),
        ),
        SelectionMode::WeightedRandom => {
            let total = eligible
                .iter()
                .try_fold(0u64, |sum, c| sum.checked_add(c.weight))
                .ok_or(Error::Overflow)?;
            let mut ticket = random_below(total)?;
            let mut winner = None;
            for candidate in &eligible {
                if ticket < candidate.weight {
                    winner = Some(candidate.key.clone());
                    break;
                }
                ticket -= candidate.weight;
            }
            winner
        }
    };
    Ok(Selection {
        mode,
        tie_policy: tie,
        eligible,
        winner,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Participant {
    option: String,
    votes: u64,
}
/// Routing state only; scoring is owned by the aggregation engine. The caller
/// applies a returned option's accepted contribution to the ranking.
pub struct Competition {
    lifecycle: Lifecycle,
    options: BTreeSet<String>,
    participants: BTreeMap<String, Participant>,
    capacity: usize,
    vote_limit: u64,
    allow_switch: bool,
    remaining_ms: Option<u64>,
    deadline_ms: Option<u64>,
}
impl Competition {
    pub fn new(
        options: impl IntoIterator<Item = String>,
        capacity: usize,
        vote_limit: u64,
        allow_switch: bool,
    ) -> Result<Self, Error> {
        let mut set = BTreeSet::new();
        for option in options {
            if option.is_empty() || option.len() > 512 {
                return Err(Error::InvalidCandidate);
            }
            if !set.insert(option) {
                return Err(Error::DuplicateCandidate);
            }
        }
        if set.is_empty()
            || set.len() > 256
            || capacity == 0
            || capacity > 100_000
            || vote_limit == 0
        {
            return Err(Error::Capacity);
        }
        Ok(Self {
            lifecycle: Lifecycle::Draft,
            options: set,
            participants: BTreeMap::new(),
            capacity,
            vote_limit,
            allow_switch,
            remaining_ms: None,
            deadline_ms: None,
        })
    }
    pub fn lifecycle(&self) -> Lifecycle {
        self.lifecycle
    }

    pub fn start(&mut self, now_ms: u64, duration_ms: Option<u64>) -> Result<(), Error> {
        if self.lifecycle != Lifecycle::Draft {
            return Err(Error::InvalidTransition);
        }
        let deadline = duration_ms
            .map(|d| now_ms.checked_add(d).ok_or(Error::Overflow))
            .transpose()?;
        self.deadline_ms = deadline;
        self.lifecycle = Lifecycle::Active;
        Ok(())
    }
    pub fn pause(&mut self, now_ms: u64) -> Result<(), Error> {
        if self.lifecycle != Lifecycle::Active {
            return Err(Error::InvalidTransition);
        }
        self.remaining_ms = self
            .deadline_ms
            .map(|deadline| deadline.saturating_sub(now_ms));
        self.deadline_ms = None;
        self.lifecycle = Lifecycle::Paused;
        Ok(())
    }
    pub fn resume(&mut self, now_ms: u64) -> Result<(), Error> {
        if self.lifecycle != Lifecycle::Paused {
            return Err(Error::InvalidTransition);
        }
        self.deadline_ms = self
            .remaining_ms
            .map(|d| now_ms.checked_add(d).ok_or(Error::Overflow))
            .transpose()?;
        self.remaining_ms = None;
        self.lifecycle = Lifecycle::Active;
        Ok(())
    }
    pub fn is_due(&self, now_ms: u64) -> bool {
        self.lifecycle == Lifecycle::Active && self.deadline_ms.is_some_and(|d| now_ms >= d)
    }
    pub fn begin_finalizing(&mut self) -> Result<(), Error> {
        if !matches!(self.lifecycle, Lifecycle::Active | Lifecycle::Paused) {
            return Err(Error::InvalidTransition);
        }
        self.lifecycle = Lifecycle::Finalizing;
        self.deadline_ms = None;
        Ok(())
    }
    pub fn finish(&mut self) -> Result<(), Error> {
        if self.lifecycle != Lifecycle::Finalizing {
            return Err(Error::InvalidTransition);
        }
        self.lifecycle = Lifecycle::Finished;
        Ok(())
    }
    pub fn cancel(&mut self) -> Result<(), Error> {
        if !matches!(
            self.lifecycle,
            Lifecycle::Draft | Lifecycle::Active | Lifecycle::Paused
        ) {
            return Err(Error::InvalidTransition);
        }
        self.lifecycle = Lifecycle::Cancelled;
        self.deadline_ms = None;
        Ok(())
    }
    /// A switch retains the user's votes; at the vote limit it moves the
    /// existing votes without granting another one.
    pub fn vote(&mut self, now_ms: u64, user: &str, option: &str) -> Result<(), Error> {
        if self.lifecycle != Lifecycle::Active || self.is_due(now_ms) {
            return Err(Error::InvalidTransition);
        }
        if user.is_empty() || user.len() > 512 {
            return Err(Error::InvalidCandidate);
        }
        if !self.options.contains(option) {
            return Err(Error::UnknownOption);
        }
        if let Some(p) = self.participants.get_mut(user) {
            if p.option != option && !self.allow_switch {
                return Err(Error::SwitchForbidden);
            }
            if p.votes >= self.vote_limit {
                if p.option == option {
                    return Err(Error::VoteLimit);
                }
            } else {
                p.votes += 1;
            }
            p.option = option.to_owned();
        } else {
            if self.participants.len() >= self.capacity {
                return Err(Error::Capacity);
            }
            self.participants.insert(
                user.to_owned(),
                Participant {
                    option: option.to_owned(),
                    votes: 1,
                },
            );
        }
        Ok(())
    }
    /// Unassigned users, paused competitions and expired competitions never route.
    pub fn selected_option(&self, now_ms: u64, user: &str) -> Option<&str> {
        if self.lifecycle != Lifecycle::Active || self.is_due(now_ms) {
            return None;
        }
        self.participants.get(user).map(|p| p.option.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn persisted_highest_score_selection_enforces_tie_policy() {
        let eligible = vec![
            Candidate {
                key: "a".into(),
                weight: 5,
            },
            Candidate {
                key: "b".into(),
                weight: 5,
            },
        ];
        let mut selection = Selection {
            mode: SelectionMode::HighestScore,
            tie_policy: TiePolicy::Stable,
            eligible,
            winner: Some("b".into()),
        };
        assert_eq!(selection.validate(), Err(Error::InvalidCandidate));
        selection.winner = Some("a".into());
        assert_eq!(selection.validate(), Ok(()));
        selection.tie_policy = TiePolicy::NoWinner;
        assert_eq!(selection.validate(), Err(Error::InvalidCandidate));
        selection.winner = None;
        assert_eq!(selection.validate(), Ok(()));
    }
    #[test]
    fn reveal_projection_keeps_winner_and_omitted_weight() {
        let selection = Selection {
            mode: SelectionMode::WeightedRandom,
            tie_policy: TiePolicy::Stable,
            eligible: (0..1000)
                .map(|i| Candidate {
                    key: format!("user-{i:04}"),
                    weight: 1,
                })
                .collect(),
            winner: Some("user-0999".into()),
        };
        let record = ResultRecord {
            schema_version: 1,
            competition_id: "c".into(),
            generation: 1,
            ranking_id: "r".into(),
            ranking_revision: 1,
            finalized_at: 1,
            selection,
        };
        let reveal = record.reveal_snapshot().unwrap();
        assert_eq!(reveal.eligible_count, 1000);
        assert_eq!(reveal.selection.eligible.len(), 256);
        assert_eq!(reveal.other_weight, 744);
        assert!(reveal
            .selection
            .eligible
            .iter()
            .any(|entry| entry.key == "user-0999"));
        assert_eq!(record.selection.eligible.len(), 1000);
    }
    #[test]
    fn command_index_is_scoped_reusable_and_updates_atomically() {
        use crate::participants::CommandIndex;
        let option = |id: &str, command: &str| OptionDefinition {
            id: id.into(),
            label: id.into(),
            commands: vec![command.into()],
            gift_ids: vec![],
            image_url: None,
            on_win: None,
        };
        let mut definition = Definition {
            schema_version: 1,
            id: "a".into(),
            name: "Poll".into(),
            kind: CompetitionKind::Poll,
            ranking_id: "r".into(),
            options: vec![option("yes", "!Yes")],
            selection_mode: SelectionMode::HighestScore,
            tie_policy: TiePolicy::Stable,
            duration_ms: None,
            vote_limit: 1,
            allow_switch: false,
            minimum_participants: 1,
            target_score: None,
            on_user_win: None,
        };
        let mut index = CommandIndex::default();
        index.register(&definition, "creator", "room").unwrap();
        index
            .register_sources("a", "creator", "room", &["tiktok.like", "tiktok.share"])
            .unwrap();
        assert_eq!(
            index.matching_source("creator", "room", "tiktok.like"),
            vec!["a"]
        );
        assert!(index
            .matching_source("creator", "other", "tiktok.like")
            .is_empty());
        assert!(index
            .register_sources("a", "creator", "room", &["unknown.source"])
            .is_err());
        assert_eq!(
            index.matching_source("creator", "room", "tiktok.like"),
            vec!["a"]
        );
        definition.id = "b".into();
        index.register(&definition, "creator", "room").unwrap();
        let matches = index.matching("creator", "room", " !YES ");
        assert_eq!(
            matches
                .iter()
                .map(|r| r.competition_id.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "b"]
        );
        assert!(index.matching("creator", "other", "!yes").is_empty());
        assert!(index.matching("other", "room", "!yes").is_empty());
        assert!(index.matching("creator", "room", "!yes extra").is_empty());
        definition.options.push(option("duplicate", "!YES"));
        assert!(index.register(&definition, "creator", "room").is_err());
        assert_eq!(index.matching("creator", "room", "!yes").len(), 2);
        definition.options = (0..33)
            .map(|option_id| OptionDefinition {
                id: format!("o{option_id}"),
                label: format!("Option {option_id}"),
                commands: (0..64)
                    .map(|command| format!("!{option_id}-{command}"))
                    .collect(),
                gift_ids: vec![],
                image_url: None,
                on_win: None,
            })
            .collect();
        assert_eq!(
            index.register(&definition, "creator", "room"),
            Err(Error::Capacity)
        );
        assert_eq!(index.matching("creator", "room", "!yes").len(), 2);
        definition.options = vec![option("yes", "!Yes")];
        index.register(&definition, "creator", "new-room").unwrap();
        assert_eq!(index.matching("creator", "room", "!yes").len(), 1);
        assert_eq!(
            index.matching("creator", "new-room", "!yes")[0].competition_id,
            "b"
        );
        assert!(index.remove("a"));
        assert!(index
            .matching_source("creator", "room", "tiktok.like")
            .is_empty());
        assert!(index
            .matching_source("creator", "room", "tiktok.share")
            .is_empty());
        assert!(index.matching("creator", "room", "!yes").is_empty());
    }
    #[test]
    fn rejects_ambiguous_option_routes() {
        let option = |id: &str, command: &str| OptionDefinition {
            id: id.into(),
            label: id.into(),
            commands: vec![command.into()],
            gift_ids: vec![],
            image_url: None,
            on_win: None,
        };
        let mut definition = Definition {
            schema_version: 1,
            id: "poll".into(),
            name: "Poll".into(),
            kind: CompetitionKind::Poll,
            ranking_id: "r".into(),
            options: vec![option("a", "!Vote"), option("b", "!vote")],
            selection_mode: SelectionMode::HighestScore,
            tie_policy: TiePolicy::Stable,
            duration_ms: Some(1000),
            vote_limit: 1,
            allow_switch: false,
            minimum_participants: 1,
            target_score: None,
            on_user_win: None,
        };
        assert!(definition.validate().is_err());
        definition.options[1].commands[0] = "!other".into();
        definition.options[0].gift_ids.push("42".into());
        definition.options[1].gift_ids.push("42".into());
        assert!(definition.validate().is_err());
        definition.options[1].gift_ids.clear();
        assert!(definition.validate().is_ok());
    }
    #[test]
    fn durable_lifecycle_freezes_remaining_time_and_rejects_late_pause() {
        let mut state = LifecycleState::default();
        state.transition(Transition::Start, 100, Some(50)).unwrap();
        assert_eq!(state.generation, 1);
        assert_eq!(
            state.transition(Transition::Pause, 150, Some(50)),
            Err(Error::InvalidTransition)
        );
        state.transition(Transition::Pause, 120, Some(50)).unwrap();
        assert_eq!(state.remaining_ms, Some(30));
        state.transition(Transition::Resume, 500, Some(50)).unwrap();
        assert_eq!(state.deadline_ms, Some(530));
        assert_eq!(
            state.transition(Transition::Expire, 529, None),
            Err(Error::InvalidTransition)
        );
        state.transition(Transition::Expire, 530, None).unwrap();
        assert_eq!(state.lifecycle, Lifecycle::Finalizing);
        assert_eq!(
            state.transition(Transition::Cancel, 531, None),
            Err(Error::InvalidTransition)
        );
    }
    #[test]
    fn lifecycle_and_participation() {
        let mut c = Competition::new(["a".into(), "b".into()], 2, 2, true).unwrap();
        c.start(10, Some(100)).unwrap();
        c.vote(11, "u", "a").unwrap();
        c.vote(12, "u", "b").unwrap();
        c.vote(13, "u", "a").unwrap();
        assert_eq!(c.selected_option(13, "u"), Some("a"));
        assert_eq!(c.vote(14, "u", "a"), Err(Error::VoteLimit));
        c.pause(20).unwrap();
        assert_eq!(c.selected_option(21, "u"), None);
        c.resume(100).unwrap();
        assert!(!c.is_due(189));
        assert!(c.is_due(190));
        assert_eq!(c.selected_option(190, "u"), None);
        c.begin_finalizing().unwrap();
        c.finish().unwrap();
        assert_eq!(c.finish(), Err(Error::InvalidTransition));
    }
    #[test]
    fn manual_selection_accepts_only_tied_leaders() {
        let candidates = vec![
            Candidate {
                key: "b".into(),
                weight: 10,
            },
            Candidate {
                key: "a".into(),
                weight: 10,
            },
            Candidate {
                key: "c".into(),
                weight: 1,
            },
        ];
        assert_eq!(
            select_manual(candidates.clone(), "c"),
            Err(Error::InvalidCandidate)
        );
        assert_eq!(
            select_manual(candidates.clone(), "missing"),
            Err(Error::InvalidCandidate)
        );
        let selected = select_manual(candidates, "b").unwrap();
        assert_eq!(selected.winner.as_deref(), Some("b"));
        assert_eq!(selected.eligible[0].key, "a");
        selected.validate().unwrap();
        assert_eq!(
            select_manual(
                vec![Candidate {
                    key: "a".into(),
                    weight: 1
                }],
                "a"
            ),
            Err(Error::InvalidCandidate)
        );
    }
    #[test]
    fn weighted_draw_distribution_and_overflow() {
        let candidates = vec![
            Candidate {
                key: "a".into(),
                weight: 1,
            },
            Candidate {
                key: "b".into(),
                weight: 3,
            },
        ];
        let mut a = 0;
        for _ in 0..4000 {
            if select(
                candidates.clone(),
                SelectionMode::WeightedRandom,
                TiePolicy::Stable,
            )
            .unwrap()
            .winner
            .as_deref()
                == Some("a")
            {
                a += 1;
            }
        }
        // A broad interval catches gross bias without making sampling noise flaky.
        assert!((800..=1200).contains(&a), "observed {a} of 4000");
        assert_eq!(
            select(
                vec![
                    Candidate {
                        key: "a".into(),
                        weight: u64::MAX
                    },
                    Candidate {
                        key: "b".into(),
                        weight: 1
                    }
                ],
                SelectionMode::WeightedRandom,
                TiePolicy::Stable
            ),
            Err(Error::Overflow)
        );
    }
    #[test]
    fn selection_boundaries_and_ties() {
        let c = vec![
            Candidate {
                key: "b".into(),
                weight: 0,
            },
            Candidate {
                key: "a".into(),
                weight: 4,
            },
        ];
        for _ in 0..100 {
            assert_eq!(
                select(c.clone(), SelectionMode::WeightedRandom, TiePolicy::Stable)
                    .unwrap()
                    .winner
                    .as_deref(),
                Some("a")
            );
        }
        let tied = vec![
            Candidate {
                key: "b".into(),
                weight: 1,
            },
            Candidate {
                key: "a".into(),
                weight: 1,
            },
        ];
        assert_eq!(
            select(tied.clone(), SelectionMode::HighestScore, TiePolicy::Stable)
                .unwrap()
                .winner
                .as_deref(),
            Some("a")
        );
        assert_eq!(
            select(tied, SelectionMode::HighestScore, TiePolicy::Manual),
            Err(Error::ManualTie)
        );
        assert_eq!(
            select(
                vec![Candidate {
                    key: "a".into(),
                    weight: 0
                }],
                SelectionMode::WeightedRandom,
                TiePolicy::Stable
            ),
            Err(Error::ZeroWeight)
        );
    }
}
