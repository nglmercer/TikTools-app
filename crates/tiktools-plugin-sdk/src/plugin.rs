use serde::Serialize;
use serde_json::Value;

use crate::{
    ActionCall, ActionResult, EventEnrichmentRequest, EventEnrichmentResult, PluginCall,
    PluginCallResult, PluginContext, PluginError, PluginResult, PollResult,
};
use tiktools_plugin_api::DomainEventEnvelope;

/// Runtime-neutral plugin business-logic trait.
pub trait Plugin: Send + 'static {
    fn initialize(&mut self, _context: &PluginContext) -> PluginResult<()> {
        Ok(())
    }

    fn action(
        &mut self,
        _context: &PluginContext,
        _call: ActionCall,
    ) -> PluginResult<ActionResult> {
        Err(PluginError::unsupported("action"))
    }

    fn poll(&mut self, _context: &PluginContext) -> PluginResult<PollResult> {
        Ok(PollResult::default())
    }

    /// Enriches one host event with derived annotations. The default is a
    /// no-op result so existing plugins compile unchanged; the host only
    /// calls this for plugins that declare `processorTypes`.
    ///
    /// ```rust
    /// use tiktools_plugin_sdk::prelude::*;
    ///
    /// #[derive(Default)]
    /// struct TextProcessor;
    ///
    /// impl Plugin for TextProcessor {
    ///     fn enrich(
    ///         &mut self,
    ///         _context: &PluginContext,
    ///         request: EventEnrichmentRequest,
    ///     ) -> PluginResult<EventEnrichmentResult> {
    ///         let comment = request
    ///             .event
    ///             .pointer("/data/comment")
    ///             .and_then(|value| value.as_str())
    ///             .unwrap_or_default();
    ///         let mut result = EventEnrichmentResult::default();
    ///         result.annotations.insert(
    ///             "comment".to_owned(),
    ///             serde_json::json!({"normalized": comment.trim()}),
    ///         );
    ///         Ok(result)
    ///     }
    /// }
    /// ```
    fn enrich(
        &mut self,
        _context: &PluginContext,
        _request: EventEnrichmentRequest,
    ) -> PluginResult<EventEnrichmentResult> {
        Ok(EventEnrichmentResult::default())
    }

    /// Observes one stable serialized host event. Delivery is asynchronous
    /// from the host's domain-event publisher and is isolated behind a
    /// bounded per-plugin queue.
    fn event(&mut self, _context: &PluginContext, _event: DomainEventEnvelope) -> PluginResult<()> {
        Ok(())
    }

    fn shutdown(&mut self, _context: &PluginContext) -> PluginResult<()> {
        Ok(())
    }
}

fn serialize_call_result<T: Serialize>(result: T) -> PluginResult<Value> {
    serde_json::to_value(result)
        .map_err(|error| PluginError::other(format!("could not encode plugin result: {error}")))
}

/// Routes one typed call to plugin business logic and returns the serialized
/// typed result. Action and poll calls serialize to the same
/// `PluginCallResult` JSON as before; enrich calls serialize to
/// `EventEnrichmentResult` JSON, which carries no side-effect intents.
pub fn dispatch_plugin_call<P: Plugin>(
    plugin: &mut P,
    context: &PluginContext,
    call: PluginCall,
) -> PluginResult<Value> {
    match call {
        PluginCall::Action { action, event } => plugin
            .action(context, ActionCall { action, event })
            .map(PluginCallResult::from)
            .and_then(serialize_call_result),
        PluginCall::Poll => plugin
            .poll(context)
            .map(PluginCallResult::from)
            .and_then(serialize_call_result),
        PluginCall::Enrich { request } => plugin
            .enrich(context, request)
            .and_then(serialize_call_result),
        PluginCall::Event { event } => plugin.event(context, event).and_then(serialize_call_result),
    }
}
