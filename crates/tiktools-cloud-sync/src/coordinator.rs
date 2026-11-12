//! Device coordinator implementation. The shared state and public contracts
//! remain at the crate root; child modules own individual operations without
//! introducing additional coordinator instances or resource locks.
use crate::{CloudAuth, ProtocolFence, Runtime, SyncCoordError, SyncCoordinator, CONFIG_FILE};
use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};
use tokio::sync::Mutex;

mod config;
mod consent;
mod cutover;
mod local;
mod run;
mod session;
mod state;
mod status;

impl SyncCoordinator {
    /// `dir` holds the state store and the opt-in file. It must be private
    /// to the device; [`cloud_client::StateStore`] enforces that on open.
    pub fn new(auth: CloudAuth, dir: impl AsRef<Path>) -> Result<Self, SyncCoordError> {
        let dir = dir.as_ref().to_path_buf();
        let protocol = ProtocolFence::open(&dir)?;
        let coordinator = Self {
            auth,
            protocol,
            config_path: dir.join(CONFIG_FILE),
            runtime: Arc::new(Mutex::new(Runtime {
                last_error: None,
                last_synced_at: None,
            })),
            syncing: Arc::new(AtomicBool::new(false)),
            recovery_binding: Arc::new(()),
        };
        // Upgraded installations may already have a preference file. Record
        // that fact durably before admitting operations, so a later loss of
        // the file cannot be mistaken for a never-configured installation.
        if !coordinator.consent_marker_present()?
            && match std::fs::symlink_metadata(&coordinator.config_path) {
                Ok(_) => true,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
                Err(error) => return Err(SyncCoordError::Io(error.to_string())),
            }
        {
            let lease = coordinator.protocol.try_v1()?;
            let _admission = lease.try_admission()?;
            coordinator.ensure_consent_marker()?;
        }
        Ok(coordinator)
    }
}
