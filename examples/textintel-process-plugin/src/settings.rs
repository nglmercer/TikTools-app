//! Validated settings for the TextIntel processor.
//!
//! Every field falls back to its default independently: one malformed value
//! never discards the rest, and unknown settings are ignored without
//! panicking.

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

impl TextIntelSettings {
    pub fn from_value(value: &Value) -> Self {
        let defaults = Self::default();
        let object = value.as_object();
        let get = |key: &str| object.and_then(|object| object.get(key));
        Self {
            analyze_comments: get_bool(get("analyzeComments"), defaults.analyze_comments),
            analyze_nicknames: get_bool(get("analyzeNicknames"), defaults.analyze_nicknames),
            analyze_usernames: get_bool(get("analyzeUsernames"), defaults.analyze_usernames),
            unicode_normalization: get_bool(
                get("unicodeNormalization"),
                defaults.unicode_normalization,
            ),
            language_detection: get_bool(get("languageDetection"), defaults.language_detection),
            rebus: get_bool(get("rebus"), defaults.rebus),
            obfuscation: get_bool(get("obfuscation"), defaults.obfuscation),
            spam: get_bool(get("spam"), defaults.spam),
            phonetic: get_bool(get("phonetic"), defaults.phonetic),
            tts_candidate: get_bool(get("ttsCandidate"), defaults.tts_candidate),
            engine_mode: get("engineMode")
                .and_then(Value::as_str)
                .map(EngineMode::parse)
                .unwrap_or(defaults.engine_mode),
            minimum_tts_confidence: get_confidence(
                get("minimumTtsConfidence"),
                defaults.minimum_tts_confidence,
            ),
            minimum_rebus_confidence: get_confidence(
                get("minimumRebusConfidence"),
                defaults.minimum_rebus_confidence,
            ),
            max_input_chars: get("maxInputChars")
                .and_then(Value::as_u64)
                .map(|value| value.clamp(1, 4_000) as usize)
                .unwrap_or(defaults.max_input_chars),
            emoji_mode: get("emojiMode")
                .and_then(Value::as_str)
                .map(EmojiMode::parse)
                .unwrap_or(defaults.emoji_mode),
        }
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

fn get_bool(value: Option<&Value>, default: bool) -> bool {
    value.and_then(Value::as_bool).unwrap_or(default)
}

fn get_confidence(value: Option<&Value>, default: f64) -> f64 {
    value
        .and_then(Value::as_f64)
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
    fn digest_changes_with_analysis_settings() {
        let base = TextIntelSettings::default();
        let mut changed = base.clone();
        changed.minimum_tts_confidence = 0.8;
        assert_ne!(base.digest(), changed.digest());
        assert_eq!(base.digest(), TextIntelSettings::default().digest());
    }
}
