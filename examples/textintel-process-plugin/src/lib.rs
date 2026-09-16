//! Reference pre-filter event processor built on the textintel crate.
//!
//! The library half owns settings, mapping, bounded caches, and the enrich
//! core so the process binary stays thin and the latency bench can drive the
//! same code path the host invokes.

pub mod mapping;
pub mod settings;

use std::collections::{HashMap, VecDeque};

use serde_json::{json, Value};
use tiktools_plugin_sdk::prelude::*;

pub use mapping::INTEL_SCHEMA_VERSION;
pub use settings::{EmojiMode, EngineMode, TextIntelSettings};

/// FIFO-bounded analysis cache. Keys already cover the input text plus the
/// settings digest, so eviction order needs no recency tracking.
pub struct BoundedCache {
    map: HashMap<String, Value>,
    order: VecDeque<String>,
    cap: usize,
}

impl BoundedCache {
    pub fn new(cap: usize) -> Self {
        Self {
            map: HashMap::new(),
            order: VecDeque::new(),
            cap: cap.max(1),
        }
    }

    pub fn get(&self, key: &str) -> Option<Value> {
        self.map.get(key).cloned()
    }

    pub fn insert(&mut self, key: String, value: Value) {
        // Refreshing an existing key must not duplicate its order entry.
        if let Some(slot) = self.map.get_mut(&key) {
            *slot = value;
            return;
        }
        while self.map.len() >= self.cap {
            if let Some(oldest) = self.order.pop_front() {
                self.map.remove(&oldest);
            } else {
                break;
            }
        }
        self.order.push_back(key.clone());
        self.map.insert(key, value);
    }

