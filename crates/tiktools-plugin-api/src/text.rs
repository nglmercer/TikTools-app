//! Provider-neutral text helpers shared by the host and text processors.
//!
//! [`TextComposition`] is a deterministic classifier any processor can reuse
//! instead of inventing its own emoji/caps/repetition heuristics. The
//! `resolve_*_tts` helpers centralize the provider-neutral TTS input lookup
//! (`event.intel.*` view first, raw text fallback) so TTS consumers never
//! duplicate the fallback chain and keep working with no processor installed.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use unicode_segmentation::UnicodeSegmentation;

/// Deterministic composition evidence for one text. All fields are directly
/// observable; nothing here claims intent.
///
/// Emoji is counted in user-perceived grapheme clusters: `❤️` and `👨‍👩‍👧`
/// each count as one emoji, and `emoji_only` requires every cluster to be
/// emoji or whitespace (so `😂!!!` is not emoji-only).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TextComposition {
    pub emoji_only: bool,
    pub emoji_count: usize,
    /// `emoji_count / max(1, non-whitespace grapheme clusters)`.
    pub emoji_ratio: f64,
    pub letters: usize,
    pub digits: usize,
    pub whitespace: usize,
    pub punctuation: usize,
    pub all_caps: bool,
    pub elongated: bool,
    /// Strongest repetition evidence in `[0.0, 1.0)`: `(repeats - 1) /
    /// repeats` for the most-repeated unit (1..=4 chars). `0.0` when no unit
    /// repeats.
    pub repetition_score: f64,
    pub urls: usize,
    pub mentions: usize,
}

/// Longest prefix (in characters) scanned for repetition evidence. Counting
/// still covers the whole text; only the quadratic sequence scan is windowed.
const REPETITION_SCAN_CHARS: usize = 2048;
/// A single character repeated this many times consecutively counts as
/// elongation (`hooooola`, never `book`).
const CHAR_RUN_THRESHOLD: usize = 4;
/// A multi-character unit repeated this many times consecutively counts as
/// elongation (`jajajajajaja`).
const SEQUENCE_REPEAT_THRESHOLD: usize = 4;

pub fn compose_text(text: &str) -> TextComposition {
    let chars: Vec<char> = text.chars().collect();
    let mut emoji_clusters = 0_usize;
    let mut non_whitespace_clusters = 0_usize;
    let mut non_emoji_content = false;
    for cluster in text.graphemes(true) {
        let cluster_chars: Vec<char> = cluster.chars().collect();
        // A keycap cluster (`1` in `1️⃣`) is emoji presentation: the
        // enclosing mark makes the whole cluster emoji-bearing.
        let emoji_bearing = cluster_chars.contains(&'\u{20E3}')
            || cluster_chars.iter().any(|character| is_emoji(*character));
        let whitespace_only = cluster_chars
            .iter()
            .all(|character| character.is_whitespace());
        if emoji_bearing {
            emoji_clusters += 1;
        }
        if !whitespace_only {
            non_whitespace_clusters += 1;
        }
        if !emoji_bearing && !whitespace_only {
            non_emoji_content = true;
        }
    }

    let mut letters = 0_usize;
    let mut digits = 0_usize;
    let mut whitespace = 0_usize;
    let mut punctuation = 0_usize;
    let mut has_uppercase = false;
    let mut has_lowercase = false;
    for character in chars.iter().copied() {
        if character.is_whitespace() {
            whitespace += 1;
        }
        if character.is_alphabetic() {
            letters += 1;
            if character.is_uppercase() {
                has_uppercase = true;
            }
            if character.is_lowercase() {
                has_lowercase = true;
            }
        }
        if character.is_numeric() {
            digits += 1;
        }
        if is_punctuation(character) {
            punctuation += 1;
        }
    }

    let emoji_only = emoji_clusters > 0 && !non_emoji_content;
    let emoji_ratio = if non_whitespace_clusters == 0 {
        0.0
    } else {
        emoji_clusters as f64 / non_whitespace_clusters as f64
    };
    let (elongated, repetition_score) = repetition_evidence(&chars);

    TextComposition {
        emoji_only,
        emoji_count: emoji_clusters,
        emoji_ratio,
        letters,
        digits,
        whitespace,
        punctuation,
        all_caps: has_uppercase && !has_lowercase,
        elongated,
        repetition_score,
        urls: count_urls(text),
        mentions: count_mentions(text),
    }
}

