//! Durable product candidates admitted into the legacy sync engine.
//!
//! The product owns its source transaction and must use the same admission
//! lease through `with_v1_local_edit`. This module never reads SQLite or
//! authorizes a value merely because a caller supplied it.
use std::{collections::BTreeSet, sync::Arc};

use cloud_client::{Edit, Pending, Session, StateSnapshot};

use crate::{
    protocol_fence::{UncertainUpload, V1Lease},
    SyncConfig, SyncCoordError, SyncCoordinator, V1LocalEditContext,
};

/// One native candidate; `generation` identifies the source transaction.
/// The product must compare identity, consent, runtime epoch, source
/// generation and the exact edit both when returning and acknowledging it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct V1OutboxCandidate {
    pub key: String,
    pub edit: Edit,
    pub generation: i64,
}

/// Native-only evidence that a specific source generation and host lifetime
/// still authorize a reviewed conflict. Implementations must derive this from
/// their durable receipt and current source, never from a UI-supplied value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct V1ConflictSourceProof {
    generation: i64,
    runtime_epoch: String,
}

impl V1ConflictSourceProof {
    pub fn new(generation: i64, runtime_epoch: String) -> Option<Self> {
        (generation > 0
            && runtime_epoch.len() == 64
            && runtime_epoch
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)))
        .then_some(Self {
            generation,
            runtime_epoch,
        })
    }

    pub fn generation(&self) -> i64 {
        self.generation
    }

    pub fn runtime_epoch(&self) -> &str {
        &self.runtime_epoch
    }
}

/// Native-only bridge to a product's transactional outbox. Implementations
/// must perform only short synchronous local I/O and must not re-enter the
/// coordinator or authentication manager. Unconfirmed/stale entries must
/// return an error rather than being skipped. A failed drain may leave an
/// adapter's in-memory observation unresolved; callers must discard that
/// adapter and construct a fresh one before retrying.
pub trait V1ProductOutbox: Send {
    /// Enter the native runtime/invocation gate before the coordinator takes
    /// its cross-process admission lease, and invoke `operation` synchronously
    /// exactly once while that gate is held. The guarded adapter must reject
    /// candidates whose observed runtime and exact source no longer agree.
    /// The coordinator releases the gate before any network await. Legacy
    /// test adapters have no such proof and fail closed by default.
    fn with_runtime_guard(
        &mut self,
        _operation: &mut dyn FnMut(&mut dyn V1ProductOutbox) -> Result<(), SyncCoordError>,
    ) -> Result<(), SyncCoordError> {
        Err(SyncCoordError::Local(
            "guarded plugin runtime observation is unavailable".into(),
        ))
    }

    fn next(
        &mut self,
        context: &V1LocalEditContext,
    ) -> Result<Option<V1OutboxCandidate>, SyncCoordError>;

    /// Atomically replace the outbox candidate with a durable receipt only
    /// after the candidate was staged. A receipt must retain identity, config
    /// generation, source generation, exact pending edit/base/conflict and
    /// runtime confirmation. Conflict resolution must update the receipt
    /// only after an explicit source-fenced review.
    fn acknowledge(
        &mut self,
        context: &V1LocalEditContext,
        candidate: &V1OutboxCandidate,
        staged: &Pending,
    ) -> Result<bool, SyncCoordError>;

    /// Revalidate the durable receipt against the current global local source
    /// generation and this host's runtime confirmation, while the admission
    /// lease and current session are held. A missing receipt (including a
    /// legacy pending edit) must fail closed until explicitly reviewed.
    fn validate_pending(
        &mut self,
        context: &V1LocalEditContext,
        key: &str,
        pending: &Pending,
    ) -> Result<bool, SyncCoordError>;

    /// Return native source evidence for this exact conflict. The receipt
    /// may contain `conflict=false` only when the current pending changed to
    /// `true` after a server 409; edit and base revision must remain exact.
    /// A new local candidate, stale source, changed consent or unconfirmed
    /// runtime must fail closed. The default deliberately grants no review.
    fn conflict_source_proof(
        &mut self,
        _context: &V1LocalEditContext,
        _key: &str,
        _pending: &Pending,
    ) -> Result<Option<V1ConflictSourceProof>, SyncCoordError> {
        Ok(None)
    }

