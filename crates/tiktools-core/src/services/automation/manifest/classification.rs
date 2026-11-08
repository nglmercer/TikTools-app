//! Closed inventory of known host effects. Classification is not approval and
//! unresolved dynamic targets never become an automatic execution permission.
use super::*;
use serde_json::Map;

fn rendered_field(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() || raw.len() > 256 || raw.chars().any(char::is_control) {
        return None;
    }
    let normalized = if raw == "globals" || raw.starts_with("globals.") {
        raw.to_owned()
    } else {
        // Match helpers::read_event_path exactly, including its legacy second
        // prefix strip and ignored empty segments. Never label a different
        // actual event field as the one reviewed by the operator.
        let path = raw.strip_prefix("event.").unwrap_or(raw);
        let path = path.strip_prefix("event").unwrap_or(path);
        let parts: Vec<_> = path
            .trim_matches('.')
            .split('.')
            .filter(|p| !p.is_empty())
            .collect();
        if parts.is_empty() {
            return Some("event".into());
        }
        // A leading-dot spelling can access a literal root key beginning with
        // `event`; spelling that label canonically would trigger the helper's
        // second prefix strip again. Keep that ambiguity review-required.
        if parts[0].starts_with("event") {
            return None;
        }
        format!("event.{}", parts.join("."))
    };
    if normalized.len() > 256
        || !normalized.split('.').all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
        })
    {
        return None;
    }
    Some(normalized)
}
fn spans(text: &str, fields: &mut BTreeSet<String>, reasons: &mut BTreeSet<ManifestReason>) {
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        let expression = &rest[start + 2..];
        let Some(end) = expression.find("}}") else {
            reasons.insert(ManifestReason::UnclassifiableRenderField);
            break;
        };
        match rendered_field(&expression[..end]) {
            Some(field) => {
                if field == "globals" || field.starts_with("globals.") {
                    reasons.insert(ManifestReason::GlobalAccessRequiresPolicy);
                }
                if field == "event" {
                    reasons.insert(ManifestReason::UnclassifiableRenderField);
                }
                if fields.len() < 256 || fields.contains(&field) {
                    fields.insert(field);
                } else {
                    reasons.insert(ManifestReason::UnclassifiableRenderField);
                }
            }
            None => {
                reasons.insert(ManifestReason::UnclassifiableRenderField);
            }
        }
        rest = &expression[end + 2..];
    }
}
fn render_fields(
    value: &Value,
    fields: &mut BTreeSet<String>,
    reasons: &mut BTreeSet<ManifestReason>,
    depth: usize,
) {
    if depth > 64 {
        reasons.insert(ManifestReason::UnclassifiableRenderField);
        return;
    }
    match value {
        Value::String(text) => spans(text, fields, reasons),
        Value::Object(object) => {
            for value in object.values() {
                render_fields(value, fields, reasons, depth + 1);
            }
        }
        Value::Array(array) => {
            for value in array {
                render_fields(value, fields, reasons, depth + 1);
            }
        }
        _ => {}
    }
}