/// Removes emoji presentation characters (keeping keycap bases like `1` in
/// `1️⃣`, which read as digits) and collapses whitespace. Used for TTS
/// `remove` mode.
pub fn strip_emoji(text: &str) -> String {
    text.chars()
        .filter(|character| !is_emoji(*character))
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Dependency-free emoji heuristic over the well-known emoji blocks plus the
/// joiners and selectors that glue sequences together. It favors recall on
/// live-chat text; exact segmentation stays the text engine's job.
fn is_emoji(character: char) -> bool {
    matches!(character,
        '\u{2600}'..='\u{26FF}' // Misc Symbols (includes U+2764 heavy black heart)
        | '\u{2700}'..='\u{27BF}' // Dingbats
        | '\u{2B00}'..='\u{2BFF}' // Misc Symbols and Arrows
        | '\u{1F000}'..='\u{1FAFF}' // Emoticons, pictographs, transport, supplemental
        | '\u{200D}' // zero-width joiner
        | '\u{FE00}'..='\u{FE0F}' // variation selectors
        | '\u{20E3}' // combining enclosing keycap
        | '\u{E0020}'..='\u{E007F}' // tag characters
    )
}

fn is_punctuation(character: char) -> bool {
    matches!(character, '!'..='/' | ':'..='@' | '['..='`' | '{'..='~')
        || matches!(
            character,
            '¡' | '¿'
                | '«'
                | '»'
                | '‹'
                | '›'
                | '„'
                | '"'
                | '‚'
                | '‘'
                | '’'
                | '“'
                | '”'
                | '–'
                | '—'
                | '·'
        )
}

fn repetition_evidence(chars: &[char]) -> (bool, f64) {
    let window = chars.len().min(REPETITION_SCAN_CHARS);
    let chars = &chars[..window];
    let mut best_repeats = 0_usize;
    let mut elongated = false;
    let mut triple_runs = 0_usize;
    for unit in 1..=4_usize {
        let mut index = 0_usize;
        while index + unit <= chars.len() {
            let mut repeats = 1_usize;
            while index + (repeats + 1) * unit <= chars.len()
                && chars[index..index + unit]
                    == chars[index + repeats * unit..index + (repeats + 1) * unit]
            {
                repeats += 1;
            }
            if repeats >= 2 {
                best_repeats = best_repeats.max(repeats);
                if unit == 1 {
                    if repeats >= CHAR_RUN_THRESHOLD {
                        elongated = true;
                    }
                    if repeats >= 3 {
                        triple_runs += 1;
                    }
                } else if repeats >= SEQUENCE_REPEAT_THRESHOLD {
                    elongated = true;
                }
                index += repeats * unit;
            } else {
                index += 1;
            }
        }
    }
    // One triple run (`coool`) is weak evidence, but two (`HOOOLAAA`)
    // corroborate elongation.
    if triple_runs >= 2 {
        elongated = true;
    }
    let score = if best_repeats >= 2 {
        (best_repeats - 1) as f64 / best_repeats as f64
    } else {
        0.0
    };
    (elongated, score)
}

fn find_url_start(rest: &str) -> Option<usize> {
    match (rest.find("http://"), rest.find("https://")) {
        (Some(plain), Some(secure)) => Some(plain.min(secure)),
        (Some(plain), None) => Some(plain),
        (None, Some(secure)) => Some(secure),
        (None, None) => None,
    }
}

fn count_urls(text: &str) -> usize {
    let mut count = 0_usize;
    let mut rest = text;
    while let Some(offset) = find_url_start(rest) {
        count += 1;
        let candidate = &rest[offset..];
        let end = candidate
            .find(|character: char| {
                character.is_whitespace()
                    || matches!(character, '"' | '\'' | '`' | ')' | '}' | ']' | ',')
            })
            .unwrap_or(candidate.len());
        rest = &candidate[end..];
        if rest == candidate {
            break;
        }
    }
    count
}

fn count_mentions(text: &str) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let mut count = 0_usize;
    let mut index = 0_usize;
    while index < chars.len() {
        if chars[index] == '@'
            && (index == 0 || !(chars[index - 1].is_alphanumeric() || chars[index - 1] == '_'))
            && chars
                .get(index + 1)
                .is_some_and(|next| next.is_alphanumeric() || *next == '_' || *next == '.')
        {
            count += 1;
            index += 1;
            while index < chars.len()
                && (chars[index].is_alphanumeric() || chars[index] == '_' || chars[index] == '.')
            {
                index += 1;
            }
        } else {
            index += 1;
        }
    }
    count
}

