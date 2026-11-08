//! Authoritative capture holds the host command/publication barrier. Each
//! domain reads matching bodies/versions in its owning SQLite transaction.
//! Legacy points awards can commit ahead of publication; versions, not stream
//! sequences, determine whether their late invalidation advances a client row.
use crate::{collaboration::*, control::OperationError, AppCore};
#[cfg(all(test, feature = "persistence"))]
mod tests;

impl AppCore {
    pub fn collaboration_subscribe(
        &self,
        auth: Authorization,
        connection: &str,
        params: SubscribeParams,
        limits: &RemoteLimits,
    ) -> Result<SubscribeResult, OperationError> {
        #[cfg(feature = "persistence")]
        {
            let _barrier = self
                .coordination_gate
                .lock()
                .map_err(|_| OperationError::internal("coordination barrier is unavailable"))?;
            if self.is_shutdown() {
                return Err(unavailable());
            }
            let identity = auth.snapshot().map_err(stream_error)?;
            let service = self.collaboration().map_err(stream_error)?;
            self.coordination_streams
                .subscribe(&identity, connection, service.instance_id(), params, limits)
                .map_err(stream_error)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (auth, connection, params, limits);
            Err(unavailable())
        }
    }
    pub fn collaboration_snapshot(
        &self,
        auth: Authorization,
        connection: &str,
        params: SnapshotParams,
        frame_bytes: usize,
    ) -> Result<SnapshotPage, OperationError> {
        #[cfg(feature = "persistence")]
        {
            let _barrier = self
                .coordination_gate
                .lock()
                .map_err(|_| OperationError::internal("coordination barrier is unavailable"))?;
            if self.is_shutdown() {
                return Err(unavailable());
            }
            let identity = auth.snapshot().map_err(stream_error)?;
            let domains = self
                .coordination_streams
                .validate(&identity, connection, &params.subscription_id)
                .map_err(stream_error)?;
            if let Some(token) = params.page_token {
                return self
                    .coordination_streams
                    .page(&identity, connection, &params.subscription_id, &token)
                    .map_err(stream_error);
            }
            self.coordination_streams
                .begin_snapshot(&identity, connection, &params.subscription_id)
                .map_err(stream_error)?;
            let page_bytes = snapshot_page_bytes(frame_bytes);
            let mut records = if domains.contains(&StreamDomain::Workflows) {
                self.db
                    .load_workflow_snapshot(&identity.grant, page_bytes, SNAPSHOT_BYTES)
                    .map_err(|error| match error {
                        crate::db::DatabaseError::SnapshotTooLarge => stream_error(Error::TooLarge),
                        _ => OperationError::internal("workflow snapshot is unavailable"),
                    })?
            } else {
                Vec::new()
            };
            if domains.contains(&StreamDomain::Points) {
                let limit = self
                    .coordination_streams
                    .points_limit(&identity, connection, &params.subscription_id)
                    .map_err(stream_error)?;
                let points = match limit {
                    Some(n) => self.db.load_points_snapshot_limited(
                        &identity.grant,
                        page_bytes,
                        SNAPSHOT_BYTES,
                        Some(n),
                    ),
                    None => {
                        self.db
                            .load_points_snapshot(&identity.grant, page_bytes, SNAPSHOT_BYTES)
                    }
                }
                .map_err(|e| match e {
                    crate::db::DatabaseError::SnapshotTooLarge => stream_error(Error::TooLarge),
                    _ => OperationError::internal("points snapshot is unavailable"),
                })?;
                records.extend(points);
            }
            if domains.contains(&StreamDomain::Automation) {
                let automation = self
                    .db
                    .load_behavior_summaries(&identity.grant, None, page_bytes, SNAPSHOT_BYTES)
                    .map_err(|e| match e {
                        crate::db::DatabaseError::SnapshotTooLarge => stream_error(Error::TooLarge),
                        _ => OperationError::internal("automation snapshot is unavailable"),
                    })?;
                records.extend(automation);
            }
            // Recheck permissions after the database read, before retaining or
            // returning a page. A changed grant requires a new baseline.
            let current = auth.snapshot().map_err(stream_error)?;
            self.coordination_streams
                .capture(
                    &current,
                    connection,
                    &params.subscription_id,
                    records,
                    page_bytes,
                )
                .map_err(stream_error)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (auth, connection, params, frame_bytes);
            Err(unavailable())
        }
    }
    pub fn collaboration_resume(
        &self,
        auth: Authorization,
        connection: &str,
        params: ResumeParams,
        limits: &RemoteLimits,
    ) -> Result<ResumeResult, OperationError> {
        #[cfg(feature = "persistence")]
        {
            let _barrier = self
                .coordination_gate
                .lock()
                .map_err(|_| OperationError::internal("coordination barrier is unavailable"))?;
            if self.is_shutdown() {
                return Err(unavailable());
            }
            let identity = auth.snapshot().map_err(stream_error)?;
            let service = self.collaboration().map_err(stream_error)?;
            self.coordination_streams
                .resume(&identity, connection, service.instance_id(), params, limits)
                .map_err(stream_error)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (auth, connection, params, limits);
            Err(unavailable())
        }
    }
    pub fn collaboration_stream_validate(
        &self,
        auth: &Authorization,
        connection: &str,
        id: &str,
        grant_revision: &str,
        snapshot_id: Option<&str>,
    ) -> Result<(), OperationError> {
        let identity = auth.snapshot().map_err(stream_error)?;
        if identity.grant_revision != grant_revision {
            return Err(stream_error(Error::ResyncRequired));
        }
        self.coordination_streams
            .validate(&identity, connection, id)
            .map_err(stream_error)?;
        if let Some(snapshot) = snapshot_id {
            self.coordination_streams
                .validate_snapshot(&identity, connection, id, snapshot)
                .map_err(stream_error)?;
        }
        Ok(())
    }
    pub fn collaboration_stream_event_authorize(
        &self,
        auth: &Authorization,
        connection: &str,
        event: &ReliableEvent,
    ) -> Result<(), OperationError> {
        self.coordination_streams
            .validate(
                &auth.snapshot().map_err(stream_error)?,
                connection,
                &event.subscription_id,
            )
            .map(|_| ())
            .map_err(stream_error)
    }
    pub fn collaboration_stream_next(
        &self,
        auth: &Authorization,
        connection: &str,
    ) -> Result<Option<ReliableEvent>, OperationError> {
        if self.is_shutdown() {
            return Err(unavailable());
        }
        self.coordination_streams
            .next(&auth.snapshot().map_err(stream_error)?, connection)
            .map_err(stream_error)
    }
    pub fn collaboration_stream_sent(
        &self,
        auth: &Authorization,
        connection: &str,
        event: &ReliableEvent,
    ) -> Result<(), OperationError> {
        self.coordination_streams
            .sent(&auth.snapshot().map_err(stream_error)?, connection, event)
            .map_err(stream_error)
    }
    pub fn collaboration_stream_detach(&self, connection: &str) {
        self.coordination_streams.detach(connection);
    }
}
fn unavailable() -> OperationError {
    OperationError::capability_unavailable("coordination is unavailable or shutting down")
}
fn stream_error(error: Error) -> OperationError {
    OperationError {
        code: error.code(),
        message: error.to_string(),
        data: None,
    }
}
