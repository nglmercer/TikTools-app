use super::*;

impl AppCore {
    pub(super) fn emit_persisted_workflows(&self) {
        #[cfg(feature = "persistence")]
        let workflows = match self.workflow_list() {
            Ok(workflows) => workflows,
            Err(error) => {
                self.emit(HostMessage::AutomationError {
                    message: error.message().to_owned(),
                });
                Vec::new()
            }
        };
        #[cfg(not(feature = "persistence"))]
        let workflows = Vec::new();
        self.emit(HostMessage::AutomationWorkflows { workflows });
    }

    pub(super) fn emit_persisted_gifts(&self) {
        self.emit(HostMessage::GiftCatalog {
            gifts: self.gift_catalog(),
        });
    }

    pub(super) fn emit_persisted_behavior(&self) {
        // Same authoritative path as the control API; the WebView only adds
        // UI-shaped error reporting around it.
        #[cfg(feature = "persistence")]
        if let Err(error) = self.db.load_behavior_snapshot() {
            self.emit(HostMessage::AutomationError {
                message: error.to_string(),
            });
        }
        self.emit(HostMessage::Behavior {
            snapshot: self.behavior_snapshot(),
        });
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
        let mut plugin_templates = Vec::new();
        let mut plugin_pages = Vec::new();
        let mut plugin_uis = Vec::new();
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
                    "longDescription": plugin.manifest.long_description.as_deref().map(|long| localized(long, "plugin.longDescription")),
                    "icon": plugin.manifest.icon,
                    "tags": plugin.manifest.tags,
                    "dependency": localized(&dependency, "plugin.dependency"),
                    "permissions": plugin.manifest.permissions,
                    "eventSubscriptions": plugin.manifest.event_subscriptions,
                    "actionTypeIds": action_ids,
                    "eventTypeIds": event_type_ids,
                    "hasSettings": plugin.manifest.settings_schema.is_some(),
                    "hasConnectionProbe": plugin.manifest.http.as_ref().and_then(|http| http.get("health")).is_some(),
                    "supportsTokenProvisioning": crate::services::token_provision::supports_token_provisioning(&plugin.manifest)
                },
                "installed": installed,
                "enabled": enabled,
                "running": plugin.running,
                "available": plugin.available,
                "unavailableReason": plugin.reason
            }));

            // Templates and pages surface only while their plugin is
            // installed, enabled, and available, so disabling or uninstalling
            // a plugin removes its modal entries and nav tabs on the next
            // snapshot.
            if installed && enabled && plugin.available {
                for template in &plugin.manifest.templates {
                    if tiktools_plugin_api::manifest::validate_plugin_template(template).is_err() {
                        tracing::warn!(plugin = %plugin.manifest.id, "plugin template is invalid; skipped");
                        continue;
                    }
                    let Some(mut stamped) = template.as_object().cloned() else {
                        continue;
                    };
                    let namespaced = stamped
                        .get("id")
                        .and_then(Value::as_str)
                        .map(|id| format!("{}/{}", plugin.manifest.id, id));
                    if let Some(namespaced) = namespaced {
                        stamped.insert("id".to_owned(), Value::String(namespaced));
                    }
                    stamped.insert(
                        "pluginId".to_owned(),
                        Value::String(plugin.manifest.id.clone()),
                    );
                    stamped.insert(
                        "source".to_owned(),
                        json!({"kind": "plugin", "pluginId": plugin.manifest.id}),
                    );
                    plugin_templates.push(Value::Object(stamped));
                }
                for page in &plugin.manifest.pages {
                    if tiktools_plugin_api::manifest::validate_plugin_page(page).is_err() {
                        tracing::warn!(plugin = %plugin.manifest.id, "plugin page is invalid; skipped");
                        continue;
                    }
                    let Some(mut stamped) = page.as_object().cloned() else {
                        continue;
                    };
                    stamped.insert(
                        "pluginId".to_owned(),
                        Value::String(plugin.manifest.id.clone()),
                    );
                    stamped.insert(
                        "source".to_owned(),
                        json!({"kind": "plugin", "pluginId": plugin.manifest.id}),
                    );
                    plugin_pages.push(Value::Object(stamped));
                }
                // Typed `ui` descriptors were validated at discovery; the
                // stamp only adds host-owned identity. Serialization of the
                // validated struct cannot fail, but a failure must never
                // break the whole snapshot.
                if let Some(ui) = &plugin.manifest.ui {
                    match serde_json::to_value(ui) {
                        Ok(Value::Object(mut stamped)) => {
                            stamped.insert(
                                "pluginId".to_owned(),
                                Value::String(plugin.manifest.id.clone()),
                            );
                            stamped.insert(
                                "source".to_owned(),
                                json!({"kind": "plugin", "pluginId": plugin.manifest.id}),
                            );
                            plugin_uis.push(Value::Object(stamped));
                        }
                        _ => {
                            tracing::warn!(plugin = %plugin.manifest.id, "plugin ui is invalid; skipped");
                        }
                    }
                }
            }

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
        object.insert("pluginTemplates".to_owned(), Value::Array(plugin_templates));
        object.insert("pluginPages".to_owned(), Value::Array(plugin_pages));
        object.insert("pluginUis".to_owned(), Value::Array(plugin_uis));
        object.insert("translations".to_owned(), builtin_translations());
    }

    pub(super) fn emit_plugin_settings(&self, id: &str) {
        match self.plugin_settings(id) {
            Ok(settings) => self.emit(HostMessage::PluginSettings {
                id: settings.plugin_id,
                schema: settings.schema,
                ui_hints: settings.ui_hints,
                values: Value::Object(settings.values.into_iter().collect()),
            }),
            Err(error) => self.emit(HostMessage::BehaviorError {
                message: error.message().to_owned(),
            }),
        }
    }

    pub(super) fn save_plugin_settings(
        &self,
        id: &str,
        values: std::collections::BTreeMap<String, Value>,
    ) {
        match self.plugin_settings_save(id, values) {
            Ok(settings) => self.emit(HostMessage::PluginSettings {
                id: settings.plugin_id,
                schema: settings.schema,
                ui_hints: settings.ui_hints,
                values: Value::Object(settings.values.into_iter().collect()),
            }),
            Err(error) => self.emit(HostMessage::BehaviorError {
                message: error.message().to_owned(),
            }),
        }
    }

    pub(super) fn save_behavior_record(&self, table: &str, value: serde_json::Value) {
        let Some(kind) = automation_kind_from_table(table) else {
            self.emit(HostMessage::AutomationError {
                message: format!("unknown behavior table `{table}`"),
            });
            return;
        };
        if let Err(error) = self.automation_save(kind, &value) {
            self.emit(HostMessage::AutomationError {
                message: error.message().to_owned(),
            });
        } else {
            self.emit_persisted_behavior();
        }
    }

    pub(super) fn delete_behavior_record(&self, table: &str, id: &str) {
        let Some(kind) = automation_kind_from_table(table) else {
            self.emit(HostMessage::AutomationError {
                message: format!("unknown behavior table `{table}`"),
            });
            return;
        };
        if let Err(error) = self.automation_delete(kind, id) {
            self.emit(HostMessage::AutomationError {
                message: error.message().to_owned(),
            });
        } else {
            self.emit_persisted_behavior();
        }
    }

    pub(super) fn set_behavior_enabled(&self, table: &str, id: &str, enabled: bool) {
        let Some(kind) = automation_kind_from_table(table) else {
            self.emit(HostMessage::AutomationError {
                message: format!("unknown behavior table `{table}`"),
            });
            return;
        };
        if let Err(error) = self.automation_set_enabled(kind, id, enabled) {
            self.emit(HostMessage::AutomationError {
                message: error.message().to_owned(),
            });
        } else {
            self.emit_persisted_behavior();
        }
    }
}

/// Maps the legacy behavior table names onto control operation kinds.
/// Callers only ever pass the two constants below; anything else is a bug.
pub(super) fn automation_kind_from_table(table: &str) -> Option<crate::control::AutomationKind> {
    match table {
        "behavior_events" => Some(crate::control::AutomationKind::Event),
        "behavior_actions" => Some(crate::control::AutomationKind::Action),
        _ => None,
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
