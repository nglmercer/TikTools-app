//! Maps textintel fingerprints onto the small provider-neutral TikTools
//! `intel` schema. Results are evidence (scores, candidates, confidences),
//! never semantic verdicts.

use std::cmp::Ordering;

use serde_json::{json, Value};
use textintel::{DecodedCandidate, LanguageCandidate, MessageFingerprint, SymbolInstance};
use tiktools_plugin_api::{compose_text, strip_emoji};

use tiktools_plugin_sdk::{TextPronunciation, TextView};

use crate::settings::{EmojiMode, TextIntelSettings};

/// Version of this stable mapping. Bumped deliberately when the emitted
/// `intel` shape changes incompatibly.
pub const INTEL_SCHEMA_VERSION: u32 = 2;
const MAX_LANGUAGE_CANDIDATES: usize = 3;
const MAX_OBFUSCATION_FLAGS: usize = 8;
const MAX_SPAM_REASONS: usize = 4;
const SPAM_DETECTED_THRESHOLD: f64 = 0.7;
const SUSPICIOUS_UNICODE_THRESHOLD: f64 = 0.5;

pub fn truncate_chars(text: &str, max_chars: usize) -> (String, bool) {
    if text.chars().count() <= max_chars {
        return (text.to_owned(), false);
    }
    (text.chars().take(max_chars).collect::<String>(), true)
}

fn clamp01(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn truncate_str(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        text.to_owned()
    } else {
        text.chars().take(max_chars).collect()
    }
}

fn top_language(candidates: &[LanguageCandidate]) -> Option<&LanguageCandidate> {
    candidates
        .iter()
        .max_by(|left, right| {
            let ordering = left
                .probability
                .partial_cmp(&right.probability)
                .unwrap_or(Ordering::Equal);
            // Deterministic tie-break so equal scores never flip between runs.
            ordering.then_with(|| right.language.cmp(&left.language))
        })
        .filter(|candidate| !candidate.language.is_empty())
}

/// Full stable `comment` annotation for one analyzed chat message.
pub fn comment_annotation(
    fingerprint: &MessageFingerprint,
    raw: &str,
    truncated: bool,
    settings: &TextIntelSettings,
) -> Value {
    let composition = compose_text(raw);
    let mut comment = json!({
        "truncated": truncated,
        "composition": serde_json::to_value(&composition).unwrap_or(Value::Null),
        "unicode": unicode_value(fingerprint),
    });
    if settings.unicode_normalization {
        comment["normalized"] = Value::String(
            fingerprint
                .normalized
                .clone()
                .unwrap_or_else(|| raw.to_owned()),
        );
        comment["nfc"] = Value::String(fingerprint.unicode_features.nfc.clone());
        comment["nfkc"] = Value::String(fingerprint.unicode_features.nfkc.clone());
        comment["casefolded"] = Value::String(fingerprint.unicode_features.casefolded.clone());
    }
    if settings.language_detection {
        if let Some(language) = language_value(&fingerprint.language_candidates) {
            comment["language"] = language;
        }
    }
    if settings.obfuscation {
        comment["obfuscation"] = obfuscation_value(fingerprint);
    }
    if settings.spam {
        comment["spam"] = spam_value(fingerprint);
    }
    if settings.rebus {
        if let Some(rebus) = rebus_value(fingerprint, settings) {
            comment["rebus"] = rebus;
        }
    }
    if settings.tts_candidate {
        comment["tts"] = tts_value(fingerprint, raw, &composition.emoji_only, settings);
    }
    comment
}

/// Stable `user.nickname` annotation. Handles analyzed under
/// `analyzeUsernames` use [`handle_annotation`] instead: language and
/// composition evidence only, never a spoken rendering of an identity.
pub fn nickname_annotation(
    fingerprint: &MessageFingerprint,
    raw: &str,
    settings: &TextIntelSettings,
) -> Value {
    let mut nickname = json!({
        "normalized": fingerprint.normalized.clone().unwrap_or_else(|| raw.trim().to_owned()),
    });
    if settings.language_detection {
        if let Some(top) = top_language(&fingerprint.language_candidates) {
            nickname["language"] = json!({
                "top": truncate_str(&top.language, 32),
                "confidence": clamp01(top.probability),
            });
        }
    }
    if settings.tts_candidate {
        nickname["tts"] = nickname_tts_value(fingerprint, raw, settings);
    }
    nickname
}

