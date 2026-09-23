//! Engine orchestration for the TextIntel processor.
//!
//! [`TextIntelProcessor`] owns the textintel engine plus the bounded analysis
//! caches. It decides *what* gets analyzed and *when* caches invalidate;
//! [`crate::mapping`] owns rendering fingerprints into annotations and views.

use std::collections::BTreeMap;

use serde_json::{json, Map, Value};
use tiktools_plugin_sdk::{PluginError, PluginResult, TextView};

use crate::{
    cache::BoundedCache,
    mapping::{self, INTEL_SCHEMA_VERSION},
    settings::{EngineMode, TextIntelSettings},
};

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

/// Cache key covering the FULL input text. Analysis runs on a truncated
/// prefix, but keying by that prefix would collide long messages that share
/// one: the length plus a hash of the complete text keeps every distinct
/// input distinct.
fn cache_key(prefix: &str, full_text: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    full_text.hash(&mut hasher);
    format!("{prefix}:{}:{:016x}", full_text.len(), hasher.finish())
}

/// Stateful TextIntel processor: lazy engine plus generation-scoped caches.
/// The settings digest is the cache generation — any analysis-affecting
/// setting change clears both caches before the next event.
pub struct TextIntelProcessor {
    engine: Option<(EngineMode, textintel::TextIntelligence)>,
    settings_digest: Option<String>,
    comment_cache: BoundedCache,
    name_cache: BoundedCache,
}

impl Default for TextIntelProcessor {
    fn default() -> Self {
        Self {
            engine: None,
            settings_digest: None,
            comment_cache: BoundedCache::new(64),
            name_cache: BoundedCache::new(256),
        }
    }
}

