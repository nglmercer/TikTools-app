//! Optional Cloud plugin-selection sync owned by the core. Strictly opt-in:
//! without an enabled flag **and** a signed-in account, staging and syncing
//! are inert and every local feature keeps working unchanged.

use tiktools_cloud_auth::CloudAuth;
use tiktools_cloud_sync::SyncCoordinator;

use crate::paths::AppPaths;

pub(crate) struct CloudSync {
    /// `Err` carries the startup failure (for example an unusable or
    /// over-permissive state directory). Sync then stays unavailable while
    /// every local feature keeps working, instead of aborting the host.
    coordinator: Result<SyncCoordinator, String>,
}

pub use tiktools_cloud_sync::SyncStatus;

impl CloudSync {
    pub(crate) fn new(paths: &AppPaths, auth: &CloudAuth) -> Self {
        let coordinator = SyncCoordinator::new(auth.clone(), paths.data.join("cloud-sync"))
            .map_err(|error| {
                tracing::warn!(%error, "cloud sync is unavailable");
                error.to_string()
            });
        Self { coordinator }
    }

    /// The coordinator, or the reason cloud sync could not start.
    pub(crate) fn coordinator(&self) -> Result<&SyncCoordinator, &str> {
        self.coordinator.as_ref().map_err(String::as_str)
    }
}