pub fn handle_annotation(fingerprint: &MessageFingerprint, settings: &TextIntelSettings) -> Value {
    let mut handle = json!({
        "composition": serde_json::to_value(compose_text(fingerprint.raw.as_str()))
            .unwrap_or(Value::Null),
    });
    if settings.language_detection {
        if let Some(top) = top_language(&fingerprint.language_candidates) {
            handle["language"] = json!({
                "top": truncate_str(&top.language, 32),
                "confidence": clamp01(top.probability),
            });
        }
    }
    handle
}

fn language_value(candidates: &[LanguageCandidate]) -> Option<Value> {
    let top = top_language(candidates)?;
    let mut ordered: Vec<&LanguageCandidate> = candidates.iter().collect();
    ordered.sort_by(|left, right| {
        right
            .probability
            .partial_cmp(&left.probability)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.language.cmp(&right.language))
    });
    Some(json!({
        "top": truncate_str(&top.language, 32),
        "confidence": clamp01(top.probability),
        "candidates": ordered
            .iter()
            .take(MAX_LANGUAGE_CANDIDATES)
            .map(|candidate| json!({
                "language": truncate_str(&candidate.language, 32),
                "confidence": clamp01(candidate.probability),
            }))
            .collect::<Vec<_>>(),
    }))
}

fn unicode_value(fingerprint: &MessageFingerprint) -> Value {
    let unicode = &fingerprint.unicode_features;
    let score = clamp01(unicode.suspicious_unicode_score);
    json!({
        "mixedScripts": unicode.mixed_scripts,
        "suspicious": score >= SUSPICIOUS_UNICODE_THRESHOLD,
        "score": score,
        "invisible": unicode.invisible_characters.len(),
        "bidirectional": unicode.bidirectional_controls.len(),
        "confusables": unicode.confusable_characters.len(),
    })
}

fn obfuscation_value(fingerprint: &MessageFingerprint) -> Value {
    let obfuscation = &fingerprint.obfuscation_features;
    json!({
        "detected": obfuscation.detected,
        "score": clamp01(obfuscation.score),
        "leetspeak": obfuscation.leetspeak,
        "repetition": obfuscation.repetition,
        "punctuationFlood": obfuscation.punctuation_flood,
        "mixedScripts": obfuscation.mixed_scripts,
        "confusables": obfuscation.confusables,
        "flags": obfuscation
            .flags
            .iter()
            .take(MAX_OBFUSCATION_FLAGS)
            .map(|flag| Value::String(truncate_str(flag, 64)))
            .collect::<Vec<_>>(),
    })
}

fn spam_value(fingerprint: &MessageFingerprint) -> Value {
    // The engine builder installs HeuristicSpamPredictor by default (a trained
    // predictor needs an explicit artifact file), so this is the engine's own
    // predictor, not a fork of it. Patterns stay empty: this offline build
    // registers no patterns, and `match_patterns`/`detect_spam` would
    // re-analyze the text just to return that empty set.
    let spam = textintel::predict_spam(fingerprint, &[]);
    let score = clamp01(spam.probability);
    json!({
        "score": score,
        "detected": score >= SPAM_DETECTED_THRESHOLD,
        "reasons": spam
            .reasons
            .iter()
            .take(MAX_SPAM_REASONS)
            .map(|reason| Value::String(truncate_str(reason, 128)))
            .collect::<Vec<_>>(),
    })
}

fn rebus_value(fingerprint: &MessageFingerprint, settings: &TextIntelSettings) -> Option<Value> {
    let best = fingerprint.rebus_candidates.iter().max_by(|left, right| {
        left.score
            .partial_cmp(&right.score)
            .unwrap_or(Ordering::Equal)
    })?;
    if best.score < settings.minimum_rebus_confidence {
        return None;
    }
    Some(decoded_value(best, settings.max_input_chars))
}

