use super::*;

impl PluginUiWindows {
    pub fn apply_subscription(
        &mut self,
        plugin_id: &str,
        page_id: &str,
        effect: crate::event::PluginUiSubscription,
        window_id: WindowId,
        generation: u64,
    ) {
        let Some(window) = self
            .windows
            .get_mut(&(plugin_id.to_owned(), page_id.to_owned()))
        else {
            return;
        };
        if window.handle().id() != window_id
            || window.state != PluginWindowState::Open
            || window.generation.load(Ordering::Acquire) != generation
        {
            return;
        }
        window.sync_generation();
        // A subscription round-trip proves the bridge is live on this
        // generation (post-navigation readiness check).
        window.mark_bridge_ready(plugin_id, page_id);
        if window.lifecycle.bridge() == PluginBridgeState::Ready {
            // A working bridge resets the auto-recreate budget: the next
            // failure sequence gets its own bounded recovery.
            self.recreate_budget
                .remove(&(plugin_id.to_owned(), page_id.to_owned()));
        }
        if let Err(error) = apply_topics(&mut window.topics, effect) {
            tracing::warn!(error, "legacy subscription effect rejected");
        }
    }

    /// Delivers one broker response to the calling window. Late
    /// responses for closed (or closing) windows are ignored safely: the
    /// entry is gone or skipped, so nothing ever evaluates script on a
    /// dead WebView.
    pub fn respond(
        &mut self,
        plugin_id: &str,
        page_id: &str,
        response: Arc<str>,
        window_id: WindowId,
        generation: u64,
        effect: Option<crate::event::PluginUiSubscription>,
    ) {
        let Some(window) = self
            .windows
            .get_mut(&(plugin_id.to_owned(), page_id.to_owned()))
            .filter(|entry| {
                entry.state == PluginWindowState::Open
                    && entry.handle().id() == window_id
                    && entry.generation.load(Ordering::Acquire) == generation
            })
        else {
            return;
        };
        window.sync_generation();
        // Any broker response proves the init script installed
        // `window.tiktools` on this generation.
        window.mark_bridge_ready(plugin_id, page_id);
        if window.lifecycle.bridge() == PluginBridgeState::Ready {
            self.recreate_budget
                .remove(&(plugin_id.to_owned(), page_id.to_owned()));
        }
        let rejection = effect.and_then(|effect| apply_topics(&mut window.topics, effect).err());
        if let Some(error) = rejection {
            let id = serde_json::from_str::<Value>(&response)
                .ok()
                .and_then(|value| value.get("id").cloned())
                .unwrap_or(Value::Null);
            window.output.push(
                serde_json::json!({"apiVersion":1,"id":id,"ok":false,"error":error}).to_string(),
            );
        } else {
            window.output.push_critical(response);
        }
    }
}

pub(super) fn apply_topics(
    current: &mut HashSet<String>,
    effect: crate::event::PluginUiSubscription,
) -> Result<(), &'static str> {
    match effect {
        crate::event::PluginUiSubscription::Subscribe(topics) => {
            if current
                .iter()
                .cloned()
                .chain(topics.iter().cloned())
                .collect::<HashSet<_>>()
                .len()
                > 32
            {
                return Err("plugin broker subscription capacity exhausted");
            }
            current.extend(topics);
        }
        crate::event::PluginUiSubscription::Unsubscribe(topics) => {
            for topic in topics {
                current.remove(&topic);
            }
        }
    }
    Ok(())
}
