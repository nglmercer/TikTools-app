//! Deterministic upload admission and durable outcome checkpoints.
use crate::{
    protocol_fence::V1Lease, SyncConfig, SyncCoordError, SyncCoordinator, SyncError,
    V1ProductOutbox,
};
use cloud_client::{Session, StateSnapshot};
use std::collections::BTreeSet;
use std::sync::Arc;

impl SyncCoordinator {
    pub(super) async fn upload_v1_pending(
        &self,
        lease: &V1Lease,
        session: &Arc<Session>,
        config: SyncConfig,
        mut snapshot: StateSnapshot,
        outbox: &mut dyn V1ProductOutbox,
        require_product_marker: bool,
    ) -> Result<(StateSnapshot, usize), SyncCoordError> {
        // Push staged edits in key order for deterministic behavior.
        // Conflicts stay staged for explicit resolution; anything else
        // aborts after persisting progress so far.
        let mut pushed = 0usize;
        let mut attempted = BTreeSet::new();
        loop {
            let (updated, admitted) = if require_product_marker {
                let mut input = Some(snapshot);
                let mut output = None;
                let mut operation = |guarded: &mut dyn V1ProductOutbox| {
                    let snapshot = input.take().ok_or_else(|| {
                        SyncCoordError::Local("runtime admission was repeated".into())
                    })?;
                    output = Some(self.admit_product_upload(
                        lease, session, config, snapshot, guarded, &attempted, true,
                    )?);
                    Ok(())
                };
                outbox.with_runtime_guard(&mut operation)?;
                drop(operation);
                output.ok_or_else(|| {
                    SyncCoordError::Local("runtime admission was not performed".into())
                })?
            } else {
                self.admit_product_upload(
                    lease, session, config, snapshot, outbox, &attempted, false,
                )?
            };
            snapshot = updated;
            let Some((key, marker)) = admitted else { break };
            attempted.insert(key.clone());
            match self
                .guarded_sync_io(session, config, session.push(&mut snapshot.state, &key))
                .await
            {
                Ok(()) => pushed += 1,
                Err(SyncCoordError::Engine(SyncError::Conflict)) => {}
                Err(SyncCoordError::Engine(SyncError::WrongAccount)) => {
                    return Err(SyncError::WrongAccount.into());
                }
                Err(error) => {
                    // Keep the persisted source generation unchanged so an
                    // explicit review can distinguish the original request
                    // from any newer local decision. Pulled cache is refetched.
                    return Err(error);
                }
            }
            // Do not erase the original candidate before the known outcome is
            // durably saved. A concurrent local stage may win the CAS; in that
            // case its newer value survives and this marker remains for review.
            snapshot = self.save_product_checkpoint(
                lease,
                session,
                config,
                snapshot,
                outbox,
                Some(marker),
                require_product_marker,
            )?;
        }
        Ok((snapshot, pushed))
    }
}
