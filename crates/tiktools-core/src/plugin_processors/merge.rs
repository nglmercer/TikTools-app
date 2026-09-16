//! Stable projection: merging processor outcomes into `event.intel`.
//!
//! Raw event fields are never touched. Every provider's output stays
//! independent under `intel.providers.<pluginId>`; the validated stable keys
//! resolve by first-writer-wins in selection order.

use super::ProcessorOutcome;
use serde_json::Value;

/// Reserved key under `providers.<pluginId>` holding text views. An
/// annotation using this name is skipped during the provider merge.
const PROVIDER_VIEWS_KEY: &str = "views";
/// View names the host projects into stable TTS fields.
const COMMENT_VIEW: &str = "comment";
const NICKNAME_VIEW: &str = "nickname";

/// Merges processor outcomes into the event's reserved `intel` namespace.
/// Stable contributions promote in canonical contract form (unknown keys stay
/// provider-namespaced). Any failure stamps
/// `intel.processing.status = "degraded"`.
pub(crate) fn merge_processor_outcomes(mut event: Value, outcomes: &[ProcessorOutcome]) -> Value {
    let mut any_failure = false;
    let mut owned = StableOwnership::default();
    for (_, plugin_id, processor_id, outcome) in outcomes {
        match outcome {
            Ok(enrichment) => {
                merge_provider_result(&mut event, plugin_id, &enrichment.result);
                if promote_stable_fields(
                    &mut event,
                    plugin_id,
                    processor_id,
                    &enrichment.result,
                    &mut owned,
                ) {
                    any_failure = true;
                }
            }
            Err(_) => {
                any_failure = true;
            }
        }
    }
    if any_failure {
        let intel = ensure_child_object(&mut event, "intel");
        let processing = ensure_child_object(intel, "processing");
        if let Some(object) = processing.as_object_mut() {
            object.insert("status".to_owned(), Value::String("degraded".to_owned()));
        }
    }
    event
}

#[derive(Default)]
struct StableOwnership {
    comment: bool,
    user: bool,
    comment_tts: bool,
    nickname_tts: bool,
}

fn merge_provider_result(
    event: &mut Value,
    plugin_id: &str,
    result: &tiktools_plugin_sdk::EventEnrichmentResult,
) {
    let intel = ensure_child_object(event, "intel");
    let providers = ensure_child_object(intel, "providers");
    let provider = ensure_child_object(providers, plugin_id);
    for (key, value) in &result.annotations {
        if key == PROVIDER_VIEWS_KEY {
            continue;
        }
        let slot = ensure_child_value(provider, key);
        deep_merge(slot, value);
    }
    if !result.views.is_empty() {
        let views = ensure_child_object(provider, PROVIDER_VIEWS_KEY);
        for (name, view) in &result.views {
            let slot = ensure_child_value(views, name);
            *slot = serde_json::to_value(view).unwrap_or(Value::Null);
        }
    }
}

