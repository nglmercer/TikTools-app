//! Read-only, source-fenced review of a product v1 conflict.
//!
//! A review is evidence for a later explicit decision, not authority to
//! rebase, discard, apply Cloud or send a write. In particular, the server
//! record must be fetched again when a decision is eventually committed.
use std::sync::Arc;

use cloud_client::{Edit, Entry, Pending, Record, Session, StateSnapshot, SyncError, SyncState};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    protocol_fence::V1Lease, SyncConfig, SyncCoordError, SyncCoordinator, V1ConflictSourceProof,
    V1LocalEditContext, V1OutboxCandidate, V1ProductOutbox,
};

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

    fn validate(&self, context: &V1LocalEditContext) -> Result<(), SyncCoordError> {
        let pending = self
            .before
            .pending
            .as_ref()
            .filter(|pending| pending.conflict)
            .ok_or(SyncCoordError::StaleReview)?;
        if self.format_version != 1
            || self.origin != context.origin
            || self.account_id != context.account_id
            || self.config_generation != context.config_generation
            || self.source_generation <= 0
            || self.key.is_empty()
            || self.before_generation == 0
            || self.before_generation.checked_add(1).is_none()
            || V1ConflictSourceProof::new(
                self.source_generation,
                self.initial_runtime_epoch.clone(),
            )
            .is_none()
            || !valid_digest(&self.before_digest)
            || !valid_digest(&self.after_digest)
        {
            return Err(SyncCoordError::StaleReview);
        }
        if self
            .cloud
            .as_ref()
            .is_some_and(|record| record.key != self.key)
        {
            return Err(SyncCoordError::StaleReview);
        }
        if let Some(cached) = self.before.remote.as_ref() {
            let Some(cloud) = self.cloud.as_ref() else {
                return Err(SyncCoordError::StaleReview);
            };
            if cloud.revision < cached.revision
                || (cloud.revision == cached.revision && cloud != cached)
            {
                return Err(SyncCoordError::StaleReview);
            }
        }
        if pending.base_revision.is_some_and(|revision| {
            self.cloud
                .as_ref()
                .is_none_or(|record| record.revision < revision)
        }) {
            return Err(SyncCoordError::StaleReview);
        }
        if desired_entry(&self.before, self.cloud.clone(), pending) != self.after {
            return Err(SyncCoordError::StaleReview);
        }
        Ok(())
    }
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn state_digest(state: &SyncState) -> Result<String, SyncCoordError> {
    let bytes = serde_json::to_vec(state).map_err(|_| SyncCoordError::StaleReview)?;
    Ok(Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn desired_entry(before: &Entry, cloud: Option<Record>, pending: &Pending) -> Entry {
    let next = if matches!(pending.edit, Edit::Delete) && cloud.is_none() {
        None
    } else {
        Some(Pending {
            base_revision: cloud.as_ref().map(|record| record.revision),
            edit: pending.edit.clone(),
            conflict: false,
        })
    };
    let mut result = before.clone();
    result.remote = cloud;
    result.pending = next;
    result
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RebasePhase {
    Before,
    After,
}

fn rebase_phase(
    intent: &V1KeepLocalIntent,
    snapshot: &StateSnapshot,
) -> Result<RebasePhase, SyncCoordError> {
    let entry = snapshot
        .state
        .entries
        .get(&intent.key)
        .ok_or(SyncCoordError::StaleReview)?;
    let digest = state_digest(&snapshot.state)?;
    if snapshot.generation == intent.before_generation
        && entry == &intent.before
        && digest == intent.before_digest
    {
        Ok(RebasePhase::Before)
    } else if snapshot.generation
        == intent
            .before_generation
            .checked_add(1)
            .ok_or(SyncCoordError::StaleReview)?
        && entry == &intent.after
        && digest == intent.after_digest
    {
        Ok(RebasePhase::After)
    } else {
        Err(SyncCoordError::StaleReview)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AbandonPhase {
    Before,
    After,
    Restored,
}

fn abandon_phase(
    intent: &V1KeepLocalIntent,
    snapshot: &StateSnapshot,
) -> Result<AbandonPhase, SyncCoordError> {
    match rebase_phase(intent, snapshot) {
        Ok(RebasePhase::Before) => return Ok(AbandonPhase::Before),
        Ok(RebasePhase::After) => return Ok(AbandonPhase::After),
        Err(SyncCoordError::StaleReview) => {}
        Err(error) => return Err(error),
    }
    // A crash can occur after restoring the frozen state but before the
    // journal is deleted. Generation +2 is the sole additional valid phase.
    if intent.before_generation.checked_add(2) == Some(snapshot.generation)
        && snapshot.state.entries.get(&intent.key) == Some(&intent.before)
        && state_digest(&snapshot.state)? == intent.before_digest
    {
        return Ok(AbandonPhase::Restored);
    }
    Err(SyncCoordError::StaleReview)
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

impl SyncCoordinator {
    fn capture_product_conflict_locked(
        &self,
        lease: &V1Lease,
        session: &Arc<Session>,
        config: SyncConfig,
        key: &str,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<CapturedConflict, SyncCoordError> {
        let origin = session.instance_origin();
        let account = session.account_id();
        lease.ensure_v1(&origin, account)?;
        lease.ensure_no_uncertain(&origin, account)?;
        let snapshot = self.load_for_identity(lease, &origin, account)?;
        let pending = snapshot
            .state
            .entries
            .get(key)
            .and_then(|entry| entry.pending.as_ref())
            .filter(|pending| pending.conflict)
            .ok_or(SyncCoordError::StaleReview)?;
        let context = V1LocalEditContext {
            origin,
            account_id: account.to_owned(),
            config_generation: config.generation,
        };
        if outbox.has_pending_rebase(&context)? {
            return Err(SyncCoordError::StaleReview);
        }
        let source = outbox
            .conflict_source_proof(&context, key, pending)?
            .ok_or(SyncCoordError::StaleReview)?;
        Ok(CapturedConflict { snapshot, source })
    }

    fn capture_product_conflict(
        &self,
        session: &Arc<Session>,
        config: SyncConfig,
        key: &str,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<CapturedConflict, SyncCoordError> {
        let lease = self.protocol.try_v1()?;
        let _admission = lease.try_admission()?;
        self.with_live_session(session, config, || {
            self.capture_product_conflict_locked(&lease, session, config, key, outbox)
        })
    }

    /// Capture a conflict only after validating its durable source, fetch a
    /// fresh Cloud snapshot without holding the admission lock, then capture
    /// again. Any local change, session switch or opt-out invalidates it.
    pub async fn review_product_conflict(
        &self,
        key: &str,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<V1ProductConflictReview, SyncCoordError> {
        let config = self.read_config();
        if !config.enabled {
            return Err(SyncCoordError::Disabled);
        }
        let session = self
            .auth
            .session()
            .await
            .ok_or(SyncCoordError::NotSignedIn)?;
        let before = self.capture_product_conflict(&session, config, key, outbox)?;
        let cloud = session
            .export_sync()
            .await?
            .records
            .into_iter()
            .find(|record| record.key == key);
        let after = self.capture_product_conflict(&session, config, key, outbox)?;
        if before != after {
            return Err(SyncCoordError::StaleReview);
        }
        let entry = &after.snapshot.state.entries[key];
        if let Some(base) = entry.remote.as_ref() {
            let Some(current) = cloud.as_ref() else {
                return Err(SyncError::Protocol.into());
            };
            if current.revision < base.revision
                || (current.revision == base.revision && current != base)
            {
                return Err(SyncError::Protocol.into());
            }
        }
        if entry
            .pending
            .as_ref()
            .and_then(|pending| pending.base_revision)
            .is_some_and(|base| cloud.as_ref().is_none_or(|current| current.revision < base))
        {
            return Err(SyncError::Protocol.into());
        }
        Ok(V1ProductConflictReview {
            binding: self.recovery_binding.clone(),
            session,
            config,
            key: key.to_owned(),
            captured: after,
            cloud,
        })
    }

    /// Check only that the local inputs of a prior review remain unchanged.
    /// A later decision must also re-fetch Cloud and use a crash-safe journal.
    pub fn product_conflict_review_is_current(
        &self,
        review: &V1ProductConflictReview,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<bool, SyncCoordError> {
        if !Arc::ptr_eq(&review.binding, &self.recovery_binding) {
            return Ok(false);
        }
        match self.capture_product_conflict(&review.session, review.config, &review.key, outbox) {
            Ok(current) => Ok(current == review.captured),
            Err(SyncCoordError::StaleReview) => Ok(false),
            Err(error) => Err(error),
        }
    }

    /// Commit a separately confirmed keep-local decision. A new Cloud read
    /// must exactly match the reviewed value; the durable journal is created
    /// before JSON or its receipt changes and survives every later failure.
    pub async fn keep_local_product_conflict(
        &self,
        review: &V1ProductConflictReview,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<(), SyncCoordError> {
        if !self.product_conflict_review_is_current(review, outbox)? {
            return Err(SyncCoordError::StaleReview);
        }
        let cloud = review
            .session
            .export_sync()
            .await?
            .records
            .into_iter()
            .find(|record| record.key == review.key);
        if cloud != review.cloud {
            return Err(SyncCoordError::StaleReview);
        }
        let lease = self.protocol.try_v1()?;
        let _admission = lease.try_admission()?;
        self.with_live_session(&review.session, review.config, || {
            let captured = self.capture_product_conflict_locked(
                &lease,
                &review.session,
                review.config,
                &review.key,
                outbox,
            )?;
            if captured != review.captured {
                return Err(SyncCoordError::StaleReview);
            }
            let context = V1LocalEditContext {
                origin: review.session.instance_origin(),
                account_id: review.session.account_id().to_owned(),
                config_generation: review.config.generation,
            };
            let before = captured
                .snapshot
                .state
                .entries
                .get(&review.key)
                .cloned()
                .ok_or(SyncCoordError::StaleReview)?;
            let pending = before.pending.as_ref().ok_or(SyncCoordError::StaleReview)?;
            let after = desired_entry(&before, cloud.clone(), pending);
            let mut next_state = captured.snapshot.state.clone();
            next_state.entries.insert(review.key.clone(), after.clone());
            let intent = V1KeepLocalIntent {
                format_version: 1,
                origin: context.origin.clone(),
                account_id: context.account_id.clone(),
                config_generation: context.config_generation,
                source_generation: captured.source.generation(),
                initial_runtime_epoch: captured.source.runtime_epoch().to_owned(),
                key: review.key.clone(),
                before_generation: captured.snapshot.generation,
                before,
                cloud: cloud.clone(),
                after,
                before_digest: state_digest(&captured.snapshot.state)?,
                after_digest: state_digest(&next_state)?,
            };
            intent.validate(&context)?;
            if !outbox.prepare_rebase(&context, &intent, &captured.source)? {
                return Err(SyncCoordError::StaleReview);
            }
            self.finish_keep_local_locked(&lease, &context, &intent, &captured.source, outbox)
        })
    }

    fn finish_keep_local_locked(
        &self,
        lease: &V1Lease,
        context: &V1LocalEditContext,
        intent: &V1KeepLocalIntent,
        proof: &V1ConflictSourceProof,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<(), SyncCoordError> {
        intent.validate(context)?;
        if proof.generation() != intent.source_generation {
            return Err(SyncCoordError::StaleReview);
        }
        lease.ensure_v1(&context.origin, &context.account_id)?;
        lease.ensure_no_uncertain(&context.origin, &context.account_id)?;
        let mut snapshot = self.load_for_identity(lease, &context.origin, &context.account_id)?;
        if rebase_phase(intent, &snapshot)? == RebasePhase::Before {
            snapshot
                .state
                .entries
                .insert(intent.key.clone(), intent.after.clone());
            if state_digest(&snapshot.state)? != intent.after_digest {
                return Err(SyncCoordError::StaleReview);
            }
            snapshot = self.save_state(snapshot, lease)?;
        }
        if rebase_phase(intent, &snapshot)? != RebasePhase::After {
            return Err(SyncCoordError::StaleReview);
        }
        if !outbox.finish_rebase(context, intent, proof)? {
            return Err(SyncCoordError::StaleReview);
        }
        Ok(())
    }

    fn capture_keep_local_recovery_locked(
        &self,
        lease: &V1Lease,
        session: &Arc<Session>,
        config: SyncConfig,
        key: &str,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<(V1KeepLocalIntent, V1ConflictSourceProof, RebasePhase), SyncCoordError> {
        let context = V1LocalEditContext {
            origin: session.instance_origin(),
            account_id: session.account_id().to_owned(),
            config_generation: config.generation,
        };
        lease.ensure_v1(&context.origin, &context.account_id)?;
        lease.ensure_no_uncertain(&context.origin, &context.account_id)?;
        let intent = outbox
            .load_rebase(&context, key)?
            .ok_or(SyncCoordError::StaleReview)?;
        intent.validate(&context)?;
        if intent.key != key {
            return Err(SyncCoordError::StaleReview);
        }
        let proof = outbox
            .rebase_source_proof(&context, &intent)?
            .ok_or(SyncCoordError::StaleReview)?;
        if proof.generation() != intent.source_generation {
            return Err(SyncCoordError::StaleReview);
        }
        let snapshot = self.load_for_identity(lease, &context.origin, &context.account_id)?;
        let phase = rebase_phase(&intent, &snapshot)?;
        Ok((intent, proof, phase))
    }

    fn capture_keep_local_recovery(
        &self,
        session: &Arc<Session>,
        config: SyncConfig,
        key: &str,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<(V1KeepLocalIntent, V1ConflictSourceProof, RebasePhase), SyncCoordError> {
        let lease = self.protocol.try_v1()?;
        let _admission = lease.try_admission()?;
        self.with_live_session(session, config, || {
            self.capture_keep_local_recovery_locked(&lease, session, config, key, outbox)
        })
    }

    /// Inspect a saved decision after interruption. This rechecks current
    /// Cloud and source and returns an opaque native review; it never resumes
    /// the decision automatically. If a newer local edit changed the source,
    /// the journal and edit remain intact and sync stays blocked. The
    /// separate explicit abandonment flow restores the original conflict
    /// before retiring the journal; deleting it alone is unsafe.
    pub async fn review_keep_local_recovery(
        &self,
        key: &str,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<V1KeepLocalRecoveryReview, SyncCoordError> {
        let config = self.read_config();
        if !config.enabled {
            return Err(SyncCoordError::Disabled);
        }
        let session = self
            .auth
            .session()
            .await
            .ok_or(SyncCoordError::NotSignedIn)?;
        let before = self.capture_keep_local_recovery(&session, config, key, outbox)?;
        let cloud = session
            .export_sync()
            .await?
            .records
            .into_iter()
            .find(|record| record.key == key);
        if cloud != before.0.cloud {
            return Err(SyncCoordError::StaleReview);
        }
        let after = self.capture_keep_local_recovery(&session, config, key, outbox)?;
        if before != after {
            return Err(SyncCoordError::StaleReview);
        }
        Ok(V1KeepLocalRecoveryReview {
            binding: self.recovery_binding.clone(),
            session,
            config,
            intent: after.0,
            proof: after.1,
            phase: after.2,
        })
    }

    /// Resume only after a new explicit recovery review. Re-fetch Cloud and
    /// compare all frozen local inputs again; no remote write is sent here.
    pub async fn resume_keep_local_recovery(
        &self,
        review: &V1KeepLocalRecoveryReview,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<(), SyncCoordError> {
        if !Arc::ptr_eq(&review.binding, &self.recovery_binding) {
            return Err(SyncCoordError::StaleReview);
        }
        let before = self.capture_keep_local_recovery(
            &review.session,
            review.config,
            &review.intent.key,
            outbox,
        )?;
        if before != (review.intent.clone(), review.proof.clone(), review.phase) {
            return Err(SyncCoordError::StaleReview);
        }
        let cloud = review
            .session
            .export_sync()
            .await?
            .records
            .into_iter()
            .find(|record| record.key == review.intent.key);
        if cloud != review.intent.cloud {
            return Err(SyncCoordError::StaleReview);
        }
        let lease = self.protocol.try_v1()?;
        let _admission = lease.try_admission()?;
        self.with_live_session(&review.session, review.config, || {
            let current = self.capture_keep_local_recovery_locked(
                &lease,
                &review.session,
                review.config,
                &review.intent.key,
                outbox,
            )?;
            if current != (review.intent.clone(), review.proof.clone(), review.phase) {
                return Err(SyncCoordError::StaleReview);
            }
            let context = V1LocalEditContext {
                origin: review.session.instance_origin(),
                account_id: review.session.account_id().to_owned(),
                config_generation: review.config.generation,
            };
            self.finish_keep_local_locked(&lease, &context, &review.intent, &review.proof, outbox)
        })
    }

    fn capture_keep_local_abandon_locked(
        &self,
        lease: &V1Lease,
        session: &Arc<Session>,
        config: SyncConfig,
        key: &str,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<CapturedAbandon, SyncCoordError> {
        let context = V1LocalEditContext {
            origin: session.instance_origin(),
            account_id: session.account_id().to_owned(),
            config_generation: config.generation,
        };
        lease.ensure_v1(&context.origin, &context.account_id)?;
        lease.ensure_no_uncertain(&context.origin, &context.account_id)?;
        let intent = outbox
            .load_rebase(&context, key)?
            .ok_or(SyncCoordError::StaleReview)?;
        intent.validate(&context)?;
        if intent.key != key {
            return Err(SyncCoordError::StaleReview);
        }
        let (candidate, proof) = outbox
            .newer_rebase_candidate(&context, &intent)?
            .ok_or(SyncCoordError::StaleReview)?;
        if candidate.key != key
            || candidate.generation <= intent.source_generation
            || proof.generation() != candidate.generation
        {
            return Err(SyncCoordError::StaleReview);
        }
        let snapshot = self.load_for_identity(lease, &context.origin, &context.account_id)?;
        let phase = abandon_phase(&intent, &snapshot)?;
        Ok(CapturedAbandon {
            intent,
            candidate,
            proof,
            phase,
        })
    }

    fn capture_keep_local_abandon(
        &self,
        session: &Arc<Session>,
        config: SyncConfig,
        key: &str,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<CapturedAbandon, SyncCoordError> {
        let lease = self.protocol.try_v1()?;
        let _admission = lease.try_admission()?;
        self.with_live_session(session, config, || {
            self.capture_keep_local_abandon_locked(&lease, session, config, key, outbox)
        })
    }

    fn restore_conflict_for_abandon(
        &self,
        lease: &V1Lease,
        context: &V1LocalEditContext,
        intent: &V1KeepLocalIntent,
        phase: AbandonPhase,
    ) -> Result<(), SyncCoordError> {
        if phase != AbandonPhase::After {
            return Ok(());
        }
        let mut snapshot = self.load_for_identity(lease, &context.origin, &context.account_id)?;
        if abandon_phase(intent, &snapshot)? != AbandonPhase::After {
            return Err(SyncCoordError::StaleReview);
        }
        snapshot
            .state
            .entries
            .insert(intent.key.clone(), intent.before.clone());
        if state_digest(&snapshot.state)? != intent.before_digest {
            return Err(SyncCoordError::StaleReview);
        }
        let saved = self.save_state(snapshot, lease)?;
        if abandon_phase(intent, &saved)? != AbandonPhase::Restored {
            return Err(SyncCoordError::StaleReview);
        }
        Ok(())
    }

    /// Review abandonment of a superseded decision. This captures an exact
    /// source-bound newer candidate and the frozen local state. No Cloud
    /// operation is made: abandonment restores a conflict, not a resolution.
    pub async fn review_keep_local_abandon(
        &self,
        key: &str,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<V1KeepLocalAbandonReview, SyncCoordError> {
        let config = self.read_config();
        if !config.enabled {
            return Err(SyncCoordError::Disabled);
        }
        let session = self
            .auth
            .session()
            .await
            .ok_or(SyncCoordError::NotSignedIn)?;
        let captured = self.capture_keep_local_abandon(&session, config, key, outbox)?;
        Ok(V1KeepLocalAbandonReview {
            binding: self.recovery_binding.clone(),
            session,
            config,
            captured,
        })
    }

    /// Confirm abandoning the saved decision. Restore the exact original
    /// conflict before clearing its journal. A crash at either step leaves
    /// one of the recognized phases and requires a fresh explicit review.
    pub fn abandon_keep_local_recovery(
        &self,
        review: &V1KeepLocalAbandonReview,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<(), SyncCoordError> {
        if !Arc::ptr_eq(&review.binding, &self.recovery_binding) {
            return Err(SyncCoordError::StaleReview);
        }
        let lease = self.protocol.try_v1()?;
        let _admission = lease.try_admission()?;
        self.with_live_session(&review.session, review.config, || {
            let captured = self.capture_keep_local_abandon_locked(
                &lease,
                &review.session,
                review.config,
                &review.captured.intent.key,
                outbox,
            )?;
            if captured != review.captured {
                return Err(SyncCoordError::StaleReview);
            }
            let intent = &captured.intent;
            let context = V1LocalEditContext {
                origin: review.session.instance_origin(),
                account_id: review.session.account_id().to_owned(),
                config_generation: review.config.generation,
            };
            self.restore_conflict_for_abandon(&lease, &context, intent, captured.phase)?;
            if !outbox.abandon_rebase(&context, intent, &captured.candidate, &captured.proof)? {
                return Err(SyncCoordError::StaleReview);
            }
            Ok(())
        })
    }

    fn capture_superseded_rebase_locked(
        &self,
        lease: &V1Lease,
        session: &Arc<Session>,
        config: SyncConfig,
        key: &str,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<CapturedSupersededRebase, SyncCoordError> {
        let context = V1LocalEditContext {
            origin: session.instance_origin(),
            account_id: session.account_id().to_owned(),
            config_generation: config.generation,
        };
        lease.ensure_v1(&context.origin, &context.account_id)?;
        lease.ensure_no_uncertain(&context.origin, &context.account_id)?;
        let intent = outbox
            .load_rebase(&context, key)?
            .ok_or(SyncCoordError::StaleReview)?;
        intent.validate(&context)?;
        if intent.key != key {
            return Err(SyncCoordError::StaleReview);
        }
        let generation = outbox
            .superseded_rebase_generation(&context, &intent)?
            .filter(|generation| *generation > intent.source_generation)
            .ok_or(SyncCoordError::StaleReview)?;
        let snapshot = self.load_for_identity(lease, &context.origin, &context.account_id)?;
        let phase = abandon_phase(&intent, &snapshot)?;
        Ok(CapturedSupersededRebase {
            intent,
            generation,
            phase,
        })
    }

    /// Review a superseded journal without retrieving the newer local value.
    /// No network request is needed: the only permitted result is the old
    /// conflict, which still requires separate review before any upload.
    pub async fn review_superseded_rebase(
        &self,
        key: &str,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<V1SupersededRebaseReview, SyncCoordError> {
        let config = self.read_config();
        if !config.enabled {
            return Err(SyncCoordError::Disabled);
        }
        let session = self
            .auth
            .session()
            .await
            .ok_or(SyncCoordError::NotSignedIn)?;
        let lease = self.protocol.try_v1()?;
        let _admission = lease.try_admission()?;
        let captured = self.with_live_session(&session, config, || {
            self.capture_superseded_rebase_locked(&lease, &session, config, key, outbox)
        })?;
        Ok(V1SupersededRebaseReview {
            binding: self.recovery_binding.clone(),
            session,
            config,
            captured,
        })
    }

    /// Restore the exact prior conflict before retiring the journal. The
    /// backend must atomically recheck the captured source generation when
    /// removing the journal. A failure retains a replayable recovery phase.
    pub fn abandon_superseded_rebase(
        &self,
        review: &V1SupersededRebaseReview,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<(), SyncCoordError> {
        if !Arc::ptr_eq(&review.binding, &self.recovery_binding) {
            return Err(SyncCoordError::StaleReview);
        }
        let lease = self.protocol.try_v1()?;
        let _admission = lease.try_admission()?;
        self.with_live_session(&review.session, review.config, || {
            let captured = self.capture_superseded_rebase_locked(
                &lease,
                &review.session,
                review.config,
                &review.captured.intent.key,
                outbox,
            )?;
            if captured != review.captured {
                return Err(SyncCoordError::StaleReview);
            }
            let context = V1LocalEditContext {
                origin: review.session.instance_origin(),
                account_id: review.session.account_id().to_owned(),
                config_generation: review.config.generation,
            };
            self.restore_conflict_for_abandon(&lease, &context, &captured.intent, captured.phase)?;
            if !outbox.abandon_superseded_rebase(&context, &captured.intent, captured.generation)? {
                return Err(SyncCoordError::StaleReview);
            }
            Ok(())
        })
    }
}