    /// A pending resolution blocks all product uploads for this identity.
    /// Implementations must fail closed if their journal cannot be read.
    fn has_pending_rebase(
        &mut self,
        _context: &V1LocalEditContext,
    ) -> Result<bool, SyncCoordError> {
        Err(SyncCoordError::Local(
            "conflict recovery is unavailable".into(),
        ))
    }

    /// Persist the frozen user decision without changing the receipt. This
    /// must be atomic with the source/consent/runtime CAS and no-clobber any
    /// unresolved journal for this identity.
    fn prepare_rebase(
        &mut self,
        _context: &V1LocalEditContext,
        _intent: &crate::V1KeepLocalIntent,
        _proof: &V1ConflictSourceProof,
    ) -> Result<bool, SyncCoordError> {
        Ok(false)
    }

    /// Load an exact durable intent; absent, unsafe or mismatched identities
    /// must never yield another account's saved values.
    fn load_rebase(
        &mut self,
        _context: &V1LocalEditContext,
        _key: &str,
    ) -> Result<Option<crate::V1KeepLocalIntent>, SyncCoordError> {
        Ok(None)
    }

    /// Load an interrupted decision for the same origin and account after
    /// consent changed. This only supports explicitly abandoning the old
    /// decision; it must not renew its receipt or authorize an upload.
    fn load_rebase_with_previous_consent(
        &mut self,
        _context: &V1LocalEditContext,
        _key: &str,
    ) -> Result<Option<crate::V1KeepLocalIntent>, SyncCoordError> {
        Ok(None)
    }

    /// Remove only the exact previously reviewed journal after the original
    /// conflict has been durably restored. Preserve receipts and outboxes.
    fn abandon_rebase_with_previous_consent(
        &mut self,
        _context: &V1LocalEditContext,
        _intent: &crate::V1KeepLocalIntent,
    ) -> Result<bool, SyncCoordError> {
        Ok(false)
    }

    /// Revalidate source/consent/runtime while accepting only the old or new
    /// exact receipt state. A new host may use a fresh runtime proof only
    /// after native reconciliation and a new explicit user review.
    fn rebase_source_proof(
        &mut self,
        _context: &V1LocalEditContext,
        _intent: &crate::V1KeepLocalIntent,
    ) -> Result<Option<V1ConflictSourceProof>, SyncCoordError> {
        Ok(None)
    }

    /// After JSON v1 matches the frozen result, update/delete the receipt and
    /// delete the same journal in one durable SQLite transaction.
    fn finish_rebase(
        &mut self,
        _context: &V1LocalEditContext,
        _intent: &crate::V1KeepLocalIntent,
        _proof: &V1ConflictSourceProof,
    ) -> Result<bool, SyncCoordError> {
        Ok(false)
    }

    /// Return the exact newer candidate only when it is durable, belongs to
    /// this account and consent epoch, still represents the global source and
    /// is confirmed by the current native runtime. A candidate from another
    /// account, an unconfirmed runtime or an unresolved journal mismatch
    /// must never authorize recovery.
    fn newer_rebase_candidate(
        &mut self,
        _context: &V1LocalEditContext,
        _intent: &crate::V1KeepLocalIntent,
    ) -> Result<Option<(V1OutboxCandidate, V1ConflictSourceProof)>, SyncCoordError> {
        Ok(None)
    }

    /// Once the original conflict is durably restored, atomically recheck
    /// the journal and exact candidate/source/runtime proof before deleting
    /// the journal. Keep the candidate and the older receipt intact.
    fn abandon_rebase(
        &mut self,
        _context: &V1LocalEditContext,
        _intent: &crate::V1KeepLocalIntent,
        _candidate: &V1OutboxCandidate,
        _proof: &V1ConflictSourceProof,
    ) -> Result<bool, SyncCoordError> {
        Ok(false)
    }

