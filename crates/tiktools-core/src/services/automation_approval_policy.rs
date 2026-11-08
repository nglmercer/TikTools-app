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
            ManifestReason::ApprovalRequired
                | ManifestReason::DynamicTarget
                | ManifestReason::OpaqueScript
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
                | EffectKind::Emit
                | EffectKind::Script
        ) || effect.dynamic_target
            && !matches!(
                effect.kind,
                EffectKind::Points | EffectKind::Http | EffectKind::Audio | EffectKind::Script
            )
            || effect
                .render_fields
                .iter()
                .any(|field| !PUBLIC_FIELDS.contains(&field.as_str()))
            || effect.reasons.iter().any(|reason| {
                !matches!(
                    reason,
                    ManifestReason::ApprovalRequired | ManifestReason::DynamicTarget
                ) && !(*reason == ManifestReason::OpaqueScript && effect.kind == EffectKind::Script)
            })
            || effect.kind == EffectKind::Script
                && (effect.type_id.as_deref() != Some("core.code")
                    || !effect.reasons.contains(&ManifestReason::OpaqueScript)
                    || !effect
                        .required_capabilities
                        .contains(&crate::collaboration::Capability::ScriptsExecute)
                    || !effect
                        .required_capabilities
                        .contains(&crate::collaboration::Capability::LiveRead))
            || effect.kind == EffectKind::Emit
                && (effect.type_id.as_deref() != Some("core.emit")
                    || !effect
                        .required_capabilities
                        .contains(&crate::collaboration::Capability::EventsEmit))
        {
            return Err("effect or rendered field has no delegated execution policy".into());
        }
    }
    if manifest.reasons.contains(&ManifestReason::OpaqueScript)
        && !manifest.effects.iter().any(|effect| {
            effect.kind == EffectKind::Script
                && effect.reasons.contains(&ManifestReason::OpaqueScript)
        })
    {
        return Err("opaque script review is not bound to an exact script effect".into());
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
    let script_limits = crate::services::SCOPED_SCRIPT_LIMITS;
    Ok(json!({
        "policyVersion":1,
        "effectPolicy":"core-log-delay-points-http-audio-emit-script-v3",
        "renderPolicy":"public-live-scalars-bounded-v2",
        "publicFields":PUBLIC_FIELDS,
        "globals":false,
        "renderLimits":{
            "urlBytes":8192,"headerValueBytes":8192,"bodyBytes":262144,
            "audioPathBytes":4096,"logBytes":65536,"pointsViewerBytes":256,
            "emitPayloadBytes":65536
        },
        "emitPolicy":"current-all-subjects-exact-normalized-topic-v1",
        "derivedAuthorityPolicy":"private-context-preserved-v1",
        "triggerPolicy":"tiktok-or-approved-scoped-input-v1",
        "customTriggerAdmission":"approved-incoming-context-only-v1",
        "scriptPolicy":"exact-source-public-input-empty-globals-per-intent-v1",
        "scriptHttpFlowPolicy":"all-public-input-fields-v1",
        "scriptHandoffPolicy":"retained-invocations-never-reemit-intents-v1",
        "scriptHttpRenderFields":PUBLIC_FIELDS,
        "scriptLimits":{
            "sourceBytes":script_limits.source_bytes,"resultBytes":script_limits.result_bytes,
            "maxDepth":script_limits.max_depth,"maxNodes":script_limits.max_nodes,
            "totalIntents":script_limits.max_intents,"logBytes":script_limits.max_log_bytes,
            "individualLogBytes":script_limits.max_log_entry_bytes,"logEntries":script_limits.max_log_entries,
            "memoryBytes":script_limits.memory_bytes,"stackBytes":script_limits.stack_bytes,
            "wallClockMs":script_limits.wall_clock_ms,"interrupts":script_limits.max_interrupts,
            "pendingJobs":script_limits.max_pending_jobs,
            "maxConcurrentEvaluations":script_limits.max_concurrent_evaluations
        },
        "configurations":configurations,
        "viewerAdmission":"current-all-subjects-exact-viewer-and-delta-v1"
    }))
}

