//! Installed/enabled state, readiness checks, and retry bookkeeping.

use crate::*;

/// Persisted install/enable state for one plugin, mirrored in memory so
/// readiness checks stay off SQLite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PluginActivation {
    pub(crate) installed: bool,
    pub(crate) enabled: bool,
}

/// Builds the activation snapshot from a behavior snapshot's `plugins`
/// array. Missing rows default to installed and enabled, matching the
/// runtime catalog merge.
#[cfg(any(test, feature = "persistence"))]
pub(crate) fn activation_from_snapshot(snapshot: &Value) -> BTreeMap<String, PluginActivation> {
    let mut activation = BTreeMap::new();
    if let Some(plugins) = snapshot.get("plugins").and_then(Value::as_array) {
        for state in plugins {
            let Some(id) = state.get("id").and_then(Value::as_str) else {
                continue;
            };
            activation.insert(
                id.to_owned(),
                PluginActivation {
                    installed: state
                        .get("installed")
                        .and_then(Value::as_bool)
                        .unwrap_or(true),
                    enabled: state
                        .get("enabled")
                        .and_then(Value::as_bool)
                        .unwrap_or(true),
                },
            );
        }
    }
    activation
}

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
        self.plugin_activation
            .read()
            .expect("plugin activation lock poisoned")
            .get(id)
            .is_none_or(|state| state.installed && state.enabled)
    }

    /// Records an install/enable write in the activation snapshot. Callers
    /// must persist first; this only moves the snapshot the hot path reads.
    pub(crate) fn set_plugin_activation(&self, id: &str, installed: bool, enabled: bool) {
        self.plugin_activation
            .write()
            .expect("plugin activation lock poisoned")
            .insert(id.to_owned(), PluginActivation { installed, enabled });
    }

    #[cfg(feature = "plugin-install")]
    pub(crate) fn clear_plugin_activation(&self, id: &str) {
        self.plugin_activation
            .write()
            .expect("plugin activation lock poisoned")
            .remove(id);
    }

    pub(crate) fn plugin_retry_allowed(&self, id: &str) -> bool {
        self.plugin_health
            .lock()
            .expect("plugin health lock poisoned")
            .get(id)
            .and_then(|health| health.next_retry_at)
            .is_none_or(|next_retry_at| std::time::Instant::now() >= next_retry_at)
    }

    pub(crate) fn record_plugin_failure(&self, id: &str, error: String) {
        let mut health = self
            .plugin_health
            .lock()
            .expect("plugin health lock poisoned");
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
        let was_unhealthy = self
            .plugin_health
            .lock()
            .expect("plugin health lock poisoned")
            .remove(id)
            .is_some_and(|health| health.consecutive_failures > 0);
        if was_unhealthy {
            tracing::info!(plugin = %id, "plugin recovered");
        }
    }
}