impl TextIntelProcessor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Drops all cached analyses without touching the engine. Diagnostics and
    /// benchmarks use this to measure cold analysis; production code relies
    /// on the settings-digest generation instead.
    pub fn clear_caches(&mut self) {
        self.comment_cache.clear();
        self.name_cache.clear();
    }

    /// Builds the engine lazily on first use and rebuilds only when the
    /// engine mode changes. Settings otherwise arrive per request because
    /// the host delivers them inside each enrich call.
    fn ensure_engine(&mut self, mode: EngineMode) -> PluginResult<()> {
        let rebuild = self
            .engine
            .as_ref()
            .is_none_or(|(current, _)| *current != mode);
        if rebuild {
            self.engine = Some((mode, build_engine(mode)?));
        }
        Ok(())
    }

    /// Analyzes one event's texts and returns the stable annotations plus
    /// text views. Pure over its inputs except for the bounded caches, which
    /// invalidate whenever the settings digest changes.
    pub fn enrich_texts(
        &mut self,
        settings_value: &Value,
        comment: &str,
        nickname: &str,
        unique_id: &str,
        logs: &mut Vec<String>,
    ) -> PluginResult<(Map<String, Value>, BTreeMap<String, TextView>)> {
        let settings = TextIntelSettings::from_value(settings_value);
        let digest = settings.digest();
        if self.settings_digest.as_deref() != Some(digest.as_str()) {
            self.comment_cache.clear();
            self.name_cache.clear();
            self.settings_digest = Some(digest);
        }
        self.ensure_engine(settings.engine_mode)?;
        // Disjoint field borrows: the engine is immutable from here on.
        let engine = &self.engine.as_ref().expect("engine was just built").1;
        let mut annotations = Map::new();
        let mut views = BTreeMap::new();

        if settings.analyze_comments && !comment.trim().is_empty() {
            let (input, truncated) = mapping::truncate_chars(comment, settings.max_input_chars);
            if truncated {
                logs.push(format!(
                    "comment truncated to {} characters",
                    settings.max_input_chars
                ));
            }
            let key = cache_key("c", comment);
            let annotation = match self.comment_cache.get(&key) {
                Some(cached) => cached,
                None => {
                    let fingerprint = engine.analyze(&input).map_err(|error| {
                        PluginError::other(format!("comment analysis failed: {error}"))
                    })?;
                    let annotation =
                        mapping::comment_annotation(&fingerprint, &input, truncated, &settings);
                    self.comment_cache.insert(key, annotation.clone());
                    annotation
                }
            };
            if let Some(view) = mapping::comment_view(&annotation) {
                views.insert("comment".to_owned(), view);
            }
            annotations.insert("comment".to_owned(), annotation);
        }

        let mut user = Map::new();
        if settings.analyze_nicknames && !nickname.trim().is_empty() {
            let (input, _) = mapping::truncate_chars(nickname, settings.max_input_chars);
            let key = cache_key("n", nickname);
            let annotation = match self.name_cache.get(&key) {
                Some(cached) => cached,
                None => {
                    let fingerprint = engine.analyze(&input).map_err(|error| {
                        PluginError::other(format!("nickname analysis failed: {error}"))
                    })?;
                    let annotation = mapping::nickname_annotation(&fingerprint, &input, &settings);
                    self.name_cache.insert(key, annotation.clone());
                    annotation
                }
            };
            if let Some(view) = mapping::nickname_view(&annotation) {
                views.insert("nickname".to_owned(), view);
            }
            user.insert("nickname".to_owned(), annotation);
        }
        if settings.analyze_usernames && !unique_id.trim().is_empty() {
            let (input, _) = mapping::truncate_chars(unique_id, settings.max_input_chars);
            let key = cache_key("u", unique_id);
            let annotation = match self.name_cache.get(&key) {
                Some(cached) => cached,
                None => {
                    let fingerprint = engine.analyze(&input).map_err(|error| {
                        PluginError::other(format!("handle analysis failed: {error}"))
                    })?;
                    let annotation = mapping::handle_annotation(&fingerprint, &settings);
                    self.name_cache.insert(key, annotation.clone());
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
            provider_payload(engine, &settings, &annotations),
        );
        Ok((annotations, views))
    }
}

/// Compact provider-native payload derived from the already-built stable
/// annotations (no duplicate analysis).
fn provider_payload(
    engine: &textintel::TextIntelligence,
    settings: &TextIntelSettings,
    annotations: &Map<String, Value>,
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
            "ipa": nickname.and_then(|nickname| nickname.pointer("/tts/phonetic/ipa")).cloned().unwrap_or(Value::Null),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn processor() -> TextIntelProcessor {
        TextIntelProcessor::new()
    }

    fn settings_json() -> Value {
        Value::Null
    }

    #[test]
    fn comment_annotation_keeps_raw_and_maps_channels() {
        let mut processor = processor();
        let mut logs = Vec::new();
        let (annotations, views) = processor
            .enrich_texts(&settings_json(), "HOOOLAAA", "J0sé", "j0se_92", &mut logs)
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
        let mut processor = processor();
        let mut logs = Vec::new();
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
            let (annotations, _) = processor
                .enrich_texts(&settings_json(), comment, "Viewer", "viewer", &mut logs)
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
        let mut processor = processor();
        let mut logs = Vec::new();
        let (annotations, _) = processor
            .enrich_texts(
                &json!({"minimumTtsConfidence": 1.0}),
                "zxqv kwyj",
                "zxqv",
                "zxqv",
                &mut logs,
            )
            .unwrap();
        let tts = &annotations["comment"]["tts"];
        assert_eq!(tts["text"], "zxqv kwyj");
        assert_eq!(tts["source"], "raw");
    }

    #[test]
    fn emoji_skip_policy_marks_speech_without_dropping_analysis() {
        let mut processor = processor();
        let mut logs = Vec::new();
        let (annotations, views) = processor
            .enrich_texts(
                &json!({"emojiMode": "skip-message-if-emoji-only"}),
                "😂😂😂",
                "Viewer",
                "viewer",
                &mut logs,
            )
            .unwrap();
        assert_eq!(annotations["comment"]["composition"]["emojiOnly"], true);
        assert_eq!(annotations["comment"]["tts"]["speak"], false);
        assert_eq!(annotations["comment"]["tts"]["reason"], "emoji-only");
        // The policy travels in the canonical view: dropping it would let TTS
        // fall back to the raw emoji message.
        let view = views.get("comment").expect("skip-policy view");
        assert!(!view.speak);
        assert_eq!(view.reason.as_deref(), Some("emoji-only"));
        assert_eq!(view.source, "policy");
    }

    #[test]
    fn disabled_channels_are_omitted() {
        let mut processor = processor();
        let mut logs = Vec::new();
        let (annotations, views) = processor
            .enrich_texts(
                &json!({
                    "analyzeComments": false,
                    "analyzeNicknames": false,
                    "rebus": false,
                    "spam": false,
                    "obfuscation": false,
                    "ttsCandidate": false,
                }),
                "hello",
                "Viewer",
                "viewer",
                &mut logs,
            )
            .unwrap();
        assert!(annotations.get("comment").is_none());
        assert!(annotations.get("user").is_none());
        assert!(views.is_empty());
        // The provider stamp is always present for provenance.
        assert!(annotations.get("provider").is_some());
    }

    #[test]
    fn long_messages_sharing_a_prefix_do_not_collide() {
        let mut processor = processor();
        let mut logs = Vec::new();
        let prefix = "a".repeat(600);
        let first = format!("{prefix} one");
        let second = format!("{prefix} two");
        let (annotations, _) = processor
            .enrich_texts(&settings_json(), &first, "Viewer", "viewer", &mut logs)
            .unwrap();
        assert_eq!(annotations["comment"]["truncated"], true);
        let (annotations, _) = processor
            .enrich_texts(&settings_json(), &second, "Viewer", "viewer", &mut logs)
            .unwrap();
        assert_eq!(annotations["comment"]["truncated"], true);
        // Both truncate to the same 500-char prefix for analysis, but the
        // cache keys cover the full text, so the two inputs occupy two
        // distinct entries instead of colliding.
        assert_eq!(processor.comment_cache.len(), 2);
    }

    #[test]
    fn settings_change_invalidates_caches() {
        let mut processor = processor();
        let mut logs = Vec::new();
        processor
            .enrich_texts(&settings_json(), "hello", "Viewer", "viewer", &mut logs)
            .unwrap();
        assert_eq!(processor.comment_cache.len(), 1);
        // Same text, different analysis settings: the stale entry is dropped.
        processor
            .enrich_texts(
                &json!({"emojiMode": "remove"}),
                "hello",
                "Viewer",
                "viewer",
                &mut logs,
            )
            .unwrap();
        assert_eq!(processor.comment_cache.len(), 1);
    }

    const SPAMMY: &str =
        "BUY NOW!!! cheap followers!!! visit https://spam.example.com FREE MONEY $$$";

    #[test]
    fn spam_filtering_uses_configured_threshold() {
        let mut processor = processor();
        let mut logs = Vec::new();
        // Disabled filter never blocks, however high the score.
        let (annotations, _) = processor
            .enrich_texts(
                &json!({"filterSpam": false}),
                SPAMMY,
                "Viewer",
                "viewer",
                &mut logs,
            )
            .unwrap();
        let score = annotations["comment"]["spam"]["score"]
            .as_f64()
            .expect("spam score");
        assert!(score > 0.0, "fixture must look spammy, got {score}");
        assert_eq!(annotations["comment"]["moderation"]["blocked"], false);
        assert_eq!(annotations["comment"]["moderation"]["spam"], false);
        assert_eq!(
            annotations["comment"]["moderation"]["spamScore"]
                .as_f64()
                .unwrap(),
            score
        );

        // Enabled with a zero threshold blocks on any evidence.
        let (annotations, _) = processor
            .enrich_texts(
                &json!({"filterSpam": true, "spamThreshold": 0.0}),
                SPAMMY,
                "Viewer",
                "viewer",
                &mut logs,
            )
            .unwrap();
        assert_eq!(annotations["comment"]["moderation"]["blocked"], true);
        assert_eq!(annotations["comment"]["moderation"]["spam"], true);
        assert_eq!(
            annotations["comment"]["moderation"]["reasons"],
            json!(["spam"])
        );

        // The decision straddles the engine's own score: exact threshold
        // blocks (`>=`), anything above it does not.
        let (annotations, _) = processor
            .enrich_texts(
                &json!({"filterSpam": true, "spamThreshold": score}),
                SPAMMY,
                "Viewer",
                "viewer",
                &mut logs,
            )
            .unwrap();
        assert_eq!(annotations["comment"]["moderation"]["blocked"], true);
        let above = (score + 0.2).min(1.0);
        assert!(above > score, "fixture score {score} leaves no room above");
        let (annotations, _) = processor
            .enrich_texts(
                &json!({"filterSpam": true, "spamThreshold": above}),
                SPAMMY,
                "Viewer",
                "viewer",
                &mut logs,
            )
            .unwrap();
        assert_eq!(annotations["comment"]["moderation"]["blocked"], false);

        // Benign chat stays unblocked at the default threshold.
        let (annotations, _) = processor
            .enrich_texts(
                &json!({"filterSpam": true}),
                "hello everyone",
                "Viewer",
                "viewer",
                &mut logs,
            )
            .unwrap();
        assert_eq!(annotations["comment"]["moderation"]["blocked"], false);

        // The stable spam evidence honors the same threshold.
        let (annotations, _) = processor
            .enrich_texts(
                &json!({"filterSpam": true, "spamThreshold": 0.0}),
                SPAMMY,
                "Viewer",
                "viewer",
                &mut logs,
            )
            .unwrap();
        assert_eq!(annotations["comment"]["spam"]["detected"], true);
    }

    #[test]
    fn bad_word_filtering_blocks_matches_only_when_enabled() {
        let mut processor = processor();
        let mut logs = Vec::new();
        // Matches are visible as evidence, but never block while disabled.
        let (annotations, _) = processor
            .enrich_texts(
                &json!({"filterBadWords": false, "badWords": ["scam"]}),
                "this is a scam",
                "Viewer",
                "viewer",
                &mut logs,
            )
            .unwrap();
        assert_eq!(annotations["comment"]["moderation"]["blocked"], false);
        assert_eq!(annotations["comment"]["moderation"]["badWords"], false);
        assert_eq!(
            annotations["comment"]["moderation"]["matches"],
            json!([{"term": "scam", "view": "raw"}])
        );

        let (annotations, _) = processor
            .enrich_texts(
                &json!({"filterBadWords": true, "badWords": ["scam"]}),
                "this is a scam",
                "Viewer",
                "viewer",
                &mut logs,
            )
            .unwrap();
        assert_eq!(annotations["comment"]["moderation"]["blocked"], true);
        assert_eq!(annotations["comment"]["moderation"]["badWords"], true);
        assert_eq!(
            annotations["comment"]["moderation"]["reasons"],
            json!(["bad-word"])
        );

        // An empty list blocks nothing even with the filter enabled.
        let (annotations, _) = processor
            .enrich_texts(
                &json!({"filterBadWords": true, "badWords": []}),
                "this is a scam",
                "Viewer",
                "viewer",
                &mut logs,
            )
            .unwrap();
        assert_eq!(annotations["comment"]["moderation"]["blocked"], false);
        assert_eq!(annotations["comment"]["moderation"]["matches"], json!([]));
    }

    #[test]
    fn bad_words_match_through_obfuscation_views() {
        let mut processor = processor();
        let mut logs = Vec::new();
        let settings = json!({"filterBadWords": true, "badWords": ["badword", "scam"]});
        for (message, term, view) in [
            ("that badword is here", "badword", "raw"),
            ("b4dw0rd", "badword", "leet"),
            ("baaaadword", "badword", "repetition_collapsed"),
            ("bаdword", "badword", "skeleton"),
            ("this is a scam", "scam", "raw"),
        ] {
            let (annotations, _) = processor
                .enrich_texts(&settings, message, "Viewer", "viewer", &mut logs)
                .unwrap();
            assert_eq!(
                annotations["comment"]["moderation"]["blocked"], true,
                "{message:?}"
            );
            assert_eq!(
                annotations["comment"]["moderation"]["matches"],
                json!([{"term": term, "view": view}]),
                "{message:?}"
            );
        }
        // Substrings of innocent words never block.
        let (annotations, _) = processor
            .enrich_texts(
                &json!({"filterBadWords": true, "badWords": ["ass"]}),
                "class dismissed",
                "Viewer",
                "viewer",
                &mut logs,
            )
            .unwrap();
        assert_eq!(annotations["comment"]["moderation"]["blocked"], false);
    }

    #[test]
    fn blocked_messages_emit_non_speakable_tts_policy() {
        let mut processor = processor();
        let mut logs = Vec::new();
        let settings = json!({
            "filterBadWords": true,
            "badWords": ["scam"],
            "muteBlockedTts": true,
        });
        let (annotations, views) = processor
            .enrich_texts(&settings, "this is a scam", "Viewer", "viewer", &mut logs)
            .unwrap();
        assert_eq!(annotations["comment"]["moderation"]["blocked"], true);
        assert_eq!(annotations["comment"]["tts"]["speak"], false);
        assert_eq!(annotations["comment"]["tts"]["source"], "policy");
        assert_eq!(
            annotations["comment"]["tts"]["reason"],
            "moderation-blocked"
        );
        assert_eq!(annotations["comment"]["tts"]["text"], "");
        // The policy travels in the canonical view: dropping it would let TTS
        // fall back to the raw blocked message.
        let view = views.get("comment").expect("moderation-policy view");
        assert!(!view.speak);
        assert_eq!(view.reason.as_deref(), Some("moderation-blocked"));
        assert_eq!(view.source, "policy");
    }

    #[test]
    fn unmuted_blocks_keep_regular_tts() {
        let mut processor = processor();
        let mut logs = Vec::new();
        let (annotations, views) = processor
            .enrich_texts(
                &json!({
                    "filterBadWords": true,
                    "badWords": ["scam"],
                    "muteBlockedTts": false,
                }),
                "this is a scam",
                "Viewer",
                "viewer",
                &mut logs,
            )
            .unwrap();
        assert_eq!(annotations["comment"]["moderation"]["blocked"], true);
        assert_eq!(annotations["comment"]["tts"]["speak"], true);
        assert!(!annotations["comment"]["tts"]["text"]
            .as_str()
            .unwrap_or_default()
            .is_empty());
        assert!(views.get("comment").is_some_and(|view| view.speak));
    }

    #[test]
    fn blocked_policy_emitted_even_without_tts_candidates() {
        let mut processor = processor();
        let mut logs = Vec::new();
        let (annotations, views) = processor
            .enrich_texts(
                &json!({
                    "filterBadWords": true,
                    "badWords": ["scam"],
                    "muteBlockedTts": true,
                    "ttsCandidate": false,
                }),
                "this is a scam",
                "Viewer",
                "viewer",
                &mut logs,
            )
            .unwrap();
        assert_eq!(annotations["comment"]["moderation"]["blocked"], true);
        // Normal candidate generation is off, yet the skip policy must still
        // exist so consumers cannot speak the raw comment.
        assert_eq!(annotations["comment"]["tts"]["speak"], false);
        assert_eq!(
            annotations["comment"]["tts"]["reason"],
            "moderation-blocked"
        );
        let view = views.get("comment").expect("moderation-policy view");
        assert!(!view.speak);
    }

    #[test]
    fn moderation_settings_invalidate_caches() {
        let mut processor = processor();
        let mut logs = Vec::new();
        let (annotations, _) = processor
            .enrich_texts(
                &json!({"filterBadWords": false, "badWords": ["scam"]}),
                "this is a scam",
                "Viewer",
                "viewer",
                &mut logs,
            )
            .unwrap();
        assert_eq!(annotations["comment"]["moderation"]["blocked"], false);
        assert_eq!(processor.comment_cache.len(), 1);
        // Same text, moderation enabled: the stale unblocked entry is dropped.
        let (annotations, _) = processor
            .enrich_texts(
                &json!({"filterBadWords": true, "badWords": ["scam"]}),
                "this is a scam",
                "Viewer",
                "viewer",
                &mut logs,
            )
            .unwrap();
        assert_eq!(annotations["comment"]["moderation"]["blocked"], true);
        assert_eq!(processor.comment_cache.len(), 1);
    }

    #[test]
    fn malformed_moderation_settings_fall_back_safely() {
        let mut processor = processor();
        let mut logs = Vec::new();
        let (annotations, _) = processor
            .enrich_texts(
                &json!({
                    "filterSpam": "yes",
                    "spamThreshold": "high",
                    "filterBadWords": 1,
                    "badWords": "not-an-array",
                    "muteBlockedTts": "no",
                }),
                "this is a scam",
                "Viewer",
                "viewer",
                &mut logs,
            )
            .unwrap();
        assert_eq!(annotations["comment"]["moderation"]["blocked"], false);
        assert!(annotations["comment"].get("spam").is_some());
        assert!(annotations["comment"].get("tts").is_some());
    }

    #[test]
    fn moderation_payload_stays_bounded() {
        let mut processor = processor();
        let mut logs = Vec::new();
        let terms: Vec<String> = (0..600).map(|index| format!("term{index:04}")).collect();
        let message = format!("hello {}", "x".repeat(2_000));
        let (annotations, _) = processor
            .enrich_texts(
                &json!({"filterBadWords": true, "badWords": terms}),
                &message,
                "Viewer",
                "viewer",
                &mut logs,
            )
            .unwrap();
        let payload = serde_json::to_vec(&annotations["comment"]).unwrap();
        assert!(payload.len() <= 32 * 1024, "{}", payload.len());
        assert_eq!(annotations["comment"]["moderation"]["blocked"], false);
    }

    #[test]
    fn nickname_phonetic_nests_without_overriting_spoken_selection() {
        let mut processor = processor();
        let mut logs = Vec::new();
        let (annotations, views) = processor
            .enrich_texts(&settings_json(), "hi", "J0sé", "j0se", &mut logs)
            .unwrap();
        let tts = &annotations["user"]["nickname"]["tts"];
        assert!(tts.get("text").and_then(Value::as_str).is_some());
        // The old flat shape overwrote the spoken selection; phonetic
        // evidence now nests and the view carries it separately.
        assert!(tts.get("ipa").is_none());
        assert!(tts.get("dialect").is_none());
        let view = views.get("nickname").expect("nickname view");
        assert_eq!(view.text, tts["text"].as_str().unwrap_or_default());
        assert_eq!(
            view.language.as_deref(),
            tts.get("language").and_then(Value::as_str)
        );
        assert_eq!(
            view.confidence,
            tts.get("confidence").and_then(Value::as_f64)
        );
        if let Some(phonetic) = tts.get("phonetic") {
            let pronunciation = view.pronunciation.as_ref().expect("pronunciation");
            assert_eq!(
                pronunciation.ipa.as_deref(),
                phonetic.get("ipa").and_then(Value::as_str)
            );
            assert_eq!(
                pronunciation.dialect.as_deref(),
                phonetic.get("dialect").and_then(Value::as_str)
            );
            assert_eq!(
                pronunciation.confidence,
                phonetic.get("confidence").and_then(Value::as_f64)
            );
        }
    }
}
