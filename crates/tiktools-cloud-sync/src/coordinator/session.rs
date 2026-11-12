//! Current account admission and revalidation around network I/O.
use crate::{ProtocolFenceError, SyncConfig, SyncError};
use crate::{SyncCoordError, SyncCoordinator};
use std::{future::Future, sync::Arc};

impl SyncCoordinator {
    /// Run a bounded synchronous state transition while the exact session is
    /// installed. The protocol lease held by sync separately protects opt-in.
    pub(crate) fn with_live_session<R>(
        &self,
        session: &Arc<cloud_client::Session>,
        config: SyncConfig,
        operation: impl FnOnce() -> Result<R, SyncCoordError>,
    ) -> Result<R, SyncCoordError> {
        self.auth
            .with_current_session(session, || {
                if self.read_config() != config || !config.allows_session(session) {
                    return Err(SyncCoordError::StaleReview);
                }
                operation()
            })
            .map_err(|_| SyncCoordError::StaleReview)?
    }

    /// Revalidate immediately before and after network work. There remains a
    /// dispatch race between the first check and polling the future; a proper
    /// revocable request gate is required for strict cancellation. Writes must
    /// retain their uncertain marker whenever a reply cannot be committed.
    pub(crate) async fn guarded_sync_io<F, T>(
        &self,
        session: &Arc<cloud_client::Session>,
        config: SyncConfig,
        future: F,
    ) -> Result<T, SyncCoordError>
    where
        F: Future<Output = Result<T, SyncError>>,
    {
        self.with_live_session(session, config, || Ok(()))?;
        let result = future.await;
        self.with_live_session(session, config, || Ok(()))?;
        result.map_err(SyncCoordError::from)
    }

    pub(crate) fn current_identity(&self) -> Result<Option<(String, String)>, SyncCoordError> {
        self.auth
            .try_sync_identity()
            .map_err(|_| ProtocolFenceError::Busy.into())
    }
}
