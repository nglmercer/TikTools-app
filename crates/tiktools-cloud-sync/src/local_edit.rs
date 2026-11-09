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

impl SyncCoordinator {
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
        // Unlike status display, an unreadable config must not be interpreted
        // as opt-out: doing so could silently omit an intended Cloud edit.
        let config = match std::fs::read(&self.config_path) {
            Ok(bytes) => {
                // The file may belong to a legacy install written after this
                // coordinator opened. Mark it before allowing any mutation.
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
                    enabled: false,
                    generation: 0,
                }
            }
            Err(_) => return Err(SyncCoordError::Io("sync config is unreadable".into())),
        };
        if config.enabled && config.generation == 0 {
            // Unlike an actual opt-out, this file records an old opt-in
            // without a durable consent identity. Do not silently stage the
            // edit as local-only or reuse an unbound historical receipt.
            return Err(SyncCoordError::Io(
                "legacy sync consent needs review".into(),
            ));
        }
        if !config.enabled {
            return Ok(operation(None));
        }
        self.auth
            .with_sync_identity(|identity| {
                let context = match identity {
                    Some((origin, account_id)) => {
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
        assert!(!called.get());
        let restarted = SyncCoordinator::new(sync.auth.clone(), dir.path().join("sync")).unwrap();
        assert!(restarted.with_v1_local_edit(|_| called.set(true)).is_err());
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
        assert!(!called.get());
    }

    #[test]
    fn corrupt_consent_marker_cannot_authorize_edit_or_overwrite_preference() {
        let (_dir, sync) = coordinator();
        std::fs::write(sync.consent_marker_path(), b"bad").unwrap();
        let called = Cell::new(false);
        assert!(sync.with_v1_local_edit(|_| called.set(true)).is_err());
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