pub(crate) fn effect_render_fields(config: &Value) -> Result<Vec<String>, String> {
    let mut fields = BTreeSet::new();
    let mut reasons = BTreeSet::new();
    render_fields(config, &mut fields, &mut reasons, 0);
    if !reasons.is_empty() {
        return Err("effect contains global or unclassifiable rendered fields".into());
    }
    Ok(fields.into_iter().collect())
}
fn dynamic(value: Option<&Value>) -> bool {
    value
        .and_then(Value::as_str)
        .is_some_and(|v| v.contains("{{"))
}
fn decision(reasons: &BTreeSet<ManifestReason>) -> EffectDecision {
    if reasons.iter().any(|reason| {
        matches!(
            reason,
            ManifestReason::MissingDependency
                | ManifestReason::MissingCapability
                | ManifestReason::ResourceForbidden
                | ManifestReason::LiveRestriction
                | ManifestReason::InvalidConfiguration
                // Reviewing exact source cannot add a global/secret-read
                // permission that the current grant model cannot express.
                | ManifestReason::GlobalAccessRequiresPolicy
                | ManifestReason::EffectPolicyUnavailable
                | ManifestReason::PluginBindingUnavailable
                | ManifestReason::UnknownActionType
        )
    }) {
        EffectDecision::Unsupported
    } else if reasons.is_empty() {
        EffectDecision::Classified
    } else {
        EffectDecision::ReviewRequired
    }
}
fn needed(
    caps: &[Capability],
    subjects: &[ManifestSubject<'_>],
    reasons: &mut BTreeSet<ManifestReason>,
) {
    for subject in subjects {
        if caps
            .iter()
            .any(|cap| !subject.grant.capabilities.contains(cap))
        {
            reasons.insert(ManifestReason::MissingCapability);
        }
    }
}

pub(super) fn classify(entry: &Entry<'_>, subjects: &[ManifestSubject<'_>]) -> ManifestEffect {
    let mut result = ManifestEffect {
        family: AutomationFamily::Action,
        id: entry.binding.id.clone(),
        type_id: None,
        kind: EffectKind::MissingDefinition,
        enabled: None,
        decision: EffectDecision::Unsupported,
        render_fields: Vec::new(),
        dynamic_target: false,
        required_capabilities: Vec::new(),
        reasons: Vec::new(),
    };
    let Some(record) = entry.record else {
        result.reasons.push(ManifestReason::MissingDependency);
        return result;
    };
    let mut reasons = BTreeSet::new();
    result.enabled = record.get("enabled").and_then(Value::as_bool);
    if record.get("enabled").is_some_and(|v| !v.is_boolean()) {
        reasons.insert(ManifestReason::InvalidConfiguration);
    }
    let type_id = record
        .get("typeId")
        .or_else(|| record.get("type"))
        .and_then(Value::as_str)
        .filter(|v| canonical_id(v, 128));
    result.type_id = type_id.map(str::to_owned);
    let empty = Map::new();
    let config = record
        .get("config")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    if record.get("config").is_some_and(|v| !v.is_object()) {
        reasons.insert(ManifestReason::InvalidConfiguration);
    }
    let mut fields = BTreeSet::new();
    match type_id {
        Some("core.log") => {
            result.kind = EffectKind::Log;
            if let Some(message) = config.get("message") {
                if let Some(message) = message.as_str() {
                    spans(message, &mut fields, &mut reasons);
                } else {
                    reasons.insert(ManifestReason::InvalidConfiguration);
                }
            }
        }
        Some("core.emit") => {
            result.kind = EffectKind::Emit;
            result.required_capabilities = vec![Capability::EventsEmit];
            if config
                .get("type")
                .is_some_and(|v| !v.as_str().is_some_and(|v| canonical_id(v, 128)))
            {
                reasons.insert(ManifestReason::InvalidConfiguration);
            }
            if let Some(data) = config.get("data") {
                if let Some(data) = data.as_object() {
                    if serde_json::to_vec(data).is_ok_and(|bytes| bytes.len() > 64 * 1024) {
                        reasons.insert(ManifestReason::InvalidConfiguration);
                    }
                    // Approved emit rendering walks nested objects and arrays;
                    // inventory every value before permitting that execution.
                    render_fields(&Value::Object(data.clone()), &mut fields, &mut reasons, 0);
                } else {
                    reasons.insert(ManifestReason::InvalidConfiguration);
                }
            }
            let topic = config
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("overlay.alert");
            match crate::helpers::normalize_emit_type(topic) {
                Ok(topic) => {
                    if subjects
                        .iter()
                        .any(|subject| !subject.grant.allows_emit_topic(&topic))
                    {
                        reasons.insert(ManifestReason::ResourceForbidden);
                    }
                }
                Err(_) => {
                    reasons.insert(ManifestReason::InvalidConfiguration);
                }
            }
            if subjects.is_empty() {
                reasons.insert(ManifestReason::EffectPolicyUnavailable);
            }
        }
        Some("core.points" | "core.points.subtract") => {
            result.kind = EffectKind::Points;
            result.required_capabilities = vec![Capability::PointsRead, Capability::PointsAdjust];
            let viewer = config.get("uniqueId").and_then(Value::as_str);
            result.dynamic_target = dynamic(config.get("uniqueId"));
            if let Some(viewer) = viewer {
                spans(viewer, &mut fields, &mut reasons);
            }
            let delta = if type_id == Some("core.points.subtract") {
                crate::helpers::number_value(config.get("amount")).map(|n| -n.abs())
            } else {
                crate::helpers::number_value(config.get("delta"))
            };
            if viewer.is_none_or(|id| id.trim().is_empty())
                || delta.is_none_or(|n| !n.is_finite() || n == 0.0 || n.abs() > 1_000_000_000.0)
            {
                reasons.insert(ManifestReason::InvalidConfiguration);
            }
            if result.dynamic_target {
                reasons.insert(ManifestReason::DynamicTarget);
            } else if let Some(viewer) = viewer {
                if !canonical_id(viewer, 128) || viewer.starts_with('@') {
                    reasons.insert(ManifestReason::InvalidConfiguration);
                }
                for subject in subjects {
                    if !subject.grant.allows_resource(ResourceKind::Viewer, viewer) {
                        reasons.insert(ManifestReason::ResourceForbidden);
                    }
                }
            }
            for subject in subjects {
                if let Some(limit) = subject.grant.max_points_delta {
                    if delta.is_some_and(|n| n.abs() > limit) {
                        reasons.insert(ManifestReason::ResourceForbidden);
                    }
                }
            }
        }
        Some("core.delay") => {
            result.kind = EffectKind::Delay;
            if config.get("ms").is_some()
                && crate::helpers::number_value(config.get("ms"))
                    .is_none_or(|n| !n.is_finite() || !(0.0..=60_000.0).contains(&n))
            {
                reasons.insert(ManifestReason::InvalidConfiguration);
            }
        }
        Some("core.code") => {
            result.kind = EffectKind::Script;
            result.required_capabilities = vec![Capability::ScriptsExecute, Capability::LiveRead];
            result.dynamic_target = true;
            if config
                .get("source")
                .and_then(Value::as_str)
                .is_none_or(|source| {
                    source.is_empty()
                        || source.len() > crate::services::SCOPED_SCRIPT_LIMITS.source_bytes
                })
            {
                reasons.insert(ManifestReason::InvalidConfiguration);
            }
            // Scripts inspect dynamic event input and may return several effect
            // classes. Regex/source matching cannot establish their authority.
            reasons.insert(ManifestReason::OpaqueScript);
        }
        Some("core.fetch") => {
            result.kind = EffectKind::Http;
            result.required_capabilities = vec![Capability::HttpRequest];
            render_fields(&Value::Object(config.clone()), &mut fields, &mut reasons, 0);
            result.dynamic_target = dynamic(config.get("url"));
            if config
                .get("url")
                .and_then(Value::as_str)
                .is_none_or(str::is_empty)
            {
                reasons.insert(ManifestReason::InvalidConfiguration);
            }
            if result.dynamic_target {
                reasons.insert(ManifestReason::DynamicTarget);
            }
            if subjects.iter().any(|subject| {
                subject.grant.effects.http.is_empty() || subject.grant.effects.validate().is_err()
            }) {
                reasons.insert(ManifestReason::ResourceForbidden);
            }
            // Static arguments can be checked now. Dynamic bodies/targets keep
            // exact-source review and their authoritative rendered sink check;
            // template syntax is not mistaken for a forbidden literal request.
            let body = config.get("body").filter(|v| !v.is_null()).map(|v| {
                v.as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| v.to_string())
            });
            if !result.dynamic_target && !body.as_deref().is_some_and(|s| s.contains("{{")) {
                if let Some(raw_url) = config.get("url").and_then(Value::as_str) {
                    match url::Url::parse(raw_url) {
                        Ok(url) => {
                            let method = config
                                .get("method")
                                .and_then(Value::as_str)
                                .unwrap_or("POST")
                                .trim()
                                .to_ascii_uppercase();
                            let headers = config
                                .get("headers")
                                .and_then(Value::as_object)
                                .map(|h| h.keys().cloned().collect::<Vec<_>>())
                                .unwrap_or_default();
                            let render_fields = fields.iter().cloned().collect::<Vec<_>>();
                            let response_limit = match config.get("maxResponseBytes") {
                                None => crate::collaboration::MAX_EFFECT_RESPONSE_BYTES,
                                Some(value) => value
                                    .as_u64()
                                    .and_then(|n| usize::try_from(n).ok())
                                    .unwrap_or(0),
                            };
                            if response_limit == 0
                                || response_limit > crate::collaboration::MAX_EFFECT_RESPONSE_BYTES
                            {
                                reasons.insert(ManifestReason::InvalidConfiguration);
                            }
                            let request = crate::collaboration::HttpEffectRequest {
                                method: &method,
                                url: url.as_str(),
                                header_names: &headers,
                                render_fields: &render_fields,
                                body: body.as_deref(),
                                response_limit,
                                allow_private_network: config
                                    .get("allowPrivateNetwork")
                                    .and_then(Value::as_bool)
                                    .unwrap_or(false),
                            };
                            if subjects
                                .iter()
                                .any(|subject| !subject.grant.allows_http_effect(&request))
                            {
                                reasons.insert(ManifestReason::ResourceForbidden);
                            }
                        }
                        Err(_) => {
                            reasons.insert(ManifestReason::InvalidConfiguration);
                        }
                    }
                }
            }
            // Response-derived event delivery has a separate provenance sink.
            if subjects.is_empty()
                || config
                    .get("emitResponseAs")
                    .and_then(Value::as_str)
                    .is_some_and(|s| !s.trim().is_empty())
            {
                reasons.insert(ManifestReason::EffectPolicyUnavailable);
            }
        }
        Some("audio.play" | "core.audio.play") => {
            result.kind = EffectKind::Audio;
            result.required_capabilities = vec![Capability::AudioPlay];
            render_fields(&Value::Object(config.clone()), &mut fields, &mut reasons, 0);
            let configured = config
                .get("fileRef")
                .or_else(|| config.get("file"))
                .or_else(|| config.get("filePath"))
                .or_else(|| config.get("path"));
            let path = configured.and_then(|value| {
                value
                    .get("path")
                    .or_else(|| value.is_string().then_some(value))
            });
            result.dynamic_target = dynamic(path);
            if path.and_then(Value::as_str).is_none_or(str::is_empty) {
                reasons.insert(ManifestReason::InvalidConfiguration);
            }
            if result.dynamic_target {
                reasons.insert(ManifestReason::DynamicTarget);
            }
            if subjects.iter().any(|subject| {
                subject.grant.effects.audio.is_empty() || subject.grant.effects.validate().is_err()
            }) {
                reasons.insert(ManifestReason::ResourceForbidden);
            }
            // Canonical paths are resolved by the media service at the final
            // sink. This pure manifest builder does not inspect the host FS.
            if subjects.is_empty() {
                reasons.insert(ManifestReason::EffectPolicyUnavailable);
            }
        }
        Some(type_id) if !type_id.starts_with("core.") => {
            result.kind = EffectKind::Plugin;
            result.required_capabilities = vec![Capability::PluginActionExecute];
            result.dynamic_target = true;
            render_fields(&Value::Object(config.clone()), &mut fields, &mut reasons, 0);
            // No descriptor/artifact/permissions/settings-generation binding was
            // supplied to this builder. A typeId is never proof of a plugin.
            reasons.insert(ManifestReason::PluginBindingUnavailable);
        }
        _ => {
            result.kind = EffectKind::Unknown;
            reasons.insert(ManifestReason::UnknownActionType);
        }
    }
    if !fields.is_empty() {
        result.required_capabilities.push(Capability::LiveRead);
    }
    needed(&result.required_capabilities, subjects, &mut reasons);
    result.render_fields = fields.into_iter().collect();
    result.decision = decision(&reasons);
    result.reasons = reasons.into_iter().collect();
    result
}
