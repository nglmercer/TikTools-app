//! Private v2 source-outbox staging. This never sends or applies remote data.
use crate::{
    PluginSelection, ProtocolFenceError, SyncCoordError, SyncCoordinator, WorkspaceCoordinator,
};
use cloud_client::{WorkspaceEdit, WorkspaceKey, WorkspaceProductProof};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    future::Future,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, OnceLock,
    },
    time::{Duration, Instant},
};

const PRODUCT_SYNC_DEADLINE: Duration = Duration::from_secs(60);
const MAX_PRODUCT_WORKERS: usize = 4;

/// Serializable comparison captured by one product review. The live
/// [`crate::WorkspaceReview`] is never stored across RPCs; resolution
/// replays the review and rejects it unless every field still matches.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V2ProductConflictSnapshot {
    pub target: cloud_client::WorkspaceKey,
    pub generation: u64,
    pub entry: cloud_client::WorkspaceEntry,
    pub cloud_record: Option<cloud_client::WorkspaceRecord>,
}

// Eligibility may itself perform blocking native I/O. Cancellation or the
// caller deadline can occur during that check, so never return an earlier
// observation without checking both again after the operation has finished.
pub(crate) fn product_eligible(
    cancelled: &AtomicBool,
    deadline: Instant,
    check: impl FnOnce() -> bool,
) -> bool {
    if cancelled.load(Ordering::Acquire) || Instant::now() >= deadline {
        return false;
    }
    let eligible = check();
    eligible && !cancelled.load(Ordering::Acquire) && Instant::now() < deadline
}

fn product_worker_slots() -> Arc<tokio::sync::Semaphore> {
    static SLOTS: OnceLock<Arc<tokio::sync::Semaphore>> = OnceLock::new();
    Arc::clone(SLOTS.get_or_init(|| Arc::new(tokio::sync::Semaphore::new(MAX_PRODUCT_WORKERS))))
}

/// Cancellation follows the original caller even though the worker cannot be
/// force-stopped while inside native disk or SQLite code.
struct CancelWorker {
    cancelled: Arc<AtomicBool>,
    notify: Option<tokio::sync::oneshot::Sender<()>>,
}
impl Drop for CancelWorker {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
        if let Some(notify) = self.notify.take() {
            let _ = notify.send(());
        }
    }
}

pub(crate) async fn run_product_worker<T, F, Fut>(
    operation: F,
) -> Result<T, crate::WorkspaceCoordError>
where
    T: Send + 'static,
    F: FnOnce(Arc<AtomicBool>, Instant) -> Fut + Send + 'static,
    Fut: Future<Output = Result<T, crate::WorkspaceCoordError>> + 'static,
{
    run_product_worker_for(PRODUCT_SYNC_DEADLINE, operation).await
}

pub(crate) async fn run_product_worker_for<T, F, Fut>(
    limit: Duration,
    operation: F,
) -> Result<T, crate::WorkspaceCoordError>
where
    T: Send + 'static,
    F: FnOnce(Arc<AtomicBool>, Instant) -> Fut + Send + 'static,
    Fut: Future<Output = Result<T, crate::WorkspaceCoordError>> + 'static,
{
    run_product_worker_with_slots(limit, product_worker_slots(), operation).await
}

