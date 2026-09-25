//! Spontaneous plugin-event polling.

use crate::*;

impl AppCore {
    pub(crate) async fn poll_plugin_events(self: &Arc<Self>) {
        self.sync_hotkey_bindings().await;
        let candidates: Vec<(String, Vec<String>)> = self
            .plugins
            .list()
            .into_iter()
            .filter(|plugin| self.plugin_ready(&plugin.manifest.id))
            // Declarative packages publish no spontaneous events; the host
            // interprets their actions on demand instead of polling them.
            .filter(|plugin| {
                plugin.manifest.runtime != tiktools_plugin_api::PluginRuntimeKind::Declarative
            })
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
            &recover_rwlock_read(&self.automation_state.last_event, "automation event")
                .clone()
                .unwrap_or_else(|| json!({})),
        );
        const MAX_CONCURRENT_POLLS: usize = 6;
        let invoker = crate::plugin_invoker::PluginInvoker::new(Arc::clone(&self.plugins));
        let mut tasks = tokio::task::JoinSet::new();
        for (plugin_id, declared) in candidates {
            if !self.plugin_retry_allowed(&plugin_id) {
                continue;
            }
            let request = serde_json::to_value(tiktools_plugin_sdk::PluginCall::Poll)
                .expect("poll call should always serialize");
            let plugin_id_for_call = plugin_id.clone();
            let declared_for_task = declared.clone();
            let invoker_for_task = invoker.clone();
            tasks.spawn(async move {
                let response = match invoker_for_task
                    .call(&plugin_id_for_call, &request, PLUGIN_POLL_DEADLINE)
                    .await
                {
                    Ok(response) => Ok(response),
                    Err(crate::plugin_invoker::InvokeError::Timeout) => {
                        Err("plugin poll timed out".to_owned())
                    }
                    Err(
                        crate::plugin_invoker::InvokeError::Unavailable(reason)
                        | crate::plugin_invoker::InvokeError::Plugin(reason),
                    ) => Err(reason),
                };
                (plugin_id, declared_for_task, response)
            });
            if tasks.len() >= MAX_CONCURRENT_POLLS {
                if let Some(Ok((plugin_id, declared, response))) = tasks.join_next().await {
                    self.handle_poll_outcome(plugin_id, declared, response, &source)
                        .await;
                }
            }
        }
        // Outcomes are processed as each poll completes: a slow sibling
        // must not delay another plugin's events behind a batch drain.
        while let Some(joined) = tasks.join_next().await {
            if let Ok((plugin_id, declared, response)) = joined {
                self.handle_poll_outcome(plugin_id, declared, response, &source)
                    .await;
            }
        }
    }

    /// Validates one completed poll response and publishes its events
    /// immediately. Runs per completion (not per batch) so fast plugins
    /// never wait for slow siblings.
    async fn handle_poll_outcome(
        self: &Arc<Self>,
        plugin_id: String,
        declared: Vec<String>,
        response: Result<serde_json::Value, String>,
        source: &serde_json::Value,
    ) {
        let response = match response {
            Ok(response) => response,
            Err(error) => {
                self.record_plugin_failure(&plugin_id, error);
                return;
            }
        };
        let response = match tiktools_plugin_sdk::decode_plugin_result(response) {
            Ok(response) => response,
            Err(error) => {
                self.record_plugin_failure(&plugin_id, error.to_string());
                return;
            }
        };
        self.record_plugin_success(&plugin_id);
        if let Some(progress) = parse_plugin_progress(&response) {
            // Poll progress travels on the domain topic; the legacy
            // push was removed with the migrated duplicates.
            let state = match progress.state {
                crate::ipc::messages::PluginProgressState::Downloading => "downloading",
                crate::ipc::messages::PluginProgressState::Loading => "loading",
                crate::ipc::messages::PluginProgressState::Ready => "ready",
                crate::ipc::messages::PluginProgressState::Failed => "failed",
            };
            self.events
                .publish_domain(crate::events::DomainEvent::PluginProgress {
                    plugin_id: plugin_id.clone(),
                    state: state.to_owned(),
                    progress: progress.progress,
                    message: progress.message,
                });
        }
        // Plugin-authored poll logs are operational notes (queue
        // overflow, backend transitions), never key contents: surface
        // them so a struggling plugin cannot look healthy.
        for log in response.logs.iter().take(8) {
            tracing::warn!(plugin = %plugin_id, message = %log, "plugin poll reported a warning");
        }
        let parsed = parse_polled_events(&declared, &response);
        let dropped = parsed.dropped();
        if dropped > 0 {
            self.record_plugin_drops(&plugin_id, dropped);
            tracing::warn!(
                plugin = %plugin_id,
                polled = response.events.len(),
                accepted = parsed.events.len(),
                truncated = parsed.truncated,
                undeclared = parsed.undeclared,
                invalid = parsed.invalid,
                "plugin poll events dropped during validation"
            );
        } else if !parsed.events.is_empty() {
            tracing::debug!(
                plugin = %plugin_id,
                polled = response.events.len(),
                accepted = parsed.events.len(),
                "plugin poll events accepted"
            );
        }
        for (event_type, data) in parsed.events {
            tracing::debug!(
                plugin = %plugin_id,
                event_type = %event_type,
                "validated plugin event entering automation"
            );
            // `publish_automation_event` publishes the authoritative
            // `plugin.event` domain event (and records diagnostics)
            // before enrichment/execution, so control-plane visibility
            // never depends on automation success.
            self.publish_automation_event(self.make_plugin_event(
                &plugin_id,
                &source,
                &event_type,
                data,
            ))
            .await;
        }
    }
}
