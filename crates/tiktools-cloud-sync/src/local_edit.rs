//! Synchronous identity and protocol fencing for product-side local edits.
//!
//! This is an admission boundary only. Callers must atomically persist the
//! local source and any candidate inside the callback; the network coordinator
//! must use the same admission lease to drain before dispatch.

use std::io::ErrorKind;

use crate::{ProtocolFenceError, SyncConfig, SyncCoordError, SyncCoordinator};

/// Nonsecret identity and consent epoch for an atomically recorded local edit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct V1LocalEditContext {
    pub origin: String,
    pub account_id: String,
    pub config_generation: u128,
}

/// The same identity-bound source evidence is recorded for v2. A distinct
/// alias keeps callers explicit about which protocol admitted the edit.
pub type V2LocalEditContext = V1LocalEditContext;

/// Protocol chosen for one local product mutation while the shared mutation
/// admission is held. Legacy callers retain the v1 staging path; v2 edits
/// reserve their source and outbox entry together in the callback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProductLocalEditContext {
    Legacy,
    OptedOut,
    V2(Option<V2LocalEditContext>),
}

impl SyncCoordinator {
    /// Run a local package source reservation while protocol admission is
    /// held. A scoped v2 preference remains tracked if it is disabled; legacy
    /// installations do not gain a v2 marker merely by installing a package.
    pub fn with_package_sync_marker<R>(
        &self,
        operation: impl FnOnce(bool) -> R,
    ) -> Result<R, SyncCoordError> {
        let lease = self.protocol.try_v1()?;
        let _admission = lease.try_admission()?;
        let config = self.read_staging_config()?;
        match (config.version, config.scope.is_some()) {
            (1, _) => Ok(operation(self.protocol.has_any_protocol_record()?)),
            (2, true) => Ok(operation(true)),
            _ => Err(SyncCoordError::StaleReview),
        }
    }

    /// A missing or damaged preference after initialization is not opt-out.
    /// Both staging paths use this strict reader before reporting no queue.
    pub(crate) fn read_staging_config(&self) -> Result<SyncConfig, SyncCoordError> {
        let config = match std::fs::symlink_metadata(&self.config_path) {
            Ok(_) => {
                let bytes = self.read_config_bytes()?;
                self.ensure_consent_marker()?;
                serde_json::from_slice::<SyncConfig>(&bytes)
                    .map_err(|_| SyncCoordError::Io("invalid sync config".into()))?
            }
            Err(error) if error.kind() == ErrorKind::NotFound => {
                if self.consent_marker_present()? {
                    return Err(SyncCoordError::Io(
                        "sync consent file is missing; review required".into(),
                    ));
                }
                SyncConfig {
                    version: 1,
                    enabled: false,
                    generation: 0,
                    scope: None,
                }
            }
            Err(_) => return Err(SyncCoordError::Io("sync config is unreadable".into())),
        };
        if !config.valid() {
            return Err(SyncCoordError::Io("invalid sync config".into()));
        }
        if config.enabled && config.generation == 0 {
            return Err(SyncCoordError::Io(
                "legacy sync consent needs review".into(),
            ));
        }
        Ok(config)
    }

    /// Run a short, synchronous native package-journal transaction while the
    /// shared cross-process mutation admission is held. Package mutations are
    /// global to the host, so this boundary does not capture an account,
    /// acquire the CloudAuth mutex, or grant permission to change files.
    ///
    /// The caller must take its plugin invocation gate first, and must not
    /// perform filesystem operations, await, re-enter this coordinator, or
    /// call an unbounded callback while admission is held. A package journal
    /// must remain durable while file effects run outside this closure.
    pub fn with_package_journal_admission<R>(
        &self,
        operation: impl FnOnce() -> R,
    ) -> Result<R, SyncCoordError> {
        let lease = self.protocol.try_v1()?;
        let _admission = lease.try_admission()?;
        Ok(operation())
    }

