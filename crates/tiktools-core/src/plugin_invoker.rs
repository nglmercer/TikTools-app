//! Shared timed plugin call path.
//!
//! Enrichment, actions, and the event poll all invoke plugins the same way:
//! start the instance, then await the loader's async call against a Tokio
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
}

impl std::fmt::Display for InvokeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout => formatter.write_str("plugin call timed out"),
            Self::Unavailable(reason) => write!(formatter, "plugin unavailable: {reason}"),
            Self::Plugin(reason) => write!(formatter, "plugin error: {reason}"),
        }
    }
}

/// Owns the async bridge between Tokio callers and the loader's worker
/// queue: deadline handling plus error mapping, with no blocking wait and
/// no `spawn_blocking` per call. Cheap to clone; every method takes `&self`
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
        // The loader call is async end to end (Tokio queue plus oneshot
        // response), so awaiting it directly spends no thread: the old
        // `spawn_blocking` wrapper and the extra Tokio-side timeout race
        // are gone, and the loader's own deadline is the single budget.
        self.plugins
            .call_with_timeout(plugin_id, request, effective)
            .await
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
    }
}