pub(crate) async fn run_product_worker_with_slots<T, F, Fut>(
    limit: Duration,
    slots: Arc<tokio::sync::Semaphore>,
    operation: F,
) -> Result<T, crate::WorkspaceCoordError>
where
    T: Send + 'static,
    F: FnOnce(Arc<AtomicBool>, Instant) -> Fut + Send + 'static,
    Fut: Future<Output = Result<T, crate::WorkspaceCoordError>> + 'static,
{
    let deadline = Instant::now() + limit;
    let permit = tokio::time::timeout_at(deadline.into(), slots.acquire_owned())
        .await
        .map_err(|_| crate::WorkspaceCoordError::TimedOut)?
        .map_err(|_| crate::WorkspaceCoordError::WorkerFailed)?;
    let handle = tokio::runtime::Handle::try_current()
        .map_err(|_| crate::WorkspaceCoordError::WorkerFailed)?;
    let cancelled = Arc::new(AtomicBool::new(false));
    let worker_cancelled = cancelled.clone();
    let (notify, cancelled_rx) = tokio::sync::oneshot::channel();
    let _cancel_on_drop = CancelWorker {
        cancelled,
        notify: Some(notify),
    };
    let mut worker = tokio::task::spawn_blocking(move || {
        // A cancelled caller cannot return capacity for a still-hung native
        // operation; the permit survives until this worker actually exits.
        let _permit = permit;
        handle.block_on(async move {
            tokio::select! {
                biased;
                _ = cancelled_rx => Err(crate::WorkspaceCoordError::Ineligible),
                result = tokio::time::timeout_at(deadline.into(), operation(worker_cancelled, deadline)) => {
                    result.unwrap_or(Err(crate::WorkspaceCoordError::TimedOut))
                }
            }
        })
    });
    tokio::select! {
        biased;
        result = &mut worker => result.map_err(|_| crate::WorkspaceCoordError::WorkerFailed)?,
        _ = tokio::time::sleep_until(deadline.into()) => Err(crate::WorkspaceCoordError::TimedOut),
    }
}

/// Collections whose staged edit is its own source of truth: there is no
/// native source row to recheck, so the staged proof (bound to the account's
/// consent generation by its digest) is the whole admission. Only credential-
/// free collections belong here.
pub(crate) const OWNED_EDIT_COLLECTIONS: &[&str] = &["variables"];

/// Digest `stage_v2_product_edit` binds to a staged owner-owned edit.
pub(crate) fn owned_edit_digest(
    target: &WorkspaceKey,
    edit: &WorkspaceEdit,
    config_generation: impl std::fmt::Display,
) -> Option<String> {
    let input = serde_json::json!({
        "collection": target.collection,
        "key": target.key,
        "edit": edit,
        "generation": config_generation.to_string(),
    });
    let encoded = serde_json::to_vec(&input).ok()?;
    Some(
        Sha256::digest(encoded)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    )
}

/// A durable product decision. Native source adapters must not construct it
/// from UI claims; its revision belongs to the locally committed source.
#[derive(Clone, Serialize)]
pub struct V2ProductDecision {
    pub origin: String,
    pub account_id: String,
    pub config_generation: u128,
    pub source_generation: u64,
    pub plugin_id: String,
    pub selection: Option<PluginSelection>,
}

impl V2ProductDecision {
    pub(crate) fn from_proof(
        origin: &str,
        account_id: &str,
        config_generation: u128,
        proof: &WorkspaceProductProof,
    ) -> Result<Self, crate::WorkspaceCoordError> {
        if proof.target().collection != "plugin-selections" {
            return Err(cloud_client::WorkspaceStateError::StaleReview.into());
        }
        let selection = match proof.edit() {
            WorkspaceEdit::Put(value) => Some(
                serde_json::from_value::<PluginSelection>(value.clone())
                    .map_err(|_| cloud_client::WorkspaceStateError::StaleReview)?,
            ),
            WorkspaceEdit::Delete => None,
        };
        if selection
            .as_ref()
            .is_some_and(|s| s.plugin_id != proof.target().key)
        {
            return Err(cloud_client::WorkspaceStateError::StaleReview.into());
        }
        let decision = Self {
            origin: origin.to_owned(),
            account_id: account_id.to_owned(),
            config_generation,
            source_generation: proof.source_generation(),
            plugin_id: proof.target().key.clone(),
            selection,
        };
        if decision.digest()? != proof.decision_digest() {
            return Err(cloud_client::WorkspaceStateError::StaleReview.into());
        }
        Ok(decision)
    }

    pub(crate) fn digest(&self) -> Result<String, cloud_client::WorkspaceStateError> {
        let encoded =
            serde_json::to_vec(self).map_err(|_| cloud_client::WorkspaceStateError::Invalid)?;
        Ok(Sha256::digest(encoded)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect())
    }
}

