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

/// Host RSS without dependencies: parses `/proc/self/status` on Linux.
/// Every other platform reports `None` rather than a fabricated value;
/// per-process plugin/WebView attribution stays with the external Linux
/// profiler (`scripts/profile-live-memory.py`), which classifies by
/// process name, because child PIDs live inside loader worker threads
/// and are not cheaply observable from the health path.
#[cfg(target_os = "linux")]
fn host_rss_bytes() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            let kib: u64 = rest.split_whitespace().next()?.parse().ok()?;
            return Some(kib.saturating_mul(1024));
        }
    }
    None
}

#[cfg(not(target_os = "linux"))]
fn host_rss_bytes() -> Option<u64> {
    None
}

/// Machine-readable memory section for `system.health`. Only the host RSS
/// is in-process observable (Linux); plugin and WebView attribution are
/// `null` here by design — the supported route is the external profiler,
/// which reads them from the OS without guessing.
fn host_memory_diagnostics() -> Value {
    json!({
        "hostRssBytes": host_rss_bytes(),
        "pluginRssBytes": Value::Null,
        "webviewRssBytes": Value::Null,
        "rssSource": if cfg!(target_os = "linux") { "proc" } else { "unsupported" },
        "note": "plugin/webview RSS: use scripts/profile-live-memory.py (Linux); health never fabricates per-process values",
    })
}

impl AppCore {
    /// Returns whether [`AppCore::shutdown`] has started.
    pub fn is_shutdown(&self) -> bool {
        self.shutdown_started.load(Ordering::Acquire)
    }

    // ------------------------------------------------------------------
    // System: info, health, snapshot, doctor.
    // ------------------------------------------------------------------

    pub fn system_info(&self) -> Value {
        json!({
            "platform": crate::platform::RuntimePlatform::detect(),
            "platformCapabilities": crate::platform::RuntimePlatform::detect().capabilities(),
            "name": "tiktools",
            "version": env!("CARGO_PKG_VERSION"),
            "features": {
                "nativeTikTok": cfg!(feature = "native-tiktok"),
                "persistence": cfg!(feature = "persistence"),
                "pluginInstall": cfg!(feature = "plugin-install"),
                "http": cfg!(feature = "http"),
                "napiVmNodeApi": cfg!(all(feature = "napi-vm-node-api", not(target_os = "android"))),
            },
            "home": self.db.paths().root.display().to_string(),
            "dataDir": self.db.paths().data.display().to_string(),
            "pid": std::process::id(),
        })
    }

    /// Probes Linux/Wayland raw-input readability and, when blocked,
    /// installs the seat rule through one polkit prompt (or directly when
    /// already privileged). Powers the UI "grant access" button so users
    /// never need terminal commands. Blocking (subprocesses): callers must
    /// stay off Tokio workers.
    pub fn request_input_access(&self) -> crate::input_access::InputAccessResult {
        crate::input_access::request_input_access()
    }