/// Promotes one result's stable contributions. Returns true when a stable
/// contribution failed validation (the provider copy is kept; the stable
/// field stays unowned, and the event is marked degraded).
fn promote_stable_fields(
    event: &mut Value,
    plugin_id: &str,
    processor_id: &str,
    result: &tiktools_plugin_sdk::EventEnrichmentResult,
    owned: &mut StableOwnership,
) -> bool {
    use tiktools_plugin_api::intel::{canonical_stable_comment, canonical_stable_user};

    let mut invalid = false;
    if !owned.comment {
        if let Some(raw) = result.annotations.get("comment") {
            match canonical_stable_comment(raw) {
                Ok(mut canonical) => {
                    // Views are canonical for spoken text: an annotation TTS
                    // object never promotes into the stable namespace.
                    if let Some(object) = canonical.as_object_mut() {
                        object.remove("tts");
                    }
                    let intel = ensure_child_object(event, "intel");
                    *ensure_child_value(intel, "comment") = canonical;
                    owned.comment = true;
                }
                Err(error) => {
                    tracing::debug!(
                        plugin = %plugin_id,
                        processor = %processor_id,
                        %error,
                        "stable comment contribution failed validation"
                    );
                    invalid = true;
                }
            }
        }
    }
    if !owned.user {
        if let Some(raw) = result.annotations.get("user") {
            match canonical_stable_user(raw) {
                Ok(mut canonical) => {
                    if let Some(nickname) =
                        canonical.get_mut("nickname").and_then(Value::as_object_mut)
                    {
                        nickname.remove("tts");
                    }
                    let intel = ensure_child_object(event, "intel");
                    *ensure_child_value(intel, "user") = canonical;
                    owned.user = true;
                }
                Err(error) => {
                    tracing::debug!(
                        plugin = %plugin_id,
                        processor = %processor_id,
                        %error,
                        "stable user contribution failed validation"
                    );
                    invalid = true;
                }
            }
        }
    }
    if !owned.comment_tts {
        if let Some(view) = result.views.get(COMMENT_VIEW) {
            match project_view_tts(view) {
                Some(tts) => {
                    let intel = ensure_child_object(event, "intel");
                    let comment = ensure_child_object(intel, "comment");
                    *ensure_child_value(comment, "tts") = tts;
                    owned.comment_tts = true;
                }
                None => {
                    tracing::debug!(
                        plugin = %plugin_id,
                        processor = %processor_id,
                        "comment view failed TTS projection"
                    );
                    invalid = true;
                }
            }
        }
    }
    if !owned.nickname_tts {
        if let Some(view) = result.views.get(NICKNAME_VIEW) {
            match project_view_tts(view) {
                Some(tts) => {
                    let intel = ensure_child_object(event, "intel");
                    let user = ensure_child_object(intel, "user");
                    let nickname = ensure_child_object(user, "nickname");
                    *ensure_child_value(nickname, "tts") = tts;
                    owned.nickname_tts = true;
                }
                None => {
                    tracing::debug!(
                        plugin = %plugin_id,
                        processor = %processor_id,
                        "nickname view failed TTS projection"
                    );
                    invalid = true;
                }
            }
        }
    }
    invalid
}

fn project_view_tts(view: &tiktools_plugin_sdk::TextView) -> Option<Value> {
    use tiktools_plugin_api::intel::{IntelPronunciation, IntelTts};

    // Skip-policy views carry empty text with speak:false; anything else
    // needs speakable text.
    if view.text.trim().is_empty() && view.speak {
        return None;
    }
    let tts = IntelTts {
        text: view.text.clone(),
        language: view.language.clone(),
        confidence: view.confidence,
        source: Some(view.source.clone()),
        speak: view.speak,
        reason: view.reason.clone(),
        pronunciation: view
            .pronunciation
            .as_ref()
            .map(|pronunciation| IntelPronunciation {
                ipa: pronunciation.ipa.clone(),
                language: pronunciation.language.clone(),
                dialect: pronunciation.dialect.clone(),
                confidence: pronunciation.confidence,
            }),
    };
    tts.validate("tts").ok()?;
    serde_json::to_value(tts).ok()
}

/// Returns the named child as an object, replacing missing or mistyped
/// values. The `intel` namespace is host-reserved, so replacement there is
/// always safe.
fn ensure_child_object<'a>(parent: &'a mut Value, key: &str) -> &'a mut Value {
    if !parent.is_object() {
        *parent = Value::Object(Default::default());
    }
    let object = parent.as_object_mut().expect("parent must be an object");
    if !object.get(key).is_some_and(Value::is_object) {
        object.insert(key.to_owned(), Value::Object(Default::default()));
    }
    object.get_mut(key).expect("child object must exist")
}

fn ensure_child_value<'a>(parent: &'a mut Value, key: &str) -> &'a mut Value {
    if !parent.is_object() {
        *parent = Value::Object(Default::default());
    }
    let object = parent.as_object_mut().expect("parent must be an object");
    object.entry(key.to_owned()).or_insert(Value::Null)
}

fn deep_merge(target: &mut Value, source: &Value) {
    match (target.as_object_mut(), source.as_object()) {
        (Some(target), Some(source)) => {
            for (key, value) in source {
                match target.get_mut(key) {
                    Some(slot) => deep_merge(slot, value),
                    None => {
                        target.insert(key.clone(), value.clone());
                    }
                }
            }
        }
        _ => {
            *target = source.clone();
        }
    }
}
