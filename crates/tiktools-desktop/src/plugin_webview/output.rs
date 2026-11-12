use super::*;

impl PluginUiWindows {
    /// Plugin event topics have no general loss/resync contract. Keep these
    /// reliable rather than silently treating arbitrary plugin events as feed.
    #[cfg(test)]
    pub fn deliver_event(&mut self, plugin_id: &str, topic: &str, data: &Value) {
        let envelope = serde_json::json!({
            "apiVersion": broker::BROKER_API_VERSION, "event": topic, "data": data,
        });
        for ((owner, _), window) in &mut self.windows {
            window.sync_generation();
            if owner == plugin_id
                && window.state == PluginWindowState::Open
                && window.topics.contains(topic)
            {
                window.output.push_value(&envelope);
            }
        }
    }

    /// Fair schedule: at most one capped evaluation per open window per tick.
    /// A failed/overflowed reliable transport closes its owner coherently;
    /// pagehide rejects broker promises. Nothing retries side effects.
    pub(crate) fn flush_batches(&mut self) -> bool {
        let mut failed = Vec::new();
        let mut pending = false;
        let mut keys: Vec<_> = self.windows.keys().cloned().collect();
        keys.sort();
        let selected: Vec<_> = keys
            .iter()
            .cycle()
            .skip(self.flush_cursor)
            .take(keys.len().min(4))
            .cloned()
            .collect();
        self.flush_cursor = if keys.is_empty() {
            0
        } else {
            (self.flush_cursor + selected.len()) % keys.len()
        };
        for key in selected {
            let window = self
                .windows
                .get_mut(&key)
                .expect("selected window is map resident");
            window.sync_generation();
            if window.state != PluginWindowState::Open {
                continue;
            }
            // Hold batches until the post-navigation bridge check passes:
            // the first broker round-trip proves `window.tiktools` is live.
            // Queued output waits (still counted as pending for wake
            // chaining) instead of evaluating into a page without a bridge.
            if !window.lifecycle.is_delivery_allowed() {
                pending |= !window.output.is_empty();
                continue;
            }
            if window.output.take_transport_failure() {
                failed.push(key.clone());
                continue;
            }
            let batch = window.output.take_prepared_batch();
            if !batch.is_empty() {
                let envelopes = batch
                    .iter()
                    .map(|message| message.body.as_ref())
                    .collect::<Vec<_>>()
                    .join(",");
                let script = format!("if (typeof window.__tiktools_broker_push__ === 'function') {{ for (const item of [{envelopes}]) window.__tiktools_broker_push__(item); }}");
                self.evaluate_calls = self.evaluate_calls.saturating_add(1);
                self.evaluate_bytes = self.evaluate_bytes.saturating_add(script.len() as u64);
                if let Err(error) = window.view().evaluate_script(&script) {
                    self.evaluate_errors = self.evaluate_errors.saturating_add(1);
                    tracing::error!(%error, plugin = %key.0, page = %key.1, reason = failure_reasons::EVALUATION_FAILED, "plugin WebView lifecycle Failed: evaluation failed; closing its transport");
                    failed.push(key.clone());
                }
            }
            pending |= !window.output.is_empty();
        }
        for (plugin, page) in failed {
            self.close(&plugin, &page);
        }
        pending
            || self
                .windows
                .values()
                .any(|window| window.state == PluginWindowState::Open && !window.output.is_empty())
    }

    pub(crate) fn forward_prepared(
        &mut self,
        message: &crate::window::outbox::QueuedWebviewMessage,
    ) {
        let Some(route) = message.plugin.as_ref() else {
            return;
        };
        if is_teardown_topic(&route.topic) {
            self.close_plugin(&route.plugin_id);
            return;
        }
        for ((owner, _), window) in &mut self.windows {
            window.sync_generation();
            if owner == &route.plugin_id
                && window.state == PluginWindowState::Open
                && window.topics.contains(&route.topic)
            {
                window.output.push_critical(message.body.clone());
            }
        }
    }

    /// Fans host JSON-RPC `event` notifications out to subscribed plugin
    /// windows, and closes windows whose plugin went away. Called with the
    /// same already-queued main-window payload (host-generated, bounded);
    /// unparseable input is ignored — the main window path owns delivery.
    #[cfg(test)]
    pub fn forward_host_message(&mut self, message: &str) {
        let Some((plugin_id, topic, data)) = parse_host_notification(message) else {
            return;
        };
        if is_teardown_topic(&topic) {
            self.close_plugin(&plugin_id);
        } else {
            self.deliver_event(&plugin_id, &topic, &data);
        }
    }
}

/// Topics that tear down a plugin's windows: the runtime went away
/// (stop covers disable; uninstall covers removal), so no window may
/// outlive it.
pub(super) fn is_teardown_topic(topic: &str) -> bool {
    matches!(topic, "plugin.uninstalled" | "plugin.stopped")
}

/// Extracts `(plugin_id, topic, data)` from a host `event` notification.
/// Only `plugin.*` topics with a string `pluginId` route to plugin
/// windows; everything else (including gap markers) is ignored.
#[cfg(test)]
pub(super) fn parse_host_notification(message: &str) -> Option<(String, String, Value)> {
    if !message.contains("\"method\":\"event\"") && !message.contains("\"method\": \"event\"") {
        return None;
    }
    let value: Value = serde_json::from_str(message).ok()?;
    let params = value.get("params")?;
    let topic = params.get("topic")?.as_str()?;
    if !topic.starts_with("plugin.") {
        return None;
    }
    let data = params.get("data")?.clone();
    let plugin_id = data.get("pluginId")?.as_str()?.to_owned();
    Some((plugin_id, topic.to_owned(), data))
}
