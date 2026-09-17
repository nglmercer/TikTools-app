use super::*;

const AUTOMATION_ACTION_DEADLINE: std::time::Duration = std::time::Duration::from_secs(125);
#[cfg(feature = "http")]
const MAX_HTTP_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

impl AppCore {
    pub(super) async fn test_action(
        self: &Arc<Self>,
        action: &Value,
        trigger: Option<&str>,
    ) -> Value {
        let event = self
            .last_automation_event
            .read()
            .expect("automation event lock poisoned")
            .clone()
            .unwrap_or_else(|| sample_automation_event(trigger.unwrap_or("tiktok.chat")));
        self.execute_action(action, &event, None, true).await
    }

    pub(super) async fn test_event(self: &Arc<Self>, record: &Value) -> Value {
        let started = now_millis();
        let trigger = record
            .get("trigger")
            .and_then(Value::as_str)
            .unwrap_or("tiktok.chat");
        let event = self
            .last_automation_event
            .read()
            .expect("automation event lock poisoned")
            .clone()
            .filter(|event| event.get("type").and_then(Value::as_str) == Some(trigger))
            .unwrap_or_else(|| {
                self.plugin_event_sample(trigger)
                    .unwrap_or_else(|| sample_automation_event(trigger))
            });

        if !self.automation.event_record_matches(record, &event) {
            // Name the sample data so a mismatch against a plugin sample
            // (hotkey.pressed ships key "k") reads as a data problem, not a
            // broken trigger.
            let preview = event.get("data").map(|data| {
                const MAX_PREVIEW: usize = 120;
                let text = serde_json::to_string(data).unwrap_or_default();
                if text.len() > MAX_PREVIEW {
                    format!("{}…", &text[..MAX_PREVIEW])
                } else {
                    text
                }
            });
            let summary = match preview {
                Some(preview) if !preview.is_empty() => {
                    format!(
                        "Event filters did not match the sample event (sample data: {preview})."
                    )
                }
                _ => "Event filters did not match the sample event.".to_owned(),
            };
            return json!({
                "id": self.automation.next_run_id("test-event", started),
                "at": started,
                "status": "error",
                "eventName": trigger,
                "actionName": record.get("name").and_then(Value::as_str).unwrap_or("Event"),
                "summary": summary.clone(),
                "durationMs": now_millis().saturating_sub(started),
                "test": true,
                "logs": [],
                "error": summary
            });
        }

        let actions = self.automation.actions_for_event(record);
        if actions.is_empty() {
            let summary = "The event has no saved actions to test.";
            return json!({
                "id": self.automation.next_run_id("test-event", started),
                "at": started,
                "status": "error",
                "eventName": trigger,
                "actionName": record.get("name").and_then(Value::as_str).unwrap_or("Event"),
                "summary": summary,
                "durationMs": now_millis().saturating_sub(started),
                "test": true,
                "logs": [],
                "error": summary
            });
        }

        let mut runs = Vec::with_capacity(actions.len());
        for action in &actions {
            runs.push(
                self.execute_action(action, &event, Some(record), true)
                    .await,
            );
        }
        let failed = runs
            .iter()
            .any(|run| run.get("status") == Some(&Value::String("error".to_owned())));
        let summary = if failed {
            "One or more actions failed."
        } else {
            "All referenced actions passed."
        };
        let mut result = json!({
            "id": self.automation.next_run_id("test-event", started),
            "at": started,
            "status": if failed { "error" } else { "ok" },
            "eventName": trigger,
            "actionName": record.get("name").and_then(Value::as_str).unwrap_or("Event"),
            "summary": summary,
            "durationMs": now_millis().saturating_sub(started),
            "test": true,
            "logs": [],
            "actions": runs
        });
        if failed {
            result["error"] = Value::String(summary.to_owned());
        }
        result
    }

    pub(super) async fn run_automation_event(self: &Arc<Self>, event: Value) {
        if self.automation.emit_depth(&event) >= 3 {
            tracing::warn!(event_type = ?event.get("type"), "automation emit depth limit reached");
            return;
        }
        for record in self.automation.matching_events(&event) {
            if !self.automation.claim_event(&record, &event, now_millis()) {
                continue;
            }
            for action in self.automation.actions_for_event(&record) {
                self.execute_action(&action, &event, Some(&record), false)
                    .await;
            }
        }
    }

