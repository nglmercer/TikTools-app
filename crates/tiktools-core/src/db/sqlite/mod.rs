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
#[cfg(test)]
pub(crate) use competitions::CompetitionActivation;
mod gift_accounting;
pub(crate) use competitions::{
    ActionClaimOutcome, GiftInput, GiftOutcome, ScoredInput, ScoredOutcome,
};
pub(crate) use competitions::{CompetitionExecutionAdmission, CompetitionExecutionFence};
mod meta;
mod owner_adoption;
mod owner_approval;
pub(crate) use owner_approval::validate_owner_approval_input;
mod owner_definition;
mod owner_library;
mod plugin_apply_quarantine;
#[cfg(feature = "cloud-auth")]
#[allow(dead_code)] // Native cutover preview is not yet exposed to the UI.
mod plugin_cutover;
pub(crate) use owner_definition::validate_owner_definition_input;
#[cfg(feature = "cloud-auth")]
pub(crate) use plugin_cutover::PluginCutoverSource;
#[allow(dead_code)] // Storage only, not yet wired to product package operations.
mod plugin_package_guard;
#[cfg(all(feature = "plugin-install", feature = "cloud-auth"))]
pub(crate) use plugin_package_guard::{
    PackageMutationIntent, PackageMutationKind, PackageMutationPhase,
};
#[cfg(feature = "cloud-auth")]
// The transactional storage boundary is not exposed to the product until the
// coordinator can enforce draining and identity/session authorization.
#[allow(dead_code)]
mod plugin_sync;
#[cfg(feature = "cloud-auth")]
// Storage only: product activation still needs coordinator/session fencing.
#[allow(dead_code)]
mod plugin_sync_apply;
mod points;
#[cfg(feature = "cloud-auth")]
pub(crate) use plugin_sync_apply::{PluginApplyIntent, PluginApplyPhase, PluginApplyReason};
#[cfg(feature = "cloud-auth")]
#[allow(dead_code)]
mod plugin_sync_rebase;
#[cfg(feature = "cloud-auth")]
pub(crate) use plugin_sync::{LocalPluginSyncIntent, PluginSyncOutboxEntry, UntrackedPluginReview};
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