/// Provider-neutral TTS input resolved from one event. Consumers must honor
/// `speak`: a resolved text with `speak == false` is an explicit instruction
/// not to speak (for example an emoji-only message under a skip policy), and
/// must never fall back to raw text.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedText {
    pub text: String,
    pub language: Option<String>,
    pub confidence: Option<f64>,
    /// Where `text` came from: the view source, `intel`, or `raw`.
    pub source: String,
    pub speak: bool,
    pub reason: Option<String>,
    pub ipa: Option<String>,
}

/// One TTS lookup: the projected intel view first, then raw fallbacks.
#[derive(Debug, Clone, Copy)]
pub struct TextResolutionSpec<'a> {
    pub intel_path: &'a str,
    pub fallbacks: &'a [&'a str],
}

/// Resolves speakable text from one event: the projected intel TTS view when
/// a processor supplied one, otherwise the first non-empty raw fallback.
/// Always succeeds while a fallback exists, so TTS works with no processor.
pub fn resolve_text(event: &Value, spec: TextResolutionSpec<'_>) -> Option<ResolvedText> {
    let tts = event.pointer(spec.intel_path);
    if tts
        .and_then(|tts| tts.get("speak"))
        .and_then(Value::as_bool)
        == Some(false)
    {
        return Some(ResolvedText {
            text: String::new(),
            language: None,
            confidence: None,
            source: "intel".to_owned(),
            speak: false,
            reason: tts
                .and_then(|tts| tts.get("reason"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            ipa: None,
        });
    }
    if let Some(text) = tts
        .and_then(|tts| tts.get("text"))
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
    {
        let pronunciation = tts.and_then(|tts| tts.get("pronunciation"));
        return Some(ResolvedText {
            text: text.to_owned(),
            language: tts
                .and_then(|tts| tts.get("language"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            confidence: tts
                .and_then(|tts| tts.get("confidence"))
                .and_then(Value::as_f64),
            source: tts
                .and_then(|tts| tts.get("source"))
                .and_then(Value::as_str)
                .unwrap_or("intel")
                .to_owned(),
            speak: true,
            reason: None,
            ipa: pronunciation
                .and_then(|pronunciation| pronunciation.get("ipa"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
        });
    }
    spec.fallbacks.iter().find_map(|path| {
        event
            .pointer(path)
            .and_then(Value::as_str)
            .filter(|text| !text.is_empty())
            .map(|text| ResolvedText {
                text: text.to_owned(),
                language: None,
                confidence: None,
                source: "raw".to_owned(),
                speak: true,
                reason: None,
                ipa: None,
            })
    })
}

/// Resolves the speakable comment: `event.intel.comment.tts` when a
/// processor supplied one, otherwise the raw `event.data.comment`.
pub fn resolve_comment_tts(event: &Value) -> Option<ResolvedText> {
    resolve_text(
        event,
        TextResolutionSpec {
            intel_path: "/intel/comment/tts",
            fallbacks: &["/data/comment"],
        },
    )
}

/// Resolves the speakable viewer name: `event.intel.user.nickname.tts` when
/// a processor supplied one, otherwise the raw `event.user.nickname`, else
/// the stable `event.user.uniqueId` handle. Identity is never rewritten;
/// only the spoken rendering is resolved.
pub fn resolve_nickname_tts(event: &Value) -> Option<ResolvedText> {
    resolve_text(
        event,
        TextResolutionSpec {
            intel_path: "/intel/user/nickname/tts",
            fallbacks: &["/user/nickname", "/user/uniqueId"],
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn classifies_emoji_only_messages() {
        for text in ["😂😂😂", "😂 😂", "❤️", "👨\u{200d}👩\u{200d}👧", "1️⃣"]
        {
            assert!(compose_text(text).emoji_only, "{text}");
        }
        // Any non-emoji cluster — letters, digits, even punctuation —
        // disqualifies emoji-only.
        for text in ["hello 😂", "123 😂", "", "   ", "hello", "😂!!!", "😂?"] {
            assert!(!compose_text(text).emoji_only, "{text}");
        }
        let composition = compose_text("hola 😂");
        assert_eq!(composition.emoji_count, 1);
        assert!((composition.emoji_ratio - 0.2).abs() < f64::EPSILON);
        assert_eq!(composition.letters, 4);
    }

    #[test]
    fn counts_user_perceived_emoji_graphemes() {
        assert_eq!(compose_text("❤️").emoji_count, 1);
        assert_eq!(compose_text("👨\u{200d}👩\u{200d}👧").emoji_count, 1);
        assert_eq!(compose_text("1️⃣").emoji_count, 1);
        assert_eq!(compose_text("😂😂😂").emoji_count, 3);
        assert_eq!(compose_text("😂 😂").emoji_count, 2);
        assert_eq!(compose_text("plain").emoji_count, 0);
    }

    #[test]
    fn classifies_all_caps_over_alphabetic_characters_only() {
        assert!(compose_text("HELLO!!!").all_caps);
        assert!(compose_text("HELLO123").all_caps);
        assert!(compose_text("HOOOLAAA 😂😂").all_caps);
        assert!(!compose_text("Hello").all_caps);
        assert!(!compose_text("😂😂").all_caps);
        assert!(!compose_text("123").all_caps);
        assert!(!compose_text("").all_caps);
    }

    #[test]
    fn detects_elongation_without_flagging_double_letters() {
        for text in ["hooooola", "nooooo", "yeeeees", "jajajajajaja", "HOOOLAAA"] {
            let composition = compose_text(text);
            assert!(composition.elongated, "{text}");
            assert!(composition.repetition_score > 0.0, "{text}");
        }
        for text in ["book", "coool", "hello", "hola", ""] {
            assert!(!compose_text(text).elongated, "{text}");
        }
        // Repetition score grows with the run: (repeats-1)/repeats.
        assert!(compose_text("nooooo").repetition_score > compose_text("coool").repetition_score);
        assert_eq!(compose_text("hola").repetition_score, 0.0);
    }

    #[test]
    fn counts_marks_urls_and_mentions() {
        let composition = compose_text("@alice see https://example.com/a and http://x.y!");
        assert_eq!(composition.mentions, 1);
        assert_eq!(composition.urls, 2);
        assert!(composition.punctuation > 0);
        assert_eq!(compose_text("mail@example.com").mentions, 0);
        assert_eq!(compose_text("no marks here").urls, 0);
        assert_eq!(compose_text("123").digits, 3);
    }

    #[test]
    fn strips_emoji_for_tts_remove_mode() {
        assert_eq!(strip_emoji("hello 😂"), "hello");
        assert_eq!(strip_emoji("😂😂😂"), "");
        assert_eq!(strip_emoji("a  😂  b"), "a b");
        assert_eq!(strip_emoji("1️⃣"), "1");
        assert_eq!(strip_emoji("plain"), "plain");
    }

    #[test]
    fn tts_resolution_prefers_intel_views_and_falls_back_to_raw() {
        let enriched = json!({
            "user": {"uniqueId": "j0se_92", "nickname": "J0sé"},
            "data": {"comment": "HOOOLAAA 😂😂"},
            "intel": {
                "comment": {"tts": {"text": "Hola", "language": "es", "confidence": 0.84, "source": "spoken"}},
                "user": {"nickname": {"tts": {"text": "José", "language": "es", "confidence": 0.86, "pronunciation": {"ipa": "xoˈse"}}}},
            },
        });
        let comment = resolve_comment_tts(&enriched).unwrap();
        assert_eq!(comment.text, "Hola");
        assert_eq!(comment.language.as_deref(), Some("es"));
        assert_eq!(comment.source, "spoken");
        assert!(comment.speak);
        let nickname = resolve_nickname_tts(&enriched).unwrap();
        assert_eq!(nickname.text, "José");
        assert_eq!(nickname.ipa.as_deref(), Some("xoˈse"));

        // No processor installed: raw fallbacks keep TTS working.
        let raw = json!({
            "user": {"uniqueId": "j0se_92", "nickname": "J0sé"},
            "data": {"comment": "HOOOLAAA 😂😂"},
        });
        let comment = resolve_comment_tts(&raw).unwrap();
        assert_eq!(comment.text, "HOOOLAAA 😂😂");
        assert_eq!(comment.source, "raw");
        let nickname = resolve_nickname_tts(&raw).unwrap();
        assert_eq!(nickname.text, "J0sé");

        // Missing nickname falls back to the stable handle.
        let handle_only = json!({"user": {"uniqueId": "j0se_92"}, "data": {}});
        assert_eq!(resolve_nickname_tts(&handle_only).unwrap().text, "j0se_92");
        assert!(resolve_comment_tts(&handle_only).is_none());

        // An explicit skip never falls back to raw text.
        let skipped = json!({
            "data": {"comment": "😂😂😂"},
            "intel": {"comment": {"tts": {"text": "", "speak": false, "reason": "emoji-only"}}},
        });
        let resolved = resolve_comment_tts(&skipped).unwrap();
        assert!(!resolved.speak);
        assert_eq!(resolved.text, "");
        assert_eq!(resolved.reason.as_deref(), Some("emoji-only"));
    }
}
