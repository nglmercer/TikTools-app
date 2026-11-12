//! Remote page ingestion, cursor recovery and durable partial progress.
use crate::{
    protocol_fence::V1Lease, SyncConfig, SyncCoordError, SyncCoordinator, SyncError,
    V1ProductOutbox,
};
use cloud_client::{Session, StateSnapshot};
use std::sync::Arc;

const MAX_PULL_PAGES: usize = 200;

impl SyncCoordinator {
    pub(super) async fn pull_v1_remote_pages(
        &self,
        lease: &V1Lease,
        session: &Arc<Session>,
        config: SyncConfig,
        mut snapshot: StateSnapshot,
        outbox: &mut dyn V1ProductOutbox,
        require_product_marker: bool,
    ) -> Result<(StateSnapshot, usize), SyncCoordError> {
        let mut pulled = 0usize;
        // Pull until the server reports no more changes, rebuilding from a
        // snapshot when the cursor expired mid-stream.
        let mut complete = false;
        for _ in 0..MAX_PULL_PAGES {
            match self
                .guarded_sync_io(
                    session,
                    config,
                    session.pull_page_with_count(&mut snapshot.state),
                )
                .await
            {
                Ok((more, count)) => {
                    pulled = pulled.checked_add(count).ok_or(SyncError::Protocol)?;
                    if !more {
                        complete = true;
                        break;
                    }
                }
                Err(SyncCoordError::Engine(SyncError::CursorExpired)) => {
                    let count = self
                        .guarded_sync_io(
                            session,
                            config,
                            session.rebuild_with_count(&mut snapshot.state),
                        )
                        .await?;
                    pulled = pulled.checked_add(count).ok_or(SyncError::Protocol)?;
                    // `rebuild_with_count` already pulls its own follow-up
                    // changes until the server says the feed is complete.
                    complete = true;
                    break;
                }
                Err(SyncCoordError::Engine(SyncError::WrongAccount)) => {
                    // Never delete another identity's durable state.
                    return Err(SyncError::WrongAccount.into());
                }
                Err(error) => {
                    if matches!(error, SyncCoordError::StaleReview) {
                        return Err(error);
                    }
                    self.save_product_checkpoint(
                        lease,
                        session,
                        config,
                        snapshot,
                        outbox,
                        None,
                        require_product_marker,
                    )?;
                    return Err(error);
                }
            }
        }
        if !complete {
            // Preserve safely validated pages so a backlog larger than the
            // per-run bound can make progress without admitting any upload.
            self.save_product_checkpoint(
                lease,
                session,
                config,
                snapshot,
                outbox,
                None,
                require_product_marker,
            )?;
            return Err(SyncError::TooManyPages.into());
        }
        Ok((snapshot, pulled))
    }
}
