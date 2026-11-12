//! Durable competition storage. Each operation owns its original SQLite
//! transaction; no event or external effect is dispatched from this layer.
//!
//! Definitions/lifecycle, delegated admission, participation, projections and
//! immutable result/action records are maintained in focused child modules.
use super::{DatabaseError, DatabaseManager};
use tiktools_competitions::{ActionClaim, Transition};

mod actions;
mod commands;
mod definitions;
mod delegation;
mod execution;
mod gifts;
mod integrity;
mod lifecycle;
mod projections;
mod remote;
mod results;
mod scored;

pub(super) use actions::validate_competition_winner_claim;

pub(crate) enum ActionClaimOutcome {
    Missing,
    Invalid { action_id: String },
    Claimed(ActionClaim),
}

/// A trusted snapshot of the generation and each current operator. It is
/// deliberately not deserializable and grants no effect by itself.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CompetitionExecutionFence {
    id: String,
    generation: u64,
    revision: i64,
    host_id: String,
    subjects: Vec<(String, String, String, String)>,
}
/// Proof that the competition transaction and current subject locks are held.
/// It cannot outlive the synchronous admission callback.
pub(crate) struct CompetitionExecutionAdmission<'a> {
    fence: &'a CompetitionExecutionFence,
    identities: &'a [crate::collaboration::ExecutionIdentity],
    winner: Option<&'a ActionClaim>,
}
impl CompetitionExecutionAdmission<'_> {
    pub(crate) fn fence(&self) -> &CompetitionExecutionFence {
        self.fence
    }
    pub(crate) fn identities(&self) -> &[crate::collaboration::ExecutionIdentity] {
        self.identities
    }
    pub(crate) fn winner(&self) -> Option<&ActionClaim> {
        self.winner
    }
}
impl CompetitionExecutionFence {
    pub(crate) fn competition_id(&self) -> &str {
        &self.id
    }
    pub(crate) fn host_id(&self) -> &str {
        &self.host_id
    }
    pub(crate) fn subjects(&self) -> Vec<(String, String)> {
        self.subjects
            .iter()
            .map(|(principal, session, _, _)| (principal.clone(), session.clone()))
            .collect()
    }
}

/// Revision-bound start/resume with retained delegated operators.
#[derive(Clone, Copy)]
pub(crate) enum CompetitionActivation {
    Start,
    Resume,
}
impl CompetitionActivation {
    fn method(self) -> &'static str {
        match self {
            Self::Start => "competitions.start",
            Self::Resume => "competitions.resume",
        }
    }
    fn transition(self) -> Transition {
        match self {
            Self::Start => Transition::Start,
            Self::Resume => Transition::Resume,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ScoredOutcome {
    Accepted { option_id: String },
    Duplicate,
    IdentityConflict,
    Unassigned,
    Closed,
    Capacity,
}

pub(crate) struct ScoredInput<'a> {
    pub creator_id: &'a str,
    pub room_id: &'a str,
    pub participant_id: &'a str,
    pub event_identity: &'a str,
    pub source: &'a str,
    pub score: u64,
    pub received_at: u64,
}

#[derive(Debug)]
pub(crate) struct GiftInput<'a> {
    pub creator_id: &'a str,
    pub room_id: &'a str,
    pub participant_id: &'a str,
    pub event_identity: &'a str,
    pub gift_id: &'a str,
    pub group_id: &'a str,
    pub observed_quantity: u64,
    pub price: u64,
    pub received_at: u64,
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum GiftOutcome {
    Accepted { option_id: String, score: u64 },
    Duplicate,
    IdentityConflict,
    Closed,
    Unassigned,
    Zero,
    Capacity,
}
