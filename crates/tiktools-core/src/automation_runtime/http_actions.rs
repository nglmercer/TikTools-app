//! Automation HTTP actions and the shared hardened sender.

use crate::*;

#[cfg(feature = "http")]
const MAX_HTTP_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

impl AppCore {
    #[cfg(feature = "http")]
    pub(crate) async fn execute_http_action(
        self: &Arc<Self>,
        config: &serde_json::Map<String, Value>,
        event: &Value,
        globals: &std::collections::BTreeMap<String, String>,
        logs: &mut Vec<String>,
        allowed_hosts: Option<&[String]>,
        test: bool,
    ) -> Result<String, String> {
        let raw_url = config
            .get("url")
            .and_then(Value::as_str)
            .ok_or_else(|| "HTTP action has no URL.".to_owned())?;
        // Two-phase render: operator-trusted globals first (so the
        // configured host is concrete), then untrusted event data. The
        // rendered host must still match the configured host below, so
        // event content can never redirect the request elsewhere.
        let globals_rendered_url = render_globals_only(raw_url, globals);
        if has_empty_authority(&globals_rendered_url) {
            return Err("HTTP URL has no host after rendering globals.".to_owned());
        }
        let configured_url = reqwest::Url::parse(&globals_rendered_url)
            .map_err(|_| "HTTP URL is invalid before template rendering.".to_owned())?;
        let configured_host = configured_url
            .host_str()
            .map(str::to_ascii_lowercase)
            .ok_or_else(|| "HTTP URL has no host.".to_owned())?;
        let rendered_url = render_template(&globals_rendered_url, event, globals);
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
                    globals,
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
                        .map(|value| render_template(value, event, globals))
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
            let base = if status >= 500 {
                "server error"
            } else {
                "request failed"
            };
            return match http_error_reason(&body) {
                Some(reason) => Err(format!("HTTP {status} {base}: {reason}")),
                None => Err(format!("HTTP {status} {base}")),
            };
        }
        Ok(format!("{status} OK · {elapsed} ms · {response_url}"))
    }

    #[cfg(not(feature = "http"))]
    pub(crate) async fn execute_http_action(
        self: &Arc<Self>,
        _config: &serde_json::Map<String, Value>,
        _event: &Value,
        _globals: &std::collections::BTreeMap<String, String>,
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
pub(crate) struct HardenedHttpRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    pub timeout_ms: u64,
}

#[cfg_attr(not(feature = "http"), allow(dead_code))]
pub(crate) struct HardenedHttpResponse {
    pub status: u16,
    pub response_url: String,
    pub body: Value,
    pub elapsed_ms: u64,
}

/// Best-effort one-line reason from an error response body.
///
/// Servers commonly explain failures as `{"message": "..."}` (SonicBoom
/// included); anything else falls back to a whitespace-collapsed, truncated
/// snippet. Pure and total: empty or unparseable bodies yield `None` and the
/// caller keeps the legacy bare status text.
#[cfg_attr(not(feature = "http"), allow(dead_code))]
pub(crate) fn http_error_reason(body: &Value) -> Option<String> {
    const MAX_REASON_CHARS: usize = 300;
    let raw = match body {
        Value::String(text) => text.clone(),
        Value::Object(map) => map
            .get("message")
            .or_else(|| map.get("error"))
            .and_then(Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| serde_json::to_string(body).unwrap_or_default()),
        _ => return None,
    };
    let collapsed = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = collapsed.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.chars().count() > MAX_REASON_CHARS {
        let snippet: String = trimmed.chars().take(MAX_REASON_CHARS).collect();
        Some(format!("{snippet}…"))
    } else {
        Some(trimmed.to_owned())
    }
}

impl AppCore {
    /// Sends one request through the hardened client (no redirects, 2 MiB
    /// response cap, host re-check). Shared by automation HTTP actions and
    /// declarative plugin fetches so both enforce identical transport policy.
    #[cfg(feature = "http")]
    pub(crate) async fn send_hardened_http_request(
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
    pub(crate) async fn send_hardened_http_request(
        &self,
        _request: &HardenedHttpRequest,
        _configured_host: &str,
    ) -> Result<HardenedHttpResponse, String> {
        Err("HTTP action execution requires the host HTTP capability.".to_owned())
    }
}
