//! Explicit external sinks. The general execution guard remains Owner-only.
use super::context::ExecutionContext;
use crate::*;
#[cfg(feature = "persistence")]
use tiktools_plugin_api::AudioOverlap;

#[cfg(feature = "persistence")]
use super::external_effects::ExternalStart;
#[cfg(feature = "persistence")]
use crate::services::effects::EffectOutcome;

impl AppCore {
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn execute_approved_audio_action(
        self: &Arc<Self>,
        action: &Value,
        config: &serde_json::Map<String, Value>,
        event: &Value,
        context: &ExecutionContext,
        logs: &mut Vec<String>,
        test: bool,
    ) -> Result<String, String> {
        #[cfg(feature = "persistence")]
        {
            self.with_core_action(action, context, |_| Ok(()))?;
            let configured = config
                .get("fileRef")
                .or_else(|| config.get("file"))
                .or_else(|| config.get("filePath"))
                .or_else(|| config.get("path"))
                .ok_or("Audio action has no file reference.")?;
            let path = configured
                .get("path")
                .and_then(Value::as_str)
                .or_else(|| configured.as_str())
                .ok_or("Audio file reference must contain a path.")?;
            let rendered =
                crate::services::automation_approval_policy::render_template(path, event, 4096)?;
            let file = crate::services::audio_file_ref_from_config(
                &rendered,
                self.db.paths().data.as_path(),
            )
            .map_err(|_| "Audio file is unavailable.".to_owned())?;
            let (volume, overlap, overlap_kind) = approved_audio_options(config)?;
            let check = {
                let core = self.clone();
                let rendered = rendered.clone();
                let path = file.path.clone();
                let overlap = overlap.to_owned();
                move |identities: &[crate::collaboration::ExecutionIdentity]| {
                    let current = crate::services::audio_file_ref_from_config(
                        &rendered,
                        core.db.paths().data.as_path(),
                    )
                    .map_err(|_| "Audio file is unavailable.".to_owned())?;
                    check_approved_audio_scope(
                        &path,
                        &current.path,
                        &identities
                            .iter()
                            .map(|identity| identity.grant())
                            .collect::<Vec<_>>(),
                        volume,
                        &overlap,
                    )
                }
            };
            self.with_core_action(action, context, &check)?;
            if !self.media.supports_audio() {
                return Err("Host audio output is unavailable.".into());
            }
            if test {
                return Ok("would play the permitted audio file".into());
            }
            let started = self.start_external_effect_owned(action.clone(), context.clone(), crate::control::EffectKind::Audio,
                json!({"file":file.path,"output":"host-default","volume":volume,"overlap":overlap}), check.clone()).await?;
            let dispatch = match started {
                ExternalStart::Retained(receipt) => {
                    return Err(format!(
                        "Audio effect already retained ({:?}); it was not replayed.",
                        receipt.state
                    ))
                }
                ExternalStart::Dispatched(dispatch) => dispatch,
            };
            // The blocking journal task introduced an await. Revalidate again
            // before this executor receives the actual sink handoff.
            self.with_core_action(action, context, &check)?;
            let result = self
                .media
                .play_audio(
                    file.clone(),
                    AudioPlayOptions {
                        volume: volume as f32,
                        overlap: overlap_kind,
                    },
                )
                .await;
            let outcome = if result.is_ok() {
                EffectOutcome::Succeeded
            } else {
                EffectOutcome::Unknown
            };
            complete_external_effect(dispatch, outcome).await?;
            // Revocation during playback cannot undo audio already started. It
            // does prevent result-derived work and subsequent action steps.
            self.with_core_action(action, context, check)?;
            let played = result
                .map_err(|_| {
                    "Audio output failed; inspect the retained effect outcome.".to_owned()
                })?
                .played;
            let summary = if played {
                "played the permitted audio file"
            } else {
                "audio playback was skipped"
            }
            .to_owned();
            if logs.len() < 40 {
                logs.push(summary.clone());
            }
            Ok(summary)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (action, config, event, context, logs, test);
            Err("delegated audio requires persistence".into())
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) async fn execute_approved_http_action(
        self: &Arc<Self>,
        action: &Value,
        config: &serde_json::Map<String, Value>,
        event: &Value,
        context: &ExecutionContext,
        logs: &mut Vec<String>,
        test: bool,
    ) -> Result<String, String> {
        #[cfg(all(feature = "persistence", feature = "http"))]
        {
            self.with_core_action(action, context, |_| Ok(()))?;
            if config
                .get("emitResponseAs")
                .and_then(Value::as_str)
                .is_some_and(|v| !v.trim().is_empty())
            {
                return Err(
                    "Delegated HTTP response emission requires a scoped event policy.".into(),
                );
            }
            let raw = config
                .get("url")
                .and_then(Value::as_str)
                .ok_or("HTTP action has no URL.")?;
            let rendered_url =
                crate::services::automation_approval_policy::render_template(raw, event, 8192)?;
            let url = reqwest::Url::parse(&rendered_url)
                .map_err(|_| "HTTP URL is invalid.".to_owned())?;
            let host = url.host_str().ok_or("HTTP URL has no host.")?.to_owned();
            let method = config
                .get("method")
                .and_then(Value::as_str)
                .unwrap_or("POST")
                .trim()
                .to_ascii_uppercase();
            let method_value = reqwest::Method::from_bytes(method.as_bytes())
                .map_err(|_| "HTTP method is invalid.")?;
            let allow_private_network = config
                .get("allowPrivateNetwork")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            validate_http_url_shape(&url, &host, None, allow_private_network)?;
            let mut headers = Vec::new();
            if let Some(values) = config.get("headers").and_then(Value::as_object) {
                if values.len() > 64 {
                    return Err("HTTP action has too many headers.".into());
                }
                for (key, value) in values {
                    headers.push((
                        key.to_ascii_lowercase(),
                        crate::services::automation_approval_policy::render_template(
                            &value_to_string(value),
                            event,
                            8192,
                        )?,
                    ));
                }
            }
            let names = headers
                .iter()
                .map(|(name, _)| name.clone())
                .collect::<Vec<_>>();
            let body = config
                .get("body")
                .filter(|value| !value.is_null())
                .map(|value| match value.as_str() {
                    Some(source) => crate::services::automation_approval_policy::render_template(
                        source,
                        event,
                        crate::collaboration::MAX_EFFECT_BODY_BYTES,
                    ),
                    None => {
                        let body = value_to_string(value);
                        if body.len() > crate::collaboration::MAX_EFFECT_BODY_BYTES {
                            Err("HTTP body exceeds its rendering budget.".into())
                        } else {
                            Ok(body)
                        }
                    }
                })
                .transpose()?;
            let fields = crate::services::automation_manifest::effect_render_fields(
                &Value::Object(config.clone()),
            )?;
            let limit = config
                .get("maxResponseBytes")
                .map(|value| {
                    value
                        .as_u64()
                        .and_then(|value| usize::try_from(value).ok())
                        .ok_or("HTTP response limit is invalid.")
                })
                .transpose()?
                .unwrap_or(crate::collaboration::MAX_EFFECT_RESPONSE_BYTES);
            let check = {
                let method = method.clone();
                let url = url.clone();
                let names = names.clone();
                let fields = fields.clone();
                let body = body.clone();
                move |identities: &[crate::collaboration::ExecutionIdentity]| {
                    let request = crate::collaboration::HttpEffectRequest {
                        method: &method,
                        url: url.as_str(),
                        header_names: &names,
                        render_fields: &fields,
                        body: body.as_deref(),
                        response_limit: limit,
                        allow_private_network,
                    };
                    if identities
                        .iter()
                        .any(|identity| !identity.grant().allows_http_effect(&request))
                    {
                        Err("HTTP request is outside execution scope.".into())
                    } else {
                        Ok(())
                    }
                }
            };
            self.with_core_action(action, context, &check)?;
            if test {
                return Ok("would send the permitted HTTP request".into());
            }
            // Resolve once and pin the checked addresses into this request's
            // private client. No proxy or second DNS resolution may bypass it.
            let port = url.port_or_known_default().ok_or("HTTP port is invalid.")?;
            let addresses = tokio::net::lookup_host((host.trim_matches(['[', ']']), port))
                .await
                .map_err(|_| "HTTP destination could not be resolved.".to_owned())?
                .collect::<Vec<_>>();
            validate_effect_addresses(&addresses, allow_private_network)?;
            let client = tiktools_plugin_sdk::http::client_builder()?
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .resolve_to_addrs(&host, &addresses)
                .build()
                .map_err(|_| "Pinned HTTP client is unavailable.".to_owned())?;
            let mut outgoing = client.request(method_value, url.clone());
            for (key, value) in &headers {
                outgoing = outgoing.header(key, value);
            }
            if let Some(body) = &body {
                outgoing = outgoing.body(body.clone());
            }
            let timeout = config
                .get("timeoutMs")
                .and_then(Value::as_u64)
                .unwrap_or(5000)
                .clamp(100, 120000);
            let outgoing = outgoing
                .timeout(std::time::Duration::from_millis(timeout))
                .build()
                .map_err(|_| "HTTP request headers or body are invalid.".to_owned())?;
            let start = self.start_external_effect_owned(action.clone(), context.clone(), crate::control::EffectKind::Http,
                json!({"url":url.as_str(),"method":method,"headers":headers,"body":body,"responseLimit":limit,"timeout":timeout,"allowPrivateNetwork":allow_private_network}), check.clone()).await?;
            let dispatch = match start {
                ExternalStart::Retained(receipt) => {
                    return Err(format!(
                        "HTTP effect already retained ({:?}); it was not replayed.",
                        receipt.state
                    ))
                }
                ExternalStart::Dispatched(dispatch) => dispatch,
            };
            let started = now_millis();
            let result = async {
                // Admission ran on the blocking pool; stale authority cannot
                // cross its scheduling boundary into an unstarted request.
                self.with_core_action(action, context, &check)?;
                let mut response = client
                    .execute(outgoing)
                    .await
                    .map_err(|_| "HTTP outcome is unknown.".to_owned())?;
                if response.url() != &url {
                    return Err("HTTP response destination changed.".into());
                }
                let status = response.status().as_u16();
                if response
                    .content_length()
                    .is_some_and(|length| length > limit as u64)
                {
                    return Err("HTTP response exceeds the approved limit.".into());
                }
                let mut count = 0usize;
                while let Some(chunk) = response
                    .chunk()
                    .await
                    .map_err(|_| "HTTP response outcome is unknown.".to_owned())?
                {
                    count = count.saturating_add(chunk.len());
                    if count > limit {
                        return Err("HTTP response exceeds the approved limit.".into());
                    }
                }
                Ok::<_, String>(status)
            }
            .await;
            complete_external_effect(
                dispatch,
                match result {
                    Ok(status) if (200..300).contains(&status) => EffectOutcome::Succeeded,
                    Ok(_) => EffectOutcome::Failed,
                    Err(_) => EffectOutcome::Unknown,
                },
            )
            .await?;
            self.with_core_action(action, context, check)?;
            let status = result?;
            let summary = format!(
                "HTTP {status} ({} ms)",
                now_millis().saturating_sub(started)
            );
            if logs.len() < 40 {
                logs.push(summary.clone());
            }
            if (200..300).contains(&status) {
                Ok(summary)
            } else {
                Err(summary)
            }
        }
        #[cfg(not(all(feature = "persistence", feature = "http")))]
        {
            let _ = (action, config, event, context, logs, test);
            Err("delegated HTTP requires host persistence and HTTP capabilities".into())
        }
    }
}

#[cfg(feature = "persistence")]
async fn complete_external_effect(
    dispatch: crate::services::effects::EffectDispatch,
    outcome: EffectOutcome,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || dispatch.complete(outcome))
        .await
        .map_err(|_| "external effect completion is unavailable".to_owned())??;
    Ok(())
}