    /// Return only the current global source generation when the exact saved
    /// journal has been superseded. This exposes no other account's candidate
    /// and does not require runtime confirmation: restoration is local and
    /// leaves the original conflict and all candidates unresolved.
    fn superseded_rebase_generation(
        &mut self,
        _context: &V1LocalEditContext,
        _intent: &crate::V1KeepLocalIntent,
    ) -> Result<Option<i64>, SyncCoordError> {
        Ok(None)
    }

    /// Atomically verify the exact journal and that the global source still
    /// equals this reviewed generation, which must be strictly greater than
    /// the intent's generation, then remove only the journal. Preserve every
    /// receipt, source row, runtime state and outbox entry.
    fn abandon_superseded_rebase(
        &mut self,
        _context: &V1LocalEditContext,
        _intent: &crate::V1KeepLocalIntent,
        _generation: i64,
    ) -> Result<bool, SyncCoordError> {
        Ok(false)
    }
}

pub(crate) struct NoProductOutbox;
impl V1ProductOutbox for NoProductOutbox {
    fn next(
        &mut self,
        _: &V1LocalEditContext,
    ) -> Result<Option<V1OutboxCandidate>, SyncCoordError> {
        Ok(None)
    }
    fn acknowledge(
        &mut self,
        _: &V1LocalEditContext,
        _: &V1OutboxCandidate,
        _: &Pending,
    ) -> Result<bool, SyncCoordError> {
        unreachable!("the empty outbox has no candidates")
    }
    fn validate_pending(
        &mut self,
        _: &V1LocalEditContext,
        _: &str,
        _: &Pending,
    ) -> Result<bool, SyncCoordError> {
        Ok(true)
    }

    fn has_pending_rebase(&mut self, _: &V1LocalEditContext) -> Result<bool, SyncCoordError> {
        Ok(false)
    }
}

// Bound synchronous work under the admission lease. If more remain, the next
// call continues from the durable journal before any upload is dispatched.
const MAX_DRAIN: usize = 64;

