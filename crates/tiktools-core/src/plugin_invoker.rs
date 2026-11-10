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
        self.call_identified(plugin_id, request, timeout)
            .await
            .map(|(_, value)| value)
    }

    /// Return the precise worker generation that produced a successful
    /// response, allowing asynchronous consumers to reject stale data.
    pub(crate) async fn call_identified(
        &self,
        plugin_id: &str,
        request: &Value,
        timeout: Duration,
    ) -> Result<(u64, Value), InvokeError> {
        let generation = self.start_generation(plugin_id).await?;
        let value = self
            .call_started(plugin_id, request, timeout, generation)
            .await?;
        Ok((generation, value))
    }

    /// Poll only a generation whose own frozen manifest authorizes event or
    /// progress production. The source reservation spans the poll itself so a
    /// package cannot replace that worker between authorization and the call.
    pub(crate) async fn call_poll_identified(
        &self,
        plugin_id: &str,
        request: &Value,
        timeout: Duration,
    ) -> Result<Option<(u64, Value)>, InvokeError> {
        // Background polling observes only an already-running generation. A
        // plugin the user disabled (or whose worker is gone) must not be
        // restarted by the periodic poll.
        let Some(generation) = self.plugins.running_generation(plugin_id) else {
            return Ok(None);
        };
        let Some(delivery) = self
            .plugins
            .try_reserve_event_delivery(plugin_id, generation)
        else {
            return Ok(None);
        };
        let manifest = delivery.manifest();
        let events_allowed = tiktools_plugin_api::capabilities::declares_capability(
            manifest,
            tiktools_plugin_api::capabilities::EVENTS_PUBLISH,
        ) && !tiktools_plugin_api::manifest::declared_event_types(manifest)
            .is_empty();
        let progress_allowed = tiktools_plugin_api::capabilities::declares_capability(
            manifest,
            crate::helpers::PLUGIN_PROGRESS_CAPABILITY,
        );
        if !events_allowed && !progress_allowed {
            return Ok(None);
        }
        let value = self
            .call_started(plugin_id, request, timeout, generation)
            .await?;
        drop(delivery);
        Ok(Some((generation, value)))
    }

    async fn start_generation(&self, plugin_id: &str) -> Result<u64, InvokeError> {
        // Runtime initialization can execute native guest code. Keep it off
        // Tokio workers; cancellation cannot stop that code, so manager
        // shutdown retains its admission fence and reports an unsafe update.
        let plugins = Arc::clone(&self.plugins);
        let id = plugin_id.to_owned();
        tokio::task::spawn_blocking(move || plugins.start_generation(&id))
            .await
            .map_err(|error| {
                InvokeError::Unavailable(format!("plugin startup task failed: {error}"))
            })?
            .map_err(|error| InvokeError::Unavailable(error.to_string()))
    }

    /// Invokes an already-running plugin without implicitly starting it.
    /// Background observers use this boundary so a manually stopped or
    /// crashed plugin cannot be resurrected by a later domain event.
    pub(crate) async fn call_running(
        &self,
        plugin_id: &str,
        request: &Value,
        timeout: Duration,
        generation: u64,
    ) -> Result<Value, InvokeError> {
        if !self.plugins.is_running(plugin_id) {
            return Err(InvokeError::Unavailable(format!(
                "plugin `{plugin_id}` is not running"
            )));
        }
        self.plugins
            .call_with_generation(plugin_id, request, timeout, generation)
            .await
            .map_err(|error| match error {
                PluginLoaderError::Timeout(_) => InvokeError::Timeout,
                PluginLoaderError::Overloaded(_) | PluginLoaderError::RuntimeUnavailable(_) => {
                    InvokeError::Unavailable(error.to_string())
                }
                error => InvokeError::Plugin(error.to_string()),
            })
    }

    pub(crate) async fn call_started(
        &self,
        plugin_id: &str,
        request: &Value,
        timeout: Duration,
        generation: u64,
    ) -> Result<Value, InvokeError> {
        // The loader call is async end to end (Tokio queue plus oneshot
        // response), so awaiting it directly spends no thread: the old
        // `spawn_blocking` wrapper and the extra Tokio-side timeout race
        // are gone, and the loader's own deadline is the single budget.
        self.plugins
            .call_with_generation(plugin_id, request, timeout, generation)
            .await
            .map_err(|error| match error {
                PluginLoaderError::Timeout(_) => InvokeError::Timeout,
                PluginLoaderError::RuntimeUnavailable(reason) => InvokeError::Unavailable(reason),
                PluginLoaderError::Overloaded(reason) => {
                    InvokeError::Unavailable(format!("call queue full: {reason}"))
                }
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
