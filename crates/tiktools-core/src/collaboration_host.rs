//! Lazy host-owned collaboration storage and compiled gateway availability.
use super::*;

impl AppCore {
    /// Open the private collaboration store on the caller's blocking pool.
    /// Local-only startup creates no credentials, listener or extra database.
    pub fn collaboration(
        &self,
    ) -> Result<Arc<collaboration::CollaborationService>, collaboration::Error> {
        if !cfg!(feature = "persistence") {
            return Err(collaboration::Error::Disabled);
        }
        let mut current = self.collaboration.lock().map_err(|_| {
            collaboration::Error::Invalid("collaboration state lock is unavailable")
        })?;
        if let Some(service) = current.as_ref() {
            return Ok(Arc::clone(service));
        }
        let service = Arc::new(collaboration::CollaborationService::open(
            &self.db.paths().data.join("collaboration"),
        )?);
        service.update_gateway_status(|status| {
            status.available = self.collaboration_gateway_available.load(Ordering::SeqCst)
        })?;
        *current = Some(Arc::clone(&service));
        Ok(service)
    }

    /// Only the native host enables its compiled-in network component. This
    /// never starts a listener or opens a database by itself.
    pub fn set_collaboration_gateway_available(&self, available: bool) {
        self.collaboration_gateway_available
            .store(available, Ordering::SeqCst);
        if let Ok(current) = self.collaboration.lock() {
            if let Some(service) = current.as_ref() {
                let _ = service.update_gateway_status(|status| status.available = available);
            }
        }
    }
}
