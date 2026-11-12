//! Nonsecret coordinator status projections for the UI.
use crate::SyncCoordinator;
use crate::SyncStatus;
use std::sync::atomic::Ordering;

impl SyncCoordinator {
    pub(crate) fn status_blocking(&self) -> SyncStatus {
        let snapshot = match self.current_identity() {
            Ok(Some((origin, account))) => self
                .protocol
                .legacy_snapshot(&origin, &account)
                .ok()
                .flatten(),
            Ok(None) => None,
            Err(_) => None,
        };
        let pending = snapshot
            .as_ref()
            .map(|s| {
                s.state
                    .entries
                    .values()
                    .filter(|e| e.pending.is_some())
                    .count()
            })
            .unwrap_or(0);
        let conflicts = snapshot
            .as_ref()
            .map(|s| {
                let mut keys: Vec<String> = s
                    .state
                    .entries
                    .iter()
                    .filter(|(_, e)| e.pending.as_ref().is_some_and(|p| p.conflict))
                    .map(|(k, _)| k.clone())
                    .collect();
                keys.sort();
                keys
            })
            .unwrap_or_default();
        // Non-blocking reads: status is called off hot paths only, and the
        // async [`Self::status`] refreshes these fields authoritatively.
        let runtime = self
            .runtime
            .try_lock()
            .map(|r| (r.last_error.clone(), r.last_synced_at));
        let (last_error, last_synced_at) = runtime.unwrap_or((None, None));
        SyncStatus {
            enabled: self.is_enabled(),
            signed_in: self.auth.try_session().is_some(),
            syncing: self.syncing.load(Ordering::SeqCst),
            pending,
            conflicts,
            last_error,
            last_synced_at,
        }
    }

    /// Nonsecret status for the UI. `signed_in` reflects the live session.
    pub async fn status(&self) -> SyncStatus {
        let mut status = self.status_blocking();
        status.signed_in = self.auth.session().await.is_some();
        status.syncing = self.syncing.load(Ordering::SeqCst);
        let runtime = self.runtime.lock().await;
        status.last_error = runtime.last_error.clone();
        status.last_synced_at = runtime.last_synced_at;
        status
    }
}
