//! Opt-in changes and exact-session consent epoch revalidation.
use crate::{SyncConfigEpoch, SyncStatus};
use crate::{SyncCoordError, SyncCoordinator};
use std::sync::Arc;

impl SyncCoordinator {
    pub fn is_enabled(&self) -> bool {
        let config = self.read_config();
        config.enabled
            && (config.scope.is_none()
                || self
                    .current_identity()
                    .ok()
                    .flatten()
                    .is_some_and(|(origin, account)| config.allows(&origin, &account)))
    }

    /// Capture opt-in only for an exact current session. A later guarded commit
    /// must recheck both this epoch and the live account session.
    pub fn capture_config_epoch(
        &self,
        session: &Arc<cloud_client::Session>,
    ) -> Option<SyncConfigEpoch> {
        let config = self.read_config();
        (config.enabled && config.allows_session(session) && self.auth.is_current_session(session))
            .then(|| SyncConfigEpoch {
                binding: self.recovery_binding.clone(),
                session: session.clone(),
                config,
            })
    }

    /// Compare only the captured inputs. Call within `CloudAuth::with_current_session`
    /// so account logout or replacement cannot race the local observation.
    pub fn is_config_epoch_current(
        &self,
        epoch: &SyncConfigEpoch,
        session: &Arc<cloud_client::Session>,
    ) -> bool {
        Arc::ptr_eq(&epoch.binding, &self.recovery_binding)
            && Arc::ptr_eq(&epoch.session, session)
            && epoch.config.enabled
            && epoch.config.allows_session(session)
            && self.read_config() == epoch.config
    }

    /// Capture consent specifically bound to this v2 origin/account. Legacy
    /// device-wide v1 opt-in is never enough to authorize v2 networking.
    /// Callers must additionally hold and recheck the selected v2 protocol.
    pub fn capture_v2_config_epoch(
        &self,
        session: &Arc<cloud_client::Session>,
    ) -> Option<SyncConfigEpoch> {
        self.capture_config_epoch(session)
            .filter(|epoch| epoch.config.version == 2 && epoch.config.scope.is_some())
    }

    /// Call within the same current-session guard as `is_config_epoch_current`.
    pub fn is_v2_config_epoch_current(
        &self,
        epoch: &SyncConfigEpoch,
        session: &Arc<cloud_client::Session>,
    ) -> bool {
        epoch.config.version == 2
            && epoch.config.scope.is_some()
            && self.is_config_epoch_current(epoch, session)
    }

    /// Flip the opt-in flag. Disabling stops staging and syncing but keeps
    /// the stored state. Enabling currently supports the v1 product path;
    /// enabling an account already switched to v2 requires product wiring.
    pub fn set_enabled(&self, enabled: bool) -> Result<SyncStatus, SyncCoordError> {
        let lease = self.protocol.try_exclusive_v1()?;
        if enabled {
            if let Some((origin, account)) = self.current_identity()? {
                lease.ensure_v1(&origin, &account)?;
            }
        }
        let previous = self.read_config();
        // Retain the account-scoped protocol identity while disabled, so
        // local package changes remain discoverable for a later v2 resume.
        let scope = if enabled { None } else { previous.scope };
        self.write_new_consent(enabled, previous.generation, scope)?;
        drop(lease);
        Ok(self.status_blocking())
    }
}