    /// Execute a short, synchronous local transaction while opt-in and the
    /// remembered account identity are stable. `None` means opt-out or no
    /// remembered identity; account transitions and lock contention are
    /// errors and never cause the callback to run as a local-only edit.
    ///
    /// The callback must not await, re-enter this coordinator or CloudAuth,
    /// perform network I/O, or retain references to mutable product state.
    /// Success only means the callback executed under these leases: the caller
    /// remains responsible for crash-safe local persistence and delivery.
    pub fn with_v1_local_edit<R>(
        &self,
        operation: impl FnOnce(Option<V1LocalEditContext>) -> R,
    ) -> Result<R, SyncCoordError> {
        let lease = self.protocol.try_v1()?;
        let _admission = lease.try_admission()?;
        let config = self.read_staging_config()?;
        if !config.enabled {
            return Ok(operation(None));
        }
        self.auth
            .with_sync_identity(|identity| {
                let context = match identity {
                    Some((origin, account_id)) => {
                        if !config.allows(&origin, &account_id) {
                            return Ok(operation(None));
                        }
                        lease.ensure_v1(&origin, &account_id)?;
                        Some(V1LocalEditContext {
                            origin,
                            account_id,
                            config_generation: config.generation,
                        })
                    }
                    None => None,
                };
                Ok(operation(context))
            })
            .map_err(|_| SyncCoordError::Protocol(ProtocolFenceError::Busy))?
    }

    /// Admit one local v2 source transaction under the same short mutation
    /// lock used by package journaling and v1 staging. A candidate requires
    /// account-scoped v2 consent, a completed v2 protocol marker, and the
    /// identity-bound product-outbox marker. An opted-out or signed-out edit
    /// remains local-only and must be reconciled explicitly before syncing.
    ///
    /// The callback must persist the source and its candidate atomically; it
    /// must not await, access the network, or re-enter the coordinator/auth.
    /// It runs without the account mutex, but the identity permit and
    /// protocol admission remain held until it returns; blocked native I/O
    /// can therefore delay account transitions. The v2 protocol lease does
    /// not fence independently constructed auth managers or noncooperating
    /// writers.
    pub fn with_v2_local_edit<R>(
        &self,
        operation: impl FnOnce(Option<V2LocalEditContext>) -> R,
    ) -> Result<R, SyncCoordError> {
        let lease = self.protocol.try_v1()?;
        let _admission = lease.try_admission()?;
        let config = self.read_staging_config()?;
        if !config.enabled {
            return Ok(operation(None));
        }
        if config.version != 2 || config.scope.is_none() {
            return Err(SyncCoordError::StaleReview);
        }
        self.auth
            .with_sync_identity_permit(|identity| {
                let context = match identity {
                    Some((origin, account_id)) if config.allows(&origin, &account_id) => {
                        // Recheck both durable markers while the protocol and
                        // short admission leases remain held. Legacy-format
                        // cutovers without a product marker cannot opt into
                        // this source path implicitly.
                        lease.ensure_product_outbox(&origin, &account_id)?;
                        let _v2 = self.protocol.try_v2(&origin, &account_id)?;
                        return Ok(operation(Some(V2LocalEditContext {
                            origin,
                            account_id,
                            config_generation: config.generation,
                        })));
                    }
                    _ => None,
                };
                Ok(operation(context))
            })
            .map_err(|_| SyncCoordError::Protocol(ProtocolFenceError::Busy))?
    }

    /// Choose v1 or v2 under the same cooperative admission that protects
    /// the source transaction. Never decide from an earlier configuration
    /// snapshot: a cutover or opt-out could otherwise overtake the writer.
    /// Damaged consent is an error: it cannot be mistaken for an opt-out.
    pub fn with_product_local_edit<R>(
        &self,
        operation: impl FnOnce(ProductLocalEditContext) -> R,
    ) -> Result<R, SyncCoordError> {
        let lease = self.protocol.try_v1()?;
        let _admission = lease.try_admission()?;
        let config = self.read_staging_config()?;
        if !config.enabled {
            return Ok(operation(ProductLocalEditContext::OptedOut));
        }
        if config.version == 1 {
            return Ok(operation(ProductLocalEditContext::Legacy));
        }
        if config.version != 2 || config.scope.is_none() {
            return Err(SyncCoordError::StaleReview);
        }
        self.auth
            .with_sync_identity_permit(|identity| {
                let context = match identity {
                    Some((origin, account_id)) if config.allows(&origin, &account_id) => {
                        lease.ensure_product_outbox(&origin, &account_id)?;
                        let _v2 = self.protocol.try_v2(&origin, &account_id)?;
                        return Ok(operation(ProductLocalEditContext::V2(Some(
                            V2LocalEditContext {
                                origin,
                                account_id,
                                config_generation: config.generation,
                            },
                        ))));
                    }
                    _ => None,
                };
                Ok(operation(ProductLocalEditContext::V2(context)))
            })
            .map_err(|_| SyncCoordError::Protocol(ProtocolFenceError::Busy))?
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, sync::Arc};

