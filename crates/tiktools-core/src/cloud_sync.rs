//! Optional Cloud plugin-selection sync owned by the core. Strictly opt-in:
//! without an enabled flag **and** a signed-in account, staging and syncing
//! are inert and every local feature keeps working unchanged.

#[cfg(feature = "persistence")]
use std::sync::OnceLock;
use tiktools_cloud_auth::CloudAuth;
use tiktools_cloud_sync::SyncCoordinator;

use crate::paths::AppPaths;

#[cfg(feature = "persistence")]
mod product_apply;
#[cfg(all(feature = "persistence", feature = "plugin-install"))]
mod product_apply_worker;
#[cfg(feature = "persistence")]
#[allow(dead_code)] // Informational native preview; cutover is not enabled.
mod product_cutover;
#[cfg(all(feature = "persistence", feature = "plugin-install"))]
mod inventory_recovery;
#[cfg(feature = "persistence")]
pub(crate) mod product_outbox;
#[cfg(feature = "persistence")]
#[allow(dead_code)] // Native staging bridge is not yet selected by the product.
pub(crate) mod product_v2_outbox;

pub(crate) struct CloudSync {
    /// `Err` carries the startup failure (for example an unusable or
    /// over-permissive state directory). Sync then stays unavailable while
    /// every local feature keeps working, instead of aborting the host.
    coordinator: Result<SyncCoordinator, String>,
    pub(crate) recovery: tokio::sync::Mutex<Option<crate::control::cloud_sync::RecoveryHandle>>,
    pub(crate) v2_conflict:
        tokio::sync::Mutex<Option<crate::control::cloud_sync::V2ConflictHandle>>,
    /// Only this host lifetime may acknowledge an observed runtime state.
    /// Lazy initialization preserves local operation when Cloud is unused.
    #[cfg(feature = "persistence")]
    #[allow(dead_code)] // Pending the product outbox drain.
    runtime_epoch: OnceLock<Result<String, String>>,
    #[cfg(feature = "persistence")]
    stopping: tokio_util::sync::CancellationToken,
    #[cfg(feature = "persistence")]
    worker: std::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,
    #[cfg(all(feature = "persistence", feature = "plugin-install"))]
    apply_running: std::sync::atomic::AtomicBool,
    #[cfg(all(feature = "persistence", feature = "plugin-install"))]
    apply_cursor: std::sync::atomic::AtomicUsize,
    #[cfg(all(feature = "persistence", feature = "plugin-install"))]
    apply_done: tokio::sync::Notify,
}

pub use tiktools_cloud_sync::SyncStatus;

impl CloudSync {
    pub(crate) fn new(paths: &AppPaths, auth: &CloudAuth) -> Self {
        let coordinator = SyncCoordinator::new(auth.clone(), paths.data.join("cloud-sync"))
            .map_err(|error| {
                tracing::warn!(%error, "cloud sync is unavailable");
                error.to_string()
            });
        Self {
            coordinator,
            recovery: tokio::sync::Mutex::new(None),
            v2_conflict: tokio::sync::Mutex::new(None),
            #[cfg(feature = "persistence")]
            runtime_epoch: OnceLock::new(),
            #[cfg(feature = "persistence")]
            stopping: tokio_util::sync::CancellationToken::new(),
            #[cfg(feature = "persistence")]
            worker: std::sync::Mutex::new(None),
            #[cfg(all(feature = "persistence", feature = "plugin-install"))]
            apply_running: std::sync::atomic::AtomicBool::new(false),
            #[cfg(all(feature = "persistence", feature = "plugin-install"))]
            apply_cursor: std::sync::atomic::AtomicUsize::new(0),
            #[cfg(all(feature = "persistence", feature = "plugin-install"))]
            apply_done: tokio::sync::Notify::new(),
        }
    }

    /// The coordinator, or the reason cloud sync could not start.
    pub(crate) fn coordinator(&self) -> Result<&SyncCoordinator, &str> {
        self.coordinator.as_ref().map_err(String::as_str)
    }

    /// A cryptographically random, native-only identifier fixed for this
    /// host lifetime. A failed entropy source is cached as a failure: a
    /// candidate must never be confirmed using an invented fallback epoch.
    #[cfg(feature = "persistence")]
    #[allow(dead_code)] // Pending the product outbox drain.
    pub(crate) fn runtime_epoch(&self) -> Result<&str, &str> {
        self.runtime_epoch
            .get_or_init(|| {
                let mut bytes = [0u8; 32];
                getrandom::fill(&mut bytes)
                    .map_err(|_| "runtime identity unavailable".to_owned())?;
                Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
            })
            .as_deref()
            .map_err(String::as_str)
    }
}

impl crate::AppCore {
    /// Start the already-consented v2 sync loop on the host runtime.
    pub fn start_cloud_sync_worker_on(
        self: &std::sync::Arc<Self>,
        runtime: &tokio::runtime::Handle,
    ) {
        #[cfg(feature = "persistence")]
        {
            use std::time::Duration;
            let mut task =
                tiktools_plugin_api::sync::recover_mutex(&self.sync.worker, "cloud sync worker");
            if self.is_shutdown() || task.is_some() || self.sync.stopping.is_cancelled() {
                return;
            }
            let weak = std::sync::Arc::downgrade(self);
            let stop = self.sync.stopping.clone();
            *task = Some(runtime.spawn(async move {
                let mut delay = Duration::ZERO;
                let mut failures = 0u32;
                loop {
                    tokio::select! {
                        _ = stop.cancelled() => break,
                        _ = tokio::time::sleep(delay) => {}
                    }
                    let Some(core) = weak.upgrade() else { break };
                    if core.is_shutdown() {
                        break;
                    }
                    // Background work never restores a logged-out session.
                    let signed_in = tokio::select! {
                        _ = stop.cancelled() => break,
                        session = core.cloud.auth().session() => session.is_some(),
                    };
                    if !signed_in {
                        failures = 0;
                        delay = Duration::from_secs(5);
                        continue;
                    }
                    let result = tokio::select! {
                        _ = stop.cancelled() => break,
                        result = core.sync_v2_with_restore(false) => result,
                    };
                    delay = match result {
                        Ok(result) => {
                            failures = 0;
                            if result.more_queued {
                                Duration::from_secs(1)
                            } else {
                                Duration::from_secs(5)
                            }
                        }
                        Err(_) => {
                            failures = failures.saturating_add(1).min(4);
                            Duration::from_secs((5u64 << failures).min(30))
                        }
                    };
                }
            }));
        }
        #[cfg(not(feature = "persistence"))]
        let _ = runtime;
    }

    pub fn start_cloud_sync_worker(self: &std::sync::Arc<Self>) {
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            self.start_cloud_sync_worker_on(&runtime);
        }
    }

    #[cfg(feature = "persistence")]
    pub(crate) async fn stop_cloud_sync_worker(&self) {
        self.sync.stopping.cancel();
        let task =
            tiktools_plugin_api::sync::recover_mutex(&self.sync.worker, "cloud sync worker").take();
        if let Some(task) = task {
            let _ = task.await;
        }
        #[cfg(feature = "plugin-install")]
        loop {
            let finished = self.sync.apply_done.notified();
            tokio::pin!(finished);
            finished.as_mut().enable();
            if !self
                .sync
                .apply_running
                .load(std::sync::atomic::Ordering::Acquire)
            {
                break;
            }
            finished.await;
        }
    }
}
