//! Private execution authority travels beside event data, never inside it.
use crate::{automation_runtime::context::ExecutionContext, Value};

#[derive(Clone)]
pub(crate) struct AutomationEvent {
    payload: Value,
    context: ExecutionContext,
}
impl std::fmt::Debug for AutomationEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Assertion/logging helpers must not accidentally dump event contents.
        f.debug_struct("AutomationEvent")
            .field("owner", &self.context.is_owner())
            .field("depth", &self.context.depth())
            .finish_non_exhaustive()
    }
}

impl AutomationEvent {
    /// Explicit trusted host/Owner entry point. JSON `authority`, `source` or
    /// `depth` fields neither select nor change this private context.
    pub(crate) fn owner(payload: Value) -> Self {
        Self::new(payload, ExecutionContext::owner())
    }

    pub(crate) fn new(payload: Value, context: ExecutionContext) -> Self {
        Self { payload, context }
    }

    pub(crate) fn payload(&self) -> &Value {
        &self.payload
    }
    pub(crate) fn context(&self) -> &ExecutionContext {
        &self.context
    }
    pub(crate) fn into_parts(self) -> (Value, ExecutionContext) {
        (self.payload, self.context)
    }

    /// Processors may replace the data projection, never the execution subject.
    pub(crate) fn replace_payload(self, payload: Value) -> Self {
        Self {
            payload,
            context: self.context,
        }
    }
}

#[cfg(test)]
mod tests;
