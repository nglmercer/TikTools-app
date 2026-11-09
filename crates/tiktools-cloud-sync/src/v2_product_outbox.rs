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
        stage: impl FnOnce(&V2ProductDecision) -> Result<T, SyncCoordError>,
    ) -> Result<T, SyncCoordError>;
}

impl SyncCoordinator {
    /// Product-only dispatcher. The source must fence and validate each
    /// receipt immediately before its durable prepare; queued workspace data
    /// without a matching native decision is never automatically sent.
    /// This is not yet wired to product RPC or local application.
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
        if cancelled.load(Ordering::Acquire) || Instant::now() >= deadline {
            return Err(crate::WorkspaceCoordError::Ineligible);
        }
        let epoch = self
            .capture_v2_config_epoch(&session)
            .ok_or(crate::WorkspaceCoordError::Ineligible)?;
        let origin = session.instance_origin();
        let account = session.account_id().to_owned();
        let _lease = self
            .protocol
            .try_v2(&origin, &account)
            .map_err(|_| crate::WorkspaceCoordError::Ineligible)?;
        let root = self
            .config_path
            .parent()
            .ok_or(crate::WorkspaceCoordError::Ineligible)?;
        let workspace = WorkspaceCoordinator::open(root, origin, account)?;
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
    }

    /// Stage one durable source decision, with the source fence outermost and
    /// protocol, opt-in and account checks held until the workspace save.
    /// This is native plumbing only: it neither dispatches nor authorizes
    /// remotely derived edits, and product wiring must prevent sync loops.
    pub fn stage_v2_product_decision<S: V2ProductSource>(
        &self,
        source: &S,
    ) -> Result<(u64, bool), SyncCoordError> {
        source.with_fenced_decision(|decision| {
            let identity = self
                .auth
                .try_sync_identity()
                .map_err(|_| ProtocolFenceError::Busy)?
                .ok_or(SyncCoordError::NotSignedIn)?;
            if identity.0 != decision.origin || identity.1 != decision.account_id {
                return Err(SyncCoordError::StaleReview);
            }
            let _lease = self.protocol.try_v2(&identity.0, &identity.1)?;
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
                    let root = self
                        .config_path
                        .parent()
                        .ok_or_else(|| SyncCoordError::Io("invalid config directory".into()))?;
                    let workspace =
                        WorkspaceCoordinator::open(root, identity.0.clone(), identity.1.clone())
                            .map_err(|_| SyncCoordError::StaleReview)?;
                    workspace
                        .stage_product_decision(target, edit, decision.source_generation, &digest)
                        .map_err(|_| SyncCoordError::StaleReview)
                })
                .map_err(|_| SyncCoordError::Protocol(ProtocolFenceError::Busy))?
        })
    }
}
