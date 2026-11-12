//! Scoped option reads and action execution. Ownership resolves through the catalog, and live
//! execution invalidates only its own option cache.

use super::PluginActionOutcome;
use crate::{
    control::{clean_plugin_id, OperationError},
    is_identifier, now_millis, AppCore,
};
use serde_json::{json, Value};
use std::{collections::BTreeMap, sync::Arc};

impl AppCore {
    /// Resolves `action-type/field` option documents for action configs.
    /// Returns the items plus the server-reported selection, if any.
    /// `refresh` bypasses the option cache for one read (manual Refresh,
    /// post-mutation re-read) without disabling caching globally.
    pub async fn plugin_action_options(
        self: &Arc<Self>,
        source: &str,
        refresh: bool,
    ) -> Result<(Vec<Value>, Option<String>), OperationError> {
        let source = source.trim();
        if source.is_empty() || source.len() > 256 {
            return Err(OperationError::invalid(
                "option source must be 1..=256 characters",
            ));
        }
        let (options, selected, error) = self.resolve_action_options(source, refresh).await;
        if let Some(error) = error {
            return Err(OperationError::internal(error));
        }
        Ok((options, selected))
    }

    /// Scoped option read for untrusted callers (plugin UI broker): the
    /// requesting plugin may only read option sources owned by its own
    /// actions. Ownership resolves through the action catalog, never by
    /// trusting name prefixes. Host-owned sources (`host.gifts`) belong to
    /// no plugin and are readable by every caller.
    pub async fn plugin_action_options_scoped(
        self: &Arc<Self>,
        requesting_plugin: &str,
        source: &str,
        refresh: bool,
    ) -> Result<(Vec<Value>, Option<String>), OperationError> {
        let requesting = clean_plugin_id(requesting_plugin)?;
        let source = source.trim();
        if source.is_empty() || source.len() > 256 {
            return Err(OperationError::invalid(
                "option source must be 1..=256 characters",
            ));
        }
        if crate::services::option_sources::is_host_option_source(source) {
            return self.plugin_action_options(source, refresh).await;
        }
        let Some((action_type, _)) = crate::services::option_sources::parse_option_source(source)
        else {
            return Err(OperationError::invalid(format!(
                "Unknown option source `{source}`."
            )));
        };
        self.verify_action_owner(&action_type, &requesting)?;
        self.plugin_action_options(source, refresh).await
    }

    /// Resolves the owning plugin of an action type from the action catalog
    /// and rejects callers outside that scope. Unknown actions report
    /// `not_found` (not `forbidden`) so scanners learn nothing about which
    /// plugin owns what.
    pub(crate) fn verify_action_owner(
        &self,
        action_type: &str,
        requesting_plugin: &str,
    ) -> Result<(), OperationError> {
        let Some((plugin, _)) = self.plugin_for_action(action_type) else {
            return Err(OperationError::not_found(format!(
                "Action type `{action_type}` is not available in this host."
            )));
        };
        if plugin.manifest.id != requesting_plugin {
            return Err(OperationError::forbidden(format!(
                "Action type `{action_type}` belongs to plugin `{}`.",
                plugin.manifest.id
            )));
        }
        Ok(())
    }

    /// Scoped action execution for untrusted callers (plugin UI broker):
    /// the requesting plugin may only execute its own actions. Ownership
    /// resolves through the action catalog, never by trusting name prefixes.
    pub async fn plugin_action_execute_scoped(
        self: &Arc<Self>,
        requesting_plugin: &str,
        action_type: &str,
        config: BTreeMap<String, Value>,
        live: bool,
    ) -> Result<PluginActionOutcome, OperationError> {
        let requesting = clean_plugin_id(requesting_plugin)?;
        let action_type = action_type.trim();
        if action_type.is_empty() || action_type.len() > 128 || !is_identifier(action_type) {
            return Err(OperationError::invalid(
                "action type is not a valid identifier",
            ));
        }
        self.verify_action_owner(action_type, &requesting)?;
        self.plugin_action_execute(action_type, config, live).await
    }

    /// Executes one plugin action. `live = false` is a dry run (no HTTP is
    /// sent, no side effects); `live = true` performs the real execution.
    pub async fn plugin_action_execute(
        self: &Arc<Self>,
        action_type: &str,
        config: BTreeMap<String, Value>,
        live: bool,
    ) -> Result<PluginActionOutcome, OperationError> {
        let action_type = action_type.trim();
        if action_type.is_empty() || action_type.len() > 128 || !is_identifier(action_type) {
            return Err(OperationError::invalid(
                "action type is not a valid identifier",
            ));
        }
        if config.len() > 64 {
            return Err(OperationError::invalid("too many config values (max 64)"));
        }
        // Actions declaring a `text` field (TTS speak) require bounded
        // spoken text, mirroring the WebView voice-tester path.
        if self.action_declares_field(action_type, "text") {
            let text = config
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if text.trim().is_empty() {
                return Err(OperationError::invalid(
                    "config.text must not be empty for this action type",
                ));
            }
            if text.len() > 4_096 {
                return Err(OperationError::invalid(
                    "Text is too long (4,096 character limit).",
                ));
            }
        }
        let started = now_millis();
        let action = json!({
            "typeId": action_type,
            "config": Value::Object(config.into_iter().collect()),
        });
        let event = json!({
            "id": format!("control-{}", started),
            "type": "control.execute",
            "timestamp": started,
            "data": {},
        });
        let mut logs = Vec::new();
        match self
            .execute_plugin_action(
                action_type,
                &action,
                &event,
                &crate::automation_runtime::context::ExecutionContext::owner(),
                &mut logs,
                !live,
            )
            .await
        {
            Ok(summary) => {
                // A live execution may have changed server state (audio
                // output switches, renames): drop this action's cached
                // option lists so the follow-up re-read reports the new
                // server selection instead of the pre-mutation one. Dry
                // runs send nothing, so their cache entries stay valid.
                if live {
                    self.option_sources.invalidate_action(action_type);
                }
                Ok(PluginActionOutcome {
                    action_type: action_type.to_owned(),
                    ok: true,
                    summary,
                    logs: logs.into_iter().take(20).collect(),
                    duration_ms: now_millis().saturating_sub(started),
                    error: None,
                })
            }
            Err(error) => Ok(PluginActionOutcome {
                action_type: action_type.to_owned(),
                ok: false,
                summary: error.clone(),
                logs: logs.into_iter().take(20).collect(),
                duration_ms: now_millis().saturating_sub(started),
                error: Some(error),
            }),
        }
    }
}
