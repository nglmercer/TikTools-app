//! Filter predicate language, event path access, and value normalization.
//!
//! Every filter evaluates to a [`FilterOutcome`]: a `matched` flag plus the
//! bounded `match.*` captures of the winning value branch.
//! `event_record_matches` stays as a `bool` wrapper so existing call sites
//! keep working.

use super::*;

/// Most captures one record may publish to action templates.
pub(crate) const MAX_MATCH_CAPTURES: usize = 8;
/// `anyOf` is flat: at most this many alternative groups per record.
pub(crate) const MAX_ANYOF_GROUPS: usize = 4;
/// …and at most this many AND-ed filters inside one group.
pub(crate) const MAX_ANYOF_GROUP_FILTERS: usize = 12;
/// Captured values are filter operands, still bounded like untrusted text.
const MAX_CAPTURE_VALUE_CHARS: usize = 200;
const MAX_CAPTURE_KEY_CHARS: usize = 32;
/// Aliases that would shadow a template namespace root stay reserved.
const RESERVED_CAPTURE_KEYS: &[&str] = &["match", "event", "globals", "data", "user"];

/// Result of one filter (or one record's filter list): whether it matched
/// and which `match.*` captures the winning branches produced.
#[derive(Debug, Default)]
pub(crate) struct FilterOutcome {
    pub matched: bool,
    pub captures: BTreeMap<String, String>,
}

impl FilterOutcome {
    fn unmatched() -> Self {
        Self {
            matched: false,
            captures: BTreeMap::new(),
        }
    }
}

pub(crate) fn event_record_matches(record: &Value, event: &Value) -> bool {
    event_record_outcome(record, event).matched
}

/// Merge helper: first claim on a key wins, the map stays bounded.
fn merge_captures(into: &mut BTreeMap<String, String>, from: BTreeMap<String, String>) {
    for (key, value) in from {
        if into.len() >= MAX_MATCH_CAPTURES {
            break;
        }
        into.entry(key).or_insert(value);
    }
}

/// AND over the record's `filters`, then `anyOf`: a flat list of alternative
/// groups where every group must internally AND. The first group whose
/// filters all pass contributes its captures; `anyOf` absent or empty is
/// `true`. Malformed groups simply never win.
pub(crate) fn event_record_outcome(record: &Value, event: &Value) -> FilterOutcome {
    let mut outcome = FilterOutcome {
        matched: true,
        captures: BTreeMap::new(),
    };
    let require_all = record.get("matchMode").and_then(Value::as_str) != Some("or");
    if let Some(filters) = record.get("filters").and_then(Value::as_array) {
        if require_all {
            for (index, filter) in filters.iter().enumerate() {
                let result = evaluate_filter(filter, event, index);
                if !result.matched {
                    return FilterOutcome::unmatched();
                }
                merge_captures(&mut outcome.captures, result.captures);
            }
        } else {
            // Global OR: the first passing filter wins and contributes its captures.
            let mut winner = None;
            for (index, filter) in filters.iter().enumerate() {
                let result = evaluate_filter(filter, event, index);
                if result.matched {
                    winner = Some(result.captures);
                    break;
                }
            }
            if !filters.is_empty() {
                match winner {
                    Some(captures) => merge_captures(&mut outcome.captures, captures),
                    None => return FilterOutcome::unmatched(),
                }
            }
        }
    }
    let Some(groups) = record.get("anyOf").and_then(Value::as_array) else {
        return outcome;
    };
    if groups.is_empty() {
        return outcome;
    }
    for group in groups.iter().take(MAX_ANYOF_GROUPS) {
        let Some(items) = group.as_array() else {
            continue;
        };
        let mut group_captures = BTreeMap::new();
        let mut group_matched = true;
        for (index, filter) in items.iter().take(MAX_ANYOF_GROUP_FILTERS).enumerate() {
            let result = evaluate_filter(filter, event, index);
            if !result.matched {
                group_matched = false;
                break;
            }
            merge_captures(&mut group_captures, result.captures);
        }
        if group_matched {
            merge_captures(&mut outcome.captures, group_captures);
            return outcome;
        }
    }
    FilterOutcome::unmatched()
}

/// `as` alias contract: `^[a-zA-Z][a-zA-Z0-9_]{0,31}$`, never a namespace
/// root. Anything else falls back to the positional `match.<index>` key.
fn capture_alias(filter: &Value) -> Option<String> {
    let alias = filter.get("as").and_then(Value::as_str)?;
    let valid = alias.len() <= MAX_CAPTURE_KEY_CHARS
        && alias
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic())
        && alias.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !RESERVED_CAPTURE_KEYS.contains(&alias);
    valid.then(|| alias.to_owned())
}

