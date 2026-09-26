//! Automation execution: timeout wrapper, run recording, and core action dispatch.

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
        let globals = self.automation_globals();
        let result = match tokio::time::timeout(
            AUTOMATION_ACTION_DEADLINE,
            self.execute_action_impl(&type_id, &config, action, event, &globals, &mut logs, test),
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
        let runs = self.automation.record_run(run.clone());
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
        self.events
            .publish_domain(crate::events::DomainEvent::AutomationRunCompleted {
                run: run.clone(),
            });
        self.events
            .publish_domain(crate::events::DomainEvent::AutomationRunsChanged {
                runs: runs.clone(),
            });
        self.emit(HostMessage::BehaviorRuns { runs });
        run
    }

    // Eight narrow params (action identity, payload, render context,
    // output): bundling would churn every arm for no clarity gain.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn execute_action_impl(
        self: &Arc<Self>,
        type_id: &str,
        config: &serde_json::Map<String, Value>,
        action: &Value,
        event: &Value,
        globals: &std::collections::BTreeMap<String, String>,
        logs: &mut Vec<String>,
        test: bool,
    ) -> Result<String, String> {
        match type_id {
            "core.log" => {
                let message = render_template(
                    config
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                    event,
                    globals,
                );
                tracing::info!(target: "tiktools::automation", message = %message, "automation log");
                logs.push(message.clone());
                Ok(message.chars().take(120).collect())
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
                    ))
                    .await;
                }
                Ok(if test {
                    format!("would emit {event_type}")
                } else {
                    format!("emit {event_type}")
                })
            }
            "core.points" => {
                let unique_id = render_template(
                    config.get("uniqueId").and_then(Value::as_str).unwrap_or(""),
                    event,
                    globals,
                );
                let delta = number_value(config.get("delta")).unwrap_or_default();
                if unique_id.trim().is_empty() || !delta.is_finite() || delta == 0.0 {
                    return Err("Points action needs a viewer and a non-zero number.".to_owned());
                }
                if test {
                    return Ok(format!("would award {unique_id} {delta:+}"));
                }
                self.adjust_points(&unique_id, delta).await
            }
            "core.points.subtract" => {
                let unique_id = render_template(
                    config.get("uniqueId").and_then(Value::as_str).unwrap_or(""),
                    event,
                    globals,
                );
                let amount = number_value(config.get("amount")).unwrap_or_default().abs();
                if unique_id.trim().is_empty() || !amount.is_finite() || amount == 0.0 {
                    return Err("Subtract action needs a viewer and a non-zero amount.".to_owned());
                }
                if test {
                    return Ok(format!("would deduct {unique_id} {amount}"));
                }
                self.adjust_points(&unique_id, -amount).await
            }
            "core.delay" => {
                let millis = number_value(config.get("ms"))
                    .unwrap_or_default()
                    .clamp(0.0, 60_000.0) as u64;
                if !test && millis > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(millis)).await;
                }
                Ok(format!("wait {millis} ms"))
            }
            "audio.play" | "core.audio.play" => {
                self.execute_audio_action(config, event, globals, logs, test)
                    .await
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
                        ))
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
                        self.execute_audio_action(intent, event, globals, logs, test)
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
                self.execute_http_action(config, event, globals, logs, None, test)
                    .await
            }
            _ if type_id.is_empty() => Err("Action has no typeId.".to_owned()),
            _ => {
                self.execute_plugin_action(type_id, action, event, logs, test)
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
}
