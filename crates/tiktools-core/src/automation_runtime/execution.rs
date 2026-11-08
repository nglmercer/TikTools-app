//! Automation execution: timeout wrapper, run recording, and core action dispatch.

use super::context::ExecutionContext;
use crate::*;

const AUTOMATION_ACTION_DEADLINE: std::time::Duration = std::time::Duration::from_secs(125);

impl AppCore {
    pub(crate) async fn execute_action(
        self: &Arc<Self>,
        action: &Value,
        event: &Value,
        origin: Option<&Value>,
        test: bool,
    ) -> Value {
        self.execute_action_with_context(action, event, origin, &ExecutionContext::owner(), test)
            .await
    }

    pub(crate) async fn execute_action_with_context(
        self: &Arc<Self>,
        action: &Value,
        event: &Value,
        origin: Option<&Value>,
        incoming: &ExecutionContext,
        test: bool,
    ) -> Value {
        if self.is_shutdown() || incoming.check_current().is_err() {
            return json!({"status":"error","test":test,"summary":"Definition requires explicit owner adoption or delegated execution approval.","error":"authority_required"});
        }
        let Ok(context) = self
            .action_execution_context(action, origin, incoming)
            .await
        else {
            return json!({"status":"error","test":test,"summary":"Execution authority is unavailable.","error":"authority_required"});
        };
        if self.core_action_guard(action, &context).is_err() {
            return json!({"status":"error","test":test,"summary":"Definition requires explicit owner adoption or delegated execution approval.","error":"authority_required"});
        }
        let started = now_millis();
        let type_id = action
            .get("typeId")
            .or_else(|| action.get("type"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let config = action
            .get("config")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let mut logs = Vec::new();
        // One snapshot per execution: url, headers, and body render the
        // same values even if a concurrent write lands mid-action.
        let globals = if context.is_approved() {
            Ok(std::collections::BTreeMap::new())
        } else {
            self.automation_globals(&context)
        };
        let Ok(globals) = globals else {
            return json!({"status":"error","test":test,"summary":"Execution authority is unavailable.","error":"authority_required"});
        };
        let result = match tokio::time::timeout(
            AUTOMATION_ACTION_DEADLINE,
            self.execute_action_impl(
                &type_id, &config, action, event, &globals, &context, &mut logs, test,
            ),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => Err(format!(
                "Automation action timed out after {} seconds.",
                AUTOMATION_ACTION_DEADLINE.as_secs()
            )),
        };

        let (status, summary, error) = match result {
            Ok(summary) => ("ok", summary, None),
            Err(error) => ("error", error.clone(), Some(error)),
        };
        let run = json!({
            "id": self.automation.next_run_id(if test { "test-action" } else { "run" }, started),
            "at": started,
            "status": status,
            "eventId": origin.and_then(|value| value.get("id")),
            "eventName": origin.and_then(|value| value.get("name")),
            "actionId": action.get("id"),
            "actionName": action.get("name").and_then(Value::as_str).unwrap_or("Action"),
            "summary": summary,
            "durationMs": now_millis().saturating_sub(started),
            "test": test,
            "logs": logs.into_iter().take(40).collect::<Vec<_>>(),
            "error": error
        });
        self.automation
            .record_run_only_scoped(run.clone(), !context.is_owner());
        let action_name = run
            .get("actionName")
            .and_then(|name| name.as_str())
            .unwrap_or_default();
        if status == "error" {
            tracing::warn!(
                action = %action_name,
                summary = %summary,
                "automation action failed"
            );
        } else {
            tracing::debug!(
                action = %action_name,
                "automation action completed"
            );
        }
        // Authoritative domain topics; the legacy push stays for compat.
        self.publish_action_outcome(
            &context,
            crate::events::DomainEvent::AutomationRunCompleted { run: run.clone() },
        );
        let legacy = self.emitter.wants_automation_history();
        let domain = self.events.history_requested()
            || !crate::plugin_events::fanout::FanoutCache::current(self)
                .matching("automation.runs.changed", false)
                .is_empty();
        if legacy || domain {
            let (runs, restricted_history) = self.automation.recent_runs_scoped();
            match (legacy, domain) {
                (true, true) => {
                    self.emit(HostMessage::BehaviorRuns { runs: runs.clone() });
                    self.publish_run_history_outcome(
                        &context,
                        restricted_history,
                        crate::events::DomainEvent::AutomationRunsChanged { runs },
                    );
                }
                (true, false) => self.emit(HostMessage::BehaviorRuns { runs }),
                (false, true) => self.publish_run_history_outcome(
                    &context,
                    restricted_history,
                    crate::events::DomainEvent::AutomationRunsChanged { runs },
                ),
                (false, false) => unreachable!(),
            }
        }
        run
    }

    // Keep action identity, render input and trusted authority explicit at
    // every effect boundary; neither payload nor output can imply authority.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn execute_action_impl(
        self: &Arc<Self>,
        type_id: &str,
        config: &serde_json::Map<String, Value>,
        action: &Value,
        event: &Value,
        globals: &std::collections::BTreeMap<String, String>,
        context: &ExecutionContext,
        logs: &mut Vec<String>,
        test: bool,
    ) -> Result<String, String> {
        // Restrict the renderer even when a trusted in-process caller invokes
        // this helper directly. Unsupported sinks retain their Owner guard.
        if context.is_approved()
            && (action
                .get("typeId")
                .or_else(|| action.get("type"))
                .and_then(Value::as_str)
                != Some(type_id)
                || action
                    .get("config")
                    .and_then(Value::as_object)
                    .cloned()
                    .unwrap_or_default()
                    != *config)
        {
            return Err("effect input differs from the approved definition".into());
        }
        let projected = context.projected_event(event)?;
        let empty_globals = std::collections::BTreeMap::new();
        let (event, globals) = if context.is_approved() {
            (&projected, &empty_globals)
        } else {
            (event, globals)
        };
        if matches!(
            type_id,
            "core.log"
                | "core.delay"
                | "core.points"
                | "core.points.subtract"
                | "core.fetch"
                | "audio.play"
                | "core.audio.play"
        ) {
            self.core_action_guard(action, context)?;
        } else {
            self.execution_context_guard(context)?;
        }
        match type_id {
            "core.log" => {
                self.with_core_action(action, context, |identities| {
                let message = render_action_template(context,
                    config
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                    event,
                    globals, 64 * 1024,
                )?;
                for identity in identities { identity.check_deadline().map_err(|_| "saved execution subject has expired".to_owned())?; }
                tracing::info!(target: "tiktools::automation", message = %message, "automation log");
                logs.push(message.clone());
                Ok(message.chars().take(120).collect())
                })
            }
            "core.emit" => {
                let event_type = normalize_emit_type(
                    config
                        .get("type")
                        .and_then(Value::as_str)
                        .unwrap_or("overlay.alert"),
                )?;
                let payload = config
                    .get("data")
                    .map(|value| render_json_map(value, event, globals))
                    .unwrap_or_default();
                if !test {
                    self.publish_automation_event(self.make_internal_automation_event(
                        event,
                        &event_type,
                        Value::Object(payload.into_iter().collect()),
                        context,
                    )?)
                    .await;
                }
                Ok(if test {
                    format!("would emit {event_type}")
                } else {
                    format!("emit {event_type}")
                })
            }
            "core.points" => {
                let unique_id = render_action_template(context,
                    config.get("uniqueId").and_then(Value::as_str).unwrap_or(""),
                    event,
                    globals, 256,
                )?;
                let delta = number_value(config.get("delta")).unwrap_or_default();
                if unique_id.trim().is_empty() || !delta.is_finite() || delta == 0.0 {
                    return Err("Points action needs a viewer and a non-zero number.".to_owned());
                }
                if test {
                    self.check_points_test_target(action, context, &unique_id, delta)?;
                    return Ok(format!("would award {unique_id} {delta:+}"));
                }
                self.adjust_points_with_context(action, context, &unique_id, delta).await
            }
            "core.points.subtract" => {
                let unique_id = render_action_template(context,
                    config.get("uniqueId").and_then(Value::as_str).unwrap_or(""),
                    event,
                    globals, 256,
                )?;
                let amount = number_value(config.get("amount")).unwrap_or_default().abs();
                if unique_id.trim().is_empty() || !amount.is_finite() || amount == 0.0 {
                    return Err("Subtract action needs a viewer and a non-zero amount.".to_owned());
                }
                if test {
                    self.check_points_test_target(action, context, &unique_id, -amount)?;
                    return Ok(format!("would deduct {unique_id} {amount}"));
                }
                self.adjust_points_with_context(action, context, &unique_id, -amount).await
            }
            "core.delay" => {
                let millis = number_value(config.get("ms"))
                    .unwrap_or_default()
                    .clamp(0.0, 60_000.0) as u64;
                if !test && millis > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(millis)).await;
                }
                self.core_action_guard(action, context)?;
                Ok(format!("wait {millis} ms"))
            }
            "audio.play" | "core.audio.play" => {
                if context.is_approved() {
                    self.execute_approved_audio_action(action, config, event, context, logs, test).await
                } else {
                    self.execute_audio_action(config, event, globals, context, logs, test).await
                }
            }
            "core.code" => {
                let source = config
                    .get("source")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "Script action has no source.".to_owned())?;
                let result = self.automation.evaluate_script(source, event, &json!({}))?;
                let mut parts = Vec::new();
                for log in result
                    .get("log")
                    .into_iter()
                    .flat_map(as_values)
                    .filter_map(Value::as_str)
                {
                    if logs.len() < 40 {
                        logs.push(log.to_owned());
                    }
                }
                for intent in result.get("emit").into_iter().flat_map(as_values) {
                    let Some(intent) = intent.as_object() else {
                        continue;
                    };
                    let Some(event_type) = intent.get("type").and_then(Value::as_str) else {
                        continue;
                    };
                    let event_type = normalize_emit_type(event_type)?;
                    let payload = intent
                        .get("data")
                        .map(|value| render_json_map(value, event, globals))
                        .unwrap_or_default();
                    if !test {
                        self.publish_automation_event(self.make_internal_automation_event(
                            event,
                            &event_type,
                            Value::Object(payload.into_iter().collect()),
                            context,
                        )?)
                        .await;
                    }
                    parts.push(if test {
                        format!("would emit {event_type}")
                    } else {
                        format!("emit {event_type}")
                    });
                }
                for intent in result
                    .get(tiktools_plugin_api::AUDIO_PLAY_INTENT)
                    .into_iter()
                    .flat_map(as_values)
                {
                    let Some(intent) = intent.as_object() else {
                        continue;
                    };
                    parts.push(
                        self.execute_audio_action(intent, event, globals, context, logs, test)
                            .await?,
                    );
                }
                if let Some(intent) = result.get("fetch").and_then(Value::as_object) {
                    let mut fetch_config = intent.clone();
                    if let Some(emit_response_as) = result.get("emitResponseAs") {
                        fetch_config.insert("emitResponseAs".to_owned(), emit_response_as.clone());
                    }
                    let allowed_hosts = hosts_in_source(source);
                    parts.push(
                        self.execute_http_action(
                            &fetch_config,
                            event,
                            globals,
                            context,
                            logs,
                            Some(&allowed_hosts),
                            test,
                        )
                        .await?,
                    );
                }
                if parts.is_empty() {
                    Ok(format!("script returned {}", result_type(&result)))
                } else {
                    Ok(parts.join(" · "))
                }
            }
            "core.fetch" => {
                if context.is_approved() {
                    self.execute_approved_http_action(action, config, event, context, logs, test).await
                } else {
                    self.execute_http_action(config, event, globals, context, logs, None, test).await
                }
            }
            _ if type_id.is_empty() => Err("Action has no typeId.".to_owned()),
            _ => {
                self.execute_plugin_action(type_id, action, event, context, logs, test)
                    .await
            }
        }
    }

    /// Applies a leaderboard delta shared by `core.points` and
    /// `core.points.subtract`: publishes the change and refreshes the board.
    async fn adjust_points(&self, unique_id: &str, delta: f64) -> Result<String, String> {
        let award = self
            .points
            .adjust(unique_id, delta)
            .ok_or_else(|| format!("Viewer `{unique_id}` is not in the leaderboard."))?;
        self.events
            .publish_domain(crate::events::DomainEvent::PointsChanged {
                unique_id: award.unique_id.clone(),
                delta: award.delta,
                total_points: award.total_points,
                level: award.level,
            });
        self.emit_leaderboard_if_due();
        Ok(format!("{} {:+}", award.unique_id, award.delta))
    }

    fn core_action_guard(&self, action: &Value, context: &ExecutionContext) -> Result<(), String> {
        self.with_core_action(action, context, |_| Ok(()))
    }

    fn publish_action_outcome(
        &self,
        context: &ExecutionContext,
        event: crate::events::DomainEvent,
    ) {
        if context.is_owner() {
            self.events.publish_domain(event);
        } else {
            self.events.publish_presentation_domain(event);
        }
    }

    fn publish_run_history_outcome(
        &self,
        context: &ExecutionContext,
        restricted_history: bool,
        event: crate::events::DomainEvent,
    ) {
        // An Owner action does not convert previously scoped history into an
        // Owner-authorized plugin callback. Local presentation retains it.
        if restricted_history {
            self.events.publish_presentation_domain(event);
        } else {
            self.publish_action_outcome(context, event);
        }
    }

    fn check_points_test_target(
        &self,
        action: &Value,
        context: &ExecutionContext,
        unique_id: &str,
        delta: f64,
    ) -> Result<(), String> {
        if context.is_owner() {
            return self.execution_context_guard(context);
        }
        self.with_core_action(action, context, |identities| {
            if !crate::services::valid_viewer_id(unique_id)
                || identities.iter().any(|identity| {
                    !identity
                        .grant()
                        .allows_resource(crate::collaboration::ResourceKind::Viewer, unique_id)
                        || identity
                            .grant()
                            .max_points_delta
                            .is_some_and(|max| delta.abs() > max)
                })
            {
                return Err("viewer or delta is outside execution scope".into());
            }
            Ok(())
        })
    }

    pub(super) fn with_core_action<R>(
        &self,
        action: &Value,
        context: &ExecutionContext,
        apply: impl FnOnce(&[crate::collaboration::ExecutionIdentity]) -> Result<R, String>,
    ) -> Result<R, String> {
        if context.is_owner() {
            self.execution_context_guard(context)?;
            return apply(&[]);
        }
        #[cfg(feature = "persistence")]
        {
            let _writer = self.automation_gate().map_err(|error| error.to_string())?;
            context.with_current_subjects(action, &self.automation, |identities| {
                if self.is_shutdown() {
                    return Err("host is shutting down".into());
                }
                Self::check_core_subjects(action, identities)?;
                apply(identities)
            })
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (action, apply);
            Err("delegated execution requires persistence".into())
        }
    }

    #[cfg(feature = "persistence")]
    pub(super) fn check_core_subjects(
        action: &Value,
        identities: &[crate::collaboration::ExecutionIdentity],
    ) -> Result<(), String> {
        use crate::collaboration::{AutomationFamily, Capability, ResourceKind};
        let id = action
            .get("id")
            .and_then(Value::as_str)
            .ok_or("saved action identity is unavailable")?;
        let type_id = action
            .get("typeId")
            .or_else(|| action.get("type"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        if identities.is_empty()
            || !matches!(
                type_id,
                "core.log"
                    | "core.delay"
                    | "core.points"
                    | "core.points.subtract"
                    | "core.fetch"
                    | "audio.play"
                    | "core.audio.play"
            )
        {
            return Err("effect policy is unavailable".into());
        }
        for identity in identities {
            let grant = identity.grant();
            if !grant.capabilities.contains(&Capability::AutomationRead)
                || !grant.allows_automation(AutomationFamily::Action, id)
                || grant.resources.contains_key(&ResourceKind::Live)
            {
                return Err("execution subject is outside the saved action scope".into());
            }
            let required = match type_id {
                "core.fetch" => Some(Capability::HttpRequest),
                "audio.play" | "core.audio.play" => Some(Capability::AudioPlay),
                _ => None,
            };
            if required.is_some_and(|cap| !grant.capabilities.contains(&cap)) {
                return Err("execution subject lacks the effect capability".into());
            }
            let config = action.get("config").cloned().unwrap_or_else(|| json!({}));
            let fields = crate::services::automation_manifest::effect_render_fields(&config)?;
            if !fields.is_empty() && !grant.capabilities.contains(&Capability::LiveRead) {
                return Err("execution subject cannot render LIVE fields".into());
            }
            if matches!(type_id, "core.points" | "core.points.subtract")
                && (!grant.capabilities.contains(&Capability::PointsRead)
                    || !grant.capabilities.contains(&Capability::PointsAdjust))
            {
                return Err("execution subject cannot adjust points".into());
            }
        }
        Ok(())
    }

    async fn adjust_points_with_context(
        self: &Arc<Self>,
        action: &Value,
        context: &ExecutionContext,
        unique_id: &str,
        delta: f64,
    ) -> Result<String, String> {
        if context.is_owner() {
            self.execution_context_guard(context)?;
            return self.adjust_points(unique_id, delta).await;
        }
        #[cfg(feature = "persistence")]
        {
            let core = self.clone();
            let context = context.clone();
            let action = action.clone();
            let unique_id = unique_id.to_owned();
            let award = tokio::task::spawn_blocking(move || {
                let _writer = core.automation_gate().map_err(|error| error.to_string())?;
                core.points
                    .adjust_approved(&unique_id, delta, |effective, commit| {
                        context
                            .with_current_subjects(&action, &core.automation, |identities| {
                                if core.is_shutdown() {
                                    return Err("host is shutting down".into());
                                }
                                Self::check_core_subjects(&action, identities)?;
                                if identities.iter().any(|identity| {
                                    let grant = identity.grant();
                                    !grant.allows_resource(
                                        crate::collaboration::ResourceKind::Viewer,
                                        &unique_id,
                                    ) || grant.max_points_delta.is_some_and(|max| {
                                        delta.abs() > max || effective.abs() > max
                                    })
                                }) {
                                    return Err(
                                        "viewer or effective delta is outside execution scope"
                                            .into(),
                                    );
                                }
                                for identity in identities {
                                    identity.check_deadline().map_err(|_| {
                                        "saved execution subject has expired".to_owned()
                                    })?;
                                }
                                commit().map_err(|error| error.to_string())
                            })
                            .map_err(crate::db::DatabaseError::Invalid)
                    })
                    .map_err(|error| error.to_string())
            })
            .await
            .map_err(|_| "approved points worker failed".to_owned())??;
            self.events
                .publish_presentation_domain(crate::events::DomainEvent::PointsChanged {
                    unique_id: award.unique_id.clone(),
                    delta: award.delta,
                    total_points: award.total_points,
                    level: award.level,
                });
            self.emit_leaderboard_if_due();
            Ok(format!("{} {:+}", award.unique_id, award.delta))
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (action, unique_id, delta);
            Err("delegated execution requires persistence".into())
        }
    }
}

/// Preserve Owner rendering while bounding delegated expansion before append.
fn render_action_template(
    context: &ExecutionContext,
    source: &str,
    event: &Value,
    globals: &std::collections::BTreeMap<String, String>,
    max_bytes: usize,
) -> Result<String, String> {
    if context.is_approved() {
        #[cfg(feature = "persistence")]
        return crate::services::automation_approval_policy::render_template(
            source, event, max_bytes,
        );
        #[cfg(not(feature = "persistence"))]
        return Err("delegated rendering requires persistence".into());
    }
    let _ = max_bytes;
    Ok(render_template(source, event, globals))
}
