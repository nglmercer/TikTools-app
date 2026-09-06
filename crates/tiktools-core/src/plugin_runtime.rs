//! Plugin-specific action orchestration kept outside the general automation engine.

use super::*;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct PluginActionDescriptor {
    pub(crate) id: String,
    #[serde(default, rename = "requiredCapabilities")]
    pub(crate) required_capabilities: Vec<String>,
    #[serde(default, rename = "timeoutMs")]
    pub(crate) timeout_ms: Option<u64>,
}

impl PluginActionDescriptor {
    fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(
            self.timeout_ms
                .unwrap_or(tiktools_plugin_api::manifest::DEFAULT_PLUGIN_ACTION_TIMEOUT_MS),
        )
    }
}

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
        if test {
            return Ok(format!(
                "would run plugin {} action {type_id}",
                plugin.manifest.id
            ));
        }

        self.plugins
            .start(&plugin.manifest.id)
            .map_err(|error| error.to_string())?;
        let plugin_id = plugin.manifest.id.clone();
        let action_timeout = descriptor.timeout();
        let request = serde_json::to_value(tiktools_plugin_sdk::PluginCall::action(
            action.clone(),
            event.clone(),
        ))
        .map_err(|error| format!("could not encode plugin action: {error}"))?;
        let plugins = Arc::clone(&self.plugins);
        let request_for_call = request.clone();
        let response = tokio::time::timeout(
            action_timeout,
            tokio::task::spawn_blocking(move || {
                plugins.call_with_timeout(&plugin_id, &request_for_call, action_timeout)
            }),
        )
        .await
        .map_err(|_| {
            format!(
                "plugin `{}` timed out after {} seconds",
                plugin.manifest.id,
                action_timeout.as_secs()
            )
        })?
        .map_err(|error| format!("plugin task failed: {error}"))?
        .map_err(|error| error.to_string())?;
        self.events.publish(AppEvent::Plugin(json!({
            "pluginId": plugin.manifest.id,
            "type": "action-result",
            "actionType": type_id,
            "response": response
        })));

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

    pub(crate) fn plugin_ready(&self, id: &str) -> bool {
        let Some(plugin) = self.plugins.get(id) else {
            return false;
        };
        if !plugin.available {
            return false;
        }
        #[cfg(feature = "persistence")]
        if let Ok(snapshot) = self.db.load_behavior_snapshot() {
            if let Some(state) = snapshot
                .get("plugins")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .find(|state| state.get("id").and_then(Value::as_str) == Some(id))
            {
                return state.get("installed").and_then(Value::as_bool) == Some(true)
                    && state.get("enabled").and_then(Value::as_bool) == Some(true);
            }
        }
        true
    }

    /// Sample event for a plugin-owned trigger, taken from the declaring
    /// plugin's manifest sample so `test-event` previews realistic data.
    pub(crate) fn plugin_event_sample(&self, trigger: &str) -> Option<Value> {
        for plugin in self.plugins.list() {
            for entry in &plugin.manifest.event_types {
                if tiktools_plugin_api::manifest::validate_event_type(entry).is_err() {
                    continue;
                }
                if entry.get("type").and_then(Value::as_str) != Some(trigger) {
                    continue;
                }
                let data = entry
                    .get("sample")
                    .and_then(Value::as_object)
                    .cloned()
                    .unwrap_or_default();
                return Some(json!({
                    "id": "sample-event",
                    "type": trigger,
                    "timestamp": now_millis(),
                    "user": {"uniqueId": "viewer_demo", "nickname": "Viewer Demo", "userId": "1"},
                    "data": Value::Object(data),
                }));
            }
        }
        None
    }

    /// Starts the background poll that lets plugins publish spontaneous
    /// events (hotkeys, timers, watchers).
    pub fn spawn_plugin_event_poll(self: &Arc<Self>, runtime: &tokio::runtime::Handle) {
        if self
            .shutdown_started
            .load(std::sync::atomic::Ordering::Acquire)
            || self
                .plugin_poll_started
                .swap(true, std::sync::atomic::Ordering::AcqRel)
        {
            return;
        }
        let core = Arc::clone(self);
        let shutdown = Arc::clone(&self.plugin_poll_shutdown);
        let task = runtime.spawn(async move {
            let mut ticker = tokio::time::interval(PLUGIN_POLL_INTERVAL);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tokio::select! {
                    _ = shutdown.notified() => break,
                    _ = ticker.tick() => core.poll_plugin_events().await,
                }
            }
        });
        *self
            .plugin_poll_task
            .lock()
            .expect("plugin poll task lock poisoned") = Some(task);
    }

    pub(crate) async fn poll_plugin_events(self: &Arc<Self>) {
        self.sync_hotkey_bindings().await;
        let candidates: Vec<(String, Vec<String>)> = self
            .plugins
            .list()
            .into_iter()
            .filter(|plugin| self.plugin_ready(&plugin.manifest.id))
            .filter_map(|plugin| {
                let declared = declared_event_types(&plugin.manifest);
                let reports_progress = self
                    .capabilities
                    .require_capability(&plugin.manifest, PLUGIN_PROGRESS_CAPABILITY)
                    .is_ok();
                // Progress-only plugins are started by an explicit action
                // (for example TTS prepare/speak). Do not launch them during
                // the global poll just to discover that they need a model.
                if declared.is_empty() && (!reports_progress || !plugin.running) {
                    return None;
                }
                if !declared.is_empty()
                    && self
                        .capabilities
                        .require_capability(
                            &plugin.manifest,
                            tiktools_plugin_api::capabilities::EVENTS_PUBLISH,
                        )
                        .is_err()
                {
                    return None;
                }
                Some((plugin.manifest.id.clone(), declared))
            })
            .collect();
        if candidates.is_empty() {
            return;
        }
        let source = fresh_poll_context(
            &self
                .last_automation_event
                .read()
                .expect("automation event lock poisoned")
                .clone()
                .unwrap_or_else(|| json!({})),
        );
        const MAX_CONCURRENT_POLLS: usize = 6;
        let mut tasks = tokio::task::JoinSet::new();
        let mut outcomes = Vec::with_capacity(candidates.len());
        for (plugin_id, declared) in candidates {
            if !self.plugin_retry_allowed(&plugin_id) {
                continue;
            }
            if let Err(error) = self.plugins.start(&plugin_id) {
                self.record_plugin_failure(&plugin_id, error.to_string());
                continue;
            }
            let plugins = Arc::clone(&self.plugins);
            let request = serde_json::to_value(tiktools_plugin_sdk::PluginCall::Poll)
                .expect("poll call should always serialize");
            let plugin_id_for_call = plugin_id.clone();
            let declared_for_task = declared.clone();
            tasks.spawn(async move {
                let response = tokio::time::timeout(
                    PLUGIN_POLL_DEADLINE,
                    tokio::task::spawn_blocking(move || {
                        plugins.call_with_timeout(
                            &plugin_id_for_call,
                            &request,
                            PLUGIN_POLL_DEADLINE,
                        )
                    }),
                )
                .await;
                let response = match response {
                    Ok(Ok(Ok(response))) => Ok(response),
                    Ok(Ok(Err(error))) => Err(error.to_string()),
                    Ok(Err(error)) => Err(format!("plugin poll task failed: {error}")),
                    Err(_) => Err("plugin poll timed out".to_owned()),
                };
                (plugin_id, declared_for_task, response)
            });
            if tasks.len() >= MAX_CONCURRENT_POLLS {
                if let Some(Ok(outcome)) = tasks.join_next().await {
                    outcomes.push(outcome);
                }
            }
        }
        while let Some(joined) = tasks.join_next().await {
            if let Ok(outcome) = joined {
                outcomes.push(outcome);
            }
        }
        outcomes.sort_by(|left, right| left.0.cmp(&right.0));
        for (plugin_id, declared, response) in outcomes {
            let response = match response {
                Ok(response) => response,
                Err(error) => {
                    self.record_plugin_failure(&plugin_id, error);
                    continue;
                }
            };
            let response = match tiktools_plugin_sdk::decode_plugin_result(response) {
                Ok(response) => response,
                Err(error) => {
                    self.record_plugin_failure(&plugin_id, error.to_string());
                    continue;
                }
            };
            self.record_plugin_success(&plugin_id);
            if let Some(progress) = parse_plugin_progress(&response) {
                self.emit(HostMessage::PluginProgress {
                    plugin_id: plugin_id.clone(),
                    state: progress.state,
                    progress: progress.progress,
                    message: progress.message,
                });
            }
            for (event_type, data) in parse_polled_events(&declared, &response) {
                self.publish_automation_event(self.make_plugin_event(&source, &event_type, data))
                    .await;
            }
        }
    }

    pub(crate) fn plugin_retry_allowed(&self, id: &str) -> bool {
        self.plugin_health
            .lock()
            .expect("plugin health lock poisoned")
            .get(id)
            .and_then(|health| health.next_retry_at)
            .is_none_or(|next_retry_at| std::time::Instant::now() >= next_retry_at)
    }

    pub(crate) fn record_plugin_failure(&self, id: &str, error: String) {
        let mut health = self
            .plugin_health
            .lock()
            .expect("plugin health lock poisoned");
        let entry = health.entry(id.to_owned()).or_insert(PluginHealth {
            consecutive_failures: 0,
            next_retry_at: None,
        });
        entry.consecutive_failures = entry.consecutive_failures.saturating_add(1);
        let delay_seconds = match entry.consecutive_failures {
            1 => 1,
            2 => 2,
            3 => 5,
            4 => 10,
            _ => 30,
        };
        entry.next_retry_at =
            Some(std::time::Instant::now() + std::time::Duration::from_secs(delay_seconds));
        if entry.consecutive_failures <= 5 {
            tracing::warn!(
                plugin = %id,
                failures = entry.consecutive_failures,
                retry_in_seconds = delay_seconds,
                %error,
                "plugin entered retry backoff"
            );
        }
    }

    pub(crate) fn record_plugin_success(&self, id: &str) {
        let was_unhealthy = self
            .plugin_health
            .lock()
            .expect("plugin health lock poisoned")
            .remove(id)
            .is_some_and(|health| health.consecutive_failures > 0);
        if was_unhealthy {
            tracing::info!(plugin = %id, "plugin recovered");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_timeout_defaults_to_thirty_seconds() {
        let descriptor: PluginActionDescriptor = serde_json::from_value(json!({
            "id": "demo.action"
        }))
        .unwrap();
        assert_eq!(descriptor.timeout(), std::time::Duration::from_secs(30));
    }

    #[test]
    fn action_timeout_uses_manifest_milliseconds() {
        let descriptor: PluginActionDescriptor = serde_json::from_value(json!({
            "id": "demo.action",
            "timeoutMs": 120000
        }))
        .unwrap();
        assert_eq!(descriptor.timeout(), std::time::Duration::from_secs(120));
    }
}
