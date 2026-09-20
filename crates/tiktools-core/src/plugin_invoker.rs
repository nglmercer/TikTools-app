//! Shared timed plugin call path.
//!
//! Enrichment, actions, and the event poll all invoke plugins the same way:
//! start the instance, then race a blocking loader call against a Tokio
//! deadline. Centralizing that bridge here keeps timeout semantics identical
//! everywhere and gives every caller the same typed failure.

use std::{sync::Arc, time::Duration};

use serde_json::Value;
use tiktools_plugin_loader::{PluginLoaderError, PluginManager};

/// Typed failure of one timed plugin call. Callers map this onto their own
/// error type so user-facing messages stay specific to each call path.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum InvokeError {
    Timeout,
    Unavailable(String),
    Plugin(String),
    Join(String),
}

impl std::fmt::Display for InvokeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout => formatter.write_str("plugin call timed out"),
            Self::Unavailable(reason) => write!(formatter, "plugin unavailable: {reason}"),
            Self::Plugin(reason) => write!(formatter, "plugin error: {reason}"),
            Self::Join(reason) => write!(formatter, "plugin task failed: {reason}"),
        }
    }
}

/// Owns the `timeout + spawn_blocking` bridge between Tokio callers and the
/// loader's blocking worker queue. Cheap to clone; every method takes `&self`
/// so one instance serves the whole core.
#[derive(Clone)]
pub(crate) struct PluginInvoker {
    plugins: Arc<PluginManager>,
}

impl PluginInvoker {
    pub(crate) fn new(plugins: Arc<PluginManager>) -> Self {
        Self { plugins }
    }

    pub(crate) fn plugins(&self) -> &Arc<PluginManager> {
        &self.plugins
    }

    /// Starts `plugin_id` when needed, then invokes it with `request`. The
    /// Tokio deadline and the loader's worker deadline cover the same
    /// effective budget: the first call of a process generation claims the
    /// loader's cold-start grace so a child still building its engine is not
    /// failed by the steady-state `timeout`. Whichever deadline fires first
    /// wins (see `PluginManager::call_with_deadline` for retire semantics).
    pub(crate) async fn call(
        &self,
        plugin_id: &str,
        request: &Value,
        timeout: Duration,
    ) -> Result<Value, InvokeError> {
        if let Err(error) = self.plugins.start(plugin_id) {
            return Err(InvokeError::Unavailable(error.to_string()));
        }
        self.call_started(plugin_id, request, timeout).await
    }

    /// Invokes an already-running plugin without implicitly starting it.
    /// Background observers use this boundary so a manually stopped or
    /// crashed plugin cannot be resurrected by a later domain event.
    pub(crate) async fn call_running(
        &self,
        plugin_id: &str,
        request: &Value,
        timeout: Duration,
    ) -> Result<Value, InvokeError> {
        if !self.plugins.is_running(plugin_id) {
            return Err(InvokeError::Unavailable(format!(
                "plugin `{plugin_id}` is not running"
            )));
        }
        self.call_started(plugin_id, request, timeout).await
    }

    async fn call_started(
        &self,
        plugin_id: &str,
        request: &Value,
        timeout: Duration,
    ) -> Result<Value, InvokeError> {
        let effective = self.plugins.claim_cold_start_grace(plugin_id, timeout);
        let plugins = Arc::clone(&self.plugins);
        let plugin_id_owned = plugin_id.to_owned();
        let request_owned = request.clone();
        tokio::time::timeout(
            effective,
            tokio::task::spawn_blocking(move || {
                plugins.call_with_timeout(&plugin_id_owned, &request_owned, effective)
            }),
        )
        .await
        .map_err(|_| InvokeError::Timeout)?
        .map_err(|error| InvokeError::Join(error.to_string()))?
        .map_err(|error| match error {
            PluginLoaderError::Timeout(_) => InvokeError::Timeout,
            error => InvokeError::Plugin(error.to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invoke_errors_render_for_logs() {
        assert_eq!(InvokeError::Timeout.to_string(), "plugin call timed out");
        assert_eq!(
            InvokeError::Unavailable("gone".to_owned()).to_string(),
            "plugin unavailable: gone"
        );
        assert_eq!(
            InvokeError::Join("panic".to_owned()).to_string(),
            "plugin task failed: panic"
        );
    }
}
