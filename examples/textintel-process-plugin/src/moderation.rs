//! Chat moderation verdicts for analyzed comments.
//!
//! [`moderation_value`] computes the plugin-owned verdict rendered under the
//! provider namespace (`event.intel.providers.textintel.comment.moderation`).
//! The verdict is pure evidence — spam score versus the configured threshold
//! plus configured-term matches — so the automation engine (never this
//! side-effect-free processor) can act on `moderation.blocked`.
//!
//! Term matching here is the Stage A local mirror of textintel's
//! `match_terms`: the pinned textintel revision predates that API, so the
//! matcher below works over the existing [`MessageFingerprint`] views while
//! folding terms with textintel's own normalization functions (no forked
//! normalization logic). When the pin advances past `match_terms`, replace
//! [`match_terms`] with `textintel::match_terms` and keep this module's
//! verdict mapping.

use std::collections::BTreeSet;

use serde_json::{json, Value};
use textintel::normalization::confusables::skeleton;
use textintel::normalization::leetspeak::apply_leet;
use textintel::normalization::repetition::collapse_repetition;
use textintel::normalization::unicode::{casefold_text, nfkc};
use textintel::normalization::whitespace::normalize_whitespace;
use textintel::MessageFingerprint;

/// Maximum configured blocked terms kept after sanitizing settings.
pub const MAX_BAD_WORDS: usize = 500;
/// Maximum length of one configured term, in characters.
pub const MAX_BAD_WORD_CHARS: usize = 128;
/// Maximum term matches probed per message (mirrors textintel's bound).
pub const MAX_TERM_MATCHES: usize = 32;
/// Maximum matches rendered into the verdict payload.
pub const MAX_MODERATION_MATCHES: usize = 8;

/// One configured term found in one fingerprint view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TermMatch {
    /// The configured term, trimmed of surrounding whitespace.
    pub term: String,
    /// View that revealed the match: `raw`, `nfkc`, `casefold`, `leet`,
    /// `repetition_collapsed`, `skeleton`, `normalized`, or `rebus`.
    pub view: String,
}

/// Sanitizes the user-configured blocked-term list: trims entries, drops
/// empties and over-long terms, deduplicates case-insensitively (keeping the
/// first spelling), and caps the count. Never logs its input.
pub fn sanitize_bad_words(raw: Vec<String>) -> Vec<String> {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut clean = Vec::new();
    for term in raw {
        if clean.len() >= MAX_BAD_WORDS {
            break;
        }
        let trimmed = term.trim();
        if trimmed.is_empty() || trimmed.chars().count() > MAX_BAD_WORD_CHARS {
            continue;
        }
        if seen.insert(casefold_text(trimmed)) {
            clean.push(trimmed.to_owned());
        }
    }
    clean
}

/// Matches sanitized blocked terms against a fingerprint. Views are checked
/// in fixed order (least transformed first) and each term reports only its
/// first matching view. See the module docs for the Stage A / Stage B plan.
pub fn match_terms(fingerprint: &MessageFingerprint, terms: &[String]) -> Vec<TermMatch> {
    let views = message_views(fingerprint);
    let mut seen: BTreeSet<(String, String)> = BTreeSet::new();
    let mut matches = Vec::new();
    for term in terms.iter().take(MAX_BAD_WORDS) {
        if matches.len() >= MAX_TERM_MATCHES {
            break;
        }
        let trimmed = term.trim();
        if trimmed.is_empty() || trimmed.chars().count() > MAX_BAD_WORD_CHARS {
            continue;
        }
        let Some(found) = match_one_term(&views, trimmed) else {
            continue;
        };
        if seen.insert((found.term.clone(), found.view.clone())) {
            matches.push(found);
        }
    }
    matches
}

/// Search text for one named view plus its optional fallback.
struct MessageView {
    name: &'static str,
    text: String,
    /// Keep-1 repetition derivation for non-default engine `repetition_keep`.
    fallback: Option<String>,
}

fn stored_or(view: Option<&String>, fallback: String) -> String {
    view.filter(|text| !text.is_empty())
        .cloned()
        .unwrap_or(fallback)
}

