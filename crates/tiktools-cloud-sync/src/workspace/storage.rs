//! Identity checks, protocol leases and bounded atomic workspace updates.
use super::WorkspaceEligibility;
use super::{WorkspaceCoordError, WorkspaceCoordinator};
use cloud_client::{
    Session, StoreError, WorkspaceState, WorkspaceStateError, WorkspaceStateSnapshot,
};

use super::MAX_LOCAL_RETRIES;

impl WorkspaceCoordinator {
    pub fn snapshot(&self) -> Result<WorkspaceStateSnapshot, WorkspaceCoordError> {
        Ok(self.store.load()?.unwrap_or(WorkspaceStateSnapshot {
            generation: 0,
            state: WorkspaceState::new(self.origin.clone(), self.account_id.clone())?,
        }))
    }

    pub(super) fn identity(&self, session: &Session) -> Result<(), WorkspaceCoordError> {
        if session.instance_origin() != self.origin || session.account_id() != self.account_id {
            return Err(WorkspaceStateError::WrongIdentity.into());
        }
        Ok(())
    }

    pub(super) fn generic_lease(
        &self,
    ) -> Result<crate::protocol_fence::V1Lease, WorkspaceCoordError> {
        let lease = self.protocol.try_v1()?;
        lease.ensure_generic_workspace_allowed(&self.origin, &self.account_id)?;
        // A product-mode workspace remains private even if its separate
        // protocol markers are lost. Older, not-yet-migrated workspaces with
        // product receipts also fail closed rather than bypassing admission.
        if self.store.load()?.is_some_and(|snapshot| {
            snapshot.state.is_product_guarded() || snapshot.state.has_product_receipts()
        }) {
            return Err(crate::ProtocolFenceError::ProductOutboxRequired.into());
        }
        Ok(lease)
    }

    pub(super) fn update<T>(
        &self,
        change: impl FnMut(&mut WorkspaceState) -> Result<T, WorkspaceCoordError>,
    ) -> Result<T, WorkspaceCoordError> {
        self.update_inner(false, change)
    }

    pub(super) fn update_inner<T>(
        &self,
        allow_product_bootstrap: bool,
        mut change: impl FnMut(&mut WorkspaceState) -> Result<T, WorkspaceCoordError>,
    ) -> Result<T, WorkspaceCoordError> {
        for _ in 0..MAX_LOCAL_RETRIES {
            let mut snapshot = self.snapshot()?;
            if !allow_product_bootstrap
                && (snapshot.state.is_product_guarded() || snapshot.state.has_product_receipts())
            {
                return Err(crate::ProtocolFenceError::ProductOutboxRequired.into());
            }
            let result = change(&mut snapshot.state)?;
            let expected = (snapshot.generation != 0).then_some(snapshot.generation);
            match self.store.save(expected, &snapshot.state) {
                Ok(_) => return Ok(result),
                Err(StoreError::ConcurrentChange) => continue,
                Err(error) => return Err(error.into()),
            }
        }
        Err(StoreError::ConcurrentChange.into())
    }

    pub(super) fn update_product<T>(
        &self,
        eligibility: &WorkspaceEligibility,
        mut change: impl FnMut(&mut WorkspaceState) -> Result<T, WorkspaceCoordError>,
    ) -> Result<T, WorkspaceCoordError> {
        for _ in 0..MAX_LOCAL_RETRIES {
            let mut snapshot = self.snapshot()?;
            let result = change(&mut snapshot.state)?;
            let expected = (snapshot.generation != 0).then_some(snapshot.generation);
            match self
                .store
                .save_with_publication_guard::<WorkspaceCoordError>(
                    expected,
                    &snapshot.state,
                    |publish| eligibility.publish(publish),
                ) {
                Ok(_) => return Ok(result),
                Err(WorkspaceCoordError::Store(StoreError::ConcurrentChange)) => continue,
                Err(error) => return Err(error),
            }
        }
        Err(StoreError::ConcurrentChange.into())
    }
}
