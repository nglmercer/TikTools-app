use super::*;

impl AppCore {
    pub(super) fn emit_persisted_workflows(&self) {
        #[cfg(feature = "persistence")]
        let workflows = match self.db.load_workflows() {
            Ok(workflows) => workflows,
            Err(error) => {
                self.emit(HostMessage::AutomationError {
                    message: error.to_string(),
                });
                Vec::new()
            }
        };
        #[cfg(not(feature = "persistence"))]
        let workflows = Vec::new();
        self.emit(HostMessage::AutomationWorkflows { workflows });
    }

    pub(super) fn emit_persisted_gifts(&self) {
        #[cfg(feature = "persistence")]
        let gifts = match self.db.load_gift_catalog() {
            Ok(gifts) => gifts,
            Err(error) => {
                tracing::warn!(%error, "could not load gift catalog");
                Vec::new()
            }
        };
        #[cfg(not(feature = "persistence"))]
        let gifts = Vec::new();
        self.emit(HostMessage::GiftCatalog { gifts });
    }

    pub(super) fn emit_persisted_behavior(&self) {
        #[cfg(feature = "persistence")]
        let mut snapshot = match self.db.load_behavior_snapshot() {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.emit(HostMessage::AutomationError {
                    message: error.to_string(),
                });
                empty_behavior_snapshot()
            }
        };
        #[cfg(not(feature = "persistence"))]
        let mut snapshot = empty_behavior_snapshot();
        self.merge_runtime_catalog(&mut snapshot);
        self.automation.replace_snapshot(&snapshot);
        self.request_hotkey_sync();
        self.emit(HostMessage::Behavior { snapshot });
    }

    /// Loads only the persisted records needed by the asynchronous hotkey
    /// projection. Runtime catalog entries are deliberately not required.
    pub(super) fn load_behavior_snapshot_for_hotkey_sync(&self) -> serde_json::Value {
        #[cfg(feature = "persistence")]
        {
            self.db.load_behavior_snapshot().unwrap_or_else(|error| {
                tracing::warn!(%error, "could not load behavior for hotkey synchronization");
                empty_behavior_snapshot()
            })
        }
        #[cfg(not(feature = "persistence"))]
        {
            empty_behavior_snapshot()
        }
    }

    pub(super) fn merge_runtime_catalog(&self, snapshot: &mut serde_json::Value) {
        let Some(object) = snapshot.as_object_mut() else {
            *snapshot = empty_behavior_snapshot();
            return;
        };

        let mut action_types = builtin_action_types();
        let mut event_types: std::collections::BTreeMap<String, Value> =
            std::collections::BTreeMap::new();
        let mut plugins = Vec::new();
        let persisted_plugins = object
            .get("plugins")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();

        for plugin in self.plugins.list() {
            let state = persisted_plugins.iter().find(|value| {
                value.get("id").and_then(serde_json::Value::as_str)
                    == Some(plugin.manifest.id.as_str())
            });
            let installed = state
                .and_then(|value| value.get("installed"))
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(true);
            let enabled = state
                .and_then(|value| value.get("enabled"))
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(true);
            let dependency = format!("{} runtime", plugin.manifest.runtime);
            let description = plugin
                .manifest
                .description
                .clone()
                .unwrap_or_else(|| format!("{} runtime plugin", plugin.manifest.name));
            let action_ids = plugin
                .manifest
                .action_types
                .iter()
                .filter_map(|value| value.get("id").and_then(serde_json::Value::as_str))
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>();
            let mut event_type_ids = Vec::new();
            for descriptor in &plugin.manifest.event_types {
                let Some((event_type, entry)) =
                    stamp_plugin_event_type(&plugin.manifest.id, descriptor.clone())
                else {
                    tracing::warn!(plugin = %plugin.manifest.id, "plugin event type is invalid; skipped");
                    continue;
                };
                if event_types.insert(event_type.clone(), entry).is_some() {
                    tracing::warn!(plugin = %plugin.manifest.id, event_type = %event_type, "plugin event type overrides an earlier declaration");
                }
                if !event_type_ids.contains(&event_type) {
                    event_type_ids.push(event_type);
                }
            }
            plugins.push(json!({
                "descriptor": {
                    "id": plugin.manifest.id,
                    "source": match plugin.source {
                        tiktools_plugin_loader::PluginSource::Builtin => "builtin",
                        tiktools_plugin_loader::PluginSource::User => "user",
                        tiktools_plugin_loader::PluginSource::Development => "development",
                    },
                    "name": localized(&plugin.manifest.name, "plugin.name"),
                    "version": plugin.manifest.version,
                    "description": localized(&description, "plugin.description"),
                    "dependency": localized(&dependency, "plugin.dependency"),
                    "permissions": plugin.manifest.permissions,
                    "actionTypeIds": action_ids,
                    "eventTypeIds": event_type_ids,
                    "hasSettings": plugin.manifest.settings_schema.is_some()
                },
                "installed": installed,
                "enabled": enabled,
                "running": plugin.running,
                "available": plugin.available,
                "unavailableReason": plugin.reason
            }));

            for descriptor in &plugin.manifest.action_types {
                let Some(mut descriptor) = descriptor.as_object().cloned() else {
                    continue;
                };
                let Some(id) = descriptor.get("id").and_then(serde_json::Value::as_str) else {
                    continue;
                };
                if !is_identifier(id) {
                    tracing::warn!(plugin = %plugin.manifest.id, action = %id, "plugin action id is invalid");
                    continue;
                }
                descriptor.insert(
                    "source".to_owned(),
                    json!({"kind": "plugin", "pluginId": plugin.manifest.id}),
                );
                if !descriptor.contains_key("requiredCapabilities") {
                    descriptor.insert(
                        "requiredCapabilities".to_owned(),
                        Value::Array(
                            plugin
                                .manifest
                                .capabilities
                                .iter()
                                .cloned()
                                .map(Value::String)
                                .collect(),
                        ),
                    );
                }
                action_types.push(Value::Object(descriptor));
            }
        }

        object.insert("actionTypes".to_owned(), Value::Array(action_types));
        let event_types: Vec<Value> = event_types.into_values().collect();
        tracing::debug!(
            count = event_types.len(),
            "merged plugin event types into behavior snapshot"
        );
        object.insert("eventTypes".to_owned(), Value::Array(event_types));
        object.insert("plugins".to_owned(), Value::Array(plugins));
        object.insert("translations".to_owned(), builtin_translations());
    }

    pub(super) fn emit_plugin_settings(&self, id: &str) {
        let Some(plugin) = self.plugins.get(id) else {
            self.emit(HostMessage::BehaviorError {
                message: format!("Plugin `{id}` is not installed."),
            });
            return;
        };
        let Some(schema) = plugin.manifest.settings_schema.clone() else {
            self.emit(HostMessage::BehaviorError {
                message: format!("Plugin settings are not declared by `{id}`."),
            });
            return;
        };
        match self.capabilities.load_plugin_settings(&plugin.manifest) {
            Ok(values) => self.emit(HostMessage::PluginSettings {
                id: id.to_owned(),
                schema,
                ui_hints: plugin.manifest.settings_ui_hints.clone(),
                values,
            }),
            Err(error) => self.emit(HostMessage::BehaviorError {
                message: error.to_string(),
            }),
        }
    }

    pub(super) fn save_plugin_settings(
        &self,
        id: &str,
        values: std::collections::BTreeMap<String, Value>,
    ) {
        let Some(plugin) = self.plugins.get(id) else {
            self.emit(HostMessage::BehaviorError {
                message: format!("Plugin `{id}` is not installed."),
            });
            return;
        };
        let Some(schema) = plugin.manifest.settings_schema.clone() else {
            self.emit(HostMessage::BehaviorError {
                message: format!("Plugin settings are not declared by `{id}`."),
            });
            return;
        };
        match self
            .capabilities
            .save_plugin_settings(&plugin.manifest, &values)
        {
            Ok(values) => self.emit(HostMessage::PluginSettings {
                id: id.to_owned(),
                schema,
                ui_hints: plugin.manifest.settings_ui_hints.clone(),
                values,
            }),
            Err(error) => self.emit(HostMessage::BehaviorError {
                message: error.to_string(),
            }),
        }
    }

    pub(super) fn save_behavior_record(&self, table: &str, value: serde_json::Value) {
        #[cfg(feature = "persistence")]
        {
            if let Err(error) = self.db.save_behavior(table, &value) {
                self.emit(HostMessage::AutomationError {
                    message: error.to_string(),
                });
            } else {
                self.emit_persisted_behavior();
            }
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (table, value);
            self.emit(HostMessage::AutomationError {
                message: "Rust persistence is disabled in this build.".to_owned(),
            });
        }
    }

    pub(super) fn delete_behavior_record(&self, table: &str, id: &str) {
        #[cfg(feature = "persistence")]
        {
            if let Err(error) = self.db.delete_behavior(table, id) {
                self.emit(HostMessage::AutomationError {
                    message: error.to_string(),
                });
            } else {
                self.emit_persisted_behavior();
            }
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (table, id);
            self.emit(HostMessage::AutomationError {
                message: "Rust persistence is disabled in this build.".to_owned(),
            });
        }
    }

    pub(super) fn set_behavior_enabled(&self, table: &str, id: &str, enabled: bool) {
        #[cfg(feature = "persistence")]
        {
            if let Err(error) = self.db.set_behavior_enabled(table, id, enabled) {
                self.emit(HostMessage::AutomationError {
                    message: error.to_string(),
                });
            } else {
                self.emit_persisted_behavior();
            }
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (table, id, enabled);
            self.emit(HostMessage::AutomationError {
                message: "Rust persistence is disabled in this build.".to_owned(),
            });
        }
    }
}

