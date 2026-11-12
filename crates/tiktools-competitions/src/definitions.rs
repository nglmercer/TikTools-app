//! Competition definitions, validation and option routing rules.
use crate::{Error, SelectionMode, TiePolicy, WinnerAction};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

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
    /// Draws and challenges select a user; other competitions select an option.
    pub fn has_user_winner(&self) -> bool {
        matches!(
            self.kind,
            CompetitionKind::RandomDraw
                | CompetitionKind::WeightedDraw
                | CompetitionKind::LeaderboardChallenge
        )
    }

    pub fn winner_action(&self, winner: &str) -> Option<&WinnerAction> {
        if self.has_user_winner() {
            self.on_user_win.as_ref()
        } else {
            self.options
                .iter()
                .find(|option| option.id == winner)
                .and_then(|option| option.on_win.as_ref())
        }
    }

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
