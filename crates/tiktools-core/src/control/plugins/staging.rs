//! Legacy Cloud staging is independent of the host SQLite feature.
//! Durable source reservation remains in local_sync; these best-effort calls
//! are also used by activation fallbacks in Cloud-only builds.
use crate::AppCore;

impl AppCore {
    /// Stage a local selection change for upload. Inert unless sync is
    /// enabled and an account is known; never fails the local operation.
    pub(super) fn stage_selection(&self, id: &str, version: &str, enabled: bool) {
        let Ok(coordinator) = self.sync.coordinator() else {
            return;
        };
        if let Err(error) = coordinator.record_local_change(id, version, enabled) {
            tracing::warn!(plugin = %id, %error, "cloud sync staging failed");
        }
    }

    /// Stage a local uninstall for upload. Same opt-in and failure
    /// semantics as [`Self::stage_selection`].
    pub(super) fn stage_removal(&self, id: &str) {
        let Ok(coordinator) = self.sync.coordinator() else {
            return;
        };
        if let Err(error) = coordinator.record_local_removal(id) {
            tracing::warn!(plugin = %id, %error, "cloud sync staging failed");
        }
    }
}