#[cfg(feature = "persistence")]
fn approved_audio_options(
    config: &serde_json::Map<String, Value>,
) -> Result<(f64, &str, AudioOverlap), String> {
    let volume = match config.get("volume") {
        None => 1.0,
        Some(value) => number_value(Some(value)).ok_or("Audio volume is invalid.")?,
    };
    if !volume.is_finite() || !(0.0..=1.0).contains(&volume) {
        return Err("Audio volume must be finite and between zero and one.".into());
    }
    let overlap = match config.get("overlap") {
        None => "allow",
        Some(value) => value.as_str().ok_or("Audio overlap policy is invalid.")?,
    };
    let kind = match overlap {
        "allow" => AudioOverlap::Allow,
        "restart" => AudioOverlap::Restart,
        "drop" => AudioOverlap::Drop,
        _ => return Err("Audio overlap policy is invalid.".into()),
    };
    Ok((volume, overlap, kind))
}

#[cfg(feature = "persistence")]
fn check_approved_audio_scope(
    expected_path: &str,
    current_path: &str,
    grants: &[&crate::collaboration::Grant],
    volume: f64,
    overlap: &str,
) -> Result<(), String> {
    // The media contract grants a canonical pathname and playback options.
    // It does not bind an inode or freeze file contents between this check and
    // the backend opening the file; that requires a separate handle-based
    // media contract. Delegated automation has no host-filesystem write grant.
    if current_path != expected_path
        || grants
            .iter()
            .any(|grant| !grant.allows_audio_effect(current_path, "host-default", volume, overlap))
    {
        return Err("Audio target or options are outside execution scope.".into());
    }
    Ok(())
}

#[cfg(all(feature = "persistence", feature = "http"))]
fn validate_effect_addresses(
    addresses: &[std::net::SocketAddr],
    allow_private: bool,
) -> Result<(), String> {
    if addresses.is_empty() || addresses.len() > 64 {
        return Err("HTTP DNS response is unavailable or too large.".into());
    }
    if !allow_private
        && addresses.iter().any(|address| {
            let ip = match address.ip() {
                std::net::IpAddr::V6(ip) => ip
                    .to_ipv4_mapped()
                    .map(std::net::IpAddr::V4)
                    .unwrap_or(std::net::IpAddr::V6(ip)),
                ip => ip,
            };
            crate::helpers::is_private_ip(ip)
                || match ip {
                    std::net::IpAddr::V4(ip) => {
                        ip.is_multicast() || ip.is_broadcast() || ip.is_link_local()
                    }
                    std::net::IpAddr::V6(ip) => ip.is_multicast(),
                }
        })
    {
        return Err("HTTP destination resolves outside the permitted network scope.".into());
    }
    Ok(())
}

#[cfg(all(test, feature = "persistence"))]
mod tests;
