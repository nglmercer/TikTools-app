//! Durable v2 workspace orchestration. This module never applies downloaded
//! source to the application. The product adapter owns opt-in, captured-session
//! eligibility and explicit local reviews; the v1 coordinator does not call it.
use std::{path::Path, sync::Arc};

use cloud_client::{
    PluginSelection, Session, StoreError, SyncError, SyncState, WorkspaceEdit, WorkspaceEntry,
    WorkspaceKey, WorkspacePendingStatus, WorkspaceProductProof, WorkspaceRead, WorkspaceRecord,
    WorkspaceState, WorkspaceStateError, WorkspaceStateSnapshot, WorkspaceStore,
};
use thiserror::Error;

const MAX_PULL_PAGES: usize = 200;
const MAX_WRITES: usize = 50;
const MAX_LOCAL_RETRIES: usize = 8;

pub(crate) struct ProductUploadAdmission<'a> {
    pub source: &'a dyn crate::V2ProductUploadSource,
    pub config_generation: u128,
}

#[derive(Debug, Error)]
pub enum WorkspaceCoordError {
    #[error("workspace cutover requires explicit review")]
    BootstrapRequired,
    #[error("workspace cutover was already completed; existing edits were preserved")]
    AlreadyBootstrapped,
    #[error("unfinished workspace contains data; review it before cutover")]
    BootstrapNeedsReview,
    #[error("another workspace network or review operation is in progress")]
    Busy,
    #[error("workspace session or opt-in changed during the operation")]
    Ineligible,
    #[error("workspace operation exceeded its time limit")]
    TimedOut,
    #[error("workspace background worker failed")]
    WorkerFailed,
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    State(#[from] WorkspaceStateError),
    #[error(transparent)]
    Transport(#[from] SyncError),
    #[error(transparent)]
    Protocol(#[from] crate::ProtocolFenceError),
}

/// Captures one product session and opt-in generation. The callback must remain
/// false after either changes, even if the same account signs in or opts in
/// again. It must be non-blocking and must not expose credentials.
#[derive(Clone)]
pub struct WorkspaceEligibility {
    binding: Arc<()>,
    session: Arc<Session>,
    current: Arc<dyn Fn() -> bool + Send + Sync>,
    publication: Option<Arc<PublicationGuard>>,
    emission: Option<Arc<EmissionGuard>>,
    emission_current: Option<Arc<dyn Fn() -> bool + Send + Sync>>,
}
type PublicationGuard = dyn Fn(&mut dyn FnMut() -> Result<(), WorkspaceCoordError>) -> Result<(), WorkspaceCoordError>
    + Send
    + Sync;
type EmissionGuard = dyn Fn() -> Result<Box<dyn Send>, WorkspaceCoordError> + Send + Sync;
impl WorkspaceEligibility {
    pub fn new(session: Arc<Session>, current: impl Fn() -> bool + Send + Sync + 'static) -> Self {
        Self {
            binding: Arc::new(()),
            session,
            current: Arc::new(current),
            publication: None,
            emission: None,
            emission_current: None,
        }
    }

    /// Product dispatcher only: revalidate and hold its session/consent
    /// boundary until the workspace's prepared state is atomically renamed.
    pub(crate) fn with_publication_guard(
        mut self,
        guard: impl Fn(
                &mut dyn FnMut() -> Result<(), WorkspaceCoordError>,
            ) -> Result<(), WorkspaceCoordError>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        self.publication = Some(Arc::new(guard));
        self
    }

    /// Product-only session admission held through the local HTTP future.
    /// Cancellation can still leave work running on the server; the durable
    /// request and backend barrier remain necessary for recovery.
    pub(crate) fn with_emission_guard(
        mut self,
        guard: impl Fn() -> Result<Box<dyn Send>, WorkspaceCoordError> + Send + Sync + 'static,
        current: impl Fn() -> bool + Send + Sync + 'static,
    ) -> Self {
        self.emission = Some(Arc::new(guard));
        self.emission_current = Some(Arc::new(current));
        self
    }
    pub fn check(&self) -> Result<(), WorkspaceCoordError> {
        if (self.current)() {
            Ok(())
        } else {
            Err(WorkspaceCoordError::Ineligible)
        }
    }

    pub(crate) fn publish(
        &self,
        publish: &mut dyn FnMut() -> Result<(), WorkspaceCoordError>,
    ) -> Result<(), WorkspaceCoordError> {
        if let Some(guard) = &self.publication {
            guard(publish)
        } else {
            // Generic workspace users supply only an eligibility predicate.
            // Product wiring installs the stronger publication guard above.
            self.check()?;
            publish()
        }
    }

    #[cfg(test)]
    fn with_test_publication_guard(self) -> Self {
        self.with_publication_guard(|publish| publish())
            .with_test_emission_guard()
    }

    #[cfg(test)]
    fn with_test_emission_guard(self) -> Self {
        self.with_emission_guard(|| Ok(Box::new(())), || true)
    }

    /// Pause only in tests after the real session permit is acquired and
    /// before the nonblocking transition check.
    #[cfg(test)]
    pub(crate) fn with_test_emission_pause(
        mut self,
        pause: impl Fn() + Send + Sync + 'static,
    ) -> Self {
        let guard = self.emission.take().expect("test requires emission guard");
        self.emission = Some(Arc::new(move || {
            let permit = guard()?;
            pause();
            Ok(permit)
        }));
        self
    }
}
fn check_eligibility(
    eligibility: Option<&WorkspaceEligibility>,
    session: &Session,
) -> Result<(), WorkspaceCoordError> {
    if let Some(eligibility) = eligibility {
        if !std::ptr::eq(eligibility.session.as_ref(), session) {
            return Err(WorkspaceCoordError::Ineligible);
        }
        eligibility.check()?;
    }
    Ok(())
}

/// Fetched data is not evidence of local application or execution approval.
pub struct WorkspaceOutcome {
    pub pulled: usize,
    pub pushed: usize,
    pub absent_deletes: usize,
    pub pending: usize,
    pub conflicts: Vec<WorkspaceKey>,
    pub remote: Vec<WorkspaceRecord>,
    pub more_queued: bool,
}

/// Native, non-deserializable comparison captured by a real authenticated GET.
/// A UI adapter must retain this object behind its own review identity rather
/// than reconstructing consent from a key and a bare boolean.
pub struct WorkspaceReview {
    binding: Arc<()>,
    eligibility: Option<Arc<()>>,
    origin: String,
    account_id: String,
    read: WorkspaceRead,
    generation: u64,
    entry: WorkspaceEntry,
    product_proof: Option<WorkspaceProductProof>,
}
impl WorkspaceReview {
    pub fn target(&self) -> &WorkspaceKey {
        self.read.target()
    }
    pub fn remote(&self) -> Option<&WorkspaceRecord> {
        self.read.record()
    }
    pub fn entry(&self) -> &WorkspaceEntry {
        &self.entry
    }
    pub fn local_generation(&self) -> u64 {
        self.generation
    }
}

/// One account on one operator-provisioned origin. Separate identities retain
/// their own files and pending work. No credential is stored in this handle.
pub struct WorkspaceCoordinator {
    binding: Arc<()>,
    store: WorkspaceStore,
    protocol: crate::ProtocolFence,
    origin: String,
    account_id: String,
}
impl WorkspaceCoordinator {
    pub fn open(
        root: impl AsRef<Path>,
        origin: String,
        account_id: String,
    ) -> Result<Self, WorkspaceCoordError> {
        let store = WorkspaceStore::open(&root, origin.clone(), account_id.clone())?;
        let protocol = crate::ProtocolFence::open(root)?;
        Ok(Self {
            binding: Arc::new(()),
            store,
            protocol,
            origin,
            account_id,
        })
    }
    pub fn snapshot(&self) -> Result<WorkspaceStateSnapshot, WorkspaceCoordError> {
        Ok(self.store.load()?.unwrap_or(WorkspaceStateSnapshot {
            generation: 0,
            state: WorkspaceState::new(self.origin.clone(), self.account_id.clone())?,
        }))
    }
    fn identity(&self, session: &Session) -> Result<(), WorkspaceCoordError> {
        if session.instance_origin() != self.origin || session.account_id() != self.account_id {
            return Err(WorkspaceStateError::WrongIdentity.into());
        }
        Ok(())
    }
    fn generic_lease(&self) -> Result<crate::protocol_fence::V1Lease, WorkspaceCoordError> {
        let lease = self.protocol.try_v1()?;
        lease.ensure_generic_workspace_allowed(&self.origin, &self.account_id)?;
        // A product-mode workspace remains private even if its separate
        // protocol markers are lost. Older, not-yet-migrated workspaces with
        // product receipts also fail closed rather than bypassing admission.
        if self.store.load()?.is_some_and(|snapshot| {
            snapshot.state.is_product_guarded() || snapshot.state.has_product_receipts()
        }) {
            return Err(crate::ProtocolFenceError::ProductOutboxRequired.into());
        }
        Ok(lease)
    }
    fn update<T>(
        &self,
        change: impl FnMut(&mut WorkspaceState) -> Result<T, WorkspaceCoordError>,
    ) -> Result<T, WorkspaceCoordError> {
        self.update_inner(false, change)
    }

    fn update_inner<T>(
        &self,
        allow_product_bootstrap: bool,
        mut change: impl FnMut(&mut WorkspaceState) -> Result<T, WorkspaceCoordError>,
    ) -> Result<T, WorkspaceCoordError> {
        for _ in 0..MAX_LOCAL_RETRIES {
            let mut snapshot = self.snapshot()?;
            if !allow_product_bootstrap
                && (snapshot.state.is_product_guarded() || snapshot.state.has_product_receipts())
            {
                return Err(crate::ProtocolFenceError::ProductOutboxRequired.into());
            }
            let result = change(&mut snapshot.state)?;
            let expected = (snapshot.generation != 0).then_some(snapshot.generation);
            match self.store.save(expected, &snapshot.state) {
                Ok(_) => return Ok(result),
                Err(StoreError::ConcurrentChange) => continue,
                Err(error) => return Err(error.into()),
            }
        }
        Err(StoreError::ConcurrentChange.into())
    }

    fn update_product<T>(
        &self,
        eligibility: &WorkspaceEligibility,
        mut change: impl FnMut(&mut WorkspaceState) -> Result<T, WorkspaceCoordError>,
    ) -> Result<T, WorkspaceCoordError> {
        for _ in 0..MAX_LOCAL_RETRIES {
            let mut snapshot = self.snapshot()?;
            let result = change(&mut snapshot.state)?;
            let expected = (snapshot.generation != 0).then_some(snapshot.generation);
            match self
                .store
                .save_with_publication_guard::<WorkspaceCoordError>(
                    expected,
                    &snapshot.state,
                    |publish| eligibility.publish(publish),
                ) {
                Ok(_) => return Ok(result),
                Err(WorkspaceCoordError::Store(StoreError::ConcurrentChange)) => continue,
                Err(error) => return Err(error),
            }
        }
        Err(StoreError::ConcurrentChange.into())
    }
    /// Caller must have reviewed these exact local selections and legacy
    /// pending intents under the current account. This operation uses no
    /// network, imports no v1 bases/cursor, and never rewrites the v1 file.
    pub fn bootstrap(
        &self,
        selections: &[PluginSelection],
        legacy: Option<&SyncState>,
    ) -> Result<(), WorkspaceCoordError> {
        let _lease = self.generic_lease()?;
        self.bootstrap_inner(selections, legacy, None)
    }

    /// Construct exactly the reviewed candidate in one atomic state save.
    pub(crate) fn bootstrap_reviewed(
        &self,
        preview: &crate::CutoverPreview,
    ) -> Result<(), WorkspaceCoordError> {
        self.bootstrap_inner(&preview.selections(), preview.legacy(), Some(preview))
    }

    fn bootstrap_inner(
        &self,
        selections: &[PluginSelection],
        legacy: Option<&SyncState>,
        preview: Option<&crate::CutoverPreview>,
    ) -> Result<(), WorkspaceCoordError> {
        let _operation = self
            .store
            .try_operation_lock()?
            .ok_or(WorkspaceCoordError::Busy)?;
        let mut ids = std::collections::BTreeSet::new();
        for selection in selections {
            if !ids.insert(&selection.plugin_id) {
                return Err(WorkspaceStateError::Invalid.into());
            }
        }
        self.update_inner(
            preview.is_some_and(|preview| preview.is_marked()),
            |state| {
                if state.bootstrap_complete {
                    return Err(WorkspaceCoordError::AlreadyBootstrapped);
                }
                if *state != WorkspaceState::new(self.origin.clone(), self.account_id.clone())? {
                    return Err(WorkspaceCoordError::BootstrapNeedsReview);
                }
                for selection in selections {
                    let key = WorkspaceKey::new("plugin-selections", selection.plugin_id.clone())?;
                    let value = serde_json::to_value(selection)
                        .map_err(|_| WorkspaceStateError::Invalid)?;
                    state.stage(key, WorkspaceEdit::Put(value))?;
                }
                if let Some(legacy) = legacy {
                    state.import_v1_pending(legacy)?;
                }
                if let Some(preview) = preview {
                    preview.apply_review_policy(state)?;
                    if preview.is_marked() {
                        state.mark_product_guarded();
                    }
                }
                state.mark_bootstrap_complete();
                if preview.is_some_and(|preview| !preview.matches_bootstrap(state)) {
                    return Err(WorkspaceStateError::StaleReview.into());
                }
                Ok(())
            },
        )
    }
    /// An offline local decision is queued only after its private atomic save.
    pub fn stage(
        &self,
        key: WorkspaceKey,
        edit: WorkspaceEdit,
    ) -> Result<u64, WorkspaceCoordError> {
        let _lease = self.generic_lease()?;
        self.update(|state| {
            if !state.bootstrap_complete {
                return Err(WorkspaceCoordError::BootstrapRequired);
            }
            Ok(state.stage(key.clone(), edit.clone())?)
        })
    }
    /// Save a fenced durable source decision and its replay receipt in the
    /// same workspace transaction. The caller owns identity, consent and
    /// source fencing and must not interpret this as permission to dispatch.
    pub(crate) fn stage_product_decision(
        &self,
        key: WorkspaceKey,
        edit: WorkspaceEdit,
        source_generation: u64,
        decision_digest: &str,
    ) -> Result<(u64, bool), WorkspaceCoordError> {
        let _operation = self
            .store
            .try_operation_lock()?
            .ok_or(WorkspaceCoordError::Busy)?;
        for _ in 0..MAX_LOCAL_RETRIES {
            let mut snapshot = self.snapshot()?;
            if !snapshot.state.bootstrap_complete {
                return Err(WorkspaceCoordError::BootstrapRequired);
            }
            if !snapshot.state.is_product_guarded() {
                return Err(crate::ProtocolFenceError::ProductOutboxNotEnabled.into());
            }
            let result = snapshot.state.stage_product_decision(
                key.clone(),
                edit.clone(),
                source_generation,
                decision_digest,
            )?;
            if !result.1 {
                return Ok(result);
            }
            let expected = (snapshot.generation != 0).then_some(snapshot.generation);
            match self.store.save(expected, &snapshot.state) {
                Ok(_) => return Ok(result),
                Err(StoreError::ConcurrentChange) => continue,
                Err(error) => return Err(error.into()),
            }
        }
        Err(StoreError::ConcurrentChange.into())
    }
    /// Fetch and conditionally upload. Prepared uncertainty is saved before
    /// HTTP. Unconfirmed/conflicted requests are never automatically resent.
    /// The product must check that the captured session is still current
    /// before returning private rows to a UI or offering local application.
    pub async fn sync(&self, session: &Session) -> Result<WorkspaceOutcome, WorkspaceCoordError> {
        let _lease = self.generic_lease()?;
        self.sync_inner(session, None, None).await
    }
    /// Product callers must use a generation-bound eligibility and recheck it
    /// when exposing or applying the returned data.
    pub async fn sync_guarded(
        &self,
        session: &Session,
        eligibility: &WorkspaceEligibility,
    ) -> Result<WorkspaceOutcome, WorkspaceCoordError> {
        let _lease = self.generic_lease()?;
        self.sync_inner(session, Some(eligibility), None).await
    }
    pub(crate) async fn sync_product_guarded(
        &self,
        session: &Session,
        eligibility: &WorkspaceEligibility,
        admission: &ProductUploadAdmission<'_>,
    ) -> Result<WorkspaceOutcome, WorkspaceCoordError> {
        if eligibility.publication.is_none()
            || eligibility.emission.is_none()
            || eligibility.emission_current.is_none()
        {
            return Err(WorkspaceCoordError::Ineligible);
        }
        self.sync_inner(session, Some(eligibility), Some(admission))
            .await
    }

    /// The short source fence encloses only the local decision and durable
    /// preparation, never the HTTP request. A later source mutation can be
    /// staged separately while the original request is in flight.
    fn admit_product<T>(
        &self,
        target: &WorkspaceKey,
        eligibility: &WorkspaceEligibility,
        admission: &ProductUploadAdmission<'_>,
        operation: impl FnOnce(&WorkspaceProductProof) -> Result<T, WorkspaceCoordError>,
    ) -> Result<T, WorkspaceCoordError> {
        let proof = self
            .snapshot()?
            .state
            .product_queued_proof(target)
            .ok_or(WorkspaceStateError::StaleReview)?;
        self.admit_product_proof(&proof, eligibility, admission, operation)
    }

    fn admit_product_proof<T>(
        &self,
        proof: &WorkspaceProductProof,
        eligibility: &WorkspaceEligibility,
        admission: &ProductUploadAdmission<'_>,
        operation: impl FnOnce(&WorkspaceProductProof) -> Result<T, WorkspaceCoordError>,
    ) -> Result<T, WorkspaceCoordError> {
        let decision = crate::V2ProductDecision::from_proof(
            &self.origin,
            &self.account_id,
            admission.config_generation,
            proof,
        )?;
        let mut operation = Some(operation);
        let mut result = None;
        let mut admit = || {
            let operation = operation.take().ok_or(WorkspaceStateError::StaleReview)?;
            eligibility.check()?;
            result = Some(operation(proof)?);
            Ok(())
        };
        admission.source.with_fenced_upload(&decision, &mut admit)?;
        result.ok_or_else(|| WorkspaceStateError::StaleReview.into())
    }

    async fn sync_inner(
        &self,
        session: &Session,
        eligibility: Option<&WorkspaceEligibility>,
        product: Option<&ProductUploadAdmission<'_>>,
    ) -> Result<WorkspaceOutcome, WorkspaceCoordError> {
        self.identity(session)?;
        check_eligibility(eligibility, session)?;
        let _operation = self
            .store
            .try_operation_lock()?
            .ok_or(WorkspaceCoordError::Busy)?;
        let initial = self.snapshot()?.state;
        if !initial.bootstrap_complete {
            return Err(WorkspaceCoordError::BootstrapRequired);
        }
        if product.is_some() != initial.is_product_guarded() {
            return Err(crate::ProtocolFenceError::ProductOutboxRequired.into());
        }
        if initial.needs_rebuild {
            return Err(WorkspaceStateError::NeedsRebuild.into());
        }
        let mut pulled = 0;
        let mut complete = false;
        for _ in 0..MAX_PULL_PAGES {
            check_eligibility(eligibility, session)?;
            let cursor = self.snapshot()?.state.cursor;
            check_eligibility(eligibility, session)?;
            let response = session.workspace_changes(cursor).await;
            check_eligibility(eligibility, session)?;
            let page = match response {
                Ok(page) => page,
                Err(SyncError::CursorExpired) => {
                    let mark = |state: &mut WorkspaceState| {
                        state.require_rebuild();
                        Ok(())
                    };
                    if product.is_some() {
                        self.update_product(
                            eligibility.ok_or(WorkspaceCoordError::Ineligible)?,
                            mark,
                        )?;
                    } else {
                        self.update(mark)?;
                    }
                    return Err(WorkspaceStateError::NeedsRebuild.into());
                }
                Err(error) => return Err(error.into()),
            };
            check_eligibility(eligibility, session)?;
            pulled += page.changes.len();
            let apply = |state: &mut WorkspaceState| {
                if state.cursor != cursor {
                    return Err(WorkspaceStateError::StaleReview.into());
                }
                Ok(state.apply_changes(page.clone())?)
            };
            let more = if product.is_some() {
                self.update_product(eligibility.ok_or(WorkspaceCoordError::Ineligible)?, apply)?
            } else {
                self.update(apply)?
            };
            if !more {
                complete = true;
                break;
            }
        }
        if !complete {
            return Err(SyncError::TooManyPages.into());
        }
        let keys = self.snapshot()?.state.pending_keys();
        let mut pushed = 0;
        let mut absent_deletes = 0;
        let mut attempts = 0;
        for key in keys {
            check_eligibility(eligibility, session)?;
            if attempts >= MAX_WRITES {
                break;
            }
            let state = self.snapshot()?.state;
            let Some(pending) = state.entry(&key).and_then(|entry| entry.pending.as_ref()) else {
                continue;
            };
            if pending.status != WorkspacePendingStatus::Queued {
                continue;
            }
            if pending.base_revision.is_none() && matches!(pending.edit, WorkspaceEdit::Delete) {
                // A GET returning absent is not a write barrier: an older
                // request may still create the record after that read. Keep
                // product tombstones queued until an authenticated barrier
                // and durable application recovery are wired end to end.
                if product.is_some() {
                    return Err(WorkspaceCoordError::Ineligible);
                }
                let generation = pending.local_generation;
                attempts += 1;
                check_eligibility(eligibility, session)?;
                let read = session.workspace_read_for_resolution(&key).await?;
                check_eligibility(eligibility, session)?;
                let complete = |proof: Option<&WorkspaceProductProof>| {
                    let change = |state: &mut WorkspaceState| {
                        if let Some(proof) = proof {
                            state.verify_product_queued(proof)?;
                        }
                        if read.record().is_none() {
                            state.complete_absent_delete(&read, generation)?;
                        } else {
                            state.observe_read(&read)?;
                        }
                        Ok(())
                    };
                    if product.is_some() {
                        self.update_product(
                            eligibility.ok_or(WorkspaceCoordError::Ineligible)?,
                            change,
                        )
                    } else {
                        self.update(change)
                    }
                };
                if let Some(product) = product {
                    self.admit_product(
                        &key,
                        eligibility.ok_or(WorkspaceCoordError::Ineligible)?,
                        product,
                        |proof| complete(Some(proof)),
                    )?;
                } else {
                    complete(None)?;
                }
                if self
                    .snapshot()?
                    .state
                    .entry(&key)
                    .and_then(|entry| entry.pending.as_ref())
                    .is_none()
                {
                    absent_deletes += 1;
                }
                continue;
            }
            // Product dispatch needs an authenticated fresh observation of
            // enforced epoch policy before preparing any uncertain request.
            // A restored epoch or a GET from an older server is not enough.
            let product_epoch = if product.is_some() {
                check_eligibility(eligibility, session)?;
                let epoch = session.workspace_write_epoch().await?;
                check_eligibility(eligibility, session)?;
                if epoch.epoch_required() != Some(true) {
                    return Err(WorkspaceCoordError::Ineligible);
                }
                Some(epoch)
            } else {
                None
            };
            let ticket = if let Some(product) = product {
                self.admit_product(
                    &key,
                    eligibility.ok_or(WorkspaceCoordError::Ineligible)?,
                    product,
                    |proof| {
                        self.update_product(
                            eligibility.ok_or(WorkspaceCoordError::Ineligible)?,
                            |state| {
                                Ok(state.prepare_product_push_fenced(
                                    proof,
                                    product_epoch
                                        .as_ref()
                                        .ok_or(WorkspaceCoordError::Ineligible)?,
                                )?)
                            },
                        )
                    },
                )?
            } else {
                self.update(|state| Ok(state.prepare_push(&key)?))?
            };
            attempts += 1;
            // Take the cross-process product fence after durable prepare and
            // before emission. A denied fence leaves this ticket Unconfirmed.
            // The product worker imposes a deadline and cancellation drops
            // the lease; the durable ticket retains any uncertain outcome.
            let emission_guard = product
                .map(|product| product.source.acquire_emission_guard())
                .transpose()?;
            // Configuration reads may block. Do them before taking the
            // account's owned session permit. The v2 protocol lease held by
            // the product caller excludes cooperating consent changes.
            check_eligibility(eligibility, session)?;
            // Admit the exact captured CloudAuth session before the last
            // eligibility check. A later account transition waits for the
            // request future to finish or be dropped; its account mutex is
            // never held over network I/O. The lease does not prevent a
            // cancelled request already received by the server from committing.
            let session_guard = if product.is_some() {
                let guard = eligibility
                    .and_then(|eligibility| eligibility.emission.as_ref())
                    .ok_or(WorkspaceCoordError::Ineligible)?;
                Some(guard()?)
            } else {
                None
            };
            // This final check must not perform filesystem I/O while logout
            // waits for session_guard. It uses only nonblocking identity and
            // cancellation/deadline checks supplied by the product caller.
            if product.is_some()
                && !eligibility
                    .and_then(|eligibility| eligibility.emission_current.as_ref())
                    .ok_or(WorkspaceCoordError::Ineligible)?()
            {
                return Err(WorkspaceCoordError::Ineligible);
            }
            let response = match (ticket.edit(), product.is_some()) {
                (WorkspaceEdit::Put(value), true) => {
                    session
                        .workspace_put_fenced(
                            &key.collection,
                            &key.key,
                            value.clone(),
                            ticket.base_revision(),
                            ticket
                                .write_epoch()
                                .ok_or(WorkspaceCoordError::Ineligible)?,
                        )
                        .await
                }
                (WorkspaceEdit::Delete, true) => {
                    session
                        .workspace_delete_fenced(
                            &key.collection,
                            &key.key,
                            ticket
                                .base_revision()
                                .ok_or(WorkspaceStateError::NeedsRead)?,
                            ticket
                                .write_epoch()
                                .ok_or(WorkspaceCoordError::Ineligible)?,
                        )
                        .await
                }
                (WorkspaceEdit::Put(value), false) => {
                    session
                        .workspace_put(
                            &key.collection,
                            &key.key,
                            value.clone(),
                            ticket.base_revision(),
                        )
                        .await
                }
                (WorkspaceEdit::Delete, false) => {
                    session
                        .workspace_delete(
                            &key.collection,
                            &key.key,
                            ticket
                                .base_revision()
                                .ok_or(WorkspaceStateError::NeedsRead)?,
                        )
                        .await
                }
            };
            // Drop before rechecking or publishing: a queued logout owns the
            // next transition, and reacquiring its gate here could deadlock.
            drop(session_guard);
            drop(emission_guard);
            // The response belongs to the captured product session and
            // consent. If either changed while it was in flight, keep the
            // durable ticket unconfirmed for an explicit later review.
            if product.is_some() {
                check_eligibility(eligibility, session)?;
            }
            match response {
                Ok(record) => {
                    let acknowledge = |state: &mut WorkspaceState| {
                        state.acknowledge(&ticket, record.clone())?;
                        Ok(())
                    };
                    if product.is_some() {
                        self.update_product(
                            eligibility.ok_or(WorkspaceCoordError::Ineligible)?,
                            acknowledge,
                        )?;
                    } else {
                        self.update(acknowledge)?;
                    }
                    pushed += 1;
                }
                Err(SyncError::Conflict) => {
                    let conflict = |state: &mut WorkspaceState| {
                        state.mark_conflict(&ticket)?;
                        Ok(())
                    };
                    if product.is_some() {
                        self.update_product(
                            eligibility.ok_or(WorkspaceCoordError::Ineligible)?,
                            conflict,
                        )?;
                    } else {
                        self.update(conflict)?;
                    }
                }
                // A failed/ambiguous response leaves the durable original
                // Unconfirmed. Product recovery must fence the old request
                // with a server barrier before relying on any subsequent GET.
                Err(error) => return Err(error.into()),
            }
        }
        check_eligibility(eligibility, session)?;
        let state = self.snapshot()?.state;
        check_eligibility(eligibility, session)?;
        Ok(WorkspaceOutcome {
            pulled,
            pushed,
            absent_deletes,
            pending: state.pending_keys().len(),
            conflicts: state.conflict_keys(),
            remote: state.ready_remote(),
            more_queued: state.entries.iter().any(|entry| {
                entry
                    .pending
                    .as_ref()
                    .is_some_and(|pending| pending.status == WorkspacePendingStatus::Queued)
            }),
        })
    }
    /// Read-only cloud comparison. It cannot authorize replay or local apply.
    pub async fn review(
        &self,
        session: &Session,
        target: &WorkspaceKey,
    ) -> Result<WorkspaceReview, WorkspaceCoordError> {
        let _lease = self.generic_lease()?;
        self.review_inner(session, target, None, None).await
    }
    pub async fn review_guarded(
        &self,
        session: &Session,
        target: &WorkspaceKey,
        eligibility: &WorkspaceEligibility,
    ) -> Result<WorkspaceReview, WorkspaceCoordError> {
        let _lease = self.generic_lease()?;
        self.review_inner(session, target, Some(eligibility), None)
            .await
    }

    /// Capture a private product recovery comparison. It can only authorize a
    /// later explicit keep-local rebase under the matching native source
    /// fence; it never applies a remote value or resends a request itself.
    #[allow(dead_code)] // Product review integration is not enabled yet.
    pub(crate) async fn review_product_guarded(
        &self,
        session: &Session,
        target: &WorkspaceKey,
        eligibility: &WorkspaceEligibility,
    ) -> Result<WorkspaceReview, WorkspaceCoordError> {
        if eligibility.publication.is_none()
            || eligibility.emission.is_none()
            || eligibility.emission_current.is_none()
        {
            return Err(WorkspaceCoordError::Ineligible);
        }
        let proof = self
            .snapshot()?
            .state
            .product_recovery_proof(target)
            .ok_or(WorkspaceStateError::StaleReview)?;
        self.review_inner(session, target, Some(eligibility), Some(proof))
            .await
    }
    async fn review_inner(
        &self,
        session: &Session,
        target: &WorkspaceKey,
        eligibility: Option<&WorkspaceEligibility>,
        product_proof: Option<WorkspaceProductProof>,
    ) -> Result<WorkspaceReview, WorkspaceCoordError> {
        self.identity(session)?;
        check_eligibility(eligibility, session)?;
        let _operation = self
            .store
            .try_operation_lock()?
            .ok_or(WorkspaceCoordError::Busy)?;
        if self.snapshot()?.state.is_product_guarded() != product_proof.is_some() {
            return Err(crate::ProtocolFenceError::ProductOutboxRequired.into());
        }
        if product_proof.is_some() {
            // Fence the original request, not a newer staged-next decision.
            // A legacy uncertain request without a recorded epoch cannot be
            // given one retroactively and must remain pending for recovery.
            let original_epoch = self
                .snapshot()?
                .state
                .entry(target)
                .and_then(|entry| entry.pending.as_ref())
                .and_then(|pending| pending.write_epoch.clone())
                .ok_or(WorkspaceCoordError::Ineligible)?;
            check_eligibility(eligibility, session)?;
            let policy = session.workspace_write_epoch().await?;
            check_eligibility(eligibility, session)?;
            if policy.epoch_required() != Some(true) {
                return Err(WorkspaceCoordError::Ineligible);
            }
            // Advancing the server epoch affects all devices on this account.
            // Admit the captured session before sending the barrier, just as
            // for a product write. A queued logout must win before admission
            // or wait for the request future to finish or be cancelled.
            let eligibility = eligibility.ok_or(WorkspaceCoordError::Ineligible)?;
            let barrier_guard = eligibility
                .emission
                .as_ref()
                .ok_or(WorkspaceCoordError::Ineligible)?()?;
            if !eligibility
                .emission_current
                .as_ref()
                .ok_or(WorkspaceCoordError::Ineligible)?()
            {
                return Err(WorkspaceCoordError::Ineligible);
            }
            let barrier = session.workspace_write_barrier().await?;
            drop(barrier_guard);
            check_eligibility(Some(eligibility), session)?;
            if !barrier.is_after(&original_epoch)? {
                return Err(WorkspaceCoordError::Ineligible);
            }
        }
        check_eligibility(eligibility, session)?;
        let read = session.workspace_read_for_resolution(target).await?;
        check_eligibility(eligibility, session)?;
        let observe = |state: &mut WorkspaceState| {
            if let Some(proof) = &product_proof {
                state.verify_product_recovery(proof)?;
            }
            state.observe_read(&read)?;
            Ok(())
        };
        if product_proof.is_some() {
            self.update_product(eligibility.ok_or(WorkspaceCoordError::Ineligible)?, observe)?;
        } else {
            self.update(observe)?;
        }
        let state = self.snapshot()?.state;
        if let Some(proof) = &product_proof {
            state.verify_product_recovery(proof)?;
        }
        let generation = state
            .local_generation(target)
            .ok_or(WorkspaceStateError::NoPendingEdit)?;
        let entry = state
            .entry(target)
            .cloned()
            .ok_or(WorkspaceStateError::NoPendingEdit)?;
        check_eligibility(eligibility, session)?;
        Ok(WorkspaceReview {
            binding: self.binding.clone(),
            eligibility: eligibility.map(|v| v.binding.clone()),
            origin: self.origin.clone(),
            account_id: self.account_id.clone(),
            read,
            generation,
            entry,
            product_proof,
        })
    }
    /// Choice is bound to the compared local generation and a new live read.
    /// Taking cloud changes only the cache/queue; local activation is separate.
    pub async fn resolve(
        &self,
        session: &Session,
        review: &WorkspaceReview,
        keep_local: bool,
    ) -> Result<(), WorkspaceCoordError> {
        let _lease = self.generic_lease()?;
        self.resolve_inner(session, review, keep_local, None, None)
            .await
    }
    pub async fn resolve_guarded(
        &self,
        session: &Session,
        review: &WorkspaceReview,
        keep_local: bool,
        eligibility: &WorkspaceEligibility,
    ) -> Result<(), WorkspaceCoordError> {
        let _lease = self.generic_lease()?;
        self.resolve_inner(session, review, keep_local, Some(eligibility), None)
            .await
    }

    /// An explicit keep-local choice only rebases the durable pending edit;
    /// it does not apply Cloud state or send a request. Both the exact source
    /// and session/consent publication fences are required through the save.
    #[allow(dead_code)] // Product review integration is not enabled yet.
    pub(crate) async fn resolve_product_keep_local_guarded(
        &self,
        session: &Session,
        review: &WorkspaceReview,
        eligibility: &WorkspaceEligibility,
        admission: &ProductUploadAdmission<'_>,
    ) -> Result<(), WorkspaceCoordError> {
        if eligibility.publication.is_none() || review.product_proof.is_none() {
            return Err(WorkspaceCoordError::Ineligible);
        }
        self.resolve_inner(session, review, true, Some(eligibility), Some(admission))
            .await
    }
    async fn resolve_inner(
        &self,
        session: &Session,
        review: &WorkspaceReview,
        keep_local: bool,
        eligibility: Option<&WorkspaceEligibility>,
        product: Option<&ProductUploadAdmission<'_>>,
    ) -> Result<(), WorkspaceCoordError> {
        self.identity(session)?;
        check_eligibility(eligibility, session)?;
        let same_eligibility = match (&review.eligibility, eligibility) {
            (None, None) => true,
            (Some(reviewed), Some(current)) => Arc::ptr_eq(reviewed, &current.binding),
            _ => false,
        };
        if !same_eligibility {
            return Err(WorkspaceStateError::StaleReview.into());
        }
        if review.origin != self.origin || review.account_id != self.account_id {
            return Err(WorkspaceStateError::WrongIdentity.into());
        }
        if !Arc::ptr_eq(&review.binding, &self.binding) {
            return Err(WorkspaceStateError::StaleReview.into());
        }
        // A product-bound review must never fall back to generic resolution,
        // which has no source or atomic session publication fence.
        if review.product_proof.is_some() != product.is_some() {
            return Err(WorkspaceStateError::StaleReview.into());
        }
        let _operation = self
            .store
            .try_operation_lock()?
            .ok_or(WorkspaceCoordError::Busy)?;
        if self.snapshot()?.state.is_product_guarded() != product.is_some() {
            return Err(crate::ProtocolFenceError::ProductOutboxRequired.into());
        }
        check_eligibility(eligibility, session)?;
        let fresh = session
            .workspace_read_for_resolution(review.target())
            .await?;
        check_eligibility(eligibility, session)?;
        if fresh.record() != review.read.record() {
            return Err(WorkspaceStateError::StaleReview.into());
        }
        let change = |state: &mut WorkspaceState| {
            check_eligibility(eligibility, session)?;
            if let Some(proof) = &review.product_proof {
                state.verify_product_recovery(proof)?;
            }
            let current = state
                .entry(review.target())
                .ok_or(WorkspaceStateError::NoPendingEdit)?;
            if current.pending != review.entry.pending || current.next != review.entry.next {
                return Err(WorkspaceStateError::StaleReview.into());
            }
            state.resolve(
                &fresh,
                review.read.revision(),
                review.generation,
                keep_local,
            )?;
            Ok(())
        };
        if let Some(product) = product {
            let eligibility = eligibility.ok_or(WorkspaceCoordError::Ineligible)?;
            self.admit_product_proof(
                review
                    .product_proof
                    .as_ref()
                    .ok_or(WorkspaceStateError::StaleReview)?,
                eligibility,
                product,
                |_| self.update_product(eligibility, change),
            )
        } else {
            self.update(change)
        }
    }
    /// Explicit 410 recovery reads a consistent snapshot and atomically
    /// replaces only the fetched cloud baseline. No application source changes.
    pub async fn rebuild_remote_cache(&self, session: &Session) -> Result<(), WorkspaceCoordError> {
        let _lease = self.generic_lease()?;
        self.rebuild_inner(session, None).await
    }
    pub async fn rebuild_remote_cache_guarded(
        &self,
        session: &Session,
        eligibility: &WorkspaceEligibility,
    ) -> Result<(), WorkspaceCoordError> {
        let _lease = self.generic_lease()?;
        self.rebuild_inner(session, Some(eligibility)).await
    }
    async fn rebuild_inner(
        &self,
        session: &Session,
        eligibility: Option<&WorkspaceEligibility>,
    ) -> Result<(), WorkspaceCoordError> {
        self.identity(session)?;
        check_eligibility(eligibility, session)?;
        let _operation = self
            .store
            .try_operation_lock()?
            .ok_or(WorkspaceCoordError::Busy)?;
        if !self.snapshot()?.state.bootstrap_complete {
            return Err(WorkspaceCoordError::BootstrapRequired);
        }
        check_eligibility(eligibility, session)?;
        let snapshot = session.workspace_snapshot().await?;
        check_eligibility(eligibility, session)?;
        self.update(|state| {
            state.install_rebuild(snapshot.clone())?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests;
