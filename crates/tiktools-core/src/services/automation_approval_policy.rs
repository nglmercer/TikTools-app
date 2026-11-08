//! Closed first execution policy. This is private host code, not a request
//! schema: a caller cannot grant effects by supplying a policy JSON object.
use super::automation_manifest::{EffectKind, ExecutionManifest, ManifestReason};
use serde_json::{json, Map, Value};

const PUBLIC_FIELDS: &[&str] = crate::collaboration::PUBLIC_EFFECT_RENDER_FIELDS;

/// Validate the entire closure, including disabled/unselected dependencies.
/// Unknown effects require a separate policy implementation and fresh review.
pub(crate) fn bindings(
    manifest: &ExecutionManifest,
    identities: &[crate::collaboration::ExecutionIdentity],
) -> Result<Value, String> {
    if manifest.policy_version != 1 || manifest.execution_authorized {
        return Err("execution approval policy is unavailable".into());
    }
    if manifest.reasons.iter().any(|reason| {
        !matches!(
            reason,
            ManifestReason::ApprovalRequired | ManifestReason::DynamicTarget
        )
    }) {
        return Err("saved execution closure does not satisfy the approval policy".into());
    }
    for effect in &manifest.effects {
        if !matches!(
            effect.kind,
            EffectKind::Log
                | EffectKind::Delay
                | EffectKind::Points
                | EffectKind::Http
                | EffectKind::Audio
        ) || effect.dynamic_target
            && !matches!(
                effect.kind,
                EffectKind::Points | EffectKind::Http | EffectKind::Audio
            )
            || effect
                .render_fields
                .iter()
                .any(|field| !PUBLIC_FIELDS.contains(&field.as_str()))
            || effect.reasons.iter().any(|reason| {
                !matches!(
                    reason,
                    ManifestReason::ApprovalRequired | ManifestReason::DynamicTarget
                )
            })
        {
            return Err("effect or rendered field has no delegated execution policy".into());
        }
    }
    let configurations: Vec<_> = identities
        .iter()
        .map(|identity| {
            json!({
                "principalId":identity.principal_id(), "sessionId":identity.session_id(),
                "configurationRevision":identity.configuration_revision()
            })
        })
        .collect();
    Ok(json!({
        "policyVersion":1,
        "effectPolicy":"core-log-delay-points-http-audio-v2",
        "renderPolicy":"public-live-scalars-bounded-v2",
        "publicFields":PUBLIC_FIELDS,
        "globals":false,
        "renderLimits":{
            "urlBytes":8192,"headerValueBytes":8192,"bodyBytes":262144,
            "audioPathBytes":4096,"logBytes":65536,"pointsViewerBytes":256
        },
        "configurations":configurations,
        "viewerAdmission":"current-all-subjects-exact-viewer-and-delta-v1"
    }))
}

/// Render delegated templates without building an unbounded expansion. Event
/// lookup keeps the existing helper's prefix/segment semantics; globals are
/// unavailable even if an input happens to contain a similarly named field.
/// Callers supply the closed public projection and a sink-specific byte limit.
pub(crate) fn render_template(
    source: &str,
    event: &Value,
    max_bytes: usize,
) -> Result<String, String> {
    if max_bytes == 0 || source.len() > max_bytes {
        return Err("template source exceeds its byte limit".into());
    }
    let mut output = BoundedTemplate {
        bytes: Vec::new(),
        max_bytes,
    };
    let mut rest = source;
    while let Some(start) = rest.find("{{") {
        output.append(&rest.as_bytes()[..start])?;
        let expression = &rest[start + 2..];
        let Some(end) = expression.find("}}") else {
            // Approval rejects unclassifiable spans. Keep a literal suffix
            // once if this helper is called independently with such input.
            output.append(&rest.as_bytes()[start..])?;
            return output.finish();
        };
        let path = expression[..end].trim();
        if path != "globals" && !path.starts_with("globals.") {
            if let Some(value) = crate::helpers::read_event_path(event, path) {
                match value {
                    Value::Null => {}
                    Value::String(text) => output.append(text.as_bytes())?,
                    // This streams compact formatting through the same bound;
                    // serializing a whole value cannot allocate a giant String
                    // before discovering that the result is too large.
                    Value::Bool(_) | Value::Number(_) | Value::Array(_) | Value::Object(_) => {
                        serde_json::to_writer(&mut output, value)
                            .map_err(|_| "rendered template exceeds its byte limit".to_owned())?;
                    }
                }
            }
        }
        rest = &expression[end + 2..];
    }
    output.append(rest.as_bytes())?;
    output.finish()
}

struct BoundedTemplate {
    bytes: Vec<u8>,
    max_bytes: usize,
}

