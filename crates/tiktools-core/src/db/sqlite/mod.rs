//! SQLite schema and conservative persistence helpers.
//!
//! The SQL intentionally mirrors the current `src/db/points-db.ts` and
//! `src/db/automation-db.ts` tables. Domain payloads that have not yet gained
//! a Rust type stay as JSON so an existing database can be opened without a
//! destructive migration.

mod aggregations;
mod automation_library;
mod behavior;
pub(crate) use behavior::{
    BehaviorApprovalActivation, BehaviorApprovalBinding, BehaviorApprovalEnableState,
    BehaviorApprovalSnapshot,
};
pub(crate) use behavior::{BehaviorAuthority, BehaviorRuntimeAuthority, BehaviorSave};
mod commands;
mod delivery;
mod effects;
pub(crate) use delivery::{DeliveryToken, PendingDelivery};
mod competitions;
pub(crate) use commands::{CommandDatabase, CommandTransaction};
mod gift_accounting;
pub(crate) use competitions::{
    ActionClaimOutcome, GiftInput, GiftOutcome, ScoredInput, ScoredOutcome,
};
mod meta;
mod owner_adoption;
mod owner_approval;
pub(crate) use owner_approval::validate_owner_approval_input;
mod owner_definition;
mod owner_library;
pub(crate) use owner_definition::validate_owner_definition_input;
mod points;
mod schema;
#[cfg(test)]
mod tests;
mod util;
mod workflows;

use thiserror::Error;

pub(super) use super::DatabaseManager;

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("an owner authority transition is unresolved")]
    OwnerAdoptionPending,
    #[error("automation definitions changed; review the library again")]
    LibraryDefinitionConflict,
    #[error("points currency changed; review the current label")]
    CurrencyChanged,
    #[error("SQLite error: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("stored JSON is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("stored record is invalid: {0}")]
    Invalid(String),
    #[error("resource revision changed; reload before saving")]
    RevisionConflict(crate::collaboration::ResourceVersion),
    #[error("record does not exist")]
    NotFound,
    #[error("command ID was already used for a different operation")]
    CommandIdReused,
    #[error("command outcome expired; the historical outcome is unknown")]
    CommandExpired,
    #[error("host restarted; refresh the connection before sending a new command")]
    HostRestarted,
    #[error("command history capacity reached")]
    CommandCapacity,
    #[error("external effect history capacity reached; retained identities cannot be removed")]
    EffectCapacity,
    #[error("external effect identity was already used with different semantics")]
    EffectIdentityReused,
    #[error("external effect state or host instance changed; no replay is permitted")]
    EffectStateChanged,
    #[error("points writer is unavailable or has unresolved durability failures")]
    PointsWriterUnavailable,
    #[error("command ledger identity is ambiguous")]
    CommandLedgerInvalid,
    #[error("stored points state or outcome is invalid")]
    PointsStateInvalid,
    #[error("command authorization changed: {0}")]
    CommandAuthorization(#[source] crate::collaboration::Error),
    #[error("workflow snapshot exceeds its bounded budget")]
    SnapshotTooLarge,
}
