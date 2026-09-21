//! Plugin lifecycle, installation, and action execution operations.

use super::{clean_plugin_id, OperationError};
use crate::events::DomainEvent;
use crate::*;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginActionOutcome {
    pub action_type: String,
    pub ok: bool,
    pub summary: String,
    pub logs: Vec<String>,
    pub duration_ms: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginInstallResult {
    pub id: String,
    pub version: String,
    pub directory: String,
    pub replaced: bool,
}

impl AppCore {
    // ------------------------------------------------------------------
    // Plugins.
    // ------------------------------------------------------------------

    /// Safe plugin summaries. Never includes settings values.
    pub fn plugin_list(&self) -> Vec<Value> {
        let snapshot = self.load_merged_behavior_snapshot();
        snapshot
            .get("plugins")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    }

    pub fn plugin_get(&self, id: &str) -> Result<Value, OperationError> {
        // Canonical existence check first so every plugin op shares one
        // not-found message.
        let plugin = self.require_discovered(id)?;
        let id = plugin.manifest.id;
        self.plugin_list()
            .into_iter()
            .find(|plugin| {
                plugin
                    .get("descriptor")
                    .and_then(|descriptor| descriptor.get("id"))
                    .and_then(Value::as_str)
                    == Some(id.as_str())
                    || plugin.get("id").and_then(Value::as_str) == Some(id.as_str())
            })
            .ok_or_else(|| OperationError::not_found(format!("Plugin `{id}` is not installed.")))
    }

    pub(crate) fn require_discovered(
        &self,
        id: &str,
    ) -> Result<tiktools_plugin_loader::DiscoveredPlugin, OperationError> {
        let id = clean_plugin_id(id)?;
        self.plugins
            .get(&id)
            .ok_or_else(|| OperationError::not_found(format!("Plugin `{id}` is not installed.")))
    }

    pub fn plugin_set_installed(&self, id: &str, installed: bool) -> Result<(), OperationError> {
        let plugin = self.require_discovered(id)?;
        let id = plugin.manifest.id.clone();
        #[cfg(feature = "persistence")]
        self.db
            .set_plugin_state(&id, installed, true)
            .map_err(|error| OperationError::internal(error.to_string()))?;
        if !installed {
            // Stopping a non-running plugin is a no-op success; surface real
            // shutdown failures.
            self.plugins.stop(&id)?;
        }
        self.set_plugin_activation(&id, installed, true);
        if installed {
            self.start_enabled_event_subscribers();
        }
        self.rebuild_processor_index();
        self.refresh_automation_snapshot();
        Ok(())
    }

    pub fn plugin_set_enabled(&self, id: &str, enabled: bool) -> Result<(), OperationError> {
        let plugin = self.require_discovered(id)?;
        let id = plugin.manifest.id.clone();
        #[cfg(feature = "persistence")]
        self.db
            .set_plugin_state(&id, true, enabled)
            .map_err(|error| OperationError::internal(error.to_string()))?;
        let result = if enabled {
            self.plugins.start(&id)
        } else {
            self.plugins.stop(&id)
        };
        result?;
        self.set_plugin_activation(&id, true, enabled);
        self.rebuild_processor_index();
        self.refresh_automation_snapshot();
        self.events.publish_domain(if enabled {
            DomainEvent::PluginStarted { plugin_id: id }
        } else {
            DomainEvent::PluginStopped { plugin_id: id }
        });
        Ok(())
    }

    pub fn plugin_start(&self, id: &str) -> Result<(), OperationError> {
        let plugin = self.require_discovered(id)?;
        let id = plugin.manifest.id.clone();
        self.plugins.start(&id)?;
        self.events
            .publish_domain(DomainEvent::PluginStarted { plugin_id: id });
        Ok(())
    }

    pub fn plugin_stop(&self, id: &str) -> Result<(), OperationError> {
        let plugin = self.require_discovered(id)?;
        let id = plugin.manifest.id.clone();
        self.plugins.stop(&id)?;
        self.events
            .publish_domain(DomainEvent::PluginStopped { plugin_id: id });
        Ok(())
    }

    #[cfg(feature = "plugin-install")]
    pub fn plugin_install(
        &self,
        archive: &str,
        replace_existing: bool,
    ) -> Result<PluginInstallResult, OperationError> {
        let trimmed = archive.trim();
        if trimmed.is_empty() || trimmed.len() > 4096 {
            return Err(OperationError::invalid(
                "plugin archive path must be 1..=4096 characters",
            ));
        }
        let installed = self
            .install_plugin(std::path::Path::new(trimmed), replace_existing)
            .map_err(|error| OperationError::internal(error.to_string()))?;
        self.start_enabled_event_subscribers();
        self.refresh_automation_snapshot();
        let id = installed.manifest.id.clone();
        self.events.publish_domain(DomainEvent::PluginInstalled {
            plugin_id: id.clone(),
        });
        Ok(PluginInstallResult {
            id,
            version: installed.manifest.version.clone(),
            directory: installed.directory.display().to_string(),
            replaced: replace_existing,
        })
    }

    #[cfg(not(feature = "plugin-install"))]
    pub fn plugin_install(
        &self,
        _archive: &str,
        _replace_existing: bool,
    ) -> Result<PluginInstallResult, OperationError> {
        Err(OperationError::unavailable(
            "plugin installation was disabled in this build",
        ))
    }

    #[cfg(feature = "plugin-install")]
    pub fn plugin_uninstall(&self, id: &str) -> Result<(), OperationError> {
        let plugin = self.require_discovered(id)?;
        let id = plugin.manifest.id.clone();
        self.uninstall_plugin(&id)
            .map_err(|error| OperationError::internal(error.to_string()))?;
        self.refresh_automation_snapshot();
        self.events
            .publish_domain(DomainEvent::PluginUninstalled { plugin_id: id });
        Ok(())
    }

    #[cfg(not(feature = "plugin-install"))]
    pub fn plugin_uninstall(&self, _id: &str) -> Result<(), OperationError> {
        Err(OperationError::unavailable(
            "plugin installation was disabled in this build",
        ))
    }

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
    /// trusting name prefixes.
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
        let Some((action_type, _)) =
            crate::services::option_sources::parse_option_source(source)
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
            .execute_plugin_action(action_type, &action, &event, &mut logs, !live)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use tiktools_plugin_loader::{PluginManager, PluginRoot, PluginSource};

    struct NullEmitter;

    impl crate::HostEmitter for NullEmitter {
        fn emit(&self, _message: crate::HostMessage) {}
    }

    fn manifest(id: &str, actions: &[&str]) -> String {
        let actions = actions
            .iter()
            .map(|action| {
                format!(
                    r#"{{"id": "{action}", "title": {{"default": "{action}"}}, "tag": "test", "requiredCapabilities": []}}"#
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        format!(
            r#"{{"schemaVersion": 3, "id": "{id}", "name": "{id}", "version": "1.0.0", "runtime": "declarative", "capabilities": [], "permissions": [], "actionTypes": [{actions}]}}"#
        )
    }

    /// Builds an AppCore whose catalog holds two declarative plugins. The
    /// actions declare no `http` block, so dry runs resolve and return
    /// without touching any runtime or network.
    fn core_with_two_plugins() -> (Arc<AppCore>, std::path::PathBuf) {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "tiktools-ownership-test-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        // plugin-b declares `plugina.spoof`: the action NAME looks like
        // plugin-a's, but the catalog owner is plugin-b. Ownership must
        // resolve through the catalog, never by prefix.
        for (dir, id, actions) in [
            ("plugin-a", "plugina", &["plugina.echo"][..]),
            (
                "plugin-b",
                "pluginb",
                &["pluginb.echo", "plugina.spoof"][..],
            ),
        ] {
            let dir = root.join(dir);
            std::fs::create_dir_all(&dir).expect("test plugin dir");
            std::fs::write(dir.join("plugin.json"), manifest(id, actions)).expect("manifest");
        }
        let manager = PluginManager::new(vec![PluginRoot {
            path: root.clone(),
            source: PluginSource::Development,
        }]);
        manager.scan().expect("test scan");
        assert_eq!(manager.list().len(), 2);
        let mut core = AppCore::new(Arc::new(NullEmitter));
        core.plugins = Arc::new(manager);
        (Arc::new(core), root)
    }

    fn cleanup(root: &std::path::Path) {
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn scoped_execute_allows_own_actions() {
        let (core, root) = core_with_two_plugins();
        let outcome = core
            .plugin_action_execute_scoped("plugina", "plugina.echo", BTreeMap::new(), false)
            .await
            .expect("own action executes");
        assert!(outcome.ok, "dry run should succeed: {outcome:?}");
        cleanup(&root);
    }

    #[tokio::test]
    async fn scoped_execute_rejects_cross_plugin_actions() {
        let (core, root) = core_with_two_plugins();
        // plugin-a cannot invoke plugin-b's action.
        let error = core
            .plugin_action_execute_scoped("plugina", "pluginb.echo", BTreeMap::new(), false)
            .await
            .expect_err("cross-plugin execute must fail");
        assert_eq!(error.code(), "forbidden");
        // Prefix spoofing buys nothing: `plugina.spoof` is owned by
        // plugin-b even though the name starts with plugin-a's id.
        let error = core
            .plugin_action_execute_scoped("plugina", "plugina.spoof", BTreeMap::new(), false)
            .await
            .expect_err("prefix-spoofed execute must fail");
        assert_eq!(error.code(), "forbidden");
        // Unknown actions report not_found, not forbidden.
        let error = core
            .plugin_action_execute_scoped("plugina", "nobody.there", BTreeMap::new(), false)
            .await
            .expect_err("unknown action must fail");
        assert_eq!(error.code(), "not_found");
        cleanup(&root);
    }

    #[tokio::test]
    async fn scoped_options_rejects_cross_plugin_sources() {
        let (core, root) = core_with_two_plugins();
        // plugin-a cannot read plugin-b's option source.
        let error = core
            .plugin_action_options_scoped(
                "plugina",
                "plugin-action-options:pluginb.echo:field",
                false,
            )
            .await
            .expect_err("cross-plugin options read must fail");
        assert_eq!(error.code(), "forbidden");
        // Prefix-spoofed source resolves to plugin-b and is rejected.
        let error = core
            .plugin_action_options_scoped(
                "plugina",
                "plugin-action-options:plugina.spoof:field",
                false,
            )
            .await
            .expect_err("prefix-spoofed options read must fail");
        assert_eq!(error.code(), "forbidden");
        // Own source passes ownership: it proceeds to resolution, which
        // reports the (declared-shape) absence of options — never forbidden.
        let error = core
            .plugin_action_options_scoped(
                "plugina",
                "plugin-action-options:plugina.echo:field",
                false,
            )
            .await
            .expect_err("undeclared options must fail resolution");
        assert_ne!(error.code(), "forbidden");
        cleanup(&root);
    }
}