impl BoundedTemplate {
    fn append(&mut self, bytes: &[u8]) -> Result<(), String> {
        if bytes.len() > self.max_bytes.saturating_sub(self.bytes.len()) {
            return Err("rendered template exceeds its byte limit".into());
        }
        let needed = self.bytes.len() + bytes.len();
        if needed > self.bytes.capacity() {
            // Keep amortized growth without Vec's implicit doubling past the
            // sink budget. Reserve only after the append has passed its bound.
            let capacity = self
                .bytes
                .capacity()
                .saturating_mul(2)
                .max(64)
                .max(needed)
                .min(self.max_bytes);
            self.bytes
                .try_reserve_exact(capacity - self.bytes.len())
                .map_err(|_| "rendered template allocation is unavailable".to_owned())?;
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }

    fn finish(self) -> Result<String, String> {
        String::from_utf8(self.bytes).map_err(|_| "rendered template is invalid UTF-8".into())
    }
}

impl std::io::Write for BoundedTemplate {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.append(bytes).map_err(std::io::Error::other)?;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Copy only public scalar leaves. Objects, arrays, oversized strings and
/// enrichment/settings/authority fields never enter a delegated renderer.
pub(crate) fn project_event(event: &Value) -> Value {
    fn insert(map: &mut Map<String, Value>, parts: &[&str], value: Value) {
        if parts.len() == 1 {
            map.insert(parts[0].into(), value);
        } else {
            let child = map
                .entry(parts[0])
                .or_insert_with(|| Value::Object(Map::new()));
            if let Some(child) = child.as_object_mut() {
                insert(child, &parts[1..], value);
            }
        }
    }
    let mut result = Map::new();
    for field in PUBLIC_FIELDS {
        let parts: Vec<_> = field
            .strip_prefix("event.")
            .unwrap_or(field)
            .split('.')
            .collect();
        let mut current = Some(event);
        for part in &parts {
            current = current.and_then(|value| value.get(*part));
        }
        if let Some(value) = current.filter(|v| match v {
            Value::String(text) => text.len() <= 4096,
            Value::Null | Value::Bool(_) | Value::Number(_) => true,
            Value::Array(_) | Value::Object(_) => false,
        }) {
            insert(&mut result, &parts, value.clone());
        }
    }
    Value::Object(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn projection_is_closed_and_scalar_even_for_public_named_objects() {
        let event = json!({"id":"e","type":"tiktok.chat","user":{"uniqueId":"alice","token":"secret"},"data":{"comment":{"secret":"no"},"count":2,"plugin":{"token":"no"}},"globals":{"secret":"no"},"authority":{"kind":"owner"}});
        assert_eq!(
            project_event(&event),
            json!({"id":"e","type":"tiktok.chat","user":{"uniqueId":"alice"},"data":{"count":2}})
        );
        assert!(project_event(&json!({"data":{"comment":"x".repeat(4097)}}))["data"].is_null());
    }

    #[test]
    fn delegated_expansion_stops_before_crossing_its_budget_or_allocating_the_remainder() {
        let event = project_event(&json!({"data":{"comment":"x".repeat(4096)}}));
        let source = "{{event.data.comment}}".repeat(128);
        assert!(source.len() < 8192);
        assert!(render_template(&source, &event, 8192).is_err());
        let mut output = BoundedTemplate {
            bytes: Vec::new(),
            max_bytes: 8192,
        };
        for _ in 0..2 {
            output.append("x".repeat(4096).as_bytes()).unwrap();
        }
        let before = output.bytes.capacity();
        assert!(output.append("x".repeat(4096).as_bytes()).is_err());
        assert_eq!(output.bytes.len(), 8192);
        assert_eq!(output.bytes.capacity(), before);
        assert!(output.bytes.capacity() <= output.max_bytes);
        // Structured formatting is also streamed through the budget; no
        // value_to_string intermediate can materialize the expanded object.
        let oversized = json!({"id":{"message":"x".repeat(4096)}});
        assert!(render_template("{{id}}", &oversized, 256).is_err());
    }

    #[test]
    fn delegated_lookup_matches_existing_event_semantics_and_drops_unknown_or_global_fields() {
        let event = project_event(
            &json!({"id":"e","user":{"uniqueId":"alice","nickname":"Hi"},"data":{"count":2},"private":"secret","globals":{"token":"secret"}}),
        );
        for source in [
            "{{ event.user.uniqueId }}",
            "{{user..nickname}}",
            "{{event.event.data.count}}",
            "{{.user.nickname.}}",
            "{{missing}} {{private}}",
            "{{user}}",
        ] {
            assert_eq!(
                render_template(source, &event, 512).unwrap(),
                crate::helpers::render_template(source, &event, &Default::default())
            );
        }
        assert_eq!(
            render_template(
                "{{globals}}/{{ globals.token }}/{{event.globals.token}}",
                &event,
                128
            )
            .unwrap(),
            "//"
        );
        assert_eq!(
            render_template("before {{unclosed", &event, 128).unwrap(),
            "before {{unclosed"
        );
    }

    #[test]
    fn delegated_render_limits_count_utf8_bytes_and_reject_zero_or_oversized_sources() {
        let event = project_event(&json!({"id":"éééé"}));
        assert_eq!(render_template("{{id}}", &event, 8).unwrap(), "éééé");
        assert!(render_template("{{id}}", &event, 7).is_err());
        assert_eq!(render_template("á🙂", &event, 6).unwrap(), "á🙂");
        assert!(render_template("á🙂", &event, 5).is_err());
        assert!(render_template("", &event, 0).is_err());
        assert!(render_template("{{missing}}", &event, 10).is_err());
    }
}