/// Control characters never reach a template; the value is capped so one
/// verbose rule cannot expand without bound.
fn sanitize_capture(value: &str) -> String {
    value
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_CAPTURE_VALUE_CHARS)
        .collect()
}

fn capture_key(filter: &Value, index: usize) -> String {
    capture_alias(filter).unwrap_or_else(|| index.to_string())
}

/// String candidates for a value-taking operator: `values` wins over `value`
/// when present as an array (`in` always reads `values`, like before).
fn filter_candidates<'a>(filter: &'a Value, operator: &str) -> Vec<&'a str> {
    let values = filter.get("values").and_then(Value::as_array);
    match operator {
        "in" => values
            .map(|items| items.iter().filter_map(Value::as_str).collect::<Vec<_>>())
            .unwrap_or_default(),
        _ => match values {
            Some(items) => items.iter().filter_map(Value::as_str).collect::<Vec<_>>(),
            None => vec![filter
                .get("value")
                .and_then(Value::as_str)
                .unwrap_or_default()],
        },
    }
}

fn predicate_matches(
    operator: &str,
    left: &str,
    left_number: Option<f64>,
    candidate: &str,
) -> bool {
    let right_number = candidate.parse::<f64>().ok();
    let numeric = left_number.is_some() && right_number.is_some() && !candidate.trim().is_empty();
    match operator {
        "gte" => numeric && left_number >= right_number,
        "gt" => numeric && left_number > right_number,
        "lte" => numeric && left_number <= right_number,
        "lt" => numeric && left_number < right_number,
        "eq" => {
            if numeric {
                left_number == right_number
            } else {
                left == candidate
            }
        }
        "neq" => {
            if numeric {
                left_number != right_number
            } else {
                left != candidate
            }
        }
        "contains" => left
            .to_ascii_lowercase()
            .contains(&candidate.to_ascii_lowercase()),
        "starts-with" => left
            .to_ascii_lowercase()
            .starts_with(&candidate.to_ascii_lowercase()),
        _ => false,
    }
}

/// One filter against one event. `index` only names the fallback capture key
/// (`match.<index>`); it never affects matching.
pub(crate) fn evaluate_filter(filter: &Value, event: &Value, index: usize) -> FilterOutcome {
    let path = filter
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let raw = read_event_path(event, path);
    let left = raw.map(value_to_string).unwrap_or_default();
    let left_number = left.parse::<f64>().ok();
    let operator = filter
        .get("operator")
        .and_then(Value::as_str)
        .unwrap_or("eq");

    let matched_capture = |candidate: &str| {
        let mut captures = BTreeMap::new();
        captures.insert(capture_key(filter, index), sanitize_capture(candidate));
        FilterOutcome {
            matched: true,
            captures,
        }
    };

    match operator {
        "is-true" => FilterOutcome {
            matched: raw == Some(&Value::Bool(true)) || left == "true" || left == "1",
            captures: BTreeMap::new(),
        },
        "is-false" => FilterOutcome {
            matched: raw == Some(&Value::Bool(false))
                || left == "false"
                || left == "0"
                || left.is_empty(),
            captures: BTreeMap::new(),
        },
        "in" => {
            // `in` keeps its own semantics (trim + ASCII fold of the whole
            // value) and ignores `quantifier`; the first matching entry is
            // the capture.
            let candidates = filter_candidates(filter, operator);
            for candidate in &candidates {
                if candidate.trim().eq_ignore_ascii_case(left.trim()) {
                    return matched_capture(candidate);
                }
            }
            FilterOutcome::unmatched()
        }
        _ => {
            let candidates = filter_candidates(filter, operator);
            if candidates.is_empty() {
                return FilterOutcome::unmatched();
            }
            // `neq` with a list means «none of»: `any` is normalized to `all`
            // so a stored draft cannot express the almost-always-true form.
            let require_all = filter.get("quantifier").and_then(Value::as_str) == Some("all")
                || operator == "neq";
            if require_all {
                if candidates
                    .iter()
                    .all(|candidate| predicate_matches(operator, &left, left_number, candidate))
                {
                    return matched_capture(candidates[0]);
                }
                return FilterOutcome::unmatched();
            }
            for candidate in &candidates {
                if predicate_matches(operator, &left, left_number, candidate) {
                    return matched_capture(candidate);
                }
            }
            FilterOutcome::unmatched()
        }
    }
}

/// Reads one `event.*` path with the shared filter/template path language
/// (a leading `event.` prefix addresses the envelope root).
pub(crate) fn read_event_path<'a>(event: &'a Value, path: &str) -> Option<&'a Value> {
    let path = path
        .trim()
        .trim_start_matches("{{")
        .trim_end_matches("}}")
        .trim();
    let path = path.strip_prefix("event.").unwrap_or(path);
    let path = path.strip_prefix("event").unwrap_or(path);
    let mut current = event;
    for part in path.trim_matches('.').split('.') {
        if part.is_empty() {
            continue;
        }
        current = current.get(part)?;
    }
    Some(current)
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Array(_) | Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
    }
}

