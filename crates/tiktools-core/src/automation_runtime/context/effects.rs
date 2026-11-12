//! Private effect execution identity and stable child occurrence slots.
use super::*;

impl ExecutionContext {
    #[cfg_attr(not(feature = "persistence"), allow(dead_code))]
    pub(crate) fn effect_child_slot(&self, namespace: &str, slot: u32) -> Result<Self, String> {
        #[cfg(feature = "persistence")]
        {
            let execution = self
                .execution()?
                .named_child("derived-output", namespace)
                .and_then(|execution| execution.child(slot))
                .map_err(|error| error.to_string())?;
            Ok(self.clone().with_execution(execution))
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (namespace, slot);
            Ok(self.clone())
        }
    }

    #[cfg(feature = "persistence")]
    pub(crate) fn with_execution(
        mut self,
        execution: crate::services::effects::EffectExecution,
    ) -> Self {
        self.execution = Some(execution);
        self
    }

    #[cfg(feature = "persistence")]
    pub(crate) fn execution(&self) -> Result<&crate::services::effects::EffectExecution, String> {
        self.execution
            .as_ref()
            .ok_or_else(|| "private execution identity is unavailable".into())
    }

    pub(crate) fn effect_named_slot(
        &self,
        event_id: &str,
        action_id: &str,
        occurrence: u32,
    ) -> Result<Self, String> {
        #[cfg(feature = "persistence")]
        {
            let execution = self
                .execution()?
                .named_child("saved-event", event_id)
                .and_then(|execution| execution.named_child("saved-action", action_id))
                .and_then(|execution| execution.child(occurrence))
                .map_err(|error| error.to_string())?;
            Ok(self.clone().with_execution(execution))
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (event_id, action_id, occurrence);
            Ok(self.clone())
        }
    }
}
