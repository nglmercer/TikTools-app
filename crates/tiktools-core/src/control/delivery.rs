//! Pure projection delivery. Replayed metadata never enters the business event
//! bus that runs automation, plugins or host media effects.
use crate::{collaboration::CoordinationEvent, control::OperationError, AppCore};
use std::sync::Arc;

impl AppCore {
    #[cfg(feature = "persistence")]
    pub(crate) fn prune_coordination_audit(&self) -> Result<(), OperationError> {
        // Idle local-only startup must not create an identity store just to
        // prune history. Release the lifecycle lock before touching SQLite.
        let existing = self
            .collaboration
            .lock()
            .map_err(|_| OperationError::internal("collaboration state is unavailable"))?
            .clone();
        if let Some(service) = existing {
            service
                .prune_audit()
                .map_err(|_| OperationError::internal("audit retention is unavailable"))?;
        }
        Ok(())
    }

    pub fn subscribe_coordination(
        &self,
    ) -> Result<tokio::sync::broadcast::Receiver<Arc<CoordinationEvent>>, OperationError> {
        if !cfg!(feature = "persistence") {
            return Err(OperationError::capability_unavailable(
                "coordination streams require persistence",
            ));
        }
        Ok(self.coordination_events.subscribe())
    }
    #[cfg(feature = "persistence")]
    pub(crate) fn deliver_coordination(&self) -> Result<usize, OperationError> {
        self.coordination_streams.prune();
        if self.is_shutdown() {
            return Ok(0);
        }
        let _barrier = self
            .coordination_gate
            .lock()
            .map_err(|_| OperationError::internal("coordination barrier is unavailable"))?;
        if self.is_shutdown() {
            return Ok(0);
        }
        let pending: Vec<crate::db::PendingDelivery> = self
            .db
            .pending_coordination()
            .map_err(|_| OperationError::internal("coordination outbox is unavailable"))?;
        if pending.is_empty() {
            return Ok(0);
        }
        // Automatic awards are high-volume projection changes, not accepted
        // user commands. They never create the private identity/audit store.
        // Manual mutations and every durable command still require audit.
        let records: Vec<_> = pending
            .iter()
            .filter(|item| item.audit_required)
            .map(|item| item.audit.clone())
            .collect();
        if !records.is_empty() {
            let audit = self.collaboration().map_err(|error| OperationError {
                code: error.code(),
                message: error.to_string(),
                data: None,
            })?;
            // All bounded audit chunks precede any publication. Importing a
            // previous chunk again after failure deduplicates its stable IDs.
            for chunk in records.chunks(100) {
                audit.import_audit(chunk).map_err(|_| {
                    OperationError::internal("coordination audit import is unavailable")
                })?;
            }
        }
        #[cfg(test)]
        if std::env::var("TIKTOOLS_DELIVERY_CRASH_STAGE").as_deref() == Ok("after_audit") {
            std::process::exit(88);
        }
        for item in &pending {
            self.coordination_streams
                .publish(&item.event)
                .map_err(|_| OperationError::internal("coordination publication is unavailable"))?;
            // No receivers is normal: a future subscriber starts from a fresh
            // state snapshot. Lagged active receivers must request recovery.
            let _ = self.coordination_events.send(Arc::new(item.event.clone()));
        }
        #[cfg(test)]
        if std::env::var("TIKTOOLS_DELIVERY_CRASH_STAGE").as_deref() == Ok("after_publish") {
            std::process::exit(89);
        }
        let tokens: Vec<crate::db::DeliveryToken> =
            pending.iter().map(|item| item.token.clone()).collect();
        self.db
            .acknowledge_coordination(&tokens)
            .map_err(|_| OperationError::internal("coordination acknowledgement is unavailable"))?;
        Ok(pending.len())
    }
}