pub(crate) fn number_u64(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| {
            value
                .as_f64()
                .filter(|value| value.is_finite() && *value >= 0.0)
                .map(|value| value as u64)
        })
        .or_else(|| value.as_str()?.parse().ok())
}

/// Event clone carrying the record's `match.*` captures for action rendering
/// only. Cooldown claims, liveness checks and delegated projections keep the
/// original envelope. Returns the input untouched when there is nothing to
/// inject so old rules observe byte-identical events.
pub(crate) fn event_with_match_captures<'a>(
    event: &'a Value,
    captures: &BTreeMap<String, String>,
) -> std::borrow::Cow<'a, Value> {
    if captures.is_empty() {
        return std::borrow::Cow::Borrowed(event);
    }
    let Some(object) = event.as_object() else {
        return std::borrow::Cow::Borrowed(event);
    };
    let mut object = object.clone();
    object.insert(
        "match".to_owned(),
        Value::Object(
            captures
                .iter()
                .map(|(key, value)| (key.clone(), Value::String(value.clone())))
                .collect(),
        ),
    );
    std::borrow::Cow::Owned(Value::Object(object))
}

#[cfg(test)]
mod corpus_tests {
    use super::*;

    /// Shared with the TypeScript test-support matcher
    /// (`src/automation/behavior/filter-corpus.test.ts`). This matcher is the
    /// contract: the corpus expectations are derived from it, and a case that
    /// changes here must change there in the same commit.
    const CORPUS: &str =
        include_str!("../../../../../tests/fixtures/automation-filters/filter-corpus.json");

    fn corpus() -> Value {
        serde_json::from_str(CORPUS).expect("filter corpus is valid JSON")
    }

    fn event_for<'a>(corpus: &'a Value, case: &'a Value) -> &'a Value {
        match &case["event"] {
            Value::String(name) => corpus["events"]
                .get(name)
                .unwrap_or_else(|| panic!("unknown corpus event {name}")),
            inline => inline,
        }
    }

    fn cases<'a>(corpus: &'a Value, key: &str) -> &'a Vec<Value> {
        let cases = corpus[key].as_array().expect("corpus section is an array");
        assert!(!cases.is_empty(), "corpus section {key} is empty");
        cases
    }

    fn expected_captures(case: &Value) -> Option<BTreeMap<String, String>> {
        case.get("captures")?.as_object().map(|object| {
            object
                .iter()
                .filter_map(|(key, value)| {
                    value.as_str().map(|text| (key.clone(), text.to_owned()))
                })
                .collect()
        })
    }

    #[test]
    fn filter_cases_match_the_shared_corpus() {
        let corpus = corpus();
        for case in cases(&corpus, "filterCases") {
            let event = event_for(&corpus, case);
            let outcome = evaluate_filter(&case["filter"], event, 0);
            assert_eq!(
                outcome.matched,
                case["matches"].as_bool().expect("matches is a bool"),
                "filter case: {}",
                case["label"]
            );
            if let Some(expected) = expected_captures(case) {
                assert_eq!(outcome.captures, expected, "captures: {}", case["label"]);
            }
        }
    }

    #[test]
    fn record_cases_combine_filters_with_and_in_the_shared_corpus() {
        let corpus = corpus();
        for case in cases(&corpus, "recordCases") {
            let mut record = serde_json::Map::new();
            if let Some(filters) = case.get("filters") {
                record.insert("filters".into(), filters.clone());
            }
            if let Some(mode) = case.get("matchMode") {
                record.insert("matchMode".into(), mode.clone());
            }
            if let Some(any_of) = case.get("anyOf") {
                record.insert("anyOf".into(), any_of.clone());
            }
            let outcome = event_record_outcome(&Value::Object(record), event_for(&corpus, case));
            assert_eq!(
                outcome.matched,
                case["matches"].as_bool().expect("matches is a bool"),
                "record case: {}",
                case["label"]
            );
            if let Some(expected) = expected_captures(case) {
                assert_eq!(outcome.captures, expected, "captures: {}", case["label"]);
            }
        }
    }

    #[test]
    fn production_cases_match_the_shared_corpus() {
        let corpus = corpus();
        for case in cases(&corpus, "productionCases") {
            let record = serde_json::json!({ "filters": case["filters"] });
            let outcome = event_record_outcome(&record, event_for(&corpus, case));
            assert_eq!(
                outcome.matched,
                case["matches"].as_bool().expect("matches is a bool"),
                "production case: {}",
                case["label"]
            );
            if let Some(expected) = expected_captures(case) {
                assert_eq!(outcome.captures, expected, "captures: {}", case["label"]);
            }
        }
    }
}