    use tiktools_cloud_auth::{CloudAuth, MemoryTokenStore};

    use super::*;

    fn coordinator() -> (tempfile::TempDir, SyncCoordinator) {
        let dir = tempfile::tempdir().unwrap();
        let auth = CloudAuth::new(
            Some("http://127.0.0.1:3000".into()),
            dir.path().join("account.json"),
            Arc::new(MemoryTokenStore::new()),
        );
        let sync = SyncCoordinator::new(auth, dir.path().join("sync")).unwrap();
        (dir, sync)
    }

    #[test]
    fn package_journal_admission_excludes_other_mutations_without_holding_auth() {
        let (_dir, sync) = coordinator();
        let peer =
            SyncCoordinator::new(sync.auth.clone(), sync.config_path.parent().unwrap()).unwrap();
        sync.with_package_journal_admission(|| {
            assert!(sync.auth.try_sync_identity().is_ok());
            assert!(matches!(
                peer.with_package_journal_admission(|| ()),
                Err(SyncCoordError::Protocol(ProtocolFenceError::Busy))
            ));
            assert!(matches!(
                peer.with_v1_local_edit(|_| ()),
                Err(SyncCoordError::Protocol(ProtocolFenceError::Busy))
            ));
        })
        .unwrap();
        assert!(peer.with_package_journal_admission(|| ()).is_ok());
    }

    #[test]
    fn missing_consent_after_opt_in_never_becomes_a_local_only_edit() {
        let (dir, sync) = coordinator();
        // An actual new installation still supports local-only editing.
        assert!(!sync.consent_marker_present().unwrap());
        assert!(sync
            .with_v1_local_edit(|context| context.is_none())
            .unwrap());
        sync.set_enabled(true).unwrap();
        assert!(sync.consent_marker_present().unwrap());
        std::fs::remove_file(&sync.config_path).unwrap();
        let called = Cell::new(false);
        assert!(sync.with_v1_local_edit(|_| called.set(true)).is_err());
        assert!(sync.with_product_local_edit(|_| called.set(true)).is_err());
        assert!(!called.get());
        let restarted = SyncCoordinator::new(sync.auth.clone(), dir.path().join("sync")).unwrap();
        assert!(restarted.with_v1_local_edit(|_| called.set(true)).is_err());
        assert!(restarted
            .with_product_local_edit(|_| called.set(true))
            .is_err());
        assert!(!called.get());
        // An explicit decision recreates the preference and resumes editing.
        restarted.set_enabled(false).unwrap();
        assert!(restarted
            .with_v1_local_edit(|context| context.is_none())
            .unwrap());
    }

    #[test]
    fn interrupted_first_consent_retains_marker_and_requires_a_decision() {
        let (dir, sync) = coordinator();
        {
            let _lease = sync.protocol.try_exclusive_v1().unwrap();
            sync.ensure_consent_marker().unwrap();
            // A crash here precedes the preference-file write.
        }
        let restarted = SyncCoordinator::new(sync.auth.clone(), dir.path().join("sync")).unwrap();
        let called = Cell::new(false);
        assert!(restarted.with_v1_local_edit(|_| called.set(true)).is_err());
        assert!(!called.get());
        restarted.set_enabled(true).unwrap();
        assert!(restarted.with_v1_local_edit(|_| called.set(true)).is_ok());
        assert!(called.get());
    }

