use super::*;

use tiktools_plugin_api::MediaPickerOptions;

impl AppCore {
    pub async fn handle_page_message(self: &Arc<Self>, message: PageMessage) {
        self.events.publish(AppEvent::Ui(message.clone()));
        match message {
            PageMessage::Disconnect => {
                self.publish_disconnected_event().await;
                self.live.disconnect().await;
                self.emit(HostMessage::connection_disconnected());
            }
            PageMessage::Connect {
                unique_id,
                session_cookie,
                room_id,
            } => {
                self.start_live_event_pump();
                self.connect_native(ConnectRequest {
                    unique_id,
                    session_cookie,
                    room_id,
                })
                .await;
            }
            PageMessage::PickLive { session_cookie } => {
                self.start_live_event_pump();
                self.pick_live(&session_cookie).await;
            }
            PageMessage::OpenMediaPicker {
                request_id,
                mode,
                kind,
                title,
                initial_directory,
                extensions,
            } => {
                let options = MediaPickerOptions {
                    mode,
                    kind,
                    title,
                    initial_directory,
                    extensions,
                };
                let (selection, error) = match self.open_media_picker(options).await {
                    Ok(selection) => (selection, None),
                    Err(error) => (None, Some(error.to_string())),
                };
                self.emit(HostMessage::MediaSelected {
                    request_id,
                    selection,
                    error,
                });
            }
            PageMessage::GetPointsConfig => {
                self.emit(HostMessage::PointsConfig {
                    config: self.points.config(),
                });
            }
            PageMessage::UpdatePointsConfig { config } => {
                self.emit(HostMessage::PointsConfig {
                    config: self.points.update_config(config),
                });
            }
            PageMessage::GetLeaderboard { limit } => {
                self.emit(HostMessage::Leaderboard {
                    viewers: self.points.leaderboard(limit),
                });
            }
            PageMessage::ResetPoints { unique_id } => {
                self.points.reset(unique_id.as_deref());
                self.emit(HostMessage::Leaderboard {
                    viewers: self.points.leaderboard(Some(100)),
                });
            }
            PageMessage::AdjustPoints { unique_id, delta } => {
                if let Some(award) = self.points.award_points(
                    &unique_id,
                    PointAction::Manual,
                    AwardOptions {
                        custom_amount: Some(delta),
                        ..AwardOptions::default()
                    },
                ) {
                    self.emit(HostMessage::PointsAwarded {
                        unique_id: award.unique_id,
                        delta: award.delta,
                        total_points: award.total_points,
                        level: award.level,
                    });
                }
                self.emit(HostMessage::Leaderboard {
                    viewers: self.points.leaderboard(Some(100)),
                });
            }
            PageMessage::GetCreator { unique_id } => {
                #[cfg(feature = "persistence")]
                let creator = match self.db.load_creator(unique_id.as_deref()) {
                    Ok(creator) => creator,
                    Err(error) => {
                        tracing::warn!(%error, "could not load creator state");
                        None
                    }
                };
                #[cfg(not(feature = "persistence"))]
                let creator = {
                    let _ = unique_id;
                    None
                };
                self.emit(HostMessage::CreatorState { creator });
            }
            PageMessage::GetRecentCreators { limit } => {
                #[cfg(feature = "persistence")]
                let creators = match self
                    .db
                    .load_recent_creators(limit.unwrap_or(10).clamp(0, 1000))
                {
                    Ok(creators) => creators,
                    Err(error) => {
                        tracing::warn!(%error, "could not load creator history");
                        Vec::new()
                    }
                };
                #[cfg(not(feature = "persistence"))]
                let creators = {
                    let _ = limit;
                    Vec::new()
                };
                self.emit(HostMessage::RecentCreators { creators });
            }
            PageMessage::GetAppState { keys } => {
                let state = self.app_state.read(keys.as_deref());
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
                                    keys.as_ref()
                                        .is_none_or(|keys| keys.is_empty() || keys.contains(key))
                                })
                                .collect();
                        }
                        Err(error) => tracing::warn!(%error, "could not load app state"),
                    }
                    state
                };
                self.emit(HostMessage::AppState { state });
            }
            PageMessage::SetAppState { key, value } => {
                self.app_state.set(key.clone(), value.clone());
                #[cfg(feature = "persistence")]
                if let Err(error) = self.db.save_app_state(&key, &value) {
                    tracing::warn!(%error, "could not persist app state");
                }
                self.emit(HostMessage::AppState {
                    state: [(key, value)].into_iter().collect(),
                });
            }
            PageMessage::ClearCreatorHistory => {
                #[cfg(feature = "persistence")]
                if let Err(error) = self.db.clear_creator_history() {
                    tracing::warn!(%error, "could not clear creator history");
                }
                self.emit(HostMessage::RecentCreators {
                    creators: Vec::new(),
                });
                self.emit(HostMessage::CreatorState { creator: None });
            }
            PageMessage::DebugGift { gift_id } => {
                self.emit(HostMessage::GiftDebug {
                    gift_id,
                    icon_url: None,
                    has_icon: false,
                    total_gifts: 0,
                });
            }
            PageMessage::GetAutomationWorkflows => {
                self.emit_persisted_workflows();
            }
            PageMessage::GetAutomationNodes => {
                self.emit(HostMessage::AutomationNodeCatalog {
                    nodes: builtin_node_catalog(),
                });
            }
            PageMessage::GetAutomationContext => {
                let event = self
                    .last_automation_event
                    .read()
                    .expect("automation event lock poisoned")
                    .clone();
                let captured_at = *self
                    .last_automation_event_at
                    .read()
                    .expect("automation timestamp lock poisoned");
                self.emit(HostMessage::AutomationContext { event, captured_at });
            }
            PageMessage::SaveAutomationWorkflow { graph } => {
                #[cfg(feature = "persistence")]
                {
                    if let Err(error) = self.db.save_workflow(&graph) {
                        self.emit(HostMessage::AutomationError {
                            message: error.to_string(),
                        });
                    } else {
                        self.emit_persisted_workflows();
                    }
                }
                #[cfg(not(feature = "persistence"))]
                {
                    let _ = graph;
                    self.emit(HostMessage::AutomationError {
                        message: "Rust persistence is disabled in this build.".to_owned(),
                    });
                }
            }
            PageMessage::DeleteAutomationWorkflow { id } => {
                #[cfg(feature = "persistence")]
                if let Err(error) = self.db.delete_workflow(&id) {
                    self.emit(HostMessage::AutomationError {
                        message: error.to_string(),
                    });
                }
                #[cfg(not(feature = "persistence"))]
                let _ = id;
                self.emit_persisted_workflows();
            }
            PageMessage::SetAutomationWorkflowEnabled { id, enabled } => {
                #[cfg(feature = "persistence")]
                if let Err(error) = self.db.set_workflow_enabled(&id, enabled) {
                    self.emit(HostMessage::AutomationError {
                        message: error.to_string(),
                    });
                }
                #[cfg(not(feature = "persistence"))]
                {
                    let _ = (id, enabled);
                    self.emit(HostMessage::AutomationError {
                        message: "Rust persistence is disabled in this build.".to_owned(),
                    });
                }
                self.emit_persisted_workflows();
            }
            PageMessage::GetGiftCatalog => {
                self.emit_persisted_gifts();
            }
            PageMessage::GetBehavior => {
                self.emit_persisted_behavior();
                self.emit(HostMessage::BehaviorRuns {
                    runs: self.automation.recent_runs(),
                });
            }
            PageMessage::SaveAction { action } => {
                self.save_behavior_record("behavior_actions", action);
            }
            PageMessage::DeleteAction { id } => {
                self.delete_behavior_record("behavior_actions", &id);
            }
            PageMessage::SetActionEnabled { id, enabled } => {
                self.set_behavior_enabled("behavior_actions", &id, enabled);
            }
            PageMessage::SaveEvent { event } => {
                self.save_behavior_record("behavior_events", event);
            }
            PageMessage::DeleteEvent { id } => {
                self.delete_behavior_record("behavior_events", &id);
            }
            PageMessage::SetEventEnabled { id, enabled } => {
                self.set_behavior_enabled("behavior_events", &id, enabled);
            }
            PageMessage::SetPluginInstall { id, installed } => {
                let state_updated = {
                    #[cfg(feature = "persistence")]
                    {
                        if let Err(error) = self.db.set_plugin_state(&id, installed, true) {
                            self.emit(HostMessage::AutomationError {
                                message: error.to_string(),
                            });
                            false
                        } else {
                            true
                        }
                    }
                    #[cfg(not(feature = "persistence"))]
                    {
                        let _ = installed;
                        self.emit(HostMessage::AutomationError {
                            message: "Rust persistence is disabled in this build.".to_owned(),
                        });
                        false
                    }
                };
                if state_updated && !installed {
                    if let Err(error) = self.plugins.stop(&id) {
                        tracing::debug!(plugin = %id, %error, "plugin was not running during uninstall");
                    }
                }
                if state_updated {
                    self.set_plugin_activation(&id, installed, true);
                    self.rebuild_processor_index();
                }
                self.emit_persisted_behavior();
            }
            PageMessage::SetPluginEnabled { id, enabled } => {
                let state_updated = {
                    #[cfg(feature = "persistence")]
                    {
                        if let Err(error) = self.db.set_plugin_state(&id, true, enabled) {
                            self.emit(HostMessage::AutomationError {
                                message: error.to_string(),
                            });
                            false
                        } else {
                            true
                        }
                    }
                    #[cfg(not(feature = "persistence"))]
                    {
                        let _ = enabled;
                        self.emit(HostMessage::AutomationError {
                            message: "Rust persistence is disabled in this build.".to_owned(),
                        });
                        false
                    }
                };
                if state_updated {
                    let result = if enabled {
                        self.plugins.start(&id)
                    } else {
                        self.plugins.stop(&id)
                    };
                    if let Err(error) = result {
                        self.emit(HostMessage::BehaviorError {
                            message: error.to_string(),
                        });
                    }
                    self.set_plugin_activation(&id, true, enabled);
                    self.rebuild_processor_index();
                }
                self.emit_persisted_behavior();
            }
            PageMessage::InstallPluginPackage {
                path,
                replace_existing,
            } => {
                self.handle_install_plugin_package(path, replace_existing);
            }
            PageMessage::UninstallPluginPackage { id } => {
                self.handle_uninstall_plugin_package(id);
            }
            PageMessage::GetActionOptions { source } => {
                let (options, error) = self.resolve_action_options(&source).await;
                self.emit(HostMessage::ActionOptions {
                    source,
                    options,
                    error,
                });
            }
            PageMessage::ExecutePluginAction {
                action_type,
                config,
            } => {
                self.execute_plugin_action_ipc(action_type, config).await;
            }
            PageMessage::TestPluginConnection { id } => {
                self.probe_plugin_connection(&id).await;
            }
            PageMessage::GetPluginSettings { id } => {
                self.emit_plugin_settings(&id);
            }
            PageMessage::SavePluginSettings { id, values } => {
                self.save_plugin_settings(&id, values);
            }
            PageMessage::AnalyzeAutomationScript {
                node_id,
                source,
                offset: _,
                event_type: _,
            } => {
                let diagnostics = self
                    .automation
                    .validate_script(&source)
                    .err()
                    .map(|message| {
                        vec![json!({
                            "line": 1,
                            "column": 1,
                            "message": message,
                            "severity": "error"
                        })]
                    })
                    .unwrap_or_default();
                self.emit(HostMessage::AutomationScriptAnalysis {
                    analysis: json!({
                        "nodeId": node_id,
                        "source": source,
                        "diagnostics": diagnostics,
                        "completions": [],
                        "hover": null
                    }),
                });
            }
            PageMessage::TestAction { action, trigger } => {
                self.emit(HostMessage::BehaviorTestResult {
                    runs: vec![self.test_action(&action, trigger.as_deref()).await],
                });
            }
            PageMessage::TestEvent { event } => {
                self.emit(HostMessage::BehaviorTestResult {
                    runs: vec![self.test_event(&event).await],
                });
            }
            PageMessage::TestProcessor {
                plugin_id,
                processor_id,
                event,
            } => {
                let outcome = self.test_processor(&plugin_id, &processor_id, event).await;
                self.emit(HostMessage::ProcessorTestResult {
                    plugin_id,
                    processor_id,
                    ok: outcome.ok,
                    duration_ms: outcome.duration_ms,
                    result: outcome.result,
                    error: outcome.error,
                });
            }
            PageMessage::GetProcessorStatus => {
                self.emit(HostMessage::ProcessorStatus {
                    processors: self.processor_status_snapshot(),
                });
            }
            PageMessage::GetAnalyticsSummary {
                creator_unique_id,
                start_day,
                end_day,
                limit,
            } => {
                self.emit_analytics_summary(creator_unique_id, start_day, end_day, limit);
            }
        }
    }

    fn emit_analytics_summary(
        &self,
        creator_unique_id: Option<String>,
        start_day: Option<i64>,
        end_day: Option<i64>,
        limit: Option<i64>,
    ) {
        #[cfg(feature = "persistence")]
        {
            let now_unix = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_secs() as i64)
                .unwrap_or(0);
            let today = crate::db::utc_day(now_unix);
            let end = end_day.unwrap_or(today);
            let start = start_day.unwrap_or(end - 6).min(end);
            let creator = creator_unique_id
                .filter(|value| !value.trim().is_empty())
                .or_else(|| self.current_creator_unique_id())
                .unwrap_or_default();
            if creator.is_empty() {
                return;
            }
            match self
                .db
                .analytics_summary(&creator, start, end, limit.unwrap_or(10))
            {
                Ok(summary) => match serde_json::to_value(summary) {
                    Ok(summary) => self.emit(HostMessage::AnalyticsSummary { summary }),
                    Err(error) => tracing::warn!(%error, "could not serialize analytics summary"),
                },
                Err(error) => tracing::warn!(%error, "could not load analytics summary"),
            }
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (creator_unique_id, start_day, end_day, limit);
        }
    }

    fn handle_install_plugin_package(&self, path: String, replace_existing: bool) {
        #[cfg(feature = "plugin-install")]
        {
            // The installer owns canonicalization, validation, staging, and
            // atomic replacement. The frontend never supplies a plugin id or
            // destination: identity always comes from `plugin.json`.
            match self.install_plugin(std::path::PathBuf::from(&path), replace_existing) {
                Ok(installed) => {
                    // `install_plugin` already rescanned; refresh the behavior
                    // snapshot so the Plugins UI updates without a restart.
                    self.emit_persisted_behavior();
                    self.emit(HostMessage::plugin_install_success(
                        installed.manifest.id,
                        installed.manifest.version,
                        replace_existing,
                    ));
                }
                Err(error) => {
                    let message = error.to_string();
                    // Never expose sensitive filesystem internals beyond the
                    // installer message itself; classify for structured UI flow.
                    let code = crate::ipc::messages::classify_plugin_install_error(&message);
                    tracing::warn!(%message, ?code, "plugin package installation failed");
                    self.emit(HostMessage::plugin_install_failure(code, message));
                }
            }
        }
        #[cfg(not(feature = "plugin-install"))]
        {
            let _ = (path, replace_existing);
            self.emit(HostMessage::plugin_install_failure(
                crate::ipc::messages::PluginInstallErrorCode::Unknown,
                "plugin installation was disabled in this build".to_owned(),
            ));
        }
    }

    fn handle_uninstall_plugin_package(&self, id: String) {
        #[cfg(feature = "plugin-install")]
        {
            match self.uninstall_plugin(&id) {
                Ok(()) => {
                    self.emit_persisted_behavior();
                    self.emit(HostMessage::plugin_uninstall_success(id));
                }
                Err(error) => {
                    let message = error.to_string();
                    tracing::warn!(plugin = %id, %message, "plugin package uninstall failed");
                    self.emit(HostMessage::plugin_uninstall_failure(id, message));
                }
            }
        }
        #[cfg(not(feature = "plugin-install"))]
        {
            self.emit(HostMessage::plugin_uninstall_failure(
                id,
                "plugin installation was disabled in this build".to_owned(),
            ));
        }
    }

    /// Real (non-dry-run) plugin action execution for host-owned surfaces
    /// such as the TTS voice tester and automatic chat TTS.
    ///
    /// Unlike `test-action`, this sends the declarative HTTP request. Input
    /// was already shape-validated by `PageMessage::parse`; this re-checks
    /// the spoken text and emits a bounded `plugin-action-result`.
    async fn execute_plugin_action_ipc(
        self: &Arc<Self>,
        action_type: String,
        config: crate::ipc::messages::JsonObject,
    ) {
        let started = crate::helpers::now_millis();
        let text = config
            .get("text")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned();
        if text.trim().is_empty() {
            let duration_ms = crate::helpers::now_millis().saturating_sub(started);
            let message = "Give the voice tester some text to speak.".to_owned();
            self.emit(HostMessage::PluginActionResult {
                action_type,
                ok: false,
                summary: message.clone(),
                logs: Vec::new(),
                duration_ms,
                error: Some(message),
            });
            return;
        }
        if text.len() > 4_096 {
            let duration_ms = crate::helpers::now_millis().saturating_sub(started);
            let message = "Text is too long (4,096 character limit).".to_owned();
            self.emit(HostMessage::PluginActionResult {
                action_type,
                ok: false,
                summary: message.clone(),
                logs: Vec::new(),
                duration_ms,
                error: Some(message),
            });
            return;
        }
        let config_object: serde_json::Map<String, serde_json::Value> =
            config.into_iter().collect();
        let action = serde_json::json!({
            "typeId": action_type.clone(),
            "config": serde_json::Value::Object(config_object),
        });
        // TTS text is already resolved frontend-side; the event only feeds
        // templates that reference `event.*`, so a minimal envelope is enough.
        let event = serde_json::json!({
            "id": format!("tts-{}", started),
            "type": "tts.speak",
            "timestamp": started,
            "data": {},
        });
        let mut logs = Vec::new();
        match self
            .execute_plugin_action(&action_type, &action, &event, &mut logs, false)
            .await
        {
            Ok(summary) => {
                let duration_ms = crate::helpers::now_millis().saturating_sub(started);
                self.emit(HostMessage::PluginActionResult {
                    action_type,
                    ok: true,
                    summary,
                    logs: logs.into_iter().take(20).collect(),
                    duration_ms,
                    error: None,
                });
            }
            Err(error) => {
                let duration_ms = crate::helpers::now_millis().saturating_sub(started);
                self.emit(HostMessage::PluginActionResult {
                    action_type,
                    ok: false,
                    summary: error.clone(),
                    logs: logs.into_iter().take(20).collect(),
                    duration_ms,
                    error: Some(error),
                });
            }
        }
    }
}
