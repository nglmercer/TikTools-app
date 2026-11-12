//! Captured-session eligibility and publication/emission guard adapters.
use super::{WorkspaceCoordError, WorkspaceEligibility};
use cloud_client::Session;
use std::sync::Arc;

impl WorkspaceEligibility {
    pub fn new(session: Arc<Session>, current: impl Fn() -> bool + Send + Sync + 'static) -> Self {
        Self {
            binding: Arc::new(()),
            session,
            current: Arc::new(current),
            publication: None,
            emission: None,
            emission_current: None,
        }
    }

    /// Product dispatcher only: revalidate and hold its session/consent
    /// boundary until the workspace's prepared state is atomically renamed.
    pub(crate) fn with_publication_guard(
        mut self,
        guard: impl Fn(
                &mut dyn FnMut() -> Result<(), WorkspaceCoordError>,
            ) -> Result<(), WorkspaceCoordError>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        self.publication = Some(Arc::new(guard));
        self
    }

    /// Product-only session admission held through the local HTTP future.
    /// Cancellation can still leave work running on the server; the durable
    /// request and backend barrier remain necessary for recovery.
    pub(crate) fn with_emission_guard(
        mut self,
        guard: impl Fn() -> Result<Box<dyn Send>, WorkspaceCoordError> + Send + Sync + 'static,
        current: impl Fn() -> bool + Send + Sync + 'static,
    ) -> Self {
        self.emission = Some(Arc::new(guard));
        self.emission_current = Some(Arc::new(current));
        self
    }

    pub fn check(&self) -> Result<(), WorkspaceCoordError> {
        if (self.current)() {
            Ok(())
        } else {
            Err(WorkspaceCoordError::Ineligible)
        }
    }

    pub(crate) fn publish(
        &self,
        publish: &mut dyn FnMut() -> Result<(), WorkspaceCoordError>,
    ) -> Result<(), WorkspaceCoordError> {
        if let Some(guard) = &self.publication {
            guard(publish)
        } else {
            // Generic workspace users supply only an eligibility predicate.
            // Product wiring installs the stronger publication guard above.
            self.check()?;
            publish()
        }
    }

    #[cfg(test)]
    pub(super) fn with_test_publication_guard(self) -> Self {
        self.with_publication_guard(|publish| publish())
            .with_test_emission_guard()
    }

    #[cfg(test)]
    pub(super) fn with_test_emission_guard(self) -> Self {
        self.with_emission_guard(|| Ok(Box::new(())), || true)
    }

    /// Pause only in tests after the real session permit is acquired and
    /// before the nonblocking transition check.
    #[cfg(test)]
    pub(crate) fn with_test_emission_pause(
        mut self,
        pause: impl Fn() + Send + Sync + 'static,
    ) -> Self {
        let guard = self.emission.take().expect("test requires emission guard");
        self.emission = Some(Arc::new(move || {
            let permit = guard()?;
            pause();
            Ok(permit)
        }));
        self
    }
}

pub(super) fn check_eligibility(
    eligibility: Option<&WorkspaceEligibility>,
    session: &Session,
) -> Result<(), WorkspaceCoordError> {
    if let Some(eligibility) = eligibility {
        if !std::ptr::eq(eligibility.session.as_ref(), session) {
            return Err(WorkspaceCoordError::Ineligible);
        }
        eligibility.check()?;
    }
    Ok(())
}
