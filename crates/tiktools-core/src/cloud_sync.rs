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
#[cfg(feature = "persistence")]
pub(crate) mod product_outbox;

pub(crate) struct CloudSync {
    /// `Err` carries the startup failure (for example an unusable or
    /// over-permissive state directory). Sync then stays unavailable while
    /// every local feature keeps working, instead of aborting the host.
    coordinator: Result<SyncCoordinator, String>,
    pub(crate) recovery: tokio::sync::Mutex<Option<crate::control::cloud_sync::RecoveryHandle>>,
    /// Only this host lifetime may acknowledge an observed runtime state.
    /// Lazy initialization preserves local operation when Cloud is unused.
    #[cfg(feature = "persistence")]
    #[allow(dead_code)] // Pending the product outbox drain.
    runtime_epoch: OnceLock<Result<String, String>>,
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
            #[cfg(feature = "persistence")]
            runtime_epoch: OnceLock::new(),
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