/// Trusted native source. It must recheck the exact decision under the
/// runtime/source fence and call `admit` once, without holding it over HTTP.
pub trait V2ProductUploadSource: Send + Sync {
    /// Acquire the native product emission fence and verify that no
    /// untracked local changes require review. The returned lease is held
    /// only for this bounded request and must be owned/Send. Implementations
    /// that have not integrated the fence fail closed by default.
    fn acquire_emission_guard(&self) -> Result<Box<dyn Send>, crate::WorkspaceCoordError> {
        Err(crate::WorkspaceCoordError::Ineligible)
    }

    /// Validate the exact decision under the native source/runtime fence and
    /// retain that fence through `admit`. The dispatcher can call this more
    /// than once for the same decision, including a read-only recheck after
    /// durable preparation; validation itself must not consume the decision.
    fn with_fenced_upload(
        &self,
        decision: &V2ProductDecision,
        admit: &mut dyn FnMut() -> Result<(), crate::WorkspaceCoordError>,
    ) -> Result<(), crate::WorkspaceCoordError>;
}

/// Trusted native adapter: hold the source mutation fence while loading the
/// durable confirmed candidate, running `stage`, and acknowledging the same
/// source revision. Never acknowledge an error or perform network work here.
/// An ack failure after staging is safe to retry: the workspace receipt is
/// persisted atomically with the decision and survives delivery/rebuild.
pub trait V2ProductSource {
    fn with_fenced_decision<T>(
        &self,
        stage: impl FnOnce(&V2ProductDecision, Option<&str>) -> Result<T, SyncCoordError>,
    ) -> Result<T, SyncCoordError>;
}

/// A product-cloud operation admitted by `admit_product_operation`. The
/// leases stay held for as long as this value (or its remaining fields) lives.
struct ProductOperation {
    epoch: crate::SyncConfigEpoch,
    workspace: WorkspaceCoordinator,
    _lease: crate::V2Lease,
    _marker_lease: crate::protocol_fence::V1Lease,
}

impl SyncCoordinator {
    /// Shared preflight for every product-cloud operation: refuses cancelled or
    /// expired work, then holds the v2 and v1 leases and opens the workspace.
    fn admit_product_operation(
        &self,
        session: &Arc<cloud_client::Session>,
        cancelled: &AtomicBool,
        deadline: Instant,
    ) -> Result<ProductOperation, crate::WorkspaceCoordError> {
        if cancelled.load(Ordering::Acquire) || Instant::now() >= deadline {
            return Err(crate::WorkspaceCoordError::Ineligible);
        }
        let epoch = self
            .capture_v2_config_epoch(session)
            .ok_or(crate::WorkspaceCoordError::Ineligible)?;
        let origin = session.instance_origin();
        let account = session.account_id().to_owned();
        let lease = self
            .protocol
            .try_v2(&origin, &account)
            .map_err(|_| crate::WorkspaceCoordError::Ineligible)?;
        let marker_lease = self
            .protocol
            .try_v1()
            .map_err(|_| crate::WorkspaceCoordError::Ineligible)?;
        marker_lease
            .ensure_product_outbox(&origin, &account)
            .map_err(|_| crate::WorkspaceCoordError::Ineligible)?;
        let root = self
            .config_path
            .parent()
            .ok_or(crate::WorkspaceCoordError::Ineligible)?;
        let workspace = WorkspaceCoordinator::open(root, origin, account)?;
        Ok(ProductOperation {
            epoch,
            workspace,
            _lease: lease,
            _marker_lease: marker_lease,
        })
    }

