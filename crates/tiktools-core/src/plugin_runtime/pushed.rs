//! Guest-pushed events (`tiktools:events`): the push counterpart to the
//! poll loop. The loader validates each emit at the call site (shape,
//! declared type, size); this forwarder re-validates authoritatively
//! before delivery: the plugin must still be known and ready, the type
//! must be declared, and the manifest must grant `events.publish`.
//! Anything else is dropped and counted, exactly like an invalid poll
//! response. Accepted events flow through the same
//! `make_plugin_event` + `publish_automation_event` path as polled ones,
//! so downstream automation cannot tell push from poll.

use crate::*;

impl AppCore {
    pub(crate) async fn handle_pushed_event(
        self: &Arc<Self>,
        emitted: tiktools_plugin_loader::EmittedEvent,
    ) {
        let plugin_id = emitted.plugin_id.clone();
        // A queued event belongs to one worker, not merely to its plugin ID.
        // Do not attribute an old worker's events to a replacement instance.
        let Some(_delivery) = self
            .plugins
            .try_reserve_event_delivery(&plugin_id, emitted.generation)
        else {
            return;
        };
        if !self.plugin_ready(&plugin_id) {
            // Stopped or failed while the emit was in flight.
            return;
        }
        let manifest = _delivery.manifest();
        let Some(current) = self.plugins.get(&plugin_id) else {
            return;
        };
        if !declared_event_types(manifest).contains(&emitted.event_type)
            || !declared_event_types(&current.manifest).contains(&emitted.event_type)
        {
            self.record_plugin_drops(&plugin_id, 1);
            return;
        }
        if self
            .capabilities
            .require_capability(manifest, tiktools_plugin_api::capabilities::EVENTS_PUBLISH)
            .is_err()
            || self
                .capabilities
                .require_capability(
                    &current.manifest,
                    tiktools_plugin_api::capabilities::EVENTS_PUBLISH,
                )
                .is_err()
        {
            self.record_plugin_drops(&plugin_id, 1);
            return;
        }
        // Owner plugin producers may replay only remembered Owner payloads.
        // A retained scoped envelope must never become ambient Owner context.
        let source = fresh_poll_context(&self.automation_context().0.unwrap_or_else(|| json!({})));
        let Ok(event) = self.make_plugin_event(
            &plugin_id,
            &source,
            &emitted.event_type,
            emitted.data,
            &crate::automation_runtime::context::ExecutionContext::owner(),
        ) else {
            return;
        };
        // Stops that are not package mutations do not yet participate in the
        // delivery permit. Preserve the existing late-generation check for
        // those paths until they acquire the same admission boundary.
        if !self
            .plugins
            .is_live_generation(&plugin_id, emitted.generation)
        {
            return;
        }
        self.publish_automation_event(event).await;
        self.record_plugin_success(&plugin_id);
    }
}
