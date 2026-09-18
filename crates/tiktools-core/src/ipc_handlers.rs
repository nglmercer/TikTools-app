use super::*;

use tiktools_plugin_api::MediaPickerOptions;

impl AppCore {
    pub async fn handle_page_message(self: &Arc<Self>, message: PageMessage) {
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
                self.emit(HostMessage::CreatorState {
                    creator: self.creator_get(unique_id.as_deref()),
                });
            }
            PageMessage::GetRecentCreators { limit } => {
                self.emit(HostMessage::RecentCreators {
                    creators: self.creator_recent(limit),
                });
            }
            PageMessage::GetAppState { keys } => {
                self.emit(HostMessage::AppState {
                    state: self.app_state_get(keys.as_deref()).unwrap_or_default(),
                });
            }
            PageMessage::SetAppState { key, value } => match self.app_state_set(&key, &value) {
                Ok(state) => self.emit(HostMessage::AppState { state }),
                Err(error) => self.emit(HostMessage::BehaviorError {
                    message: error.message().to_owned(),
                }),
            },
            PageMessage::ClearCreatorHistory => {
                self.creator_history_clear();
                self.emit(HostMessage::RecentCreators {
                    creators: Vec::new(),
                });
                self.emit(HostMessage::CreatorState { creator: None });
            }
            PageMessage::DebugGift { gift_id } => {
                let debug = self.gift_debug(gift_id.as_deref());
                self.emit(HostMessage::GiftDebug {
                    gift_id: debug.gift_id,
                    icon_url: debug.icon_url,
                    has_icon: debug.has_icon,
                    total_gifts: debug.total_gifts,
                });
            }
            PageMessage::GetAutomationWorkflows => {
                self.emit_persisted_workflows();
            }
            PageMessage::GetAutomationNodes => {
                self.emit(HostMessage::AutomationNodeCatalog {
                    nodes: self.automation_nodes(),
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
                    runs: self.automation_runs(),
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
            PageMessage::GetActionOptions { source, refresh } => {
                match self.plugin_action_options(&source, refresh).await {
                    Ok((options, selected)) => self.emit(HostMessage::ActionOptions {
                        source,
                        options,
                        selected,
                        error: None,
                    }),
                    Err(error) => self.emit(HostMessage::ActionOptions {
                        source,
                        options: Vec::new(),
                        selected: None,
                        error: Some(error.message().to_owned()),
                    }),
                }
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
                offset,
                event_type,
            } => {
                match self.automation_script_analyze(
                    &node_id,
                    &source,
                    offset,
                    event_type.as_deref(),
                ) {
                    Ok(analysis) => match serde_json::to_value(analysis) {
                        Ok(analysis) => {
                            self.emit(HostMessage::AutomationScriptAnalysis { analysis })
                        }
                        Err(error) => self.emit(HostMessage::AutomationError {
                            message: error.to_string(),
                        }),
                    },
                    Err(error) => self.emit(HostMessage::AutomationError {
                        message: error.message().to_owned(),
                    }),
                }
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
                if let Some(summary) =
                    self.analytics_summary(creator_unique_id, start_day, end_day, limit, None)
                {
                    self.emit(HostMessage::AnalyticsSummary { summary });
                }
            }
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