    /// Product-only dispatcher. The source must fence and validate each
    /// receipt immediately before its durable prepare; queued workspace data
    /// without a matching native decision is never automatically sent.
    /// The explicit product RPC caches fetched records but does not apply them
    /// to local sources.
    pub async fn sync_v2_product(
        &self,
        session: Arc<cloud_client::Session>,
        source: Arc<dyn V2ProductUploadSource + 'static>,
    ) -> Result<crate::WorkspaceOutcome, crate::WorkspaceCoordError> {
        let sync = self.clone();
        run_product_worker(move |cancelled, deadline| async move {
            sync.sync_v2_product_inner(session, source, cancelled, deadline)
                .await
        })
        .await
    }

    async fn sync_v2_product_inner(
        &self,
        session: Arc<cloud_client::Session>,
        source: Arc<dyn V2ProductUploadSource + 'static>,
        cancelled: Arc<AtomicBool>,
        deadline: Instant,
    ) -> Result<crate::WorkspaceOutcome, crate::WorkspaceCoordError> {
        let admission = self.admit_product_operation(&session, &cancelled, deadline)?;
        let (epoch, workspace) = (admission.epoch, admission.workspace);
        let generation = epoch.config.generation;
        let eligibility = self.v2_product_eligibility(session.clone(), epoch, cancelled, deadline);
        workspace
            .sync_product_guarded(
                &session,
                &eligibility,
                &crate::workspace::ProductUploadAdmission {
                    source: source.as_ref(),
                    config_generation: generation,
                },
            )
            .await
    }

    /// Product-only explicit rebuild from a consistent export. Preserves
    /// pending edits and never adopts contents into the running host.
    pub async fn rebuild_v2_product(
        &self,
        session: Arc<cloud_client::Session>,
    ) -> Result<(), crate::WorkspaceCoordError> {
        let sync = self.clone();
        run_product_worker(move |cancelled, deadline| async move {
            sync.rebuild_v2_product_inner(session, cancelled, deadline)
                .await
        })
        .await
    }

    async fn rebuild_v2_product_inner(
        &self,
        session: Arc<cloud_client::Session>,
        cancelled: Arc<AtomicBool>,
        deadline: Instant,
    ) -> Result<(), crate::WorkspaceCoordError> {
        let admission = self.admit_product_operation(&session, &cancelled, deadline)?;
        let (epoch, workspace) = (admission.epoch, admission.workspace);
        let eligibility = self.v2_product_eligibility(session.clone(), epoch, cancelled, deadline);
        workspace
            .rebuild_remote_cache_guarded(&session, &eligibility)
            .await
    }

    /// Local-only conflict listing for the current account. Reads the durable
    /// workspace without network I/O, session fences or product-worker
    /// leases. Callers must revalidate session and consent around this read.
    pub fn v2_product_conflict_keys(
        &self,
        session: &Arc<cloud_client::Session>,
    ) -> Result<Vec<cloud_client::WorkspaceKey>, crate::WorkspaceCoordError> {
        let origin = session.instance_origin();
        let account = session.account_id().to_owned();
        let root = self
            .config_path
            .parent()
            .ok_or(crate::WorkspaceCoordError::Ineligible)?;
        let workspace = WorkspaceCoordinator::open(root, origin, account)?;
        Ok(workspace.snapshot()?.state.conflict_keys())
    }

    /// Local-only read of every durable workspace entry for the current
    /// account: the cached Cloud record, any pending edit with its frozen base
    /// and a newer local decision. Like the conflict listing it performs no
    /// network I/O and takes no fence; callers revalidate the session around it.
    pub fn v2_product_entries(
        &self,
        session: &Arc<cloud_client::Session>,
    ) -> Result<Vec<cloud_client::WorkspaceEntry>, crate::WorkspaceCoordError> {
        let origin = session.instance_origin();
        let account = session.account_id().to_owned();
        let root = self
            .config_path
            .parent()
            .ok_or(crate::WorkspaceCoordError::Ineligible)?;
        let workspace = WorkspaceCoordinator::open(root, origin, account)?;
        Ok(workspace.snapshot()?.state.entries)
    }

