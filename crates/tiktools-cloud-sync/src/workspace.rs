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
}
type PublicationGuard = dyn Fn(&mut dyn FnMut() -> Result<(), WorkspaceCoordError>) -> Result<(), WorkspaceCoordError>
    + Send
    + Sync;
impl WorkspaceEligibility {
    pub fn new(session: Arc<Session>, current: impl Fn() -> bool + Send + Sync + 'static) -> Self {
        Self {
            binding: Arc::new(()),
            session,
            current: Arc::new(current),
            publication: None,
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
    origin: String,
    account_id: String,
}
impl WorkspaceCoordinator {
    pub fn open(
        root: impl AsRef<Path>,
        origin: String,
        account_id: String,
    ) -> Result<Self, WorkspaceCoordError> {
        let store = WorkspaceStore::open(root, origin.clone(), account_id.clone())?;
        Ok(Self {
            binding: Arc::new(()),
            store,
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
    fn update<T>(
        &self,
        mut change: impl FnMut(&mut WorkspaceState) -> Result<T, WorkspaceCoordError>,
    ) -> Result<T, WorkspaceCoordError> {
        for _ in 0..MAX_LOCAL_RETRIES {
            let mut snapshot = self.snapshot()?;
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
        self.update(|state| {
            if state.bootstrap_complete {
                return Err(WorkspaceCoordError::AlreadyBootstrapped);
            }
            if *state != WorkspaceState::new(self.origin.clone(), self.account_id.clone())? {
                return Err(WorkspaceCoordError::BootstrapNeedsReview);
            }
            for selection in selections {
                let key = WorkspaceKey::new("plugin-selections", selection.plugin_id.clone())?;
                let value =
                    serde_json::to_value(selection).map_err(|_| WorkspaceStateError::Invalid)?;
                state.stage(key, WorkspaceEdit::Put(value))?;
            }
            if let Some(legacy) = legacy {
                state.import_v1_pending(legacy)?;
            }
            if let Some(preview) = preview {
                preview.apply_review_policy(state)?;
            }
            state.mark_bootstrap_complete();
            if preview.is_some_and(|preview| !preview.matches_bootstrap(state)) {
                return Err(WorkspaceStateError::StaleReview.into());
            }
            Ok(())
        })
    }
    /// An offline local decision is queued only after its private atomic save.
    pub fn stage(
        &self,
        key: WorkspaceKey,
        edit: WorkspaceEdit,
    ) -> Result<u64, WorkspaceCoordError> {
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
        self.sync_inner(session, None, None).await
    }
    /// Product callers must use a generation-bound eligibility and recheck it
    /// when exposing or applying the returned data.
    pub async fn sync_guarded(
        &self,
        session: &Session,
        eligibility: &WorkspaceEligibility,
    ) -> Result<WorkspaceOutcome, WorkspaceCoordError> {
        self.sync_inner(session, Some(eligibility), None).await
    }
    pub(crate) async fn sync_product_guarded(
        &self,
        session: &Session,
        eligibility: &WorkspaceEligibility,
        admission: &ProductUploadAdmission<'_>,
    ) -> Result<WorkspaceOutcome, WorkspaceCoordError> {
        if eligibility.publication.is_none() {
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
        let decision = crate::V2ProductDecision::from_proof(
            &self.origin,
            &self.account_id,
            admission.config_generation,
            &proof,
        )?;
        let mut operation = Some(operation);
        let mut result = None;
        let mut admit = || {
            let operation = operation.take().ok_or(WorkspaceStateError::StaleReview)?;
            eligibility.check()?;
            result = Some(operation(&proof)?);
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
            let ticket = if let Some(product) = product {
                self.admit_product(
                    &key,
                    eligibility.ok_or(WorkspaceCoordError::Ineligible)?,
                    product,
                    |proof| {
                        self.update_product(
                            eligibility.ok_or(WorkspaceCoordError::Ineligible)?,
                            |state| Ok(state.prepare_product_push(proof)?),
                        )
                    },
                )?
            } else {
                self.update(|state| Ok(state.prepare_push(&key)?))?
            };
            attempts += 1;
            // If eligibility changed after prepare, keep the durable ticket
            // unconfirmed; never dispatch or infer an outcome.
            check_eligibility(eligibility, session)?;
            let response = match ticket.edit() {
                WorkspaceEdit::Put(value) => {
                    session
                        .workspace_put(
                            &key.collection,
                            &key.key,
                            value.clone(),
                            ticket.base_revision(),
                        )
                        .await
                }
                WorkspaceEdit::Delete => {
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
                // Unconfirmed. A later explicit review performs a fresh GET.
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
        self.review_inner(session, target, None).await
    }
    pub async fn review_guarded(
        &self,
        session: &Session,
        target: &WorkspaceKey,
        eligibility: &WorkspaceEligibility,
    ) -> Result<WorkspaceReview, WorkspaceCoordError> {
        self.review_inner(session, target, Some(eligibility)).await
    }
    async fn review_inner(
        &self,
        session: &Session,
        target: &WorkspaceKey,
        eligibility: Option<&WorkspaceEligibility>,
    ) -> Result<WorkspaceReview, WorkspaceCoordError> {
        self.identity(session)?;
        check_eligibility(eligibility, session)?;
        let _operation = self
            .store
            .try_operation_lock()?
            .ok_or(WorkspaceCoordError::Busy)?;
        check_eligibility(eligibility, session)?;
        let read = session.workspace_read_for_resolution(target).await?;
        check_eligibility(eligibility, session)?;
        self.update(|state| {
            state.observe_read(&read)?;
            Ok(())
        })?;
        let state = self.snapshot()?.state;
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
        self.resolve_inner(session, review, keep_local, None).await
    }
    pub async fn resolve_guarded(
        &self,
        session: &Session,
        review: &WorkspaceReview,
        keep_local: bool,
        eligibility: &WorkspaceEligibility,
    ) -> Result<(), WorkspaceCoordError> {
        self.resolve_inner(session, review, keep_local, Some(eligibility))
            .await
    }
    async fn resolve_inner(
        &self,
        session: &Session,
        review: &WorkspaceReview,
        keep_local: bool,
        eligibility: Option<&WorkspaceEligibility>,
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
        let _operation = self
            .store
            .try_operation_lock()?
            .ok_or(WorkspaceCoordError::Busy)?;
        check_eligibility(eligibility, session)?;
        let fresh = session
            .workspace_read_for_resolution(review.target())
            .await?;
        check_eligibility(eligibility, session)?;
        if fresh.record() != review.read.record() {
            return Err(WorkspaceStateError::StaleReview.into());
        }
        self.update(|state| {
            check_eligibility(eligibility, session)?;
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
        })
    }
    /// Explicit 410 recovery reads a consistent snapshot and atomically
    /// replaces only the fetched cloud baseline. No application source changes.
    pub async fn rebuild_remote_cache(&self, session: &Session) -> Result<(), WorkspaceCoordError> {
        self.rebuild_inner(session, None).await
    }
    pub async fn rebuild_remote_cache_guarded(
        &self,
        session: &Session,
        eligibility: &WorkspaceEligibility,
    ) -> Result<(), WorkspaceCoordError> {
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