    pub fn system_health(&self) -> Value {
        #[cfg(feature = "persistence")]
        let analytics = self.analytics_writer.diagnostics();
        #[cfg(not(feature = "persistence"))]
        let analytics = Value::Null;
        #[cfg(feature = "native-tiktok")]
        let automation_active =
            32usize.saturating_sub(self.automation_state.slots.available_permits());
        #[cfg(not(feature = "native-tiktok"))]
        let automation_active = 0usize;
        let automation_admitted = self.automation_state.admitted.load(Ordering::Relaxed);
        let automation_rejected = self.automation_state.rejected.load(Ordering::Relaxed);
        let automation_completed = self
            .automation_state
            .events_completed
            .load(Ordering::Relaxed);
        let automation_ok = self.automation_state.actions_ok.load(Ordering::Relaxed);
        let automation_failed = self.automation_state.actions_failed.load(Ordering::Relaxed);
        let automation_latency = self
            .automation_state
            .exec_latency_sum_ms
            .load(Ordering::Relaxed);
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
        #[cfg(feature = "native-tiktok")]
        let live_ingress = self.live_ingress.diagnostics();
        #[cfg(not(feature = "native-tiktok"))]
        let live_ingress = Value::Null;
        #[cfg(feature = "native-tiktok")]
        let live_dedup = self.live_dedup.diagnostics();
        #[cfg(not(feature = "native-tiktok"))]
        let live_dedup = Value::Null;
        #[cfg(feature = "native-tiktok")]
        let live_generation_seq = self.connection_sequence.load(Ordering::Acquire);
        #[cfg(not(feature = "native-tiktok"))]
        let live_generation_seq = 0u64;
        let points_diagnostics = self.points.diagnostics();
        // Rejected or permanently failed writes are user-visible data loss:
        // memory served the update, SQLite did not receive it. Surface that
        // as degraded health instead of leaving it buried in counters.
        if points_diagnostics.write_rejected > 0 || points_diagnostics.write_failed > 0 {
            degraded.push(format!(
                "{} points write(s) rejected and {} failed; those updates are memory-only; see points diagnostics",
                points_diagnostics.write_rejected, points_diagnostics.write_failed
            ));
        }
        let points = points_diagnostics.to_value();
        let plugin_delivery = crate::plugin_events::fanout::delivery_snapshot(&recover_mutex(
            &self.plugin_delivery,
            "plugin delivery",
        ));
        let (
            processor_calls,
            processor_ok,
            processor_failed,
            processor_timeouts,
            processor_overloaded,
            processor_latency,
        ) = self.processor_totals();
        let memory = host_memory_diagnostics();
        #[cfg(feature = "native-tiktok")]
        let (live_pump_alive, live_worker_alive) = (
            recover_mutex(&self.live_pump_task, "LIVE pump").is_some(),
            recover_mutex(&self.live_ingress_task, "LIVE ingress worker").is_some(),
        );
        #[cfg(not(feature = "native-tiktok"))]
        let (live_pump_alive, live_worker_alive) = (false, false);
        #[cfg(feature = "native-tiktok")]
        let live_connection =
            serde_json::to_value(self.live.connection_snapshot()).expect("connection snapshot");
        #[cfg(not(feature = "native-tiktok"))]
        let live_connection = Value::Null;
        #[cfg(feature = "native-tiktok")]
        let live_points_work = self.live_points_work.diagnostics();
        #[cfg(not(feature = "native-tiktok"))]
        let live_points_work = Value::Null;
        // An abandoned points worker means awards behind a wedge may never
        // reach SQLite: report it as degraded health, not a buried counter.
        #[cfg(feature = "native-tiktok")]
        if live_points_work
            .get("abandonedJoins")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            > 0
        {
            degraded.push(
                "LIVE points worker abandoned past its join deadline; queued awards may remain unresolved".to_owned(),
            );
        }
        json!({
            "status": if degraded.is_empty() { "ok" } else { "degraded" },
            "reasons": degraded,
            "events": {
                "status": if event_gaps > 0 { "degraded" } else { "ok" },
                "reliableGaps": event_gaps,
                "lastGapAt": last_gap_at,
            },
            "pid": std::process::id(),
            "liveConnected": self.live.is_connected(),
            "liveGeneration": recover_rwlock_read(&self.connection_context, "connection context").as_ref().map(|context| context.connection_id.clone()),
            "liveConnection": live_connection,
            "livePointsWork": live_points_work,
            "liveGenerationSeq": live_generation_seq,
            "liveIngress": live_ingress,
            "liveDedup": live_dedup,
            "liveMonitor": self.live_monitor_status(),
            "points": points,
            "memory": memory,
            "desktop": self.emitter.diagnostics(),
            "tasks": {
                "livePump": live_pump_alive,
                "liveWorker": live_worker_alive,
                "pluginWorkers": self.plugin_state.observer_worker_count.load(Ordering::Acquire),
                "automationActive": automation_active,
            },
            "analytics": analytics,
            "automationActiveTasks": automation_active,
            "automation": {
                "activeTasks": automation_active,
                "admitted": automation_admitted,
                "rejected": automation_rejected,
                "eventsCompleted": automation_completed,
                "actionsOk": automation_ok,
                "actionsFailed": automation_failed,
                "meanLatencyMs": if automation_completed == 0 {
                    Value::Null
                } else {
                    json!(automation_latency as f64 / automation_completed as f64)
                },
            },
            "plugins": {
                "total": plugins.len(),
                "running": running,
                "unavailable": unavailable,
                "droppedEvents": dropped_plugin_events,
                "eventWorkers": self.plugin_state.observer_worker_count.load(Ordering::Acquire),
                "outstandingCalls": self.plugins.outstanding_call_count(),
                "delivery": plugin_delivery,
            },
            "processors": self.processor_list().len(),
            "processorStats": {
                "calls": processor_calls,
                "successes": processor_ok,
                "failures": processor_failed,
                "timeouts": processor_timeouts,
                "overloaded": processor_overloaded,
                "meanLatencyMs": if processor_ok == 0 {
                    Value::Null
                } else {
                    json!(processor_latency as f64 / processor_ok as f64)
                },
            },
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
                status: if connected { "ok" } else { "skipped" }.to_owned(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HostEmitter;

    struct NoopEmitter;
    impl HostEmitter for NoopEmitter {
        fn emit(&self, _: crate::ipc::messages::HostMessage) {}
    }

    /// The diagnostics matrix is additive and machine-readable: every
    /// section the refactor owns is present with the documented shape,
    /// and pre-existing fields keep their types.
    #[test]
    fn health_exposes_throughput_and_memory_matrix() {
        let core = AppCore::new(std::sync::Arc::new(NoopEmitter));
        let health = core.system_health();
        // Pre-existing fields keep their shape.
        assert!(health["status"].is_string());
        assert!(health["plugins"]["eventWorkers"].is_u64());
        assert!(health["automationActiveTasks"].is_u64());
        // LIVE staged-ingestion matrix (native transport only; the keys
        // are explicit nulls without it).
        if cfg!(feature = "native-tiktok") {
            assert!(health["liveIngress"]["capacity"].is_u64());
            assert!(health["liveIngress"]["queued"].is_u64());
            assert!(health["liveIngress"]["accepted"].is_u64());
            assert!(health["liveIngress"]["droppedData"].is_u64());
            assert!(health["liveIngress"]["droppedCritical"].is_u64());
            assert!(health["liveIngress"]["broadcastLagged"].is_u64());
            assert!(health["liveDedup"]["duplicates"].is_u64());
            assert!(health["liveGenerationSeq"].is_u64());
        } else {
            assert!(health["liveIngress"].is_null());
            assert!(health["liveDedup"].is_null());
        }
        // Points bounded-cache matrix.
        assert!(health["points"]["hotSize"].is_u64());
        assert!(health["points"]["hotCapacity"].is_u64());
        assert!(health["points"]["cacheHits"].is_u64());
        assert!(health["points"]["writeBatches"].is_u64());
        assert!(health["points"]["writerOverload"].is_u64());
        // Plugin delivery matrix.
        assert!(health["plugins"]["delivery"].is_array());
        // Automation saturation matrix.
        assert!(health["automation"]["admitted"].is_u64());
        assert!(health["automation"]["rejected"].is_u64());
        assert!(health["automation"]["actionsOk"].is_u64());
        assert!(health["automation"]["actionsFailed"].is_u64());
        // Processor overload matrix.
        assert!(health["processorStats"]["calls"].is_u64());
        assert!(health["processorStats"]["overloaded"].is_u64());
        // Memory section: host RSS is a number on Linux, null elsewhere —
        // never fabricated; plugin/WebView attribution stays external.
        assert!(
            health["memory"]["hostRssBytes"].is_u64() || health["memory"]["hostRssBytes"].is_null()
        );
        assert!(health["memory"]["pluginRssBytes"].is_null());
        assert!(health["memory"]["webviewRssBytes"].is_null());
        assert!(health["memory"]["rssSource"].is_string());
        // Worker/task ownership.
        assert!(health["tasks"]["livePump"].is_boolean());
        assert!(health["tasks"]["liveWorker"].is_boolean());
        assert!(health["tasks"]["pluginWorkers"].is_u64());
    }

    #[test]
    fn host_rss_is_positive_on_linux() {
        // Corroborates the /proc parsing against the process itself: a
        // live test binary always has a nonzero resident set.
        if cfg!(target_os = "linux") {
            let rss = host_rss_bytes().expect("VmRSS parses on Linux");
            assert!(rss > 0, "resident set must be nonzero");
        }
    }
}