impl SyncCoordinator {
    /// No checkpoint may advance JSON while a cross-store decision is
    /// unresolved. The shared admission lease closes the race between the
    /// journal check and the state/marker commit.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn save_product_checkpoint(
        &self,
        lease: &V1Lease,
        session: &Arc<Session>,
        config: SyncConfig,
        snapshot: StateSnapshot,
        outbox: &mut dyn V1ProductOutbox,
        marker: Option<UncertainUpload>,
        require_product_marker: bool,
    ) -> Result<StateSnapshot, SyncCoordError> {
        let _admission = lease.try_admission()?;
        self.with_live_session(session, config, || {
            let context = V1LocalEditContext {
                origin: session.instance_origin(),
                account_id: session.account_id().to_owned(),
                config_generation: config.generation,
            };
            lease.ensure_outbox_mode(
                &context.origin,
                &context.account_id,
                require_product_marker,
            )?;
            if outbox.has_pending_rebase(&context)? {
                return Err(SyncCoordError::Local(
                    "review the unfinished conflict decision".into(),
                ));
            }
            let saved = self.save_state(snapshot, lease)?;
            if let Some(marker) = marker {
                lease.acknowledge_upload(marker)?;
            }
            Ok(saved)
        })
    }

    /// Atomically admit a single upload relative to all cooperating local
    /// mutations: drain and durably stage the outbox, ACK exact candidates,
    /// select one not yet attempted and publish its uncertainty marker. The
    /// lease is dropped before HTTP. Edits admitted after this point remain
    /// pending for a later admission; this is not a local-application fence.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn admit_product_upload(
        &self,
        lease: &V1Lease,
        session: &Arc<Session>,
        config: SyncConfig,
        mut snapshot: StateSnapshot,
        outbox: &mut dyn V1ProductOutbox,
        attempted: &BTreeSet<String>,
        require_product_marker: bool,
    ) -> Result<(StateSnapshot, Option<(String, UncertainUpload)>), SyncCoordError> {
        let _admission = lease.try_admission()?;
        let context = V1LocalEditContext {
            origin: session.instance_origin(),
            account_id: session.account_id().to_owned(),
            config_generation: config.generation,
        };
        self.with_live_session(session, config, || {
            Ok(lease.ensure_outbox_mode(
                &context.origin,
                &context.account_id,
                require_product_marker,
            )?)
        })?;
        if self.with_live_session(session, config, || outbox.has_pending_rebase(&context))? {
            return Err(SyncCoordError::Local(
                "review the unfinished conflict decision".into(),
            ));
        }
        // Reject an older in-memory pull if a direct v1 staging writer won
        // the race before admission. The generation CAS must happen under
        // the same lease as local staging and marker publication.
        snapshot = self.with_live_session(session, config, || {
            lease.ensure_outbox_mode(
                &context.origin,
                &context.account_id,
                require_product_marker,
            )?;
            self.save_state(snapshot, lease)
        })?;
        for count in 0..=MAX_DRAIN {
            let candidate = self.with_live_session(session, config, || {
                lease.ensure_outbox_mode(
                    &context.origin,
                    &context.account_id,
                    require_product_marker,
                )?;
                outbox.next(&context)
            })?;
            let Some(candidate) = candidate else { break };
            if count == MAX_DRAIN {
                return Err(SyncCoordError::Local(
                    "too many local sync changes; retry".into(),
                ));
            }
            if candidate.generation <= 0 {
                return Err(SyncCoordError::Local(
                    "invalid local sync generation".into(),
                ));
            }
            snapshot
                .state
                .stage(candidate.key.clone(), candidate.edit.clone())?;
            // A crash between the two stores leaves the outbox entry intact;
            // staging the same edit again retains the original base revision.
            snapshot = self.with_live_session(session, config, || {
                lease.ensure_outbox_mode(
                    &context.origin,
                    &context.account_id,
                    require_product_marker,
                )?;
                self.save_state(snapshot, lease)
            })?;
            let staged = snapshot
                .state
                .entries
                .get(&candidate.key)
                .and_then(|entry| entry.pending.as_ref())
                .ok_or_else(|| SyncCoordError::Local("local sync candidate disappeared".into()))?;
            if !self.with_live_session(session, config, || {
                lease.ensure_outbox_mode(
                    &context.origin,
                    &context.account_id,
                    require_product_marker,
                )?;
                outbox.acknowledge(&context, &candidate, staged)
            })? {
                return Err(SyncCoordError::Local(
                    "local sync change changed before acknowledgement".into(),
                ));
            }
        }
        let key = snapshot
            .state
            .entries
            .iter()
            .find(|(key, entry)| {
                !attempted.contains(*key)
                    && entry
                        .pending
                        .as_ref()
                        .is_some_and(|pending| !pending.conflict)
            })
            .map(|(key, _)| key.clone());
        let Some(key) = key else {
            return Ok((snapshot, None));
        };
        let pending = snapshot
            .state
            .entries
            .get(&key)
            .and_then(|entry| entry.pending.as_ref())
            .ok_or_else(|| SyncCoordError::Local("local sync candidate disappeared".into()))?;
        if !self.with_live_session(session, config, || {
            lease.ensure_outbox_mode(
                &context.origin,
                &context.account_id,
                require_product_marker,
            )?;
            outbox.validate_pending(&context, &key, pending)
        })? {
            return Err(SyncCoordError::Local(
                "local sync source changed; review required".into(),
            ));
        }
        let marker = self.with_live_session(session, config, || {
            lease.ensure_outbox_mode(
                &context.origin,
                &context.account_id,
                require_product_marker,
            )?;
            Ok(lease.prepare_upload(&snapshot, &key)?)
        })?;
        Ok((snapshot, Some((key, marker))))
    }
}

#[cfg(test)]
mod proof_tests {
    use super::V1ConflictSourceProof;

    #[test]
    fn conflict_source_proof_requires_a_positive_generation_and_native_epoch() {
        assert!(V1ConflictSourceProof::new(0, "a".repeat(64)).is_none());
        assert!(V1ConflictSourceProof::new(1, "A".repeat(64)).is_none());
        assert!(V1ConflictSourceProof::new(1, "a".repeat(63)).is_none());
        assert_eq!(
            V1ConflictSourceProof::new(42, "a".repeat(64))
                .unwrap()
                .generation(),
            42
        );
    }
}
