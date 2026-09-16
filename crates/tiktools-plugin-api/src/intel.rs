//! Provider-neutral stable enrichment DTOs shared by processors and the host.
//!
//! These types ARE the `event.intel` contract: processors build them, the
//! host validates them before promoting anything into the stable namespace,
//! and the automation contracts generate UI schema from them. Anything that
//! does not fit these shapes stays under `intel.providers.<pluginId>` as
//! provider-native JSON. Fields marked HOST-PROJECTED are filled by the host
//! (from text views), never by processors.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

/// Stable provider-neutral enrichment attached by event processors.
/// Always optional on the event: the base trigger contract works with no
/// processor installed.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct EventIntel {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<IntelComment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<IntelUser>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processing: Option<IntelProcessing>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub providers: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct IntelComment {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normalized: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nfc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nfkc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub casefolded: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub truncated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<IntelLanguage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub composition: Option<IntelComposition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unicode: Option<IntelUnicode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub obfuscation: Option<IntelObfuscation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spam: Option<IntelSpam>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rebus: Option<IntelRebus>,
    /// HOST-PROJECTED from the selected text view.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tts: Option<IntelTts>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct IntelLanguage {
    pub top: String,
    pub confidence: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidates: Option<Vec<IntelLanguageCandidate>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct IntelLanguageCandidate {
    pub language: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct IntelComposition {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji_ratio: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub letters: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub digits: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all_caps: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elongated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repetition_score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub urls: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mentions: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct IntelUnicode {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mixed_scripts: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suspicious: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invisible: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bidirectional: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confusables: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct IntelObfuscation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detected: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leetspeak: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repetition: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub punctuation_flood: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mixed_scripts: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confusables: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct IntelSpam {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detected: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasons: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calibrated: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct IntelRebus {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strong: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IntelTts {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default = "default_speak", skip_serializing_if = "is_true")]
    pub speak: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pronunciation: Option<IntelPronunciation>,
}

impl Default for IntelTts {
    fn default() -> Self {
        Self {
            text: String::new(),
            language: None,
            confidence: None,
            source: None,
            speak: true,
            reason: None,
            pronunciation: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct IntelPronunciation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ipa: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dialect: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct IntelUser {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nickname: Option<IntelNickname>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unique_id: Option<IntelHandle>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct IntelNickname {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normalized: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<IntelLanguage>,
    /// HOST-PROJECTED from the selected text view.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tts: Option<IntelTts>,
}

/// Language/composition evidence about a stable handle. Never a spoken
/// rendering: identity is never rewritten or pronounced from here.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct IntelHandle {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<IntelLanguage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub composition: Option<IntelComposition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct IntelProcessing {
    pub status: String,
}

fn is_false(value: &bool) -> bool {
    !value
}

fn is_true(value: &bool) -> bool {
    *value
}

fn default_speak() -> bool {
    true
}

/// Bounds every plugin-controlled string and collection promoted into the
/// stable namespace, on top of the result-level byte cap.
pub const MAX_INTEL_TEXT_CHARS: usize = 8_192;
pub const MAX_INTEL_CODE_CHARS: usize = 64;
pub const MAX_INTEL_LANGUAGE_CANDIDATES: usize = 8;
pub const MAX_INTEL_FLAGS: usize = 16;
pub const MAX_INTEL_REASONS: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum IntelError {
    #[error("intel field `{field}` is not the expected shape: {message}")]
    Shape { field: String, message: String },
    #[error("intel field `{field}` exceeds {max} characters")]
    TooLong { field: String, max: usize },
    #[error("intel field `{field}` allows at most {max} entries")]
    TooMany { field: String, max: usize },
    #[error("intel field `{field}` must be within 0.0..=1.0")]
    OutOfRange { field: String },
}

fn check_text(value: Option<&str>, field: &str, max: usize) -> Result<(), IntelError> {
    if value.is_some_and(|text| text.chars().count() > max) {
        return Err(IntelError::TooLong {
            field: field.to_owned(),
            max,
        });
    }
    Ok(())
}

fn check_unit(value: Option<f64>, field: &str) -> Result<(), IntelError> {
    if value.is_some_and(|confidence| !confidence.is_finite() || !(0.0..=1.0).contains(&confidence))
    {
        return Err(IntelError::OutOfRange {
            field: field.to_owned(),
        });
    }
    Ok(())
}

fn check_ratio(value: Option<f64>, field: &str) -> Result<(), IntelError> {
    if value.is_some_and(|ratio| !ratio.is_finite() || ratio < 0.0) {
        return Err(IntelError::OutOfRange {
            field: field.to_owned(),
        });
    }
    Ok(())
}

impl IntelLanguage {
    pub fn validate(&self, field: &str) -> Result<(), IntelError> {
        check_text(Some(&self.top), &format!("{field}.top"), 32)?;
        check_unit(Some(self.confidence), &format!("{field}.confidence"))?;
        if let Some(candidates) = &self.candidates {
            if candidates.len() > MAX_INTEL_LANGUAGE_CANDIDATES {
                return Err(IntelError::TooMany {
                    field: format!("{field}.candidates"),
                    max: MAX_INTEL_LANGUAGE_CANDIDATES,
                });
            }
            for candidate in candidates {
                check_text(
                    Some(&candidate.language),
                    &format!("{field}.candidates.language"),
                    32,
                )?;
                check_unit(
                    Some(candidate.confidence),
                    &format!("{field}.candidates.confidence"),
                )?;
            }
        }
        Ok(())
    }
}

impl IntelComposition {
    pub fn validate(&self, field: &str) -> Result<(), IntelError> {
        check_ratio(self.emoji_ratio, &format!("{field}.emojiRatio"))?;
        check_ratio(self.repetition_score, &format!("{field}.repetitionScore"))?;
        Ok(())
    }
}

impl IntelUnicode {
    pub fn validate(&self, field: &str) -> Result<(), IntelError> {
        check_unit(self.score, &format!("{field}.score"))?;
        Ok(())
    }
}

impl IntelObfuscation {
    pub fn validate(&self, field: &str) -> Result<(), IntelError> {
        check_unit(self.score, &format!("{field}.score"))?;
        if let Some(flags) = &self.flags {
            if flags.len() > MAX_INTEL_FLAGS {
                return Err(IntelError::TooMany {
                    field: format!("{field}.flags"),
                    max: MAX_INTEL_FLAGS,
                });
            }
            for flag in flags {
                check_text(Some(flag), &format!("{field}.flags"), MAX_INTEL_CODE_CHARS)?;
            }
        }
        Ok(())
    }
}

impl IntelSpam {
    pub fn validate(&self, field: &str) -> Result<(), IntelError> {
        check_unit(self.score, &format!("{field}.score"))?;
        if let Some(reasons) = &self.reasons {
            if reasons.len() > MAX_INTEL_REASONS {
                return Err(IntelError::TooMany {
                    field: format!("{field}.reasons"),
                    max: MAX_INTEL_REASONS,
                });
            }
            for reason in reasons {
                check_text(Some(reason), &format!("{field}.reasons"), 256)?;
            }
        }
        check_text(self.model.as_deref(), &format!("{field}.model"), 64)?;
        Ok(())
    }
}

impl IntelRebus {
    pub fn validate(&self, field: &str) -> Result<(), IntelError> {
        check_text(
            self.candidate.as_deref(),
            &format!("{field}.candidate"),
            MAX_INTEL_TEXT_CHARS,
        )?;
        check_unit(self.confidence, &format!("{field}.confidence"))?;
        check_unit(self.score, &format!("{field}.score"))?;
        Ok(())
    }
}

impl IntelPronunciation {
    pub fn validate(&self, field: &str) -> Result<(), IntelError> {
        check_text(self.ipa.as_deref(), &format!("{field}.ipa"), 256)?;
        check_text(self.language.as_deref(), &format!("{field}.language"), 32)?;
        check_text(self.dialect.as_deref(), &format!("{field}.dialect"), 32)?;
        check_unit(self.confidence, &format!("{field}.confidence"))?;
        Ok(())
    }
}

impl IntelTts {
    pub fn validate(&self, field: &str) -> Result<(), IntelError> {
        check_text(Some(&self.text), field, MAX_INTEL_TEXT_CHARS)?;
        check_text(self.language.as_deref(), &format!("{field}.language"), 32)?;
        check_unit(self.confidence, &format!("{field}.confidence"))?;
        check_text(self.source.as_deref(), &format!("{field}.source"), 64)?;
        check_text(self.reason.as_deref(), &format!("{field}.reason"), 128)?;
        if let Some(pronunciation) = &self.pronunciation {
            pronunciation.validate(&format!("{field}.pronunciation"))?;
        }
        Ok(())
    }
}

impl IntelComment {
    pub fn validate(&self) -> Result<(), IntelError> {
        check_text(
            self.normalized.as_deref(),
            "comment.normalized",
            MAX_INTEL_TEXT_CHARS,
        )?;
        check_text(self.nfc.as_deref(), "comment.nfc", MAX_INTEL_TEXT_CHARS)?;
        check_text(self.nfkc.as_deref(), "comment.nfkc", MAX_INTEL_TEXT_CHARS)?;
        check_text(
            self.casefolded.as_deref(),
            "comment.casefolded",
            MAX_INTEL_TEXT_CHARS,
        )?;
        if let Some(language) = &self.language {
            language.validate("comment.language")?;
        }
        if let Some(composition) = &self.composition {
            composition.validate("comment.composition")?;
        }
        if let Some(unicode) = &self.unicode {
            unicode.validate("comment.unicode")?;
        }
        if let Some(obfuscation) = &self.obfuscation {
            obfuscation.validate("comment.obfuscation")?;
        }
        if let Some(spam) = &self.spam {
            spam.validate("comment.spam")?;
        }
        if let Some(rebus) = &self.rebus {
            rebus.validate("comment.rebus")?;
        }
        if let Some(tts) = &self.tts {
            tts.validate("comment.tts")?;
        }
        Ok(())
    }
}

impl IntelNickname {
    pub fn validate(&self, field: &str) -> Result<(), IntelError> {
        check_text(
            self.normalized.as_deref(),
            &format!("{field}.normalized"),
            MAX_INTEL_TEXT_CHARS,
        )?;
        if let Some(language) = &self.language {
            language.validate(&format!("{field}.language"))?;
        }
        if let Some(tts) = &self.tts {
            tts.validate(&format!("{field}.tts"))?;
        }
        Ok(())
    }
}

impl IntelHandle {
    pub fn validate(&self, field: &str) -> Result<(), IntelError> {
        if let Some(language) = &self.language {
            language.validate(&format!("{field}.language"))?;
        }
        if let Some(composition) = &self.composition {
            composition.validate(&format!("{field}.composition"))?;
        }
        Ok(())
    }
}

impl IntelUser {
    pub fn validate(&self) -> Result<(), IntelError> {
        if let Some(nickname) = &self.nickname {
            nickname.validate("user.nickname")?;
        }
        if let Some(handle) = &self.unique_id {
            handle.validate("user.uniqueId")?;
        }
        Ok(())
    }
}

fn shape_error(field: &str, error: serde_json::Error) -> IntelError {
    IntelError::Shape {
        field: field.to_owned(),
        message: error.to_string(),
    }
}

/// Validates one processor `comment` annotation and returns its canonical
/// form. Unknown keys are dropped: only contract fields promote into the
/// stable namespace.
pub fn canonical_stable_comment(value: &Value) -> Result<Value, IntelError> {
    let comment: IntelComment =
        serde_json::from_value(value.clone()).map_err(|error| shape_error("comment", error))?;
    comment.validate()?;
    serde_json::to_value(&comment).map_err(|error| shape_error("comment", error))
}

/// Validates one processor `user` annotation and returns its canonical form.
pub fn canonical_stable_user(value: &Value) -> Result<Value, IntelError> {
    let user: IntelUser =
        serde_json::from_value(value.clone()).map_err(|error| shape_error("user", error))?;
    user.validate()?;
    serde_json::to_value(&user).map_err(|error| shape_error("user", error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn canonical_form_drops_unknown_keys_and_enforces_bounds() {
        let canonical = canonical_stable_comment(&json!({
            "normalized": "hello",
            "language": {"top": "en", "confidence": 0.9},
            "composition": {"emojiOnly": true, "whitespace": 2},
            "futureField": {"nested": true},
        }))
        .unwrap();
        assert_eq!(canonical["normalized"], "hello");
        assert!(canonical.get("futureField").is_none());
        assert!(canonical["composition"].get("whitespace").is_none());

        assert!(canonical_stable_comment(&json!({
            "language": {"top": "en", "confidence": 1.5},
        }))
        .is_err());
        assert!(canonical_stable_comment(&json!({
            "spam": {"reasons": ["a", "b", "c", "d", "e", "f", "g", "h", "i"]},
        }))
        .is_err());
        assert!(canonical_stable_comment(&json!("not-an-object")).is_err());
    }

    #[test]
    fn user_validation_covers_nickname_and_handle() {
        let canonical = canonical_stable_user(&json!({
            "nickname": {"normalized": "Viewer", "extra": 1},
            "uniqueId": {"composition": {"emojiOnly": false}},
        }))
        .unwrap();
        assert!(canonical["nickname"].get("extra").is_none());
        assert!(canonical_stable_user(&json!({
            "nickname": {"tts": {"text": "x", "confidence": 9.0}},
        }))
        .is_err());
    }
}