    /// Product-only conflict comparison. Runs one fenced review under a
    /// worker lease and returns a serializable snapshot; the caller stores
    /// only the snapshot and a random handle, never the live review.
    pub async fn review_v2_product(
        &self,
        session: Arc<cloud_client::Session>,
        target: cloud_client::WorkspaceKey,
    ) -> Result<V2ProductConflictSnapshot, crate::WorkspaceCoordError> {
        let sync = self.clone();
        run_product_worker(move |cancelled, deadline| async move {
            sync.review_v2_product_inner(session, target, cancelled, deadline)
                .await
        })
        .await
    }

    async fn review_v2_product_inner(
        &self,
        session: Arc<cloud_client::Session>,
        target: cloud_client::WorkspaceKey,
        cancelled: Arc<AtomicBool>,
        deadline: Instant,
    ) -> Result<V2ProductConflictSnapshot, crate::WorkspaceCoordError> {
        let admission = self.admit_product_operation(&session, &cancelled, deadline)?;
        let (epoch, workspace) = (admission.epoch, admission.workspace);
        let eligibility = self.v2_product_eligibility(session.clone(), epoch, cancelled, deadline);
        let review = workspace
            .review_product_guarded(&session, &target, &eligibility)
            .await?;
        Ok(V2ProductConflictSnapshot {
            target: review.target().clone(),
            generation: review.local_generation(),
            entry: review.entry().clone(),
            cloud_record: review.remote().cloned(),
        })
    }

