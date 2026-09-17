use super::*;

use tiktools_plugin_api::MediaPickerOptions;

impl AppCore {
    pub async fn handle_page_message(self: &Arc<Self>, message: PageMessage) {
        self.events.publish(AppEvent::Ui(message.clone()));
        match message {
            PageMessage::Disconnect => {
                self.live_disconnect().await;
                self.emit(HostMessage::connection_disconnected());
            }
            PageMessage::Connect {
                unique_id,
                session_cookie,
                room_id,
            } => {
                #[cfg(feature = "native-tiktok")]
                let display_id = clean_unique_id(&unique_id);
                #[cfg(not(feature = "native-tiktok"))]
                let display_id = {
                    let trimmed = unique_id.trim().trim_start_matches('@');
                    (!trimmed.is_empty()).then(|| trimmed.to_owned())
                };
                self.emit(HostMessage::Connection {
                    status: crate::ipc::messages::ConnectionStatus::Connecting,
                    unique_id: display_id,
                    title: None,
                    room_id: room_id.clone(),
                    avatar_url: None,
                });
                if let Err(error) = self.live_connect(unique_id, session_cookie, room_id).await {
                    self.emit(HostMessage::Error {
                        phase: crate::ipc::messages::ErrorPhase::Connect,
                        message: error.message().to_owned(),
                    });
                    self.emit(HostMessage::connection_disconnected());
                }
            }
            PageMessage::PickLive { session_cookie } => {
                if let Err(error) = self.live_pick(session_cookie).await {
                    self.emit(HostMessage::Error {
                        phase: crate::ipc::messages::ErrorPhase::Connect,
                        message: error.message().to_owned(),
                    });
                    self.emit(HostMessage::connection_disconnected());
                }
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
                    config: self.points_config(),
                });
            }
            PageMessage::UpdatePointsConfig { config } => {
                self.emit(HostMessage::PointsConfig {
                    config: self.points_update_config(config),
                });
            }
            PageMessage::GetLeaderboard { limit } => {
                self.emit(HostMessage::Leaderboard {
                    viewers: self.points_leaderboard(limit),
                });
            }
            PageMessage::ResetPoints { unique_id } => {
                self.points_reset(unique_id.as_deref());
                self.emit(HostMessage::Leaderboard {
                    viewers: self.points_leaderboard(Some(100)),
                });
            }
            PageMessage::AdjustPoints { unique_id, delta } => {
                if let Ok(award) = self.points_adjust(&unique_id, delta) {
                    self.emit(HostMessage::PointsAwarded {
                        unique_id: award.unique_id,
                        delta: award.delta,
                        total_points: award.total_points,
                        level: award.level,
                    });
                }
                self.emit(HostMessage::Leaderboard {
                    viewers: self.points_leaderboard(Some(100)),
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
                let (event, captured_at) = self.automation_context();
                self.emit(HostMessage::AutomationContext { event, captured_at });
            }
            PageMessage::SaveAutomationWorkflow { graph } => match self.workflow_save(graph) {
                Ok(_) => self.emit_persisted_workflows(),
                Err(error) => self.emit(HostMessage::AutomationError {
                    message: error.message().to_owned(),
                }),
            },
            PageMessage::DeleteAutomationWorkflow { id } => {
                #[cfg(feature = "persistence")]
                if let Err(error) = self.workflow_delete(&id) {
                    self.emit(HostMessage::AutomationError {
                        message: error.message().to_owned(),
                    });
                }
                #[cfg(not(feature = "persistence"))]
                let _ = id;
                self.emit_persisted_workflows();
            }
            PageMessage::SetAutomationWorkflowEnabled { id, enabled } => {
                if let Err(error) = self.workflow_set_enabled(&id, enabled) {
                    self.emit(HostMessage::AutomationError {
                        message: error.message().to_owned(),
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
                if let Err(error) = self.plugin_set_installed(&id, installed) {
                    self.emit(HostMessage::AutomationError {
                        message: error.message().to_owned(),
                    });
                }
                self.emit_persisted_behavior();
            }
            PageMessage::SetPluginEnabled { id, enabled } => {
                if let Err(error) = self.plugin_set_enabled(&id, enabled) {
                    self.emit(HostMessage::BehaviorError {
                        message: error.message().to_owned(),
                    });
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
                let (options, selected, error) = self.resolve_action_options(&source).await;
                self.emit(HostMessage::ActionOptions {
                    source,
                    options,
                    selected,
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
            PageMessage::ProvisionPluginToken {
                id,
                username,
                password,
            } => {
                self.provision_plugin_token(id, username, password).await;
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
                    runs: vec![
                        self.test_automation_action(&action, trigger.as_deref())
                            .await,
                    ],
                });
            }
            PageMessage::TestEvent { event } => {
                self.emit(HostMessage::BehaviorTestResult {
                    runs: vec![self.test_automation_event(&event).await],
                });
            }
            PageMessage::TestProcessor {
                plugin_id,
                processor_id,
                event,
            } => match self.processor_test(&plugin_id, &processor_id, event).await {
                Ok(outcome) => self.emit(HostMessage::ProcessorTestResult {
                    plugin_id: outcome.plugin_id,
                    processor_id: outcome.processor_id,
                    ok: outcome.ok,
                    duration_ms: outcome.duration_ms,
                    result: outcome.result,
                    error: outcome.error,
                }),
                Err(error) => self.emit(HostMessage::ProcessorTestResult {
                    plugin_id,
                    processor_id,
                    ok: false,
                    duration_ms: 0,
                    result: Value::Null,
                    error: Some(error.message().to_owned()),
                }),
            },
            PageMessage::GetProcessorStatus => {
                self.emit(HostMessage::ProcessorStatus {
                    processors: self.processor_status(),
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
        // The installer owns canonicalization, validation, staging, and
        // atomic replacement. The frontend never supplies a plugin id or
        // destination: identity always comes from `plugin.json`.
        match self.plugin_install(&path, replace_existing) {
            Ok(installed) => {
                self.emit_persisted_behavior();
                self.emit(HostMessage::plugin_install_success(
                    installed.id,
                    installed.version,
                    replace_existing,
                ));
            }
            Err(error) => {
                let message = error.message().to_owned();
                // Never expose sensitive filesystem internals beyond the
                // installer message itself; classify for structured UI flow.
                let code = crate::ipc::messages::classify_plugin_install_error(&message);
                tracing::warn!(%message, ?code, "plugin package installation failed");
                self.emit(HostMessage::plugin_install_failure(code, message));
            }
        }
    }

    fn handle_uninstall_plugin_package(&self, id: String) {
        match self.plugin_uninstall(&id) {
            Ok(()) => {
                self.emit_persisted_behavior();
                self.emit(HostMessage::plugin_uninstall_success(id));
            }
            Err(error) => {
                let message = error.message().to_owned();
                tracing::warn!(plugin = %id, %message, "plugin package uninstall failed");
                self.emit(HostMessage::plugin_uninstall_failure(id, message));
            }
        }
    }

    /// Real (non-dry-run) plugin action execution for host-owned surfaces
    /// such as the TTS voice tester, automatic chat TTS, and the TTS audio
    /// output selector.
    ///
    /// Unlike `test-action`, this sends the declarative HTTP request. Input
    /// was already shape-validated by `PageMessage::parse`; actions whose
    /// descriptor declares a `text` field additionally require non-empty
    /// bounded spoken text, while textless actions (output switching) run
    /// with the given config. Every path emits a bounded
    /// `plugin-action-result`.
    async fn execute_plugin_action_ipc(
        self: &Arc<Self>,
        action_type: String,
        config: crate::ipc::messages::JsonObject,
    ) {
        let started = crate::helpers::now_millis();
        let needs_text = self.action_declares_field(&action_type, "text");
        let text = config
            .get("text")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned();
        if needs_text && text.trim().is_empty() {
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
        if needs_text && text.len() > 4_096 {
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
        // Same authoritative execution as the control API; the pre-checks
        // above only preserve the voice-tester-specific UX messages.
        match self.plugin_action_execute(&action_type, config, true).await {
            Ok(outcome) => self.emit(HostMessage::PluginActionResult {
                action_type: outcome.action_type,
                ok: outcome.ok,
                summary: outcome.summary,
                logs: outcome.logs,
                duration_ms: outcome.duration_ms,
                error: outcome.error,
            }),
            Err(error) => {
                let duration_ms = crate::helpers::now_millis().saturating_sub(started);
                let message = error.message().to_owned();
                self.emit(HostMessage::PluginActionResult {
                    action_type,
                    ok: false,
                    summary: message.clone(),
                    logs: Vec::new(),
                    duration_ms,
                    error: Some(message),
                });
            }
        }
    }
}