/// Opaque scripts may turn any projected input into an apparent literal. Keep
/// this complete conservative flow set on every descendant HTTP sink, even
/// after nested emits, so template syntax cannot launder event-derived data.
pub(crate) fn script_http_render_fields() -> Vec<String> {
    PUBLIC_FIELDS
        .iter()
        .map(|field| (*field).to_owned())
        .collect()
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
    use crate::collaboration::{AutomationFamily, Capability, Grant, HttpEffectRequest};
    use crate::services::automation_manifest::{DefinitionBinding, EffectDecision, ManifestEffect};

    fn manifest(kind: EffectKind) -> ExecutionManifest {
        let script = kind == EffectKind::Script;
        ExecutionManifest {
            policy_version: 1,
            digest_algorithm: "sha256-canonical-json-v1",
            host_id: "host".into(),
            subjects: vec![],
            primary: DefinitionBinding {
                kind: AutomationFamily::Action,
                id: "saved".into(),
                revision: "1".into(),
                exists: true,
                content_digest: Some("a".repeat(64)),
                provenance: None,
            },
            dependencies: vec![],
            effects: vec![ManifestEffect {
                family: AutomationFamily::Action,
                id: "saved".into(),
                type_id: Some(
                    if script {
                        "core.code"
                    } else if kind == EffectKind::Emit {
                        "core.emit"
                    } else {
                        "core.log"
                    }
                    .into(),
                ),
                kind,
                enabled: Some(true),
                decision: if script {
                    EffectDecision::ReviewRequired
                } else {
                    EffectDecision::Classified
                },
                render_fields: vec![],
                dynamic_target: script,
                required_capabilities: if script {
                    vec![Capability::ScriptsExecute, Capability::LiveRead]
                } else if kind == EffectKind::Emit {
                    vec![Capability::EventsEmit]
                } else {
                    vec![]
                },
                reasons: if script {
                    vec![ManifestReason::OpaqueScript]
                } else {
                    vec![]
                },
            }],
            reasons: if script {
                vec![
                    ManifestReason::ApprovalRequired,
                    ManifestReason::OpaqueScript,
                ]
            } else {
                vec![ManifestReason::ApprovalRequired]
            },
            execution_authorized: false,
            manifest_digest: "b".repeat(64),
        }
    }

    #[test]
    fn v3_binds_scoped_trigger_script_flow_and_actual_evaluator_limits() {
        let binding = bindings(&manifest(EffectKind::Script), &[]).unwrap();
        assert_eq!(
            binding["effectPolicy"],
            "core-log-delay-points-http-audio-emit-script-v3"
        );
        assert_eq!(
            binding["triggerPolicy"],
            "tiktok-or-approved-scoped-input-v1"
        );
        assert_eq!(
            binding["customTriggerAdmission"],
            "approved-incoming-context-only-v1"
        );
        assert_eq!(
            binding["scriptHttpFlowPolicy"],
            "all-public-input-fields-v1"
        );
        assert_eq!(
            binding["scriptHandoffPolicy"],
            "retained-invocations-never-reemit-intents-v1"
        );
        assert_eq!(binding["scriptHttpRenderFields"], json!(PUBLIC_FIELDS));
        assert_eq!(
            binding["scriptLimits"]["sourceBytes"],
            crate::services::SCOPED_SCRIPT_LIMITS.source_bytes
        );
        assert_eq!(
            binding["scriptLimits"]["resultBytes"],
            crate::services::SCOPED_SCRIPT_LIMITS.result_bytes
        );
        assert_eq!(
            binding["scriptLimits"]["pendingJobs"],
            crate::services::SCOPED_SCRIPT_LIMITS.max_pending_jobs
        );
        assert_eq!(
            binding["scriptLimits"]["maxConcurrentEvaluations"],
            crate::services::SCOPED_SCRIPT_LIMITS.max_concurrent_evaluations
        );
        // Adding an actual engine/admission limit requires its policy pin.
        assert_eq!(
            binding["scriptLimits"].as_object().unwrap().len(),
            serde_json::to_value(crate::services::SCOPED_SCRIPT_LIMITS)
                .unwrap()
                .as_object()
                .unwrap()
                .len()
        );
        assert_eq!(binding["renderLimits"]["emitPayloadBytes"], 65536);
        assert!(!manifest(EffectKind::Script).execution_authorized);
        assert!(bindings(&manifest(EffectKind::Emit), &[]).is_ok());
    }

    #[test]
    fn opaque_source_review_cannot_admit_other_effects_fields_or_missing_capabilities() {
        let mut wrong = manifest(EffectKind::Log);
        wrong.effects[0].reasons.push(ManifestReason::OpaqueScript);
        wrong.reasons.push(ManifestReason::OpaqueScript);
        assert!(bindings(&wrong, &[]).is_err());
        let mut missing = manifest(EffectKind::Script);
        missing.effects[0]
            .required_capabilities
            .retain(|cap| *cap != Capability::ScriptsExecute);
        assert!(bindings(&missing, &[]).is_err());
        let mut private = manifest(EffectKind::Script);
        private.effects[0]
            .render_fields
            .push("event.settings.secret".into());
        assert!(bindings(&private, &[]).is_err());
        let mut global = manifest(EffectKind::Script);
        global
            .reasons
            .push(ManifestReason::GlobalAccessRequiresPolicy);
        assert!(bindings(&global, &[]).is_err());
        let mut plugin = manifest(EffectKind::Script);
        plugin.effects[0].kind = EffectKind::Plugin;
        assert!(bindings(&plugin, &[]).is_err());
        let mut wrong_type = manifest(EffectKind::Script);
        wrong_type.effects[0].type_id = Some("untrusted.script".into());
        assert!(bindings(&wrong_type, &[]).is_err());
        let mut unbound = manifest(EffectKind::Log);
        unbound.reasons.push(ManifestReason::OpaqueScript);
        assert!(bindings(&unbound, &[]).is_err());
    }

    #[test]
    fn script_literal_outputs_retain_all_public_input_fields_for_one_complete_http_rule() {
        let mut grant: Grant = serde_json::from_value(json!({
            "capabilities":["http.request","live.read","scripts.execute"],"resources":{},"settingsFields":{},"actions":{},"maxPointsDelta":null,
            "effects":{"http":[{"origin":"https://hooks.example","methods":["POST"],"paths":["/live"],"queryFields":[],"headerNames":[],"renderFields":[],"bodyFields":["message"],"allowTextBody":false,"maxBodyBytes":1024,"maxResponseBytes":1024,"allowPrivateNetwork":false}],"audio":[],"emitTopics":[]}
        })).unwrap();
        let flow = script_http_render_fields();
        let request = HttpEffectRequest {
            method: "POST",
            url: "https://hooks.example/live",
            header_names: &[],
            render_fields: &flow,
            body: Some(r#"{"message":"apparent literal from opaque script"}"#),
            response_limit: 1024,
            allow_private_network: false,
        };
        assert_eq!(flow.len(), PUBLIC_FIELDS.len());
        assert!(!grant.allows_http_effect(&request));
        let mut other = grant.effects.http[0].clone();
        grant.effects.http[0].render_fields = flow[..9].iter().cloned().collect();
        other.render_fields = flow[9..].iter().cloned().collect();
        grant.effects.http.push(other);
        assert!(!grant.allows_http_effect(&request));
        grant.effects.http[0].render_fields = flow.iter().cloned().collect();
        assert!(grant.allows_http_effect(&request));
        grant.capabilities.remove(&Capability::HttpRequest);
        assert!(!grant.allows_http_effect(&request));
    }
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
