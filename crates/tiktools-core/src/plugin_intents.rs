//! Host-owned execution of plugin result intents.
//!
//! Runtime adapters stop at the typed SDK result boundary. This module is the
//! single place where a plugin result is checked against host capabilities and
//! translated into application-owned operations.

use super::*;

impl AppCore {
    pub(crate) async fn execute_plugin_intents(
        self: &Arc<Self>,
        plugin: &tiktools_plugin_loader::DiscoveredPlugin,
        intents: Vec<tiktools_plugin_sdk::HostIntent>,
        event: &Value,
        logs: &mut Vec<String>,
        test: bool,
    ) -> Result<Vec<String>, String> {
        let globals = self.automation_globals();
        let mut parts = Vec::new();
        for intent in intents {
            match intent {
                tiktools_plugin_sdk::HostIntent::AudioPlay(intent) => {
                    self.capabilities
                        .require_capability(
                            &plugin.manifest,
                            tiktools_plugin_api::CAPABILITY_AUDIO_PLAY,
                        )
                        .map_err(|error| error.to_string())?;
                    self.capabilities
                        .require_permission(
                            &plugin.manifest,
                            tiktools_plugin_api::capabilities::AUDIO_OUTPUT_PERMISSION,
                        )
                        .map_err(|error| error.to_string())?;
                    let config = serde_json::to_value(intent)
                        .map_err(|error| format!("invalid audio intent: {error}"))?;
                    let config = config
                        .as_object()
                        .ok_or_else(|| "audio intent must be an object".to_owned())?;
                    parts.push(
                        self.execute_audio_action(config, event, &globals, logs, test)
                            .await?,
                    );
                }
                tiktools_plugin_sdk::HostIntent::Emit(intent) => {
                    let event_type = normalize_emit_type(&intent.event_type)?;
                    let payload = Value::Object(
                        render_json_map(&intent.data, event, &globals)
                            .into_iter()
                            .collect(),
                    );
                    let published = if let Some(typed) =
                        self.plugin_typed_event(&plugin.manifest, &event_type, &payload, event)?
                    {
                        typed
                    } else {
                        self.make_internal_automation_event(event, &event_type, payload.clone())
                    };
                    if !test {
                        self.observe_plugin_ranking_event(
                            &plugin.manifest.id,
                            &event_type,
                            &payload,
                            &published,
                        );
                        self.publish_automation_event(published).await;
                    }
                    parts.push(if test {
                        format!("would emit {event_type}")
                    } else {
                        format!("emit {event_type}")
                    });
                }
            }
        }
        Ok(parts)
    }

    /// Plugin emits feed rankings through `plugin.<pluginId>.<eventType>`
    /// (`event.count`, plus `value` when the payload carries a positive
    /// integer). Keys come from `user`/`userId`, `team` and `key`. The host
    /// event id makes delivery idempotent; without a LIVE context the event is
    /// not attributable to a ranking scope and is skipped.
    pub(crate) fn observe_plugin_ranking_event(
        &self,
        plugin_id: &str,
        event_type: &str,
        payload: &Value,
        published: &Value,
    ) {
        let Some(event_id) = published.get("id").and_then(Value::as_str) else {
            return;
        };
        let Some((creator_id, room_id)) =
            recover_rwlock_read(&self.connection_context, "connection context")
                .as_ref()
                .map(|context| (context.unique_id.clone(), context.room_id.clone()))
        else {
            return;
        };
        let text = |names: &[&str]| {
            names.iter().find_map(|name| match payload.get(*name) {
                Some(Value::String(value)) if !value.is_empty() => Some(value.clone()),
                Some(Value::Number(value)) => Some(value.to_string()),
                _ => None,
            })
        };
        let mut metrics = std::collections::BTreeMap::from([("event.count".to_owned(), 1)]);
        if let Some(value) = payload
            .get("value")
            .and_then(Value::as_u64)
            .filter(|v| *v > 0)
        {
            metrics.insert("value".to_owned(), value);
        }
        let source = format!("plugin.{plugin_id}.{event_type}");
        if source.len() > 128 {
            return;
        }
        let result = self.ranking_contribute(crate::control::ExternalContribution {
            event_id: event_id.to_owned(),
            creator_id,
            room_id,
            source,
            user: text(&["userId", "user", "uniqueId"]),
            label: text(&["nickname", "label", "uniqueId"]),
            team: text(&["team"]),
            custom_key: text(&["key"]),
            option: None,
            metrics,
        });
        if let Err(error) = result {
            tracing::debug!(plugin = %plugin_id, %event_type, ?error, "plugin ranking contribution skipped");
        }
    }

