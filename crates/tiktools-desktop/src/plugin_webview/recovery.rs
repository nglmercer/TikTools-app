use super::*;

impl PluginUiWindows {
    /// Bounded recovery poll for pop-out windows, run on the UI thread.
    /// For each open window the bridge watchdog is checked: first expiry
    /// attempts one controlled `reload()`, the next expiry closes the
    /// faulty window and queues one automatic recreate (bounded per page),
    /// then parks without further retries. Never restarts `AppCore`, never
    /// replays broker calls, never loops.
    pub(crate) fn poll_recovery(&mut self) {
        let now = std::time::Instant::now();
        let keys: Vec<(String, String)> = self.windows.keys().cloned().collect();
        let mut to_close: Vec<(String, String)> = Vec::new();
        for key in keys {
            let Some(window) = self.windows.get_mut(&key) else {
                continue;
            };
            if window.state != PluginWindowState::Open {
                continue;
            }
            window.sync_generation();
            match window.lifecycle.check_timeout(now) {
                RecoveryDecision::Wait => {}
                RecoveryDecision::ReloadOnce => {
                    tracing::warn!(
                        plugin = %key.0,
                        page = %key.1,
                        reason = failure_reasons::RELOAD_TIMEOUT,
                        "plugin WebView reload timeout: attempting one controlled navigation recovery"
                    );
                    window.output.discard_pending();
                    match window.view().reload() {
                        Ok(()) => {
                            tracing::info!(
                                plugin = %key.0,
                                page = %key.1,
                                "plugin WebView navigation recovery reload issued"
                            );
                        }
                        Err(error) => {
                            tracing::error!(%error, plugin = %key.0, page = %key.1, "plugin WebView reload failed; closing");
                            to_close.push(key.clone());
                        }
                    }
                }
                RecoveryDecision::Recreate => {
                    tracing::warn!(
                        plugin = %key.0,
                        page = %key.1,
                        reason = failure_reasons::RELOAD_TIMEOUT,
                        "plugin WebView reload timeout persists: recreating the window"
                    );
                    to_close.push(key.clone());
                    let budget = self.recreate_budget.get(&key).copied().unwrap_or(0);
                    if budget < 1 {
                        self.recreate_budget.insert(key.clone(), budget + 1);
                        self.pending_recreates.push(key.clone());
                    } else {
                        tracing::error!(
                            plugin = %key.0,
                            page = %key.1,
                            "plugin WebView recreate budget exhausted; not reopening"
                        );
                    }
                }
                RecoveryDecision::GiveUp => {
                    tracing::error!(
                        plugin = %key.0,
                        page = %key.1,
                        reason = ?window.lifecycle.failure_reason(),
                        "plugin WebView recovery exhausted; closing without further retries"
                    );
                    to_close.push(key.clone());
                }
            }
        }
        for (plugin, page) in to_close {
            self.close(&plugin, &page);
        }
    }

    /// Drains plugin pages queued for automatic recreate (at most one per
    /// page per failure sequence). The caller reopens them on the UI thread
    /// with the same `open` path, preserving `AppCore` and user data.
    pub(crate) fn drain_pending_recreates(&mut self) -> Vec<(String, String)> {
        std::mem::take(&mut self.pending_recreates)
    }

    /// Opens the inspector for one plugin window on the UI thread.
    /// Release builds without the `devtools` cargo feature keep the
    /// shortcut compiled but an intentional no-op, mirroring the main
    /// window.
    pub(crate) fn open_devtools_for_window(&self, window_id: WindowId) {
        #[cfg(any(debug_assertions, feature = "devtools"))]
        {
            for window in self.windows.values() {
                if window.state == PluginWindowState::Open && window.handle().id() == window_id {
                    window.view().open_devtools();
                    return;
                }
            }
        }
        #[cfg(not(any(debug_assertions, feature = "devtools")))]
        {
            let _ = window_id;
        }
    }

    pub(crate) fn diagnostics(&self) -> Value {
        let mut queued = 0;
        let mut bytes = 0;
        let mut tasks = 0;
        let mut loading = 0;
        let mut ready = 0;
        let mut failed = 0;
        let mut recovering = 0;
        for window in self.windows.values() {
            let snapshot = window.output.diagnostics();
            queued += snapshot["reliableCount"].as_u64().unwrap_or(0);
            bytes += snapshot["reliableBytes"].as_u64().unwrap_or(0);
            tasks += 128usize.saturating_sub(window.ipc_slots.available_permits());
            match window.lifecycle.state() {
                WebviewLifecycle::Loading => loading += 1,
                WebviewLifecycle::Ready => ready += 1,
                WebviewLifecycle::Failed => failed += 1,
                WebviewLifecycle::Recovering => recovering += 1,
            }
        }
        serde_json::json!({"openWindows": self.windows.len(), "queued": queued, "bytes": bytes, "brokerTasks": tasks, "evaluateCalls": self.evaluate_calls, "evaluateBytes": self.evaluate_bytes, "evaluateErrors": self.evaluate_errors, "lifecycle": {"loading": loading, "ready": ready, "failed": failed, "recovering": recovering}, "pendingRecreates": self.pending_recreates.len()})
    }
}
