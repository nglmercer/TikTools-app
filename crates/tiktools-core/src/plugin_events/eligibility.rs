//! Subscription matching and delivery eligibility policy.

use crate::AppCore;
use tiktools_plugin_api::capabilities::EVENTS_SUBSCRIBE;
use tiktools_plugin_api::event_subscription_matches;
use tiktools_plugin_loader::DiscoveredPlugin;

pub(crate) fn eligible_for_event(
    core: &AppCore,
    plugin: &DiscoveredPlugin,
    topic: &str,
    force_all_subscribers: bool,
) -> bool {
    eligible_for_delivery(core, plugin)
        && (force_all_subscribers
            || plugin
                .manifest
                .event_subscriptions
                .iter()
                .any(|subscription| event_subscription_matches(subscription, topic)))
}

/// Topic-independent delivery eligibility. Used both for per-event fan-out
/// and for pruning workers whose plugin can no longer receive events.
pub(crate) fn eligible_for_delivery(core: &AppCore, plugin: &DiscoveredPlugin) -> bool {
    // Declarative packages have no process call boundary and cannot implement
    // the SDK event hook. A plugin must also be both enabled and running;
    // observing events must never resurrect a stopped or crashed instance.
    plugin.manifest.runtime != tiktools_plugin_api::PluginRuntimeKind::Declarative
        && plugin.running
        && core.plugin_ready(&plugin.manifest.id)
        && core
            .capabilities
            .require_capability(&plugin.manifest, EVENTS_SUBSCRIBE)
            .is_ok()
        && !plugin.manifest.event_subscriptions.is_empty()
}