    /// Builds a typed event for one of the calling plugin's declared event
    /// types. The host stamps identity, ownership, timing, depth, and
    /// connection context; the plugin only supplies the type and its data
    /// payload. `plugin_id` is host-determined (the polling plugin / the
    /// emitting manifest), so a plugin can never spoof another owner: the
    /// stamp is set after the payload is attached and survives the whole
    /// poll → DomainEvent → automation → IPC → WebView path.
    pub(crate) fn make_plugin_event(
        &self,
        plugin_id: &str,
        source: &Value,
        event_type: &str,
        data: Value,
    ) -> Value {
        let mut event = json!({
            "id": format!("plugin-event-{}", self.next_sequence()),
            "type": event_type,
            "timestamp": now_millis(),
            "source": {"kind": "plugin", "pluginId": plugin_id},
            "data": data,
        });
        if let Some(data) = event.get_mut("data").and_then(Value::as_object_mut) {
            data.insert(
                "depth".to_owned(),
                Value::from(self.automation.emit_depth(source).saturating_add(1)),
            );
        }
        for key in ["connectionId", "creator", "user"] {
            if let Some(value) = source.get(key) {
                event[key] = value.clone();
            }
        }
        if let Some(source_id) = source.get("id") {
            event["sourceEventId"] = source_id.clone();
        }
        event
    }

    /// Resolves a plugin emit intent to a typed event when the type is one of
    /// that plugin's declared event types. Returns `Ok(None)` for anything
    /// else so the caller keeps the internal `plugin.emit` channel, and an
    /// error when the plugin did not declare the `events.publish` capability.
    pub(crate) fn plugin_typed_event(
        &self,
        manifest: &tiktools_plugin_api::manifest::PluginManifest,
        event_type: &str,
        data: &Value,
        source: &Value,
    ) -> Result<Option<Value>, String> {
        if !declared_event_types(manifest).contains(&event_type.to_owned()) {
            return Ok(None);
        }
        self.capabilities
            .require_capability(manifest, tiktools_plugin_api::capabilities::EVENTS_PUBLISH)
            .map_err(|error| error.to_string())?;
        if !data.is_object()
            || serde_json::to_vec(data)
                .map(|bytes| bytes.len() > MAX_PLUGIN_EVENT_BYTES)
                .unwrap_or(true)
        {
            return Err(format!(
                "Plugin event `{event_type}` needs an object payload under 64 KB."
            ));
        }
        Ok(Some(self.make_plugin_event(
            &manifest.id,
            source,
            event_type,
            data.clone(),
        )))
    }

    pub(crate) fn make_internal_automation_event(
        &self,
        source: &Value,
        event_type: &str,
        payload: Value,
    ) -> Value {
        let mut event = json!({
            "id": format!("plugin-emit-{}", self.next_sequence()),
            "type": "plugin.emit",
            "timestamp": now_millis(),
            "data": {
                "emitType": event_type,
                "depth": self.automation.emit_depth(source).saturating_add(1),
                "payload": payload
            }
        });
        for key in ["connectionId", "creator", "user"] {
            if let Some(value) = source.get(key) {
                event[key] = value.clone();
            }
        }
        if let Some(source_id) = source.get("id") {
            event["sourceEventId"] = source_id.clone();
        }
        event
    }
}
