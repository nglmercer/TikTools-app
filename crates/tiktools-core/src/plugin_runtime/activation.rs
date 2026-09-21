//! Installed/enabled activation snapshot shared by plugin readiness checks.
//! (Readiness, retry bookkeeping, and poll lifecycle live in `runtime_state`.)

#[cfg(any(test, feature = "persistence"))]
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
