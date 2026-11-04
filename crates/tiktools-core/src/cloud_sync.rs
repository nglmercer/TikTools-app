//! Optional Cloud plugin-selection sync owned by the core. Strictly opt-in:
//! without an enabled flag **and** a signed-in account, staging and syncing
//! are inert and every local feature keeps working unchanged.

use tiktools_cloud_auth::CloudAuth;
use tiktools_cloud_sync::SyncCoordinator;

use crate::paths::AppPaths;

pub(crate) struct CloudSync {
    coordinator: SyncCoordinator,
}

pub use tiktools_cloud_sync::SyncStatus;

impl CloudSync {
    pub(crate) fn new(paths: &AppPaths, auth: &CloudAuth) -> Self {
        let coordinator = SyncCoordinator::new(auth.clone(), paths.data.join("cloud-sync"))
            .expect("cloud sync directory must be usable");
        Self { coordinator }
    }

    pub(crate) fn coordinator(&self) -> &SyncCoordinator {
        &self.coordinator
    }
}
