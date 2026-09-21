//! Generic asynchronous delivery of domain events to subscribing plugins.
//!
//! This module is deliberately transport-neutral. It knows about the stable
//! plugin event envelope and plugin lifecycle/capability policy, but it does
//! not know whether a plugin forwards events to WebSockets, MQTT, OSC,
//! Discord, or another system.

mod delivery;
mod eligibility;
mod queue;
mod supervisor;
#[cfg(test)]
mod tests;
mod worker;

use std::sync::Arc;
use supervisor::run_observer;

use crate::AppCore;

impl AppCore {
    /// Starts the generic host-side observer once. This is separate from the
    /// plugin poller so future hosts can opt into either lifecycle component.
    pub fn spawn_plugin_event_observer(self: &Arc<Self>, runtime: &tokio::runtime::Handle) {
        if self
            .plugin_event_observer_started
            .swap(true, std::sync::atomic::Ordering::AcqRel)
        {
            return;
        }
        let core = Arc::clone(self);
        let shutdown = Arc::clone(&self.plugin_event_observer_shutdown);
        // Subscribe before spawning so an event published immediately after
        // this method returns cannot race the observer's first poll.
        let subscription = self.events.subscribe_domain();
        tracing::info!("plugin event observer started");
        let task = runtime.spawn(async move {
            run_observer(core, shutdown, subscription).await;
        });
        *self
            .plugin_event_observer_task
            .lock()
            .expect("plugin event observer task lock poisoned") = Some(task);
    }
}
