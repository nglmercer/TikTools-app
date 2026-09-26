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
        let Some(plugin) = self.plugins.get(&plugin_id) else {
            // Unknown or rescanned-away plugin: stale in-flight emit.
            return;
        };
        if !self.plugin_ready(&plugin_id) {
            // Stopped or failed while the emit was in flight.
            return;
        }
        if !declared_event_types(&plugin.manifest).contains(&emitted.event_type) {
            self.record_plugin_drops(&plugin_id, 1);
            return;
        }
        if self
            .capabilities
            .require_capability(
                &plugin.manifest,
                tiktools_plugin_api::capabilities::EVENTS_PUBLISH,
            )
            .is_err()
        {
            self.record_plugin_drops(&plugin_id, 1);
            return;
        }
        let source = fresh_poll_context(
            &recover_rwlock_read(&self.automation_state.last_event, "automation event")
                .clone()
                .unwrap_or_else(|| json!({})),
        );
        self.publish_automation_event(self.make_plugin_event(
            &plugin_id,
            &source,
            &emitted.event_type,
            emitted.data,
        ))
        .await;
        self.record_plugin_success(&plugin_id);
    }
}
