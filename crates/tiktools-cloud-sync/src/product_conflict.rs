//! Source-fenced product v1 conflict reviews and journaled local decisions.
//!
//! A review is evidence for a later explicit decision, not authority to
//! rebase, discard, apply Cloud or send a write. In particular, the server
//! record must be fetched again when a decision is eventually committed.
use std::sync::Arc;

use cloud_client::{Entry, Pending, Record, Session, StateSnapshot};
use serde::{Deserialize, Serialize};

use crate::{SyncConfig, V1ConflictSourceProof, V1OutboxCandidate};

mod abandon;
mod intent;
mod keep_local;
mod previous_consent;
mod recovery;
mod review;
mod superseded;

struct CapturedConflict {
    snapshot: StateSnapshot,
    source: V1ConflictSourceProof,
}

impl PartialEq for CapturedConflict {
    fn eq(&self, other: &Self) -> bool {
        self.snapshot.generation == other.snapshot.generation
            && self.snapshot.state == other.snapshot.state
            && self.source == other.source
    }
}
impl Eq for CapturedConflict {}

/// Frozen native decision persisted before either the v1 state or its SQLite
/// receipt changes. Never construct this from a webview or remote caller.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct V1KeepLocalIntent {
    format_version: u8,
    origin: String,
    account_id: String,
    #[serde(with = "crate::consent_generation")]
    config_generation: u128,
    source_generation: i64,
    initial_runtime_epoch: String,
    key: String,
    before_generation: u64,
    before: Entry,
    cloud: Option<Record>,
    after: Entry,
    before_digest: String,
    after_digest: String,
}

impl V1KeepLocalIntent {
    pub fn origin(&self) -> &str {
        &self.origin
    }
    pub fn account_id(&self) -> &str {
        &self.account_id
    }
    pub fn config_generation(&self) -> u128 {
        self.config_generation
    }
    pub fn source_generation(&self) -> i64 {
        self.source_generation
    }
    pub fn initial_runtime_epoch(&self) -> &str {
        &self.initial_runtime_epoch
    }
    pub fn key(&self) -> &str {
        &self.key
    }
    pub fn before_generation(&self) -> u64 {
        self.before_generation
    }
    pub fn before(&self) -> &Entry {
        &self.before
    }
    pub fn cloud(&self) -> Option<&Record> {
        self.cloud.as_ref()
    }
    pub fn after(&self) -> &Entry {
        &self.after
    }
    pub fn before_digest(&self) -> &str {
        &self.before_digest
    }
    pub fn after_digest(&self) -> &str {
        &self.after_digest
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RebasePhase {
    Before,
    After,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AbandonPhase {
    Before,
    After,
    Restored,
}

#[derive(Clone, PartialEq, Eq)]
struct CapturedAbandon {
    intent: V1KeepLocalIntent,
    candidate: V1OutboxCandidate,
    proof: V1ConflictSourceProof,
    phase: AbandonPhase,
}

/// Opaque review of an interrupted decision superseded by a newer local edit.
/// Abandoning restores the original conflict; it does not resolve it or
/// authorize a Cloud write.
pub struct V1KeepLocalAbandonReview {
    binding: Arc<()>,
    session: Arc<Session>,
    config: SyncConfig,
    captured: CapturedAbandon,
}

impl V1KeepLocalAbandonReview {
    pub fn intent(&self) -> &V1KeepLocalIntent {
        &self.captured.intent
    }

    pub fn candidate(&self) -> &V1OutboxCandidate {
        &self.captured.candidate
    }
}

#[derive(Clone, PartialEq, Eq)]
struct CapturedSupersededRebase {
    intent: V1KeepLocalIntent,
    generation: i64,
    phase: AbandonPhase,
}

/// Review of an interrupted decision whose local source has changed. This
/// never exposes a newer edit belonging to any other account; abandonment
/// restores the original conflict and does not authorize Cloud or runtime
/// effects. Product projections must recheck the captured session.
pub struct V1SupersededRebaseReview {
    binding: Arc<()>,
    session: Arc<Session>,
    config: SyncConfig,
    captured: CapturedSupersededRebase,
}

impl V1SupersededRebaseReview {
    pub fn intent(&self) -> &V1KeepLocalIntent {
        &self.captured.intent
    }

    pub fn source_generation(&self) -> i64 {
        self.captured.generation
    }
}

/// Explicit review to undo a decision saved under an earlier consent epoch.
/// This restores the original conflict, without renewing any upload authority.
pub struct V1PreviousConsentRebaseReview {
    binding: Arc<()>,
    session: Arc<Session>,
    config: SyncConfig,
    intent: V1KeepLocalIntent,
    phase: AbandonPhase,
}

impl V1PreviousConsentRebaseReview {
    /// Product projections must recheck the captured session before display.
    pub fn intent(&self) -> &V1KeepLocalIntent {
        &self.intent
    }
}

/// A new explicit review after interruption. The original intent is retained
/// until the JSON state and receipt are reconciled under a current session.
pub struct V1KeepLocalRecoveryReview {
    binding: Arc<()>,
    session: Arc<Session>,
    config: SyncConfig,
    intent: V1KeepLocalIntent,
    proof: V1ConflictSourceProof,
    phase: RebasePhase,
}

impl V1KeepLocalRecoveryReview {
    /// Product code must guard any projection against session changes.
    pub fn intent(&self) -> &V1KeepLocalIntent {
        &self.intent
    }
}

/// Opaque, native-only review bound to one coordinator, exact session,
/// consent epoch, local sync snapshot and durable product-source proof.
/// Product code must guard any projection against session transitions; a
/// successful capture alone cannot authorize exposing it after logout.
pub struct V1ProductConflictReview {
    binding: Arc<()>,
    session: Arc<Session>,
    config: SyncConfig,
    key: String,
    captured: CapturedConflict,
    cloud: Option<Record>,
}

impl V1ProductConflictReview {
    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn pending(&self) -> &Pending {
        self.captured.snapshot.state.entries[&self.key]
            .pending
            .as_ref()
            .expect("a review always captures a pending conflict")
    }

    /// Last cached Cloud record; v1 does not retain the original base body.
    pub fn cached_cloud(&self) -> Option<&Record> {
        self.captured.snapshot.state.entries[&self.key]
            .remote
            .as_ref()
    }

    pub fn cloud(&self) -> Option<&Record> {
        self.cloud.as_ref()
    }

    pub fn source_generation(&self) -> i64 {
        self.captured.source.generation()
    }
}