fn message_views(fingerprint: &MessageFingerprint) -> Vec<MessageView> {
    let unicode = &fingerprint.unicode_features;
    let views = &fingerprint.normalization_views;
    let raw = fingerprint.raw.as_str();

    let casefold_source = stored_or(Some(&unicode.casefolded), casefold_text(raw));
    let mut collected = vec![
        MessageView {
            name: "raw",
            text: raw.to_owned(),
            fallback: None,
        },
        MessageView {
            name: "nfkc",
            text: stored_or(Some(&unicode.nfkc), nfkc(raw)),
            fallback: None,
        },
        MessageView {
            name: "casefold",
            text: casefold_source.clone(),
            fallback: None,
        },
        MessageView {
            name: "leet",
            text: stored_or(views.get("leet"), apply_leet(casefold_source.as_str())),
            fallback: None,
        },
        MessageView {
            name: "repetition_collapsed",
            text: stored_or(
                views.get("repetition_collapsed"),
                collapse_repetition(&casefold_source, 1),
            ),
            fallback: Some(collapse_repetition(&casefold_source, 1)),
        },
        MessageView {
            name: "skeleton",
            text: unicode
                .confusable_skeleton
                .as_deref()
                .filter(|skeleton| !skeleton.is_empty())
                .map(str::to_owned)
                .unwrap_or_else(|| skeleton(raw)),
            fallback: None,
        },
        MessageView {
            name: "normalized",
            text: fingerprint
                .normalized
                .as_deref()
                .filter(|normalized| !normalized.is_empty())
                .map(str::to_owned)
                .unwrap_or_else(|| full_pipeline(raw)),
            fallback: None,
        },
    ];
    // Strong rebus candidates only: weak decodings are hypotheses, not
    // readings worth blocking on.
    let mut seen_rebus: BTreeSet<String> = BTreeSet::new();
    for candidate in fingerprint
        .rebus_candidates
        .iter()
        .filter(|candidate| candidate.strong)
        .take(8)
    {
        let folded = casefold_text(&candidate.text);
        if !folded.is_empty() && seen_rebus.insert(folded.clone()) {
            collected.push(MessageView {
                name: "rebus",
                text: folded,
                fallback: None,
            });
        }
    }
    collected
}

/// The analyzer's full `normalized` pipeline.
fn full_pipeline(text: &str) -> String {
    normalize_whitespace(&collapse_repetition(
        &apply_leet(&casefold_text(&nfkc(text))),
        1,
    ))
}

fn term_key_for_view(term: &str, view: &str) -> String {
    match view {
        "raw" => term.to_owned(),
        "nfkc" => nfkc(term),
        "casefold" | "rebus" => casefold_text(term),
        "leet" => apply_leet(&casefold_text(term)),
        "repetition_collapsed" => collapse_repetition(&casefold_text(term), 1),
        "skeleton" => skeleton(term),
        "normalized" => full_pipeline(term),
        _ => casefold_text(term),
    }
}

fn match_one_term(views: &[MessageView], term: &str) -> Option<TermMatch> {
    for view in views {
        let key = term_key_for_view(term, view.name);
        if key.is_empty() {
            continue;
        }
        if find_bounded(&view.text, &key) {
            return Some(TermMatch {
                term: term.to_owned(),
                view: view.name.to_owned(),
            });
        }
        let fallback_hit = view
            .fallback
            .as_deref()
            .filter(|fallback| *fallback != view.text.as_str())
            .is_some_and(|fallback| find_bounded(fallback, &key));
        if fallback_hit {
            return Some(TermMatch {
                term: term.to_owned(),
                view: view.name.to_owned(),
            });
        }
    }
    None
}

/// Substring search requiring word boundaries at the match edges: adjacent
/// characters must be absent, non-alphanumeric, and (for punctuation-heavy
/// terms) different from the term's own edge character, so `ass` never
/// matches `class` and `!!` never matches `!!!`.
fn find_bounded(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() || haystack.len() < needle.len() {
        return false;
    }
    let mut start = 0;
    while start + needle.len() <= haystack.len() {
        let Some(offset) = haystack[start..].find(needle) else {
            break;
        };
        let match_start = start + offset;
        let match_end = match_start + needle.len();
        if has_boundary_before(haystack, match_start, needle)
            && has_boundary_after(haystack, match_end, needle)
        {
            return true;
        }
        start = match_start + 1;
    }
    false
}

fn has_boundary_before(haystack: &str, match_start: usize, needle: &str) -> bool {
    let Some(before) = haystack[..match_start].chars().next_back() else {
        return true;
    };
    boundary_char(before, needle.chars().next())
}

fn has_boundary_after(haystack: &str, match_end: usize, needle: &str) -> bool {
    let Some(after) = haystack[match_end..].chars().next() else {
        return true;
    };
    boundary_char(after, needle.chars().next_back())
}

fn boundary_char(adjacent: char, edge: Option<char>) -> bool {
    if adjacent.is_alphanumeric() {
        return false;
    }
    edge.is_none_or(|edge| adjacent != edge)
}

