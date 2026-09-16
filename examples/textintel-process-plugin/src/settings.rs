//! Validated settings for the TextIntel processor.
//!
//! Parsing is lenient per field: every setting deserializes as optional, a
//! mistyped value degrades to `None` (then the default), and unknown keys are
//! ignored. One malformed value never discards the rest.

use serde::{Deserialize, Deserializer};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EngineMode {
    Default,
    #[default]
    ProductionLocalLite,
    ProductionLocal,
}

impl EngineMode {
    pub fn parse(value: &str) -> Self {
        match value {
            "default" => Self::Default,
            "production-local" => Self::ProductionLocal,
            "production-local-lite" => Self::ProductionLocalLite,
            _ => Self::ProductionLocalLite,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::ProductionLocalLite => "production-local-lite",
            Self::ProductionLocal => "production-local",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EmojiMode {
    #[default]
    Keep,
    Describe,
    Remove,
    SkipMessageIfEmojiOnly,
}

impl EmojiMode {
    pub fn parse(value: &str) -> Self {
        match value {
            "describe" => Self::Describe,
            "remove" => Self::Remove,
            "skip-message-if-emoji-only" => Self::SkipMessageIfEmojiOnly,
            _ => Self::Keep,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Keep => "keep",
            Self::Describe => "describe",
            Self::Remove => "remove",
            Self::SkipMessageIfEmojiOnly => "skip-message-if-emoji-only",
        }
    }
}

/// Accepts any JSON type: a correctly typed value becomes `Some`, anything
/// else (including explicit null) becomes `None` so the default applies.
fn lenient<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Ok(T::deserialize(deserializer).ok())
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct RawSettings {
    #[serde(deserialize_with = "lenient")]
    analyze_comments: Option<bool>,
    #[serde(deserialize_with = "lenient")]
    analyze_nicknames: Option<bool>,
    #[serde(deserialize_with = "lenient")]
    analyze_usernames: Option<bool>,
    #[serde(deserialize_with = "lenient")]
    unicode_normalization: Option<bool>,
    #[serde(deserialize_with = "lenient")]
    language_detection: Option<bool>,
    #[serde(deserialize_with = "lenient")]
    rebus: Option<bool>,
    #[serde(deserialize_with = "lenient")]
    obfuscation: Option<bool>,
    #[serde(deserialize_with = "lenient")]
    spam: Option<bool>,
    #[serde(deserialize_with = "lenient")]
    phonetic: Option<bool>,
    #[serde(deserialize_with = "lenient")]
    tts_candidate: Option<bool>,
    #[serde(deserialize_with = "lenient")]
    engine_mode: Option<String>,
    #[serde(deserialize_with = "lenient")]
    minimum_tts_confidence: Option<f64>,
    #[serde(deserialize_with = "lenient")]
    minimum_rebus_confidence: Option<f64>,
    #[serde(deserialize_with = "lenient")]
    max_input_chars: Option<u64>,
    #[serde(deserialize_with = "lenient")]
    emoji_mode: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextIntelSettings {
    pub analyze_comments: bool,
    pub analyze_nicknames: bool,
    pub analyze_usernames: bool,
    pub unicode_normalization: bool,
    pub language_detection: bool,
    pub rebus: bool,
    pub obfuscation: bool,
    pub spam: bool,
    pub phonetic: bool,
    pub tts_candidate: bool,
    pub engine_mode: EngineMode,
    pub minimum_tts_confidence: f64,
    pub minimum_rebus_confidence: f64,
    pub max_input_chars: usize,
    pub emoji_mode: EmojiMode,
}

impl Default for TextIntelSettings {
    fn default() -> Self {
        Self {
            analyze_comments: true,
            analyze_nicknames: true,
            analyze_usernames: false,
            unicode_normalization: true,
            language_detection: true,
            rebus: true,
            obfuscation: true,
            spam: true,
            phonetic: true,
            tts_candidate: true,
            // Lite is the default because benchmarks show `default` mode has
            // an ~870ms cliff on long repetitive input while lite stays near
            // 10ms cold; both stay under 25ms cold on typical chat. See the
            // latency bench before changing this.
            engine_mode: EngineMode::ProductionLocalLite,
            minimum_tts_confidence: 0.7,
            minimum_rebus_confidence: 0.5,
            max_input_chars: 500,
            emoji_mode: EmojiMode::Keep,
        }
    }
}

impl From<RawSettings> for TextIntelSettings {
    fn from(raw: RawSettings) -> Self {
        let defaults = Self::default();
        Self {
            analyze_comments: raw.analyze_comments.unwrap_or(defaults.analyze_comments),
            analyze_nicknames: raw.analyze_nicknames.unwrap_or(defaults.analyze_nicknames),
            analyze_usernames: raw.analyze_usernames.unwrap_or(defaults.analyze_usernames),
            unicode_normalization: raw
                .unicode_normalization
                .unwrap_or(defaults.unicode_normalization),
            language_detection: raw
                .language_detection
                .unwrap_or(defaults.language_detection),
            rebus: raw.rebus.unwrap_or(defaults.rebus),
            obfuscation: raw.obfuscation.unwrap_or(defaults.obfuscation),
            spam: raw.spam.unwrap_or(defaults.spam),
            phonetic: raw.phonetic.unwrap_or(defaults.phonetic),
            tts_candidate: raw.tts_candidate.unwrap_or(defaults.tts_candidate),
            engine_mode: raw
                .engine_mode
                .as_deref()
                .map(EngineMode::parse)
                .unwrap_or(defaults.engine_mode),
            minimum_tts_confidence: clamp_confidence(
                raw.minimum_tts_confidence,
                defaults.minimum_tts_confidence,
            ),
            minimum_rebus_confidence: clamp_confidence(
                raw.minimum_rebus_confidence,
                defaults.minimum_rebus_confidence,
            ),
            max_input_chars: raw
                .max_input_chars
                .map(|value| value.clamp(1, 4_000) as usize)
                .unwrap_or(defaults.max_input_chars),
            emoji_mode: raw
                .emoji_mode
                .as_deref()
                .map(EmojiMode::parse)
                .unwrap_or(defaults.emoji_mode),
        }
    }
}

impl TextIntelSettings {
    pub fn from_value(value: &Value) -> Self {
        RawSettings::deserialize(value)
            .map(Self::from)
            .unwrap_or_default()
    }

    /// Revision covering every setting that changes analysis output. The
    /// plugin clears its bounded caches whenever this changes.
    pub fn digest(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{:?}|{:?}|{}|{}",
            self.analyze_comments,
            self.analyze_nicknames,
            self.analyze_usernames,
            self.unicode_normalization,
            self.language_detection,
            self.rebus,
            self.obfuscation,
            self.spam,
            self.phonetic,
            self.tts_candidate,
            self.engine_mode.as_str(),
            ordered_float_bits(self.minimum_tts_confidence),
            ordered_float_bits(self.minimum_rebus_confidence),
            self.max_input_chars,
            self.emoji_mode.as_str(),
        )
    }
}

fn ordered_float_bits(value: f64) -> u64 {
    if value.is_finite() {
        value.to_bits()
    } else {
        0
    }
}

fn clamp_confidence(value: Option<f64>, default: f64) -> f64 {
    value
        .filter(|value| value.is_finite())
        .map(|value| value.clamp(0.0, 1.0))
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn defaults_apply_to_missing_and_unknown_settings() {
        let settings = TextIntelSettings::from_value(&json!({
            "analyzeComments": false,
            "emojiMode": "remove",
            "futureSetting": {"nested": true},
        }));
        assert!(!settings.analyze_comments);
        assert_eq!(settings.emoji_mode, EmojiMode::Remove);
        assert!(settings.analyze_nicknames);
        assert_eq!(settings.minimum_tts_confidence, 0.7);

        let null = TextIntelSettings::from_value(&Value::Null);
        assert_eq!(null, TextIntelSettings::default());
    }

    #[test]
    fn malformed_values_fall_back_per_field_without_panicking() {
        let settings = TextIntelSettings::from_value(&json!({
            "analyzeComments": "yes",
            "minimumTtsConfidence": "high",
            "minimumRebusConfidence": 7.5,
            "maxInputChars": 0,
            "engineMode": "quantum",
            "emojiMode": "yell",
        }));
        assert!(settings.analyze_comments);
        assert_eq!(settings.minimum_tts_confidence, 0.7);
        assert_eq!(settings.minimum_rebus_confidence, 1.0);
        assert_eq!(settings.max_input_chars, 1);
        assert_eq!(settings.engine_mode, EngineMode::ProductionLocalLite);
        assert_eq!(settings.emoji_mode, EmojiMode::Keep);
    }

    #[test]
    fn mistyped_and_negative_numbers_fall_back_per_field() {
        let settings = TextIntelSettings::from_value(&json!({
            "maxInputChars": -5,
            "minimumTtsConfidence": true,
            "analyzeNicknames": 1,
        }));
        assert_eq!(settings.max_input_chars, 500);
        assert_eq!(settings.minimum_tts_confidence, 0.7);
        assert!(settings.analyze_nicknames);
    }

    #[test]
    fn digest_changes_with_analysis_settings() {
        let base = TextIntelSettings::default();
        let mut changed = base.clone();
        changed.minimum_tts_confidence = 0.8;
        assert_ne!(base.digest(), changed.digest());
        assert_eq!(base.digest(), TextIntelSettings::default().digest());
    }
}
