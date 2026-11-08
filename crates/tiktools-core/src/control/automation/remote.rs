//! Remote reads expose closed status summaries, never owner definition bodies.
use super::*;
use crate::collaboration::{Grant, Versioned};

impl AppCore {
    pub fn automation_list_scoped(
        &self,
        grant: &Grant,
        kind: Option<AutomationKind>,
        bytes: usize,
    ) -> Result<Versioned<AutomationRecords>, OperationError> {
        if !automation_read_allowed(grant) {
            return Err(OperationError::forbidden("automation is not available"));
        }
        #[cfg(feature = "persistence")]
        {
            let rows = self
                .db
                .load_behavior_summaries(
                    grant,
                    kind.map(AutomationKind::family),
                    bytes.min(256 * 1024),
                    bytes,
                )
                .map_err(remote_read_error)?;
            let mut events = Vec::new();
            let mut actions = Vec::new();
            let mut versions = Vec::new();
            for (record, version) in rows {
                if version.scope == "event" {
                    events.push(record);
                } else {
                    actions.push(record);
                }
                versions.push(version);
            }
            Ok(Versioned::new((events, actions), versions))
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (grant, kind, bytes);
            Err(OperationError::capability_unavailable(
                "remote automation requires persistence",
            ))
        }
    }
    pub fn automation_get_scoped(
        &self,
        grant: &Grant,
        kind: AutomationKind,
        id: &str,
        bytes: usize,
    ) -> Result<Versioned<Option<Value>>, OperationError> {
        if !automation_read_allowed(grant) || !grant.allows_automation(kind.family(), id) {
            return Err(OperationError::forbidden("automation is not available"));
        }
        #[cfg(feature = "persistence")]
        {
            self.db
                .behavior_summary(kind.family(), id, bytes)
                .map_err(remote_read_error)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = bytes;
            Err(OperationError::capability_unavailable(
                "remote automation requires persistence",
            ))
        }
    }
}
#[cfg(feature = "persistence")]
fn remote_read_error(error: crate::db::DatabaseError) -> OperationError {
    match error {
        crate::db::DatabaseError::SnapshotTooLarge => OperationError {
            code: "too_large",
            message: "automation summary exceeds the frame budget; use collaboration.snapshot"
                .into(),
            data: None,
        },
        other => versioned::automation_error(other),
    }
}
