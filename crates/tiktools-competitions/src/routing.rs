//! Bounded in-memory participation and vote routing.
use crate::{Error, Lifecycle};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

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
