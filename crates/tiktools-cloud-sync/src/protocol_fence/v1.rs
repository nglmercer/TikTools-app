//! Protocol eligibility for legacy operations under a retained v1 lease.
use super::ProtocolFenceError;
use super::{ProtocolMode, V1Lease};
use cloud_client::StateStore;
use std::path::PathBuf;

impl V1Lease {
    /// A raw workspace coordinator must not bypass product source, consent,
    /// and uncertainty fences. Hold this lease for the entire generic
    /// operation, including network awaits, so cutover cannot change the
    /// classification after this check.
    pub(crate) fn ensure_generic_workspace_allowed(
        &self,
        origin: &str,
        account: &str,
    ) -> Result<(), ProtocolFenceError> {
        let product_outbox = self.fence.product_outbox_required(origin, account)?;
        let protocol = self.fence.read_record(origin, account)?;
        if product_outbox
            || protocol
                .as_ref()
                .is_some_and(|record| record.format_version == 3)
        {
            return Err(ProtocolFenceError::ProductOutboxRequired);
        }
        if protocol.is_some_and(|record| record.mode == ProtocolMode::Preparing) {
            return Err(ProtocolFenceError::StaleCutover);
        }
        Ok(())
    }

    /// A product drain may run only after explicit migration installed the
    /// per-account marker that fences cooperating legacy writers. Call this
    /// while holding the shared admission lease, not as a preflight alone.
    pub(crate) fn ensure_product_outbox(
        &self,
        origin: &str,
        account: &str,
    ) -> Result<(), ProtocolFenceError> {
        if self.fence.product_outbox_required(origin, account)? {
            Ok(())
        } else {
            Err(ProtocolFenceError::ProductOutboxNotEnabled)
        }
    }

    pub(crate) fn ensure_legacy_allowed(
        &self,
        origin: &str,
        account: &str,
    ) -> Result<(), ProtocolFenceError> {
        self.fence.ensure_legacy_allowed(origin, account)
    }

    pub(crate) fn ensure_outbox_mode(
        &self,
        origin: &str,
        account: &str,
        marked: bool,
    ) -> Result<(), ProtocolFenceError> {
        if marked {
            self.ensure_product_outbox(origin, account)
        } else {
            self.ensure_legacy_allowed(origin, account)
        }
    }

    pub(crate) fn legacy_store(
        &self,
        origin: &str,
        account: &str,
    ) -> Result<(StateStore, PathBuf), ProtocolFenceError> {
        self.fence.legacy_store(origin, account)
    }

    pub(crate) fn ensure_no_uncertain(
        &self,
        origin: &str,
        account: &str,
    ) -> Result<(), ProtocolFenceError> {
        if self.fence.uncertain_count(origin, account)? != 0 {
            return Err(ProtocolFenceError::UncertainV1);
        }
        Ok(())
    }

    pub(crate) fn ensure_v1(&self, origin: &str, account: &str) -> Result<(), ProtocolFenceError> {
        if self.fence.read_record(origin, account)?.is_some() {
            return Err(ProtocolFenceError::NotV1);
        }
        Ok(())
    }
}