    #[test]
    fn opening_existing_legacy_file_marks_it_before_later_loss() {
        let dir = tempfile::tempdir().unwrap();
        let sync_dir = dir.path().join("sync");
        cloud_client::StateStore::open(&sync_dir).unwrap();
        let config = sync_dir.join(crate::CONFIG_FILE);
        let original = br#"{"enabled":true,"generation":7}"#;
        std::fs::write(&config, original).unwrap();
        let auth = CloudAuth::new(
            Some("http://127.0.0.1:3000".into()),
            dir.path().join("account.json"),
            Arc::new(MemoryTokenStore::new()),
        );
        let sync = SyncCoordinator::new(auth, &sync_dir).unwrap();
        assert!(sync.consent_marker_present().unwrap());
        assert_eq!(std::fs::read(&config).unwrap(), original);
        std::fs::remove_file(config).unwrap();
        let called = Cell::new(false);
        assert!(sync.with_v1_local_edit(|_| called.set(true)).is_err());
        assert!(sync.with_product_local_edit(|_| called.set(true)).is_err());
        assert!(!called.get());
    }

    #[test]
    fn corrupt_consent_marker_cannot_authorize_edit_or_overwrite_preference() {
        let (_dir, sync) = coordinator();
        std::fs::write(sync.consent_marker_path(), b"bad").unwrap();
        let called = Cell::new(false);
        assert!(sync.with_v1_local_edit(|_| called.set(true)).is_err());
        assert!(sync.with_product_local_edit(|_| called.set(true)).is_err());
        assert!(sync.set_enabled(true).is_err());
        assert!(!called.get());
        assert!(!sync.config_path.exists());
    }

    #[test]
    fn legacy_opt_in_without_identity_requires_reconfirmation_without_editing_local_state() {
        let (_dir, sync) = coordinator();
        let legacy = br#"{"enabled":true}"#;
        std::fs::write(&sync.config_path, legacy).unwrap();
        assert!(!sync.is_enabled());
        let invoked = Cell::new(false);
        assert!(sync.with_v1_local_edit(|_| invoked.set(true)).is_err());
        assert!(!invoked.get());
        assert_eq!(std::fs::read(&sync.config_path).unwrap(), legacy);
        sync.set_enabled(true).unwrap();
        let fresh = sync.read_config();
        assert!(fresh.enabled);
        assert!(fresh.generation > u64::MAX.into());
    }

    #[test]
    fn local_only_callback_holds_the_protocol_lease() {
        let (_dir, sync) = coordinator();
        sync.with_v1_local_edit(|context| {
            assert!(context.is_none());
            assert!(matches!(
                sync.set_enabled(true),
                Err(SyncCoordError::Protocol(ProtocolFenceError::Busy))
            ));
        })
        .unwrap();
        sync.set_enabled(true).unwrap();
        // Enabled without a remembered account is also local-only.
        assert!(sync
            .with_v1_local_edit(|context| context)
            .unwrap()
            .is_none());
    }

    #[test]
    fn contention_or_corrupt_config_never_runs_local_transaction() {
        let (_dir, sync) = coordinator();
        let called = Cell::new(false);
        let cutover = sync.protocol.try_cutover().unwrap();
        assert!(matches!(
            sync.with_v1_local_edit(|_| called.set(true)),
            Err(SyncCoordError::Protocol(ProtocolFenceError::Busy))
        ));
        assert!(!called.get());
        drop(cutover);

        std::fs::write(&sync.config_path, b"not json").unwrap();
        assert!(matches!(
            sync.with_v1_local_edit(|_| called.set(true)),
            Err(SyncCoordError::Io(_))
        ));
        assert!(!called.get());
    }

    #[test]
    fn upload_admission_excludes_local_transaction_across_coordinators() {
        let (dir, sync) = coordinator();
        let other = SyncCoordinator::new(sync.auth.clone(), dir.path().join("sync")).unwrap();
        let v1 = other.protocol.try_v1().unwrap();
        let admission = v1.try_admission().unwrap();
        let called = Cell::new(false);
        assert!(matches!(
            sync.with_v1_local_edit(|_| called.set(true)),
            Err(SyncCoordError::Protocol(ProtocolFenceError::Busy))
        ));
        assert!(!called.get());
        drop(admission);
        sync.with_v1_local_edit(|_| called.set(true)).unwrap();
        assert!(called.get());
    }
}