fn decoded_value(candidate: &DecodedCandidate, max_chars: usize) -> Value {
    json!({
        "candidate": truncate_str(&candidate.text, max_chars),
        "confidence": clamp01(candidate.score),
        "strong": candidate.strong,
    })
}

struct SpokenSelection {
    text: String,
    language: Option<String>,
    confidence: Option<f64>,
    source: String,
}

/// Picks the spoken rendering: the most confident candidate at or above the
/// configured threshold, else the original text with a `raw` source marker.
fn select_spoken(
    fingerprint: &MessageFingerprint,
    raw: &str,
    settings: &TextIntelSettings,
) -> SpokenSelection {
    let mut candidates = fingerprint.spoken_candidates.iter().collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .confidence
            .partial_cmp(&left.confidence)
            .unwrap_or(Ordering::Equal)
    });
    if let Some(candidate) = candidates.iter().find(|candidate| {
        candidate.confidence.is_finite() && candidate.confidence >= settings.minimum_tts_confidence
    }) {
        let source = if candidate.source.trim().is_empty() {
            "spoken".to_owned()
        } else {
            truncate_str(&candidate.source, 64)
        };
        return SpokenSelection {
            text: candidate.text.clone(),
            language: candidate
                .language
                .as_deref()
                .filter(|language| !language.is_empty())
                .map(|language| truncate_str(language, 32)),
            confidence: Some(clamp01(candidate.confidence)),
            source,
        };
    }
    SpokenSelection {
        text: raw.to_owned(),
        language: top_language(&fingerprint.language_candidates)
            .map(|top| truncate_str(&top.language, 32)),
        confidence: None,
        source: "raw".to_owned(),
    }
}

fn tts_value(
    fingerprint: &MessageFingerprint,
    raw: &str,
    emoji_only: &bool,
    settings: &TextIntelSettings,
) -> Value {
    if settings.emoji_mode == EmojiMode::SkipMessageIfEmojiOnly && *emoji_only {
        return json!({"text": "", "source": "policy", "speak": false, "reason": "emoji-only"});
    }
    let spoken = select_spoken(fingerprint, raw, settings);
    let text = apply_emoji_mode(&spoken.text, settings.emoji_mode, &fingerprint.symbols);
    let mut tts = json!({"text": text, "source": spoken.source, "speak": true});
    if let Some(language) = spoken.language {
        tts["language"] = Value::String(language);
    }
    if let Some(confidence) = spoken.confidence {
        tts["confidence"] = Value::Number(
            serde_json::Number::from_f64(confidence).unwrap_or(serde_json::Number::from(0)),
        );
    }
    tts
}

fn nickname_tts_value(
    fingerprint: &MessageFingerprint,
    raw: &str,
    settings: &TextIntelSettings,
) -> Value {
    let spoken = select_spoken(fingerprint, raw, settings);
    let mut tts = json!({"text": spoken.text, "source": spoken.source});
    if let Some(language) = spoken.language {
        tts["language"] = Value::String(language);
    }
    if let Some(confidence) = spoken.confidence {
        tts["confidence"] = Value::Number(
            serde_json::Number::from_f64(confidence).unwrap_or(serde_json::Number::from(0)),
        );
    }
    if settings.phonetic {
        if let Some(phonetic) = fingerprint
            .phonetic_candidates
            .iter()
            .max_by(|left, right| {
                left.confidence
                    .partial_cmp(&right.confidence)
                    .unwrap_or(Ordering::Equal)
            })
        {
            // Phonetic evidence nests under its own object: it must never
            // overwrite the spoken selection's language/confidence, which
            // describe the rendered text, not the pronunciation guess.
            let mut nested = serde_json::Map::new();
            nested.insert(
                "confidence".to_owned(),
                Value::Number(
                    serde_json::Number::from_f64(clamp01(phonetic.confidence))
                        .unwrap_or(serde_json::Number::from(0)),
                ),
            );
            if !phonetic.language.is_empty() {
                nested.insert(
                    "language".to_owned(),
                    Value::String(truncate_str(&phonetic.language, 32)),
                );
            }
            if let Some(dialect) = phonetic
                .dialect
                .as_deref()
                .filter(|value| !value.is_empty())
            {
                nested.insert(
                    "dialect".to_owned(),
                    Value::String(truncate_str(dialect, 32)),
                );
            }
            if let Some(ipa) = phonetic.ipa.as_deref().filter(|value| !value.is_empty()) {
                nested.insert("ipa".to_owned(), Value::String(truncate_str(ipa, 256)));
            }
            tts["phonetic"] = Value::Object(nested);
        }
    }
    tts
}