    pub(super) async fn execute_action(
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
        let result = match tokio::time::timeout(
            AUTOMATION_ACTION_DEADLINE,
            self.execute_action_impl(&type_id, &config, action, event, &mut logs, test),
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
        self.emit(HostMessage::BehaviorRuns { runs });
        run
    }

    pub(super) async fn execute_action_impl(
        self: &Arc<Self>,
        type_id: &str,
        config: &serde_json::Map<String, Value>,
        action: &Value,
        event: &Value,
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
                    .map(|value| render_json_map(value, event))
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
                );
                let delta = number_value(config.get("delta")).unwrap_or_default();
                if unique_id.trim().is_empty() || !delta.is_finite() || delta == 0.0 {
                    return Err("Points action needs a viewer and a non-zero number.".to_owned());
                }
                if test {
                    return Ok(format!("would award {unique_id} {delta:+}"));
                }
                let award = self
                    .points
                    .adjust(&unique_id, delta)
                    .ok_or_else(|| format!("Viewer `{unique_id}` is not in the leaderboard."))?;
                self.emit(HostMessage::PointsAwarded {
                    unique_id: award.unique_id.clone(),
                    delta: award.delta,
                    total_points: award.total_points,
                    level: award.level,
                });
                self.emit_leaderboard_if_due();
                Ok(format!("{} {:+}", award.unique_id, award.delta))
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
                self.execute_audio_action(config, event, logs, test).await
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
                        .map(|value| render_json_map(value, event))
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
                    parts.push(self.execute_audio_action(intent, event, logs, test).await?);
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
                self.execute_http_action(config, event, logs, None, test)
                    .await
            }
            _ if type_id.is_empty() => Err("Action has no typeId.".to_owned()),
            _ => {
                self.execute_plugin_action(type_id, action, event, logs, test)
                    .await
            }
        }
    }

    pub(super) async fn execute_audio_action(
        self: &Arc<Self>,
        config: &serde_json::Map<String, Value>,
        event: &Value,
        logs: &mut Vec<String>,
        test: bool,
    ) -> Result<String, String> {
        let configured = config
            .get("fileRef")
            .or_else(|| config.get("file"))
            .or_else(|| config.get("filePath"))
            .or_else(|| config.get("path"))
            .ok_or_else(|| "Audio action has no file reference.".to_owned())?;
        let raw_path = configured
            .get("path")
            .and_then(Value::as_str)
            .or_else(|| configured.as_str())
            .ok_or_else(|| "Audio file reference must contain a path.".to_owned())?;
        let rendered_path = render_template(raw_path, event);
        let file = crate::services::audio_file_ref_from_config(
            &rendered_path,
            self.db.paths().data.as_path(),
        )
        .map_err(|error| error.to_string())?;
        let volume = number_value(config.get("volume"))
            .unwrap_or(1.0)
            .clamp(0.0, 1.0);
        if !volume.is_finite() {
            return Err("Audio volume must be finite.".to_owned());
        }
        let overlap = match config
            .get("overlap")
            .and_then(Value::as_str)
            .unwrap_or("allow")
        {
            "restart" => tiktools_plugin_api::AudioOverlap::Restart,
            "drop" => tiktools_plugin_api::AudioOverlap::Drop,
            _ => tiktools_plugin_api::AudioOverlap::Allow,
        };
        if test {
            let summary = format!("would play {}", file.name);
            if logs.len() < 40 {
                logs.push(summary.clone());
            }
            return Ok(summary);
        }
        let result = self
            .play_audio(
                file.clone(),
                tiktools_plugin_api::AudioPlayOptions {
                    volume: volume as f32,
                    overlap,
                },
            )
            .await
            .map_err(|error| error.to_string())?;
        let summary = if result.played {
            format!("played {}", file.name)
        } else {
            format!(
                "skipped {}{}",
                file.name,
                result
                    .reason
                    .as_deref()
                    .map(|reason| format!(" ({reason})"))
                    .unwrap_or_default()
            )
        };
        tracing::info!(target: "tiktools::automation", file = %file.path, played = result.played, "audio action completed");
        if logs.len() < 40 {
            logs.push(summary.clone());
        }
        Ok(summary)
    }

    #[cfg(feature = "http")]
    pub(super) async fn execute_http_action(
        self: &Arc<Self>,
        config: &serde_json::Map<String, Value>,
        event: &Value,
        logs: &mut Vec<String>,
        allowed_hosts: Option<&[String]>,
        test: bool,
    ) -> Result<String, String> {
        let raw_url = config
            .get("url")
            .and_then(Value::as_str)
            .ok_or_else(|| "HTTP action has no URL.".to_owned())?;
        let configured_url = reqwest::Url::parse(raw_url)
            .map_err(|_| "HTTP URL is invalid before template rendering.".to_owned())?;
        let configured_host = configured_url
            .host_str()
            .map(str::to_ascii_lowercase)
            .ok_or_else(|| "HTTP URL has no host.".to_owned())?;
        let rendered_url = render_template(raw_url, event);
        let url = reqwest::Url::parse(&rendered_url)
            .map_err(|_| "HTTP URL is invalid after template rendering.".to_owned())?;
        let allow_private_network = config
            .get("allowPrivateNetwork")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if test {
            validate_http_url_shape(&url, &configured_host, allowed_hosts, allow_private_network)?;
        } else {
            validate_http_url(&url, &configured_host, allowed_hosts, allow_private_network).await?;
        }

        let method_name = config
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or("POST")
            .trim()
            .to_ascii_uppercase();
        let method = reqwest::Method::from_bytes(method_name.as_bytes())
            .map_err(|_| format!("HTTP method is invalid: {method_name}"))?;
        let mut content_type = String::new();
        let mut rendered_headers = Vec::new();
        if let Some(headers) = config.get("headers").and_then(Value::as_object) {
            if headers.len() > 64 {
                return Err("HTTP action has too many headers.".to_owned());
            }
            for (key, value) in headers {
                let value = value_to_string(&Value::String(render_template(
                    &value_to_string(value),
                    event,
                )));
                if key.eq_ignore_ascii_case("content-type") {
                    content_type = value.clone();
                }
                rendered_headers.push((key.clone(), value));
            }
        }
        let body = config.get("body").and_then(|value| {
            if value.is_null() {
                None
            } else {
                Some(
                    value
                        .as_str()
                        .map(|value| render_template(value, event))
                        .unwrap_or_else(|| value_to_string(value)),
                )
            }
        });
        if let Some(body) = body.as_deref() {
            if !matches!(method, reqwest::Method::GET | reqwest::Method::HEAD)
                && content_type
                    .to_ascii_lowercase()
                    .contains("application/json")
                && !body.trim().is_empty()
                && serde_json::from_str::<Value>(body).is_err()
            {
                return Err("The JSON body is invalid after applying the template.".to_owned());
            }
        }
        if test {
            let summary = format!("would {method_name} request to {configured_host}");
            if logs.len() < 40 {
                logs.push(summary.clone());
            }
            return Ok(summary);
        }
        let timeout_ms = number_value(config.get("timeoutMs"))
            .unwrap_or(5_000.0)
            .clamp(100.0, 120_000.0) as u64;
        let sent = self
            .send_hardened_http_request(
                &HardenedHttpRequest {
                    method: method_name.clone(),
                    url: url.to_string(),
                    headers: rendered_headers,
                    body,
                    timeout_ms,
                },
                &configured_host,
            )
            .await?;
        let status = sent.status;
        let response_url = sent.response_url;
        let body = sent.body;
        let elapsed = sent.elapsed_ms;
        let log = format!("{method_name} {configured_host} → {status} ({elapsed} ms)");
        tracing::info!(target: "tiktools::automation", message = %log, "HTTP action completed");
        logs.push(log);

        if let Some(emit_response_as) = config
            .get("emitResponseAs")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
        {
            let event_type = normalize_emit_type(emit_response_as)?;
            Box::pin(
                self.publish_automation_event(self.make_internal_automation_event(
                    event,
                    &event_type,
                    json!({"status": status, "ok": (200..300).contains(&status), "body": body}),
                )),
            )
            .await;
        }
        if !(200..300).contains(&status) {
            return Err(format!(
                "HTTP {status} {}",
                if status >= 500 {
                    "server error"
                } else {
                    "request failed"
                }
            ));
        }
        Ok(format!("{status} OK · {elapsed} ms · {response_url}"))
    }

    #[cfg(not(feature = "http"))]
    pub(super) async fn execute_http_action(
        self: &Arc<Self>,
        _config: &serde_json::Map<String, Value>,
        _event: &Value,
        _logs: &mut Vec<String>,
        _allowed_hosts: Option<&[String]>,
        _test: bool,
    ) -> Result<String, String> {
        Err("HTTP action execution requires the host HTTP capability.".to_owned())
    }
}

