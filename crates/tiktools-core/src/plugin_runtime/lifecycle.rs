//! Starting plugins required by host background services.

use crate::*;

impl AppCore {
    /// Event observers are background plugin services rather than poll
    /// sources. Start enabled subscribers once when the host runtime starts;
    /// explicit stop/disable operations remain authoritative and do not get
    /// undone by the observer.
    pub(crate) fn start_enabled_event_subscribers(&self) {
        for plugin in self.plugins.list() {
            if plugin.running
                || !self.plugin_ready(&plugin.manifest.id)
                || plugin.manifest.runtime == tiktools_plugin_api::PluginRuntimeKind::Declarative
                || plugin.manifest.event_subscriptions.is_empty()
                || self
                    .capabilities
                    .require_capability(
                        &plugin.manifest,
                        tiktools_plugin_api::capabilities::EVENTS_SUBSCRIBE,
                    )
                    .is_err()
            {
                continue;
            }
            if let Err(error) = self.plugins.start(&plugin.manifest.id) {
                tracing::warn!(
                    plugin = %plugin.manifest.id,
                    %error,
                    "enabled event-subscriber plugin could not be started"
                );
            }
        }
    }
}
