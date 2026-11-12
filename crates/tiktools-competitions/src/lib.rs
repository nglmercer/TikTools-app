//! Headless competition state machine. The host owns clocks and persistence;
//! callers must persist a frozen result before publishing or dispatching it.
mod actions;
mod definitions;
mod lifecycle;
pub mod participants;
mod results;
mod routing;
mod selection;

pub use actions::{ActionClaim, ActionExecution, ActionResolution, ActionStatus, WinnerAction};
pub use definitions::{CompetitionKind, Definition, OptionDefinition};
pub use lifecycle::{Lifecycle, LifecycleState, Transition};
pub use results::{ResultRecord, RevealSelection, RevealSnapshot};
pub use routing::{CommandParticipation, Competition, ParticipationOutcome};
pub use selection::{select, select_manual, Candidate, Selection, SelectionMode, TiePolicy};

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

#[cfg(test)]
mod tests;
