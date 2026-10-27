//! Filter predicate language, event path access, and value normalization.

use super::*;

pub(crate) fn event_record_matches(record: &Value, event: &Value) -> bool {
    let Some(filters) = record.get("filters").and_then(Value::as_array) else {
        return true;
    };
    filters.iter().all(|filter| matches_filter(filter, event))
}

fn matches_filter(filter: &Value, event: &Value) -> bool {
    let path = filter
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let raw = read_event_path(event, path);
    let left = raw.map(value_to_string).unwrap_or_default();
    let right = filter
        .get("value")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let left_number = left.parse::<f64>().ok();
    let right_number = right.parse::<f64>().ok();
    let numeric = left_number.is_some() && right_number.is_some() && !right.trim().is_empty();
    match filter
        .get("operator")
        .and_then(Value::as_str)
        .unwrap_or("eq")
    {
        "crosses-multiple" => periodic_like_crossings(filter, event) > 0,
        "gte" => numeric && left_number >= right_number,
        "gt" => numeric && left_number > right_number,
        "lte" => numeric && left_number <= right_number,
        "lt" => numeric && left_number < right_number,
        "eq" => {
            if numeric {
                left_number == right_number
            } else {
                left == right
            }
        }
        "neq" => {
            if numeric {
                left_number != right_number
            } else {
                left != right
            }
        }
        "contains" => left
            .to_ascii_lowercase()
            .contains(&right.to_ascii_lowercase()),
        "starts-with" => left
            .to_ascii_lowercase()
            .starts_with(&right.to_ascii_lowercase()),
        "in" => filter
            .get("values")
            .and_then(Value::as_array)
            .is_some_and(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .any(|value| value.trim().eq_ignore_ascii_case(left.trim()))
            }),
        "is-true" => raw == Some(&Value::Bool(true)) || left == "true" || left == "1",
        "is-false" => {
            raw == Some(&Value::Bool(false)) || left == "false" || left == "0" || left.is_empty()
        }
        _ => false,
    }
}


/**
 * New opt-in recurring likes policy. Existing filters never become recurring.
 * A packet may cross multiple milestones; the caller executes actions that
 * many times, within an explicit per-packet safety limit.
 */
pub(crate) const MAX_LIKE_ACTION_REPEATS_PER_PACKET: usize = 64;

fn periodic_like_crossings(filter: &Value, event: &Value) -> usize {
    if event.get("type").and_then(Value::as_str) != Some("tiktok.like") {
        return 0;
    }
    let scope = filter.get("path").and_then(Value::as_str).unwrap_or_default();
    if scope != "event.data.aggregation.global" && scope != "event.data.aggregation.user" {
        return 0;
    }
    let Some(step) = filter
        .get("value")
        .and_then(Value::as_str)
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
    else {
        return 0;
    };
    let Some(raw) = read_event_path(event, scope) else { return 0 };
    let (Some(previous), Some(total)) = (
        raw.get("previous").and_then(Value::as_u64),
        raw.get("total").and_then(Value::as_u64),
    ) else { return 0 };
    if total <= previous {
        return 0;
    }
    let crossed = total / step - previous / step;
    if crossed > MAX_LIKE_ACTION_REPEATS_PER_PACKET as u64 {
        tracing::warn!(
            crossed,
            cap = MAX_LIKE_ACTION_REPEATS_PER_PACKET,
            "like milestone batch exceeded safe action execution budget"
        );
    }
    crossed.min(MAX_LIKE_ACTION_REPEATS_PER_PACKET as u64) as usize
}

/// Execute once for old configurations; only an explicit recurring filter
/// changes the number of executions. All filters must pass before dispatch.
/// If more than one periodic filter exists, use the smaller crossing count.
pub(crate) fn event_record_repeat_count(record: &Value, event: &Value) -> usize {
    if !event_record_matches(record, event) {
        return 0;
    }
    record
        .get("filters")
        .and_then(Value::as_array)
        .map(|filters| {
            filters
                .iter()
                .filter(|filter| {
                    filter.get("operator").and_then(Value::as_str) == Some("crosses-multiple")
                })
                .map(|filter| periodic_like_crossings(filter, event))
                .min()
                .unwrap_or(1)
        })
        .unwrap_or(1)
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

#[cfg(test)]
mod like_goal_tests {
    use super::*;
    use serde_json::json;

    fn matches(previous: u64, total: u64, step: &str) -> bool {
        event_record_matches(
            &json!({"filters": [{
                "path": "event.data.aggregation.global",
                "operator": "crosses-multiple",
                "value": step
            }]}),
            &json!({"type": "tiktok.like", "data": {
                "aggregation": {"global": {"previous": previous, "total": total}}
            }}),
        )
    }

    #[test]
    fn periodic_goal_repeats_at_every_multiple_and_handles_batched_crossings() {
        assert!(!matches(0, 14, "15"));
        assert!(matches(14, 15, "15"));
        assert!(!matches(15, 15, "15"));
        assert!(!matches(15, 29, "15"));
        assert!(matches(29, 30, "15"));
        assert!(matches(30, 45, "15"));
        assert!(matches(0, 45, "15"));
        assert_eq!(periodic_like_crossings(&json!({"path": "event.data.aggregation.global", "value": "15"}), &json!({"type": "tiktok.like", "data": {"aggregation": {"global": {"previous": 0, "total": 45}}}})), 3);
        assert!(!matches(45, 30, "15"));
    }

    #[test]
    fn batch_execution_count_preserves_legacy_rules_and_caps_large_bursts() {
        let recurring = json!({"filters": [{
            "path": "event.data.aggregation.global",
            "operator": "crosses-multiple",
            "value": "150"
        }]});
        let legacy = json!({"filters": [
            {"path": "event.data.aggregation.global.previous", "operator": "lt", "value": "150"},
            {"path": "event.data.aggregation.global.total", "operator": "gte", "value": "150"}
        ]});
        let batch = |previous: u64, total: u64| json!({
            "type": "tiktok.like",
            "data": {"aggregation": {"global": {"previous": previous, "total": total}}}
        });
        assert_eq!(event_record_repeat_count(&recurring, &batch(0, 149)), 0);
        assert_eq!(event_record_repeat_count(&recurring, &batch(0, 150)), 1);
        assert_eq!(event_record_repeat_count(&recurring, &batch(150, 450)), 2);
        assert_eq!(event_record_repeat_count(&legacy, &batch(0, 450)), 1);
        assert_eq!(event_record_repeat_count(&legacy, &batch(150, 450)), 0);
        assert_eq!(event_record_repeat_count(&recurring, &batch(0, 150 * 1000)),
                   MAX_LIKE_ACTION_REPEATS_PER_PACKET);
        assert_eq!(event_record_repeat_count(&json!({"filters": []}), &batch(0, 150)), 1);
    }

    #[test]
    fn periodic_goal_fails_closed_for_invalid_steps_or_missing_transitions() {
        for step in ["", "0", "-15", "1.5", "not-a-number"] {
            assert!(!matches(14, 30, step));
        }
        let record = json!({"filters": [{
            "path": "event.data.aggregation.user",
            "operator": "crosses-multiple",
            "value": "15"
        }]});
        assert!(!event_record_matches(&record, &json!({
            "type": "tiktok.like", "data": {"count": 15, "aggregation": {"user": null}}
        })));
        assert!(event_record_matches(&record, &json!({
            "type": "tiktok.like", "data": {"count": 30,
                "aggregation": {"user": {"previous": 12, "total": 42}}}
        })));
    }
}
