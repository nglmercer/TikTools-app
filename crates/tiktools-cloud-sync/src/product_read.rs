//! Shared protocol observation for local UI reads, never mutation admission.
use crate::{protocol_fence::V1Lease, SyncConfig, SyncCoordError, SyncCoordinator};

/// Keeps cooperating consent/cutover writers out while a local snapshot is
/// read. It does not reserve product mutation admission or authorize a write.
/// Callers must revalidate the exact live session before returning private
/// values. Keep disk work off the async executor and drop promptly afterwards.
pub struct ProductReadLease {
    coordinator: SyncCoordinator,
    config: SyncConfig,
    _protocol: V1Lease,
}

impl SyncCoordinator {
    pub fn product_read(&self) -> Result<ProductReadLease, SyncCoordError> {
        let protocol = self.protocol.try_v1()?;
        let config = self.read_checked_config(false)?;
        Ok(ProductReadLease {
            coordinator: self.clone(),
            config,
            _protocol: protocol,
        })
    }
}

impl ProductReadLease {
    /// Validate scope and both durable v2 markers using the captured live
    /// session, rather than a remembered identity or the account mutex.
    pub fn v2_selected(
        &self,
        session: Option<&cloud_client::Session>,
    ) -> Result<bool, SyncCoordError> {
        if !self.config.enabled || self.config.version == 1 {
            return Ok(false);
        }
        let session = session.ok_or(SyncCoordError::NotSignedIn)?;
        if self.config.version != 2
            || self.config.scope.is_none()
            || !self.config.allows_session(session)
        {
            return Err(SyncCoordError::StaleReview);
        }
        let origin = session.instance_origin();
        let account = session.account_id();
        self._protocol.ensure_product_outbox(&origin, account)?;
        let _v2 = self.coordinator.protocol.try_v2(&origin, account)?;
        Ok(true)
    }

    /// Detect damaged files and noncooperating preference changes too. Normal
    /// consent changes are excluded by the shared protocol lease until drop.
    pub fn check_current(&self) -> Result<(), SyncCoordError> {
        if self.coordinator.read_checked_config(false)? != self.config {
            return Err(SyncCoordError::StaleReview);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tiktools_cloud_auth::{CloudAuth, MemoryTokenStore};

    fn coordinator() -> (tempfile::TempDir, SyncCoordinator) {
        let dir = tempfile::tempdir().unwrap();
        let auth = CloudAuth::new(
            None,
            dir.path().join("account.json"),
            Arc::new(MemoryTokenStore::new()),
        );
        let sync = SyncCoordinator::new(auth, dir.path().join("sync")).unwrap();
        (dir, sync)
    }

    #[test]
    fn readers_share_protocol_and_exclude_consent_changes_without_reserving_mutations() {
        let (_dir, sync) = coordinator();
        sync.set_enabled(true).unwrap();
        let first = sync.product_read().unwrap();
        let second = sync.product_read().unwrap();
        assert!(!first.v2_selected(None).unwrap());
        sync.with_package_journal_admission(|| {
            let third = sync.product_read().unwrap();
            third.check_current().unwrap();
        })
        .unwrap();
        assert!(sync.set_enabled(false).is_err());
        assert!(sync.protocol.try_cutover().is_err());
        drop(first);
        assert!(sync.set_enabled(false).is_err());
        drop(second);
        sync.set_enabled(false).unwrap();
    }

    #[test]
    fn reading_never_repairs_damaged_or_missing_consent() {
        let (_dir, sync) = coordinator();
        assert!(!sync.product_read().unwrap().v2_selected(None).unwrap());
        assert!(!sync.consent_marker_present().unwrap());
        sync.set_enabled(true).unwrap();
        let config = std::fs::read(&sync.config_path).unwrap();
        let lease = sync.product_read().unwrap();
        // Simulate a noncooperating writer; normal writers are fenced out.
        std::fs::write(&sync.config_path, b"invalid").unwrap();
        assert!(lease.check_current().is_err());
        drop(lease);
        assert!(sync.product_read().is_err());
        std::fs::remove_file(&sync.config_path).unwrap();
        assert!(sync.product_read().is_err());
        std::fs::write(&sync.config_path, config).unwrap();
        std::fs::remove_file(sync.consent_marker_path()).unwrap();
        assert!(sync.product_read().is_err());
        assert!(!sync.consent_marker_path().exists());
    }
}
