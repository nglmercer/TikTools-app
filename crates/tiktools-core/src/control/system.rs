//! System health, lifecycle flags, diagnostics, and app-state operations.

use super::OperationError;
use crate::*;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DoctorCheck {
    pub id: String,
    pub status: String,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DoctorReport {
    pub ok: bool,
    pub checks: Vec<DoctorCheck>,
}

impl AppCore {
    /// Returns whether [`AppCore::shutdown`] has started.
    pub fn is_shutdown(&self) -> bool {
        self.shutdown_started.load(Ordering::Acquire)
    }

    /// Records a control IPC failure so `system.health` reports degraded
    /// instead of silently losing CLI/agent connectivity while the GUI
    /// looks healthy. `None` clears the degraded state after a retry.
    pub fn set_ipc_error(&self, message: Option<String>) {
        *self.ipc_error.write().expect("ipc error lock poisoned") = message;
    }

    pub fn ipc_error(&self) -> Option<String> {
        self.ipc_error
            .read()
            .expect("ipc error lock poisoned")
            .clone()
    }

    /// Records a broken WebView transport (reliable outbox overflow) so
    /// `system.health` reports degraded instead of silently losing the UI
    /// while RPCs fail. `None` clears the degraded state after recovery.
    pub fn set_webview_error(&self, message: Option<String>) {
        *self
            .webview_error
            .write()
            .expect("webview error lock poisoned") = message;
    }

    pub fn webview_error(&self) -> Option<String> {
        self.webview_error
            .read()
            .expect("webview error lock poisoned")
            .clone()
    }

    /// Records one reliable-lane event gap observed by a transport. The
    /// transport already sent the affected client an explicit `event.gap`
    /// notification; this cumulative counter plus timestamp lets
    /// `system.health` tell agents their local state may be stale.
    pub fn record_event_gap(&self) {
        self.event_gaps.fetch_add(1, Ordering::Relaxed);
        self.last_event_gap_at
            .store(now_millis(), Ordering::Relaxed);
    }

    /// Cumulative reliable gaps plus the most recent gap timestamp
    /// (`None` when no gap was recorded since boot/acknowledge).
    pub fn event_gap_stats(&self) -> (u64, Option<u64>) {
        let gaps = self.event_gaps.load(Ordering::Relaxed);
        let at = self.last_event_gap_at.load(Ordering::Relaxed);
        (gaps, if at == 0 { None } else { Some(at) })
    }

    /// Clears recorded event gaps after every affected client resynced
    /// authoritative state, returning health to OK. Only call this once
    /// resync completed; clearing early hides staleness from agents that
    /// have not refreshed yet.
    pub fn acknowledge_event_gaps(&self) {
        self.event_gaps.store(0, Ordering::Relaxed);
        self.last_event_gap_at.store(0, Ordering::Relaxed);
    }

    // ------------------------------------------------------------------
    // System: info, health, snapshot, doctor.
    // ------------------------------------------------------------------

    pub fn system_info(&self) -> Value {
        json!({
            "name": "tiktools",
            "version": env!("CARGO_PKG_VERSION"),
            "features": {
                "nativeTikTok": cfg!(feature = "native-tiktok"),
                "persistence": cfg!(feature = "persistence"),
                "pluginInstall": cfg!(feature = "plugin-install"),
                "http": cfg!(feature = "http"),
            },
            "home": self.db.paths().root.display().to_string(),
            "dataDir": self.db.paths().data.display().to_string(),
            "pid": std::process::id(),
        })
    }

    pub fn system_health(&self) -> Value {
        let plugins = self.plugins.list();
        let running = plugins.iter().filter(|plugin| plugin.running).count();
        let unavailable = plugins.iter().filter(|plugin| !plugin.available).count();
        let mut degraded: Vec<String> = Vec::new();
        if unavailable > 0 {
            degraded.push(format!("{unavailable} plugin(s) unavailable"));
        }
        #[cfg(feature = "http")]
        if let Some(message) = self.http_client_error.as_ref() {
            degraded.push(message.clone());
        }
        if let Some(message) = self.ipc_error() {
            degraded.push(message);
        }
        if let Some(message) = self.webview_error() {
            degraded.push(message);
        }
        let (event_gaps, last_gap_at) = self.event_gap_stats();
        if event_gaps > 0 {
            degraded.push(format!(
                "{event_gaps} reliable event gap(s); resync authoritative state"
            ));
        }
        let dropped_plugin_events = self.plugin_drop_total();
        if dropped_plugin_events > 0 {
            degraded.push(format!(
                "{dropped_plugin_events} plugin event(s) dropped during validation or delivery; see plugins.diagnostics"
            ));
        }
        json!({
            "status": if degraded.is_empty() { "ok" } else { "degraded" },
            "reasons": degraded,
            "events": {
                "status": if event_gaps > 0 { "degraded" } else { "ok" },
                "reliableGaps": event_gaps,
                "lastGapAt": last_gap_at,
            },
            "liveConnected": self.live.is_connected(),
            "plugins": {
                "total": plugins.len(),
                "running": running,
                "unavailable": unavailable,
                "droppedEvents": dropped_plugin_events,
            },
            "processors": self.processor_list().len(),
            "shutdown": self.is_shutdown(),
        })
    }

    /// Safe observable state. Never includes settings values or secrets.
    pub fn system_snapshot(&self) -> Value {
        let behavior = self.load_merged_behavior_snapshot();
        json!({
            "live": serde_json::to_value(self.live_status()).unwrap_or(Value::Null),
            "plugins": behavior.get("plugins").cloned().unwrap_or(Value::Array(vec![])),
            "processors": self.processor_status(),
            "automations": {
                "events": behavior.get("events").cloned().unwrap_or(Value::Array(vec![])),
                "actions": behavior.get("actions").cloned().unwrap_or(Value::Array(vec![])),
            },
            "pointsConfig": serde_json::to_value(self.points.config()).unwrap_or(Value::Null),
            "health": self.system_health(),
        })
    }

    pub fn system_doctor(&self) -> DoctorReport {
        let mut checks = Vec::new();
        let paths = self.db.paths();
        let writable = paths.temp.exists()
            && std::fs::write(paths.temp.join(".doctor-write-test"), b"ok")
                .and_then(|()| std::fs::remove_file(paths.temp.join(".doctor-write-test")))
                .is_ok();
        checks.push(DoctorCheck {
            id: "storage.temp-writable".to_owned(),
            status: if writable { "ok" } else { "failed" }.to_owned(),
            message: Some(paths.temp.display().to_string()),
        });
        #[cfg(feature = "persistence")]
        {
            match self.db.load_behavior_snapshot() {
                Ok(_) => checks.push(DoctorCheck {
                    id: "database.behavior".to_owned(),
                    status: "ok".to_owned(),
                    message: None,
                }),
                Err(error) => checks.push(DoctorCheck {
                    id: "database.behavior".to_owned(),
                    status: "failed".to_owned(),
                    message: Some(error.to_string()),
                }),
            }
        }
        #[cfg(not(feature = "persistence"))]
        checks.push(DoctorCheck {
            id: "database.behavior".to_owned(),
            status: "skipped".to_owned(),
            message: Some("persistence is disabled in this build".to_owned()),
        });
        for plugin in self.plugins.list() {
            if plugin.available {
                checks.push(DoctorCheck {
                    id: format!("plugin.{}.available", plugin.manifest.id),
                    status: "ok".to_owned(),
                    message: None,
                });
            } else {
                checks.push(DoctorCheck {
                    id: format!("plugin.{}.available", plugin.manifest.id),
                    status: "failed".to_owned(),
                    message: plugin
                        .reason
                        .clone()
                        .or_else(|| Some("plugin is unavailable".to_owned())),
                });
            }
        }
        if cfg!(feature = "native-tiktok") {
            let connected = self.live.is_connected();
            checks.push(DoctorCheck {
                id: "live.transport".to_owned(),
                status: "ok".to_owned(),
                message: Some(
                    if connected {
                        "connected"
                    } else {
                        "disconnected"
                    }
                    .to_owned(),
                ),
            });
        } else {
            checks.push(DoctorCheck {
                id: "live.transport".to_owned(),
                status: "skipped".to_owned(),
                message: Some("native TikTok client is disabled in this build".to_owned()),
            });
        }
        let processors = self.processor_list().len();
        checks.push(DoctorCheck {
            id: "processors.index".to_owned(),
            status: "ok".to_owned(),
            message: Some(format!("{processors} processor(s) indexed")),
        });
        let ok = !checks.iter().any(|check| check.status == "failed");
        DoctorReport { ok, checks }
    }

    pub fn app_state_get(
        &self,
        keys: Option<&[String]>,
    ) -> Result<BTreeMap<String, String>, OperationError> {
        if let Some(keys) = keys {
            if keys.len() > 256 {
                return Err(OperationError::invalid("at most 256 keys per request"));
            }
            if keys.iter().any(|key| key.is_empty() || key.len() > 256) {
                return Err(OperationError::invalid(
                    "app state keys must be 1..=256 characters",
                ));
            }
        }
        let state = self.app_state.read(keys);
        #[cfg(feature = "persistence")]
        let state = {
            let mut state = state;
            match self.db.load_app_state() {
                Ok(persisted) => {
                    state = persisted
                        .into_iter()
                        .filter_map(|(key, value)| {
                            value.as_str().map(|value| (key, value.to_owned()))
                        })
                        .filter(|(key, _)| {
                            keys.is_none_or(|keys| keys.is_empty() || keys.contains(key))
                        })
                        .collect();
                }
                Err(error) => tracing::warn!(%error, "could not load app state"),
            }
            state
        };
        Ok(state)
    }

    pub fn app_state_set(
        &self,
        key: &str,
        value: &str,
    ) -> Result<BTreeMap<String, String>, OperationError> {
        if key.is_empty() || key.len() > 256 {
            return Err(OperationError::invalid(
                "app state key must be 1..=256 characters",
            ));
        }
        if value.len() > 65_536 {
            return Err(OperationError::invalid(
                "app state value exceeds 65536 characters",
            ));
        }
        self.app_state.set(key.to_owned(), value.to_owned());
        #[cfg(feature = "persistence")]
        if let Err(error) = self.db.save_app_state(key, value) {
            tracing::warn!(%error, "could not persist app state");
        }
        Ok([(key.to_owned(), value.to_owned())].into_iter().collect())
    }
}
