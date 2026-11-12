//! Current authority checks, public event projection and host dispatch guards.
use super::*;

impl ExecutionContext {
    pub(crate) fn is_owner(&self) -> bool {
        matches!(self.authority, Authority::Owner)
    }

    pub(crate) fn is_approved(&self) -> bool {
        #[cfg(feature = "persistence")]
        return matches!(self.authority, Authority::Approved { .. });
        #[cfg(not(feature = "persistence"))]
        false
    }

    /// Revalidates identity/lifetime only. This does not approve definitions,
    /// grant effect capabilities or authorize any currently restricted effect.
    pub(crate) fn check_current(&self) -> Result<(), &'static str> {
        match &self.authority {
            Authority::Owner => Ok(()),
            Authority::Denied => Err("execution authority is unavailable"),
            Authority::Restricted(subjects) => subjects
                .snapshot()
                .map(|_| ())
                .map_err(|_| "saved execution subjects are unavailable"),
            #[cfg(feature = "persistence")]
            Authority::Approved {
                subjects, permits, ..
            } => subjects
                .with_current(|identities| {
                    for permit in permits {
                        permit.check_subjects(identities)?;
                    }
                    Ok(())
                })
                .map_err(|_| "saved execution subjects are unavailable")?,
        }
    }

    /// Approved renderers receive only public scalar fields and no globals.
    pub(crate) fn projected_event(
        &self,
        event: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        if self.is_owner() {
            return Ok(event.clone());
        }
        if !self.is_approved() {
            return Err("delegated execution approval is unavailable".into());
        }
        self.check_current().map_err(str::to_owned)?;
        #[cfg(feature = "persistence")]
        return Ok(crate::services::automation_approval_policy::project_event(
            event,
        ));
        #[cfg(not(feature = "persistence"))]
        Err("delegated execution approval is unavailable".into())
    }

    pub(crate) fn require_owner(&self) -> Result<(), &'static str> {
        if self.depth >= DEPTH_LIMIT {
            return Err("automation emit depth limit reached");
        }
        if self.is_owner() {
            Ok(())
        } else {
            Err("delegated execution approval is unavailable")
        }
    }
}

impl AppCore {
    /// Dispatch admits only trusted Owner or an approved retained chain. This
    /// guard is separate from the Owner-only generic transport/read guard.
    pub(crate) fn automation_dispatch_guard(
        &self,
        context: &ExecutionContext,
    ) -> Result<(), String> {
        if context.is_owner() {
            return self.execution_context_guard(context);
        }
        if self.is_shutdown() {
            return Err("host is shutting down".into());
        }
        #[cfg(feature = "persistence")]
        {
            let _writer = self.automation_gate().map_err(|error| error.to_string())?;
            context.with_current_chain(self, |_| Ok(()))
        }
        #[cfg(not(feature = "persistence"))]
        {
            Err("delegated execution requires persistence".into())
        }
    }

    /// Final unsupported-effect guard, before globals/settings, plugins or
    /// effect sinks. Shutdown applies even to otherwise trusted Owner work.
    /// Keep this guard Owner-only. Approved delegated effects need separate,
    /// closed effect/target policies; accepting a valid subject here would
    /// otherwise enable every unsupported transport and unrestricted read.
    pub(crate) fn execution_context_guard(&self, context: &ExecutionContext) -> Result<(), String> {
        if self.is_shutdown() {
            return Err("host is shutting down".into());
        }
        context.require_owner().map_err(str::to_owned)?;
        context.check_current().map_err(str::to_owned)
    }
}
