//! Plugin runtime-state operations: install/enable activation, readiness
//! checks, retry bookkeeping, and poll-task lifecycle.

use crate::plugin_runtime::PluginActivation;
use crate::*;

impl AppCore {
    pub(crate) fn plugin_ready(&self, id: &str) -> bool {
        let Some(plugin) = self.plugins.get(id) else {
            return false;
        };
        if !plugin.available {
            return false;
        }
        // Activation is an in-memory snapshot refreshed on every state write,
        // so readiness checks never hit SQLite on hot paths. Plugins without
        // a persisted row default to installed and enabled.
        read_or_recover(&self.plugin_state.activation, "plugin activation")
            .get(id)
            .is_none_or(|state| state.installed && state.enabled)
    }

    /// Records an install/enable write in the activation snapshot. Callers
    /// must persist first; this only moves the snapshot the hot path reads.
    pub(crate) fn set_plugin_activation(&self, id: &str, installed: bool, enabled: bool) {
        write_or_recover(&self.plugin_state.activation, "plugin activation")
            .insert(id.to_owned(), PluginActivation { installed, enabled });
    }

    #[cfg(feature = "plugin-install")]
    pub(crate) fn clear_plugin_activation(&self, id: &str) {
        write_or_recover(&self.plugin_state.activation, "plugin activation").remove(id);
    }

    pub(crate) fn plugin_retry_allowed(&self, id: &str) -> bool {
        mutex_or_recover(&self.plugin_state.health, "plugin health")
            .get(id)
            .and_then(|health| health.next_retry_at)
            .is_none_or(|next_retry_at| std::time::Instant::now() >= next_retry_at)
    }

    pub(crate) fn record_plugin_failure(&self, id: &str, error: String) {
        let mut health = mutex_or_recover(&self.plugin_state.health, "plugin health");
        let entry = health.entry(id.to_owned()).or_insert(PluginHealth {
            consecutive_failures: 0,
            next_retry_at: None,
        });
        entry.consecutive_failures = entry.consecutive_failures.saturating_add(1);
        let delay_seconds = plugin_backoff_seconds(entry.consecutive_failures);
        entry.next_retry_at =
            Some(std::time::Instant::now() + std::time::Duration::from_secs(delay_seconds));
        if entry.consecutive_failures <= 5 {
            tracing::warn!(
                plugin = %id,
                failures = entry.consecutive_failures,
                retry_in_seconds = delay_seconds,
                %error,
                "plugin entered retry backoff"
            );
        }
    }

    pub(crate) fn record_plugin_success(&self, id: &str) {
        let was_unhealthy = mutex_or_recover(&self.plugin_state.health, "plugin health")
            .remove(id)
            .is_some_and(|health| health.consecutive_failures > 0);
        if was_unhealthy {
            tracing::info!(plugin = %id, "plugin recovered");
        }
    }

    /// Starts the background poll that lets plugins publish spontaneous
    /// events (hotkeys, timers, watchers).
    pub fn spawn_plugin_event_poll(self: &Arc<Self>, runtime: &tokio::runtime::Handle) {
        if self
            .shutdown_started
            .load(std::sync::atomic::Ordering::Acquire)
            || self
                .plugin_state
                .poll_started
                .swap(true, std::sync::atomic::Ordering::AcqRel)
        {
            return;
        }
        self.start_enabled_event_subscribers();
        self.spawn_plugin_event_observer(runtime);
        let core = Arc::clone(self);
        let shutdown = Arc::clone(&self.plugin_state.poll_shutdown);
        tracing::info!("plugin event poll started");
        let task = runtime.spawn(async move {
            let mut ticker = tokio::time::interval(PLUGIN_POLL_INTERVAL);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tokio::select! {
                    _ = shutdown.notified() => break,
                    _ = ticker.tick() => core.poll_plugin_events().await,
                }
            }
        });
        *mutex_or_recover(&self.plugin_state.poll_task, "plugin poll task") = Some(task);
    }
}