/// A fully rendered request for the shared hardened sender. Callers validate
/// the URL with the HTTP policy helpers before sending; the sender re-checks
/// the destination host after redirects.
#[cfg_attr(not(feature = "http"), allow(dead_code))]
pub(super) struct HardenedHttpRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    pub timeout_ms: u64,
}

#[cfg_attr(not(feature = "http"), allow(dead_code))]
pub(super) struct HardenedHttpResponse {
    pub status: u16,
    pub response_url: String,
    pub body: Value,
    pub elapsed_ms: u64,
}

impl AppCore {
    /// Sends one request through the hardened client (no redirects, 2 MiB
    /// response cap, host re-check). Shared by automation HTTP actions and
    /// declarative plugin fetches so both enforce identical transport policy.
    #[cfg(feature = "http")]
    pub(super) async fn send_hardened_http_request(
        &self,
        request: &HardenedHttpRequest,
        configured_host: &str,
    ) -> Result<HardenedHttpResponse, String> {
        let http_client = self.http_client.as_ref().ok_or_else(|| {
            self.http_client_error.clone().unwrap_or_else(|| {
                "HTTP automation is disabled because its hardened client is unavailable.".to_owned()
            })
        })?;
        let method = reqwest::Method::from_bytes(request.method.as_bytes())
            .map_err(|_| format!("HTTP method is invalid: {}", request.method))?;
        let url = reqwest::Url::parse(&request.url)
            .map_err(|_| "HTTP URL is invalid after template rendering.".to_owned())?;
        let mut outgoing = http_client.request(method, url.clone());
        for (key, value) in &request.headers {
            outgoing = outgoing.header(key, value);
        }
        if let Some(body) = request.body.as_deref() {
            outgoing = outgoing.body(body.to_owned());
        }
        let timeout_ms = request.timeout_ms;
        let started = now_millis();
        let response = outgoing
            .timeout(std::time::Duration::from_millis(timeout_ms))
            .send()
            .await
            .map_err(|error| {
                if error.is_timeout() {
                    format!("HTTP request timed out after {timeout_ms} ms.")
                } else {
                    format!("HTTP request failed: {error}")
                }
            })?;
        if response.url() != &url && response.url().host_str() != Some(configured_host) {
            return Err("HTTP redirect changed the destination host.".to_owned());
        }
        if response.status().is_redirection() && response.headers().get("location").is_some() {
            return Err("HTTP redirects are blocked by policy.".to_owned());
        }
        let status = response.status().as_u16();
        let response_url = response.url().to_string();
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_owned();
        if response
            .content_length()
            .is_some_and(|length| length > MAX_HTTP_RESPONSE_BYTES as u64)
        {
            return Err("HTTP response exceeds the 2 MiB limit.".to_owned());
        }
        let mut response = response;
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|error| format!("could not read HTTP response: {error}"))?
        {
            if bytes.len().saturating_add(chunk.len()) > MAX_HTTP_RESPONSE_BYTES {
                return Err("HTTP response exceeds the 2 MiB limit.".to_owned());
            }
            bytes.extend_from_slice(&chunk);
        }
        let body = if content_type.to_ascii_lowercase().contains("json") {
            serde_json::from_slice::<Value>(&bytes)
                .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()))
        } else {
            Value::String(String::from_utf8_lossy(&bytes).into_owned())
        };
        Ok(HardenedHttpResponse {
            status,
            response_url,
            body,
            elapsed_ms: now_millis().saturating_sub(started),
        })
    }

    #[cfg(not(feature = "http"))]
    pub(super) async fn send_hardened_http_request(
        &self,
        _request: &HardenedHttpRequest,
        _configured_host: &str,
    ) -> Result<HardenedHttpResponse, String> {
        Err("HTTP action execution requires the host HTTP capability.".to_owned())
    }
}