/// Validates one manifest `eventTypes` entry and stamps its plugin source.
/// Returns the event type plus the snapshot-ready entry, or `None` when the
/// entry is invalid (the catalog merge warns and skips it).
pub(super) fn stamp_plugin_event_type(
    plugin_id: &str,
    descriptor: Value,
) -> Option<(String, Value)> {
    if tiktools_plugin_api::manifest::validate_event_type(&descriptor).is_err() {
        return None;
    }
    let mut entry = descriptor.as_object()?.clone();
    let event_type = entry.get("type").and_then(Value::as_str)?.to_owned();
    entry.insert(
        "source".to_owned(),
        json!({"kind": "plugin", "pluginId": plugin_id}),
    );
    Some((event_type, Value::Object(entry)))
}

#[cfg(test)]
mod persistence_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn stamps_valid_event_types_and_rejects_reserved_ones() {
        let (event_type, entry) = stamp_plugin_event_type(
            "hotkeys",
            json!({"type": "hotkey.pressed", "title": {"default": "Hotkey pressed"}}),
        )
        .expect("valid entry should stamp");
        assert_eq!(event_type, "hotkey.pressed");
        assert_eq!(
            entry.get("source"),
            Some(&json!({"kind": "plugin", "pluginId": "hotkeys"}))
        );
        assert!(stamp_plugin_event_type(
            "hotkeys",
            json!({"type": "tiktok.chat", "title": {"default": "Chat"}}),
        )
        .is_none());
        assert!(stamp_plugin_event_type("hotkeys", json!({"type": "hotkey.pressed"}),).is_none());
    }
}
