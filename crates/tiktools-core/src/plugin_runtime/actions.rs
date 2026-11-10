//! Plugin action lookup and execution orchestration.

use super::{manifest_action_declares_field, PluginActionDescriptor};
use crate::automation_runtime::context::ExecutionContext;
use crate::*;

impl AppCore {
    pub(crate) async fn execute_plugin_action(
        self: &Arc<Self>,
        type_id: &str,
        action: &Value,
        event: &Value,
        context: &ExecutionContext,
        logs: &mut Vec<String>,
        test: bool,
    ) -> Result<String, String> {
        self.execution_context_guard(context)?;
        let Some((plugin, descriptor)) = self.plugin_for_action(type_id) else {
            return Err(format!(
                "Action type `{type_id}` is not available in this host."
            ));
        };
        if !self.plugin_ready(&plugin.manifest.id) {
            return Err(format!(
                "Plugin `{}` is not installed, enabled, or available.",
                plugin.manifest.id
            ));
        }
        let requires_audio_output = descriptor.required_capabilities.iter().any(|capability| {
            tiktools_plugin_api::capabilities::capability_matches(
                capability,
                tiktools_plugin_api::CAPABILITY_AUDIO_PLAY,
            )
        });
        for capability in &descriptor.required_capabilities {
            self.capabilities
                .require_capability(&plugin.manifest, capability)
                .map_err(|error| error.to_string())?;
        }
        if requires_audio_output {
            self.capabilities
                .require_permission(
                    &plugin.manifest,
                    tiktools_plugin_api::capabilities::AUDIO_OUTPUT_PERMISSION,
                )
                .map_err(|error| error.to_string())?;
        }
        // Declarative actions (schema v3 `http` block) run in the host
        // through the automation HTTP engine: no process call, no foreign
        // code. The executor describes test runs without sending.
        if plugin.manifest.schema_version >= 3 {
            if let Some(action_http) = descriptor.http.as_ref() {
                self.capabilities
                    .require_capability(
                        &plugin.manifest,
                        crate::services::declarative_http::HTTP_REQUEST_CAPABILITY,
                    )
                    .map_err(|error| error.to_string())?;
                let execution = self.execute_declarative_action(
                    &plugin,
                    action_http,
                    action,
                    event,
                    context,
                    logs,
                    test,
                );
                if !test
                    && action_http
                        .pointer("/idempotency/enabled")
                        .and_then(Value::as_bool)
                        == Some(true)
                {
                    // Dropping the HTTP future cancels client work. A command
                    // already accepted by the game may still complete: never
                    // recover an uncertain cancellation with a new key.
                    let cancelled = async {
                        loop {
                            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                            if !self.plugin_ready(&plugin.manifest.id) {
                                break;
                            }
                        }
                    };
                    return tokio::select! {
                        result = execution => result,
                        _ = cancelled => Err("Plugin disabled or unloaded; request cancelled. Game outcome may be uncertain; do not automatically replay.".to_owned()),
                    };
                }
                return execution.await;
            }
        }
        if test {
            return Ok(format!(
                "would run plugin {} action {type_id}",
                plugin.manifest.id
            ));
        }

        let action_timeout = descriptor.timeout();
        let request = serde_json::to_value(tiktools_plugin_sdk::PluginCall::action(
            action.clone(),
            event.clone(),
        ))
        .map_err(|error| format!("could not encode plugin action: {error}"))?;
        let invoker = crate::plugin_invoker::PluginInvoker::new(Arc::clone(&self.plugins));
        self.execution_context_guard(context)?;
        let invoke_error = |error: crate::plugin_invoker::InvokeError| match error {
            crate::plugin_invoker::InvokeError::Timeout => format!(
                "plugin `{}` timed out after {} seconds",
                plugin.manifest.id,
                action_timeout.as_secs()
            ),
            crate::plugin_invoker::InvokeError::Unavailable(reason)
            | crate::plugin_invoker::InvokeError::Plugin(reason) => reason,
        };
        // Start only while still enabled, under the same gate disable uses:
        // a dispatch that passed the readiness check above cannot resurrect a
        // plugin the user disabled in the meantime.
        let generation = self
            .start_plugin_for_invocation(&plugin.manifest.id)
            .await
            .map_err(invoke_error)?;
        let response = invoker
            .call_started(&plugin.manifest.id, &request, action_timeout, generation)
            .await
            .map_err(invoke_error)?;
        // A response can arrive after recovery quarantined its plugin. Do
        // not accept its returned intents in that case. This check alone does
        // not serialize later effects with a concurrent quarantine transition.
        if self.plugin_apply_quarantined(&plugin.manifest.id) {
            return Err("Plugin application requires recovery.".to_owned());
        }
        self.execution_context_guard(context)?;
        self.events
            .publish_domain(crate::events::DomainEvent::PluginProgress {
                plugin_id: plugin.manifest.id.clone(),
                state: "action-result".to_owned(),
                progress: None,
                message: format!("action `{type_id}` completed"),
            });

        let typed = tiktools_plugin_sdk::decode_plugin_result(response)
            .map_err(|error| format!("invalid plugin result: {error}"))?;
        for log in typed.logs {
            if logs.len() < 40 {
                logs.push(log);
            }
        }
        let mut parts = self
            .execute_plugin_intents(&plugin, typed.intents, event, context, logs, test)
            .await?;
        if let Some(summary) = typed.summary {
            parts.push(summary);
        }
        if parts.is_empty() {
            parts.push(format!("plugin {} completed", plugin.manifest.id));
        }
        Ok(parts.join(" · "))
    }