/// Builds the canonical `comment` text view from a comment annotation.
/// Skip-policy annotations (empty text with `speak: false`) still produce a
/// view: views are canonical for spoken text, so dropping the view would drop
/// the policy and let TTS fall back to the raw message.
pub fn comment_view(annotation: &Value) -> Option<TextView> {
    let tts = annotation.get("tts")?;
    let text = tts.get("text").and_then(Value::as_str)?;
    let speak = tts.get("speak").and_then(Value::as_bool).unwrap_or(true);
    if text.trim().is_empty() && speak {
        return None;
    }
    let mut view = TextView::new(
        text,
        tts.get("source")
            .and_then(Value::as_str)
            .unwrap_or("comment"),
    );
    view.speak = speak;
    if let Some(reason) = tts.get("reason").and_then(Value::as_str) {
        view.reason = Some(reason.to_owned());
    }
    if let Some(language) = tts.get("language").and_then(Value::as_str) {
        view.language = Some(language.to_owned());
        view.confidence = tts.get("confidence").and_then(Value::as_f64);
    }
    Some(view)
}

/// Builds the canonical `nickname` text view, carrying pronunciation evidence
/// separately from the spoken text selection.
pub fn nickname_view(annotation: &Value) -> Option<TextView> {
    let tts = annotation.get("tts")?;
    let text = tts.get("text").and_then(Value::as_str)?;
    if text.trim().is_empty() {
        return None;
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
    if let Some(phonetic) = tts.get("phonetic") {
        let pronunciation = TextPronunciation {
            ipa: phonetic
                .get("ipa")
                .and_then(Value::as_str)
                .map(str::to_owned),
            language: phonetic
                .get("language")
                .and_then(Value::as_str)
                .map(str::to_owned),
            dialect: phonetic
                .get("dialect")
                .and_then(Value::as_str)
                .map(str::to_owned),
            confidence: phonetic.get("confidence").and_then(Value::as_f64),
        };
        if pronunciation.ipa.is_some()
            || pronunciation.language.is_some()
            || pronunciation.dialect.is_some()
            || pronunciation.confidence.is_some()
        {
            view.pronunciation = Some(pronunciation);
        }
    }
    Some(view)
}

pub fn apply_emoji_mode(text: &str, mode: EmojiMode, symbols: &[SymbolInstance]) -> String {
    match mode {
        EmojiMode::Keep | EmojiMode::SkipMessageIfEmojiOnly => text.to_owned(),
        EmojiMode::Remove => strip_emoji(text),
        EmojiMode::Describe => describe_emoji(text, symbols),
    }
}

/// Replaces emoji with their Unicode name words where the engine identified
/// them; unidentified emoji stay in place for the speech engine to skip.
pub fn describe_emoji(text: &str, symbols: &[SymbolInstance]) -> String {
    let mut replacements: Vec<(&str, String)> = Vec::new();
    for symbol in symbols {
        let Some(name) = symbol
            .unicode_name
            .as_deref()
            .filter(|name| !name.is_empty())
        else {
            continue;
        };
        if symbol.raw.is_empty() || !text.contains(&symbol.raw) {
            continue;
        }
        if replacements.iter().any(|(raw, _)| *raw == symbol.raw) {
            continue;
        }
        let words = name.to_lowercase().replace('_', " ");
        replacements.push((symbol.raw.as_str(), format!(" {words} ")));
    }
    replacements.sort_by_key(|(raw, _)| std::cmp::Reverse(raw.len()));
    let mut described = text.to_owned();
    for (raw, words) in replacements {
        described = described.replace(raw, &words);
    }
    described.split_whitespace().collect::<Vec<_>>().join(" ")
}