    /// Product-only conflict resolution bound to a prior snapshot. Replays
    /// one fenced review with the same eligibility and rejects it unless the
    /// fresh generation, entry and Cloud read still match the snapshot, then
    /// resolves under the same eligibility and product source admission.
    pub async fn resolve_v2_product(
        &self,
        session: Arc<cloud_client::Session>,
        source: Arc<dyn V2ProductUploadSource + 'static>,
        target: cloud_client::WorkspaceKey,
        expected: V2ProductConflictSnapshot,
        keep_local: bool,
    ) -> Result<(), crate::WorkspaceCoordError> {
        let sync = self.clone();
        run_product_worker(move |cancelled, deadline| async move {
            sync.resolve_v2_product_inner(
                session, source, target, expected, keep_local, cancelled, deadline,
            )
            .await
        })
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn resolve_v2_product_inner(
        &self,
        session: Arc<cloud_client::Session>,
        source: Arc<dyn V2ProductUploadSource + 'static>,
        target: cloud_client::WorkspaceKey,
        expected: V2ProductConflictSnapshot,
        keep_local: bool,
        cancelled: Arc<AtomicBool>,
        deadline: Instant,
    ) -> Result<(), crate::WorkspaceCoordError> {
        let admission = self.admit_product_operation(&session, &cancelled, deadline)?;
        let (epoch, workspace) = (admission.epoch, admission.workspace);
        let generation = epoch.config.generation;
        let eligibility = self.v2_product_eligibility(session.clone(), epoch, cancelled, deadline);
        let review = workspace
            .review_product_guarded(&session, &target, &eligibility)
            .await?;
        if review.target() != &expected.target
            || review.local_generation() != expected.generation
            || review.entry() != &expected.entry
            || review.remote() != expected.cloud_record.as_ref()
        {
            return Err(cloud_client::WorkspaceStateError::StaleReview.into());
        }
        let admission = crate::workspace::ProductUploadAdmission {
            source: source.as_ref(),
            config_generation: generation,
        };
        if keep_local {
            workspace
                .resolve_product_keep_local_guarded(&session, &review, &eligibility, &admission)
                .await
        } else {
            workspace
                .resolve_product_take_cloud_guarded(&session, &review, &eligibility, &admission)
                .await
        }
    }

    // The caller must hold the v2 protocol lease for the lifetime of this guard.
    pub(crate) fn v2_product_eligibility(
        &self,
        session: Arc<cloud_client::Session>,
        epoch: crate::SyncConfigEpoch,
        cancelled: Arc<AtomicBool>,
        deadline: Instant,
    ) -> crate::WorkspaceEligibility {
        let check_sync = self.clone();
        let check_session = session.clone();
        let publish_sync = self.clone();
        let publish_session = session.clone();
        let publish_epoch = epoch.clone();
        let publish_cancelled = cancelled.clone();
        let emission_auth = self.auth.clone();
        let emission_session = session.clone();
        let emission_check_auth = self.auth.clone();
        let emission_check_session = session.clone();
        let emission_cancelled = cancelled.clone();
        crate::WorkspaceEligibility::new(session.clone(), move || {
            product_eligible(&cancelled, deadline, || {
                check_sync.is_v2_config_epoch_current(&epoch, &check_session)
                    && check_sync
                        .auth
                        .with_current_session_permit(&check_session, || ())
                        .is_ok()
            })
        })
        .with_publication_guard(move |publish| {
            // The v2 protocol lease held by this worker excludes cooperative
            // consent changes. Do disk reads before taking the account gate;
            // a blocked read therefore does not hold up logout.
            if !product_eligible(&publish_cancelled, deadline, || {
                publish_sync.is_v2_config_epoch_current(&publish_epoch, &publish_session)
            }) {
                return Err(crate::WorkspaceCoordError::Ineligible);
            }
            publish_sync
                .auth
                .with_current_session_permit(&publish_session, || {
                    if publish_cancelled.load(Ordering::Acquire) || Instant::now() >= deadline {
                        return Err(crate::WorkspaceCoordError::Ineligible);
                    }
                    // This callback is the atomic rename, not serialization,
                    // file fsync, HTTP, or a wait for the workspace lock.
                    publish()
                })
                .map_err(|_| crate::WorkspaceCoordError::Ineligible)?
        })
        .with_emission_guard(
            move || {
                emission_auth
                    .try_session_emission_permit(&emission_session)
                    .map(|permit| Box::new(permit) as Box<dyn Send>)
                    .map_err(|_| crate::WorkspaceCoordError::Ineligible)
            },
            move || {
                product_eligible(&emission_cancelled, deadline, || {
                    emission_check_auth.is_current_session(&emission_check_session)
                })
            },
        )
    }

    /// Stage one durable source decision, with the source fence outermost and
    /// protocol, opt-in and account checks held until the workspace save.
    /// This is native plumbing only: it neither dispatches nor authorizes
    /// remotely derived edits, and product wiring must prevent sync loops.
    pub fn stage_v2_product_decision<S: V2ProductSource>(
        &self,
        source: &S,
    ) -> Result<(u64, bool), SyncCoordError> {
        source.with_fenced_decision(|decision, reviewed_base| {
            let identity = self
                .auth
                .try_sync_identity()
                .map_err(|_| ProtocolFenceError::Busy)?
                .ok_or(SyncCoordError::NotSignedIn)?;
            if identity.0 != decision.origin || identity.1 != decision.account_id {
                return Err(SyncCoordError::StaleReview);
            }
            let _lease = self.protocol.try_v2(&identity.0, &identity.1)?;
            let marker_lease = self.protocol.try_v1()?;
            marker_lease.ensure_product_outbox(&identity.0, &identity.1)?;
            self.auth
                .with_sync_identity_permit(|current| {
                    if current.as_ref() != Some(&identity) {
                        return Err(SyncCoordError::StaleReview);
                    }
                    let config = self.read_staging_config()?;
                    if !config.enabled
                        || config.version != 2
                        || config.scope.is_none()
                        || !config.allows(&identity.0, &identity.1)
                        || config.generation != decision.config_generation
                    {
                        return Err(SyncCoordError::StaleReview);
                    }
                    if decision
                        .selection
                        .as_ref()
                        .is_some_and(|v| v.plugin_id != decision.plugin_id)
                    {
                        return Err(SyncCoordError::StaleReview);
                    }
                    let target = WorkspaceKey::new("plugin-selections", &decision.plugin_id)
                        .map_err(|_| SyncCoordError::StaleReview)?;
                    let edit = match &decision.selection {
                        Some(selection) => WorkspaceEdit::Put(
                            serde_json::to_value(selection)
                                .map_err(|_| SyncCoordError::StaleReview)?,
                        ),
                        None => WorkspaceEdit::Delete,
                    };
                    let digest = decision.digest().map_err(|_| SyncCoordError::StaleReview)?;
                    let base = reviewed_base
                        .map(serde_json::from_str::<Option<cloud_client::WorkspaceRecord>>)
                        .transpose()
                        .map_err(|_| SyncCoordError::StaleReview)?;
                    let root = self
                        .config_path
                        .parent()
                        .ok_or_else(|| SyncCoordError::Io("invalid config directory".into()))?;
                    let workspace =
                        WorkspaceCoordinator::open(root, identity.0.clone(), identity.1.clone())
                            .map_err(|_| SyncCoordError::StaleReview)?;
                    workspace
                        .stage_product_decision_with_base(
                            target,
                            edit,
                            decision.source_generation,
                            &digest,
                            base.as_ref().map(Option::as_ref),
                        )
                        .map_err(|_| SyncCoordError::StaleReview)
                })
                .map_err(|_| SyncCoordError::Protocol(ProtocolFenceError::Busy))?
        })
    }

    /// Stage one owner-owned edit (for example `preferences/desktop`) under
    /// the same fences as source decisions, without a source row. The edit
    /// becomes `Queued` and syncs like any other pending edit. The digest
    /// makes re-staging the same edit idempotent.
    pub fn stage_v2_product_edit(
        &self,
        session: Arc<cloud_client::Session>,
        target: cloud_client::WorkspaceKey,
        edit: cloud_client::WorkspaceEdit,
    ) -> Result<(u64, bool), SyncCoordError> {
        let identity = self
            .auth
            .try_sync_identity()
            .map_err(|_| ProtocolFenceError::Busy)?
            .ok_or(SyncCoordError::NotSignedIn)?;
        if identity.0 != session.instance_origin() || identity.1 != session.account_id() {
            return Err(SyncCoordError::StaleReview);
        }
        let _lease = self.protocol.try_v2(&identity.0, &identity.1)?;
        let marker_lease = self.protocol.try_v1()?;
        marker_lease.ensure_product_outbox(&identity.0, &identity.1)?;
        self.auth
            .with_sync_identity_permit(|current| {
                if current.as_ref() != Some(&identity) {
                    return Err(SyncCoordError::StaleReview);
                }
                let config = self.read_staging_config()?;
                if !config.enabled
                    || config.version != 2
                    || config.scope.is_none()
                    || !config.allows(&identity.0, &identity.1)
                {
                    return Err(SyncCoordError::StaleReview);
                }
                let digest = owned_edit_digest(&target, &edit, config.generation)
                    .ok_or(SyncCoordError::StaleReview)?;
                let root = self
                    .config_path
                    .parent()
                    .ok_or_else(|| SyncCoordError::Io("invalid config directory".into()))?;
                let workspace =
                    WorkspaceCoordinator::open(root, identity.0.clone(), identity.1.clone())
                        .map_err(|_| SyncCoordError::StaleReview)?;
                let state = workspace
                    .snapshot()
                    .map_err(|_| SyncCoordError::StaleReview)?
                    .state;
                if let Some(proof) = state.product_queued_proof(&target) {
                    if proof.decision_digest() == digest && proof.edit() == &edit {
                        return Ok((proof.local_generation(), false));
                    }
                }
                let next_generation = state
                    .product_queued_proof(&target)
                    .map(|proof| proof.source_generation().saturating_add(1))
                    .unwrap_or(1);
                workspace
                    .stage_product_decision_with_base(
                        target.clone(),
                        edit.clone(),
                        next_generation,
                        &digest,
                        None,
                    )
                    .map_err(|_| SyncCoordError::StaleReview)
            })
            .map_err(|_| SyncCoordError::Protocol(ProtocolFenceError::Busy))?
    }
}