    pub fn clear(&mut self) {
        self.map.clear();
        self.order.clear();
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

/// Builds the textintel engine for one mode. `production-local-lite` and
/// `production-local` share the graceful local preset in this build: without
/// the heavy transformer/espeak/ANN features compiled in, unavailable pieces
/// degrade and are reported by diagnostics instead of failing.
pub fn build_engine(mode: EngineMode) -> PluginResult<textintel::TextIntelligence> {
    match mode {
        EngineMode::Default => Ok(textintel::TextIntelligence::default()),
        EngineMode::ProductionLocalLite | EngineMode::ProductionLocal => {
            textintel::TextIntelligence::production_local().map_err(|error| {
                PluginError::other(format!("could not build the textintel engine: {error}"))
            })
        }
    }
}

/// Analyzes one event's texts and returns the stable annotations plus text
/// views. Pure over its inputs except for the bounded caches, which the
/// caller clears whenever the settings digest changes.
#[allow(clippy::too_many_arguments)]
pub fn annotate_event(
    engine: &textintel::TextIntelligence,
    settings: &TextIntelSettings,
    comment: &str,
    nickname: &str,
    unique_id: &str,
    comment_cache: &mut BoundedCache,
    name_cache: &mut BoundedCache,
    logs: &mut Vec<String>,
) -> PluginResult<(
    serde_json::Map<String, Value>,
    std::collections::BTreeMap<String, TextView>,
)> {
    let mut annotations = serde_json::Map::new();
    let mut views = std::collections::BTreeMap::new();

    if settings.analyze_comments && !comment.trim().is_empty() {
        let (input, truncated) = mapping::truncate_chars(comment, settings.max_input_chars);
        if truncated {
            logs.push(format!(
                "comment truncated to {} characters",
                settings.max_input_chars
            ));
        }
        let annotation = match comment_cache.get(&input) {
            Some(cached) => cached,
            None => {
                let fingerprint = engine.analyze(&input).map_err(|error| {
                    PluginError::other(format!("comment analysis failed: {error}"))
                })?;
                let annotation =
                    mapping::comment_annotation(&fingerprint, &input, truncated, settings);
                comment_cache.insert(input.clone(), annotation.clone());
                if truncated {
                    // Keep the raw text out of the provider payload; the
                    // annotation already carries the truncated views.
                }
                let _ = &fingerprint;
                insert_comment_view(&mut views, &annotation);
                annotation
            }
        };
        if !views.contains_key("comment") {
            insert_comment_view(&mut views, &annotation);
        }
        annotations.insert("comment".to_owned(), annotation);
    }

    let mut user = serde_json::Map::new();
    if settings.analyze_nicknames && !nickname.trim().is_empty() {
        let (input, _) = mapping::truncate_chars(nickname, settings.max_input_chars);
        let key = format!("n:{input}");
        let annotation = match name_cache.get(&key) {
            Some(cached) => cached,
            None => {
                let fingerprint = engine.analyze(&input).map_err(|error| {
                    PluginError::other(format!("nickname analysis failed: {error}"))
                })?;
                let annotation = mapping::nickname_annotation(&fingerprint, &input, settings);
                name_cache.insert(key, annotation.clone());
                insert_nickname_view(&mut views, &annotation);
                annotation
            }
        };
        if !views.contains_key("nickname") {
            insert_nickname_view(&mut views, &annotation);
        }
        user.insert("nickname".to_owned(), annotation);
    }
    if settings.analyze_usernames && !unique_id.trim().is_empty() {
        let (input, _) = mapping::truncate_chars(unique_id, settings.max_input_chars);
        let key = format!("u:{input}");
        let annotation = match name_cache.get(&key) {
            Some(cached) => cached,
            None => {
                let fingerprint = engine.analyze(&input).map_err(|error| {
                    PluginError::other(format!("handle analysis failed: {error}"))
                })?;
                let annotation = mapping::handle_annotation(&fingerprint, settings);
                name_cache.insert(key, annotation.clone());
                annotation
            }
        };
        user.insert("uniqueId".to_owned(), annotation);
    }
    if !user.is_empty() {
        annotations.insert("user".to_owned(), Value::Object(user));
    }

    annotations.insert(
        "provider".to_owned(),
        provider_payload(engine, settings, &annotations),
    );
    Ok((annotations, views))
}

fn insert_comment_view(
    views: &mut std::collections::BTreeMap<String, TextView>,
    annotation: &Value,
) {
    let Some(tts) = annotation.get("tts") else {
        return;
    };
    let Some(text) = tts.get("text").and_then(Value::as_str) else {
        return;
    };
    if text.is_empty() || tts.get("speak").and_then(Value::as_bool) == Some(false) {
        return;
    }
    let mut view = TextView::new(
        text,
        tts.get("source")
            .and_then(Value::as_str)
            .unwrap_or("comment"),
    );
    if let Some(language) = tts.get("language").and_then(Value::as_str) {
        view.language = Some(language.to_owned());
        view.confidence = tts.get("confidence").and_then(Value::as_f64);
    }
    views.insert("comment".to_owned(), view);
}

fn insert_nickname_view(
    views: &mut std::collections::BTreeMap<String, TextView>,
    annotation: &Value,
) {
    let Some(tts) = annotation.get("tts") else {
        return;
    };
    let Some(text) = tts.get("text").and_then(Value::as_str) else {
        return;
    };
    if text.is_empty() {
        return;
    }
    let mut view = TextView::new(
        text,
        tts.get("source")
            .and_then(Value::as_str)
            .unwrap_or("nickname"),
    );
    if let Some(language) = tts.get("language").and_then(Value::as_str) {
        view.language = Some(language.to_owned());
        view.confidence = tts.get("confidence").and_then(Value::as_f64);
    }
    if let Some(ipa) = tts.get("ipa").and_then(Value::as_str) {
        view.ipa = Some(ipa.to_owned());
    }
    views.insert("nickname".to_owned(), view);
}

/// Compact provider-native payload derived from the already-built stable
/// annotations (no duplicate analysis).
fn provider_payload(
    engine: &textintel::TextIntelligence,
    settings: &TextIntelSettings,
    annotations: &serde_json::Map<String, Value>,
) -> Value {
    let diagnostics = engine.diagnostics();
    let comment = annotations.get("comment");
    let nickname = annotations
        .get("user")
        .and_then(|user| user.get("nickname"));
    json!({
        "apiVersion": diagnostics.api_version,
        "schemaVersion": INTEL_SCHEMA_VERSION,
        "engineMode": settings.engine_mode.as_str(),
        "comment": {
            "topLanguage": comment.and_then(|comment| comment.pointer("/language/top")).cloned().unwrap_or(Value::Null),
            "languageConfidence": comment.and_then(|comment| comment.pointer("/language/confidence")).cloned().unwrap_or(Value::Null),
            "normalized": comment.and_then(|comment| comment.get("normalized")).cloned().unwrap_or(Value::Null),
            "obfuscationScore": comment.and_then(|comment| comment.pointer("/obfuscation/score")).cloned().unwrap_or(Value::Null),
            "repetition": comment.and_then(|comment| comment.pointer("/obfuscation/repetition")).cloned().unwrap_or(Value::Null),
            "rebusCandidate": comment.and_then(|comment| comment.get("rebus")).cloned().unwrap_or(Value::Null),
        },
        "nickname": {
            "topLanguage": nickname.and_then(|nickname| nickname.pointer("/language/top")).cloned().unwrap_or(Value::Null),
            "spokenCandidate": nickname.and_then(|nickname| nickname.pointer("/tts/text")).cloned().unwrap_or(Value::Null),
            "ipa": nickname.and_then(|nickname| nickname.pointer("/tts/ipa")).cloned().unwrap_or(Value::Null),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_cache_evicts_oldest_first() {
        let mut cache = BoundedCache::new(2);
        cache.insert("a".to_owned(), Value::from(1));
        cache.insert("b".to_owned(), Value::from(2));
        cache.insert("c".to_owned(), Value::from(3));
        assert!(cache.get("a").is_none());
        assert_eq!(cache.get("b"), Some(Value::from(2)));
        assert_eq!(cache.len(), 2);
        cache.clear();
        assert!(cache.is_empty());
    }

    fn engine() -> textintel::TextIntelligence {
        textintel::TextIntelligence::default()
    }

    fn harness() -> (TextIntelSettings, BoundedCache, BoundedCache, Vec<String>) {
        (
            TextIntelSettings::default(),
            BoundedCache::new(64),
            BoundedCache::new(256),
            Vec::new(),
        )
    }

    #[test]
    fn comment_annotation_keeps_raw_and_maps_channels() {
        let engine = engine();
        let (settings, mut comments, mut names, mut logs) = harness();
        let (annotations, views) = annotate_event(
            &engine,
            &settings,
            "HOOOLAAA",
            "J0sé",
            "j0se_92",
            &mut comments,
            &mut names,
            &mut logs,
        )
        .unwrap();
        let comment = &annotations["comment"];
        assert_eq!(comment["composition"]["allCaps"], true);
        assert_eq!(comment["composition"]["elongated"], true);
        assert!(comment.get("language").is_some());
        assert!(comment.get("obfuscation").is_some());
        assert!(comment.get("spam").is_some());
        assert!(comment.get("tts").is_some());
        assert_eq!(comment["tts"]["speak"], true);
        assert!(!comment
            .get("normalized")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .is_empty());
        assert!(views.contains_key("comment"));

        let nickname = annotations
            .get("user")
            .and_then(|user| user.get("nickname"))
            .expect("nickname annotation");
        assert!(nickname.get("tts").is_some());
        // Handles stay unanalyzed by default.
        assert!(annotations
            .get("user")
            .and_then(|user| user.get("uniqueId"))
            .is_none());
    }

    #[test]
    fn deterministic_inputs_never_panic_and_stay_bounded() {
        let engine = engine();
        let (settings, mut comments, mut names, mut logs) = harness();
        for comment in [
            "Fra🏠do",
            "salU2",
            "c0mpr4 ah0r4",
            "😂😂😂",
            "",
            "   ",
            "a",
            &"x".repeat(2_000),
            "héllo wörld \u{200b}\u{202a}bidirectional",
            "日本語のテスト",
            "مرحبا بالعالم",
        ] {
            let (annotations, _) = annotate_event(
                &engine,
                &settings,
                comment,
                "Viewer",
                "viewer",
                &mut comments,
                &mut names,
                &mut logs,
            )
            .unwrap();
            if comment.trim().is_empty() {
                assert!(annotations.get("comment").is_none());
            } else {
                let payload = serde_json::to_vec(&annotations["comment"]).unwrap();
                assert!(payload.len() <= 32 * 1024, "{comment}");
                assert!(annotations["comment"].get("composition").is_some());
            }
        }
    }

    #[test]
    fn tts_falls_back_to_raw_on_weak_candidates() {
        let engine = engine();
        let settings = TextIntelSettings {
            minimum_tts_confidence: 1.0,
            ..TextIntelSettings::default()
        };
        let (_, mut comments, mut names, mut logs) = harness();
        let (annotations, _) = annotate_event(
            &engine,
            &settings,
            "zxqv kwyj",
            "zxqv",
            "zxqv",
            &mut comments,
            &mut names,
            &mut logs,
        )
        .unwrap();
        let tts = &annotations["comment"]["tts"];
        assert_eq!(tts["text"], "zxqv kwyj");
        assert_eq!(tts["source"], "raw");
    }

    #[test]
    fn emoji_skip_policy_marks_speech_without_dropping_analysis() {
        let engine = engine();
        let settings = TextIntelSettings {
            emoji_mode: EmojiMode::SkipMessageIfEmojiOnly,
            ..TextIntelSettings::default()
        };
        let (_, mut comments, mut names, mut logs) = harness();
        let (annotations, _) = annotate_event(
            &engine,
            &settings,
            "😂😂😂",
            "Viewer",
            "viewer",
            &mut comments,
            &mut names,
            &mut logs,
        )
        .unwrap();
        assert_eq!(annotations["comment"]["composition"]["emojiOnly"], true);
        assert_eq!(annotations["comment"]["tts"]["speak"], false);
        assert_eq!(annotations["comment"]["tts"]["reason"], "emoji-only");
    }

    #[test]
    fn disabled_channels_are_omitted() {
        let engine = engine();
        let settings = TextIntelSettings {
            analyze_comments: false,
            analyze_nicknames: false,
            rebus: false,
            spam: false,
            obfuscation: false,
            tts_candidate: false,
            ..TextIntelSettings::default()
        };
        let (_, mut comments, mut names, mut logs) = harness();
        let (annotations, views) = annotate_event(
            &engine,
            &settings,
            "hello",
            "Viewer",
            "viewer",
            &mut comments,
            &mut names,
            &mut logs,
        )
        .unwrap();
        assert!(annotations.get("comment").is_none());
        assert!(annotations.get("user").is_none());
        assert!(views.is_empty());
        // The provider stamp is always present for provenance.
        assert!(annotations.get("provider").is_some());
    }
}
