//! Durable product-outbox migration markers and workspace guard upgrades.
use super::ProtocolFenceError;
use super::{
    marker_bytes, sync_dir, CutoverLease, ProductOutboxRecord, ProtocolMode,
    MAX_PROTOCOL_MARKER_BYTES,
};
use cloud_client::WorkspaceStore;
use std::io::Write;

impl CutoverLease {
    /// Upgrade an existing, explicitly marked product workspace to the
    /// durable workspace-level guard. This is local metadata only: it does
    /// not reconcile or dispatch pending work. The exclusive protocol lease,
    /// operation lock and store CAS protect cooperating writers; old binaries
    /// that ignore these locks are outside this migration's guarantee.
    pub(crate) fn upgrade_marked_workspace(
        &self,
        origin: &str,
        account: &str,
    ) -> Result<bool, ProtocolFenceError> {
        let record = self
            .fence
            .read_record(origin, account)?
            .ok_or(ProtocolFenceError::StaleCutover)?;
        if record.format_version != 3 || !self.fence.product_outbox_required(origin, account)? {
            return Err(ProtocolFenceError::ProductOutboxNotEnabled);
        }
        let workspace = WorkspaceStore::open(&self.fence.dir, origin, account)?;
        let _operation = workspace
            .try_operation_lock()?
            .ok_or(ProtocolFenceError::Busy)?;
        let Some(mut snapshot) = workspace.load()? else {
            if record.mode == ProtocolMode::Preparing {
                return Ok(false);
            }
            return Err(ProtocolFenceError::StaleCutover);
        };
        if snapshot.state.is_product_guarded() {
            return Ok(false);
        }
        snapshot.state.mark_product_guarded();
        workspace.save(Some(snapshot.generation), &snapshot.state)?;
        Ok(true)
    }

    /// Marker creation is idempotent across a crash between publication and
    /// preparing the migration. The caller has already revalidated the review.
    pub(crate) fn ensure_legacy_promotion_marker(
        &self,
        origin: &str,
        account: &str,
    ) -> Result<(), ProtocolFenceError> {
        if self.fence.product_outbox_required(origin, account)? {
            return Ok(());
        }
        self.require_product_outbox(origin, account)
    }

    /// Persist the requirement before product mutations start writing an
    /// outbox. Future activation must perform a reviewed source migration
    /// before invoking this; older binaries cannot honor this cooperative fence.
    #[allow(dead_code)]
    pub(crate) fn require_product_outbox(
        &self,
        origin: &str,
        account: &str,
    ) -> Result<(), ProtocolFenceError> {
        self.ensure_v1(origin, account)?;
        if self.fence.uncertain_count(origin, account)? != 0 {
            return Err(ProtocolFenceError::UncertainV1);
        }
        if self.fence.product_outbox_required(origin, account)? {
            return Ok(());
        }
        // Generic operations hold the shared protocol lease. The exclusive
        // cutover lease held by this method therefore prevents new generic
        // work; also hold both workspace locks through marker publication so
        // even a cooperating direct store writer cannot race this check.
        let workspace = WorkspaceStore::open(&self.fence.dir, origin, account)?;
        let _operation = workspace
            .try_operation_lock()?
            .ok_or(ProtocolFenceError::Busy)?;
        workspace
            .try_with_locked_snapshot(|snapshot| {
                if snapshot.is_some_and(|snapshot| {
                    snapshot
                        .state
                        .entries
                        .iter()
                        .any(|entry| entry.pending.is_some() || entry.next.is_some())
                }) {
                    return Err(ProtocolFenceError::UnsettledV2);
                }
                let target = self.fence.product_outbox_path(origin, account)?;
                let record = ProductOutboxRecord {
                    format_version: 1,
                    origin: origin.to_owned(),
                    account_id: account.to_owned(),
                };
                let bytes = marker_bytes(&record, MAX_PROTOCOL_MARKER_BYTES)?;
                let mut temp = tempfile::NamedTempFile::new_in(&self.fence.dir)?;
                temp.write_all(&bytes)?;
                temp.flush()?;
                temp.as_file().sync_all()?;
                temp.persist_noclobber(target)
                    .map_err(|error| error.error)?;
                sync_dir(&self.fence.dir)
            })?
            .ok_or(ProtocolFenceError::Busy)
    }
}
