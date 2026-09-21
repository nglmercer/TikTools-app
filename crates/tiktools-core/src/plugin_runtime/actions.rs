//! Plugin action lookup and execution orchestration.

use super::{manifest_action_declares_field, PluginActionDescriptor};
use crate::*;

impl AppCore {
    pub(crate) async fn execute_plugin_action(
        self: &Arc<Self>,
        type_id: &str,
        action: &Value,
        event: &Value,
        logs: &mut Vec<String>,
        test: bool,
    ) -> Result<String, String> {
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
                return self
                    .execute_declarative_action(&plugin, action_http, action, event, logs, test)
                    .await;
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
        let response = invoker
            .call(&plugin.manifest.id, &request, action_timeout)
            .await
            .map_err(|error| match error {
                crate::plugin_invoker::InvokeError::Timeout => format!(
                    "plugin `{}` timed out after {} seconds",
                    plugin.manifest.id,
                    action_timeout.as_secs()
                ),
                crate::plugin_invoker::InvokeError::Join(reason) => {
                    format!("plugin task failed: {reason}")
                }
                crate::plugin_invoker::InvokeError::Unavailable(reason)
                | crate::plugin_invoker::InvokeError::Plugin(reason) => reason,
            })?;
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
            .execute_plugin_intents(&plugin, typed.intents, event, logs, test)
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
