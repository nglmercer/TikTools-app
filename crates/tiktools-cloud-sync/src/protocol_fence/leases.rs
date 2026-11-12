//! Advisory lock acquisition for protocol, network, admission and emission.
use super::ProtocolFenceError;
use super::{
    CutoverLease, ProductEmissionFence, ProductEmissionLease, ProtocolFence, ProtocolMode,
    V1AdmissionLease, V1Lease, V1NetworkLease, V2Lease,
};
use cloud_client::StateStore;
use fs2::FileExt;
use std::{io, path::Path};

impl ProtocolFence {
    /// The path is application-owned, private and shared by both protocols.
    pub fn open(dir: impl AsRef<Path>) -> Result<Self, ProtocolFenceError> {
        let dir = dir.as_ref().to_path_buf();
        StateStore::open(&dir)?;
        Ok(Self { dir })
    }

    pub(crate) fn try_v1(&self) -> Result<V1Lease, ProtocolFenceError> {
        let file = self.lock_file()?;
        match FileExt::try_lock_shared(&file) {
            Ok(()) => Ok(V1Lease {
                fence: self.clone(),
                _file: file,
            }),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                Err(ProtocolFenceError::Busy)
            }
            Err(error) => Err(error.into()),
        }
    }

    /// Lifecycle operations exclude local staging as well as network runs.
    pub(crate) fn try_exclusive_v1(&self) -> Result<V1Lease, ProtocolFenceError> {
        let file = self.lock_file()?;
        match FileExt::try_lock_exclusive(&file) {
            Ok(()) => Ok(V1Lease {
                fence: self.clone(),
                _file: file,
            }),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                Err(ProtocolFenceError::Busy)
            }
            Err(error) => Err(error.into()),
        }
    }

    pub fn try_cutover(&self) -> Result<CutoverLease, ProtocolFenceError> {
        let file = self.lock_file()?;
        match FileExt::try_lock_exclusive(&file) {
            Ok(()) => Ok(CutoverLease {
                fence: self.clone(),
                _file: file,
            }),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                Err(ProtocolFenceError::Busy)
            }
            Err(error) => Err(error.into()),
        }
    }

    pub fn try_v2(&self, origin: &str, account: &str) -> Result<V2Lease, ProtocolFenceError> {
        let file = self.lock_file()?;
        match FileExt::try_lock_shared(&file) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                return Err(ProtocolFenceError::Busy)
            }
            Err(error) => return Err(error.into()),
        }
        let lease = V2Lease { _file: file };
        let record = self
            .read_record(origin, account)?
            .ok_or(ProtocolFenceError::StaleCutover)?;
        if record.mode != ProtocolMode::V2 {
            return Err(ProtocolFenceError::StaleCutover);
        }
        if record.format_version == 3 && !self.product_outbox_required(origin, account)? {
            return Err(ProtocolFenceError::ProductOutboxNotEnabled);
        }
        Ok(lease)
    }
}

impl ProductEmissionFence {
    pub fn open(dir: impl AsRef<Path>) -> Result<Self, ProtocolFenceError> {
        Ok(Self {
            fence: ProtocolFence::open(dir)?,
        })
    }

    /// Hold while one admitted request may be in flight. Callers must record
    /// an uncertain durable ticket before taking this lease and dispatching.
    pub fn try_emission(&self) -> Result<ProductEmissionLease, ProtocolFenceError> {
        self.try_lock(false)
    }

    /// Hold while atomically marking and committing an untracked local edit.
    /// A busy lock is reported immediately, never waited on synchronously.
    pub fn try_untracked_edit(&self) -> Result<ProductEmissionLease, ProtocolFenceError> {
        self.try_lock(true)
    }

    pub(super) fn try_lock(
        &self,
        exclusive: bool,
    ) -> Result<ProductEmissionLease, ProtocolFenceError> {
        let file = self.fence.lock_file_named("product-emission.lock")?;
        let result = if exclusive {
            FileExt::try_lock_exclusive(&file)
        } else {
            FileExt::try_lock_shared(&file)
        };
        match result {
            Ok(()) => Ok(ProductEmissionLease { _file: file }),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                Err(ProtocolFenceError::Busy)
            }
            Err(error) => Err(error.into()),
        }
    }
}

impl V1Lease {
    /// Reserve a local mutation or admission of an upload. Callers must take
    /// the protocol lease first and keep it while this lease is live.
    pub(crate) fn try_admission(&self) -> Result<V1AdmissionLease, ProtocolFenceError> {
        let file = self.fence.lock_file_named("v1-admission.lock")?;
        match FileExt::try_lock_exclusive(&file) {
            Ok(()) => Ok(V1AdmissionLease { _file: file }),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                Err(ProtocolFenceError::Busy)
            }
            Err(error) => Err(error.into()),
        }
    }

    /// At most one cooperating v1 network run can dispatch at a time. Local
    /// staging retains its independent short state lock and remains available.
    pub(crate) fn try_network(&self) -> Result<V1NetworkLease, ProtocolFenceError> {
        let file = self.fence.lock_file_named("v1-network.lock")?;
        match FileExt::try_lock_exclusive(&file) {
            Ok(()) => Ok(V1NetworkLease { _file: file }),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                Err(ProtocolFenceError::Busy)
            }
            Err(error) => Err(error.into()),
        }
    }
}