    pub(crate) fn plugin_for_action(
        &self,
        type_id: &str,
    ) -> Option<(
        tiktools_plugin_loader::DiscoveredPlugin,
        PluginActionDescriptor,
    )> {
        self.plugins.list().into_iter().find_map(|plugin| {
            plugin.manifest.action_types.iter().find_map(|descriptor| {
                let descriptor =
                    serde_json::from_value::<PluginActionDescriptor>(descriptor.clone()).ok()?;
                (descriptor.id == type_id).then_some((plugin.clone(), descriptor))
            })
        })
    }

    /// True when the named action's manifest descriptor declares a config
    /// field with this key. The immediate-execution IPC uses it to keep the
    /// TTS tester text requirement without imposing it on textless actions
    /// (output switching, toggles) that legitimately send no text.
    pub(crate) fn action_declares_field(&self, type_id: &str, key: &str) -> bool {
        let Some((plugin, _)) = self.plugin_for_action(type_id) else {
            return false;
        };
        manifest_action_declares_field(&plugin.manifest, type_id, key)
    }
}

impl AppCore {
    /// Starts `id` for an invocation only while it is still installed and
    /// enabled. Disable and stop run under `invocation_gate`, so checking
    /// readiness and starting under that gate is atomic against them.
    pub(crate) async fn start_plugin_for_invocation(
        self: &Arc<Self>,
        id: &str,
    ) -> Result<u64, crate::plugin_invoker::InvokeError> {
        use crate::plugin_invoker::InvokeError;
        let core = Arc::clone(self);
        let id = id.to_owned();
        tokio::task::spawn_blocking(move || {
            let _gate = core.plugin_state.invocation_gate.lock().map_err(|_| {
                InvokeError::Unavailable("plugin invocation lease is unavailable".to_owned())
            })?;
            if !core.plugin_ready(&id) {
                return Err(InvokeError::Unavailable(format!(
                    "Plugin `{id}` is not installed, enabled, or available."
                )));
            }
            core.plugins
                .start_generation(&id)
                .map_err(|error| InvokeError::Unavailable(error.to_string()))
        })
        .await
        .map_err(|error| InvokeError::Unavailable(format!("plugin startup task failed: {error}")))?
    }
}