/// Builds the plugin-owned moderation verdict. `spam_score` is `Some` only
/// when spam evidence was calculated (`spam` setting enabled); without
/// evidence the spam branch can never trigger.
pub fn moderation_value(
    filter_spam: bool,
    spam_threshold: f64,
    spam_score: Option<f64>,
    filter_bad_words: bool,
    matches: &[TermMatch],
) -> Value {
    let spam_hit =
        filter_spam && spam_score.is_some_and(|score| score.is_finite() && score >= spam_threshold);
    let bad_words_hit = filter_bad_words && !matches.is_empty();
    let mut reasons = Vec::new();
    if spam_hit {
        reasons.push(Value::String("spam".to_owned()));
    }
    if bad_words_hit {
        reasons.push(Value::String("bad-word".to_owned()));
    }
    json!({
        "blocked": spam_hit || bad_words_hit,
        "spam": spam_hit,
        "spamScore": spam_score.map(|score| {
            if score.is_finite() {
                serde_json::Number::from_f64(score.clamp(0.0, 1.0)).unwrap_or(serde_json::Number::from(0))
            } else {
                serde_json::Number::from(0)
            }
        }).map(Value::Number).unwrap_or(Value::Null),
        "badWords": bad_words_hit,
        "matches": matches
            .iter()
            .take(MAX_MODERATION_MATCHES)
            .map(|found| json!({"term": found.term, "view": found.view}))
            .collect::<Vec<_>>(),
        "reasons": reasons,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fingerprint_for(engine: &textintel::TextIntelligence, text: &str) -> MessageFingerprint {
        engine.analyze(text).expect("analysis must succeed")
    }

    fn engine() -> textintel::TextIntelligence {
        textintel::TextIntelligence::production_local().expect("local engine must build")
    }

    fn terms(words: &[&str]) -> Vec<String> {
        words.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn sanitize_trims_dedups_and_bounds() {
        let long = "x".repeat(MAX_BAD_WORD_CHARS + 1);
        let mut raw = vec![
            "  scam ".to_owned(),
            "SCAM".to_owned(),
            String::new(),
            "   ".to_owned(),
            long,
            "spam phrase".to_owned(),
        ];
        raw.extend((0..MAX_BAD_WORDS + 50).map(|i| format!("term{i:04}")));
        let clean = sanitize_bad_words(raw);
        assert_eq!(clean.len(), MAX_BAD_WORDS);
        assert_eq!(clean[0], "scam");
        assert_eq!(clean[1], "spam phrase");
        assert!(clean
            .iter()
            .all(|term| term.chars().count() <= MAX_BAD_WORD_CHARS));
    }

    #[test]
    fn matches_plain_and_obfuscated_terms() {
        let engine = engine();
        let cases = [
            ("that badword is here", "badword", "raw"),
            ("that BADWORD is here", "badword", "casefold"),
            ("that ｂａｄｗｏｒｄ is here", "badword", "nfkc"),
            ("b4dw0rd", "badword", "leet"),
            ("baaaadword", "badword", "repetition_collapsed"),
            ("bаdword", "badword", "skeleton"),
            ("b4aaadw0rd", "badword", "normalized"),
            ("this is a spam phrase here", "spam phrase", "raw"),
        ];
        for (message, term, view) in cases {
            let fingerprint = fingerprint_for(&engine, message);
            let matches = match_terms(&fingerprint, &terms(&[term]));
            assert_eq!(matches.len(), 1, "{message:?} term {term:?}");
            assert_eq!(matches[0].view, view, "{message:?} term {term:?}");
            assert_eq!(matches[0].term, term);
        }
    }

    #[test]
    fn word_boundaries_reject_substrings() {
        let engine = engine();
        let fingerprint = fingerprint_for(&engine, "class dismissed");
        assert!(match_terms(&fingerprint, &terms(&["ass"])).is_empty());
        let fingerprint = fingerprint_for(&engine, "you are ass.");
        assert_eq!(match_terms(&fingerprint, &terms(&["ass"])).len(), 1);
    }

    #[test]
    fn strong_rebus_candidates_match() {
        let engine = engine();
        let mut fingerprint = fingerprint_for(&engine, "hello");
        fingerprint.rebus_candidates = vec![textintel::DecodedCandidate {
            text: "Badword here".to_owned(),
            score: 0.9,
            transformations: Vec::new(),
            language: None,
            lexical_score: 0.0,
            phonetic_score: 0.0,
            context_score: 0.0,
            symbol_score: 0.0,
            confidence_gap: 0.5,
            strong: true,
        }];
        let matches = match_terms(&fingerprint, &terms(&["badword"]));
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].view, "rebus");
    }

    #[test]
    fn verdict_combines_spam_and_terms() {
        let hit = TermMatch {
            term: "scam".to_owned(),
            view: "raw".to_owned(),
        };
        let verdict = moderation_value(true, 0.7, Some(0.9), true, &[hit]);
        assert_eq!(verdict["blocked"], true);
        assert_eq!(verdict["spam"], true);
        assert_eq!(verdict["badWords"], true);
        assert_eq!(verdict["reasons"], json!(["spam", "bad-word"]));
        assert_eq!(verdict["matches"], json!([{"term": "scam", "view": "raw"}]));

        // Disabled filters never block, even with evidence present.
        let verdict = moderation_value(false, 0.7, Some(0.9), false, &[]);
        assert_eq!(verdict["blocked"], false);
        // Missing spam evidence can never trigger the spam branch.
        let verdict = moderation_value(true, 0.0, None, false, &[]);
        assert_eq!(verdict["spam"], false);
        assert_eq!(verdict["spamScore"], Value::Null);
    }

    #[test]
    fn verdict_bounds_match_payload() {
        let matches: Vec<TermMatch> = (0..MAX_TERM_MATCHES + 10)
            .map(|i| TermMatch {
                term: format!("term{i}"),
                view: "raw".to_owned(),
            })
            .collect();
        let verdict = moderation_value(false, 0.7, None, true, &matches);
        assert_eq!(
            verdict["matches"].as_array().map(Vec::len),
            Some(MAX_MODERATION_MATCHES)
        );
    }
}
