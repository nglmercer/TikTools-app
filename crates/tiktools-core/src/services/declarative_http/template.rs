//! Scoped template rendering for declarative requests.

use crate::helpers::value_to_string;
use serde_json::Value;

/// Renders `{{ dotted.path }}` templates against a scope object such as
/// `{"event": …, "settings": …, "config": …}`. Unknown paths render empty,
/// matching the automation template behavior.
pub(crate) fn render_scoped_template(source: &str, scope: &Value) -> String {
    let mut rendered = String::with_capacity(source.len());
    let mut rest = source;
    while let Some(start) = rest.find("{{") {
        rendered.push_str(&rest[..start]);
        let expression = &rest[start + 2..];
        let Some(end) = expression.find("}}") else {
            rendered.push_str(&rest[start..]);
            return rendered;
        };
        let path = expression[..end].trim();
        if let Some(value) = read_scope_path(scope, path) {
            rendered.push_str(&value_to_string(value));
        }
        rest = &expression[end + 2..];
    }
    rendered.push_str(rest);
    rendered
}

/// Recursively renders `{{ }}` spans in every string leaf of a JSON value.
/// Declarative `bodyTransforms` specs use it so `{{ config.* }}`/`{{ settings.* }}`
/// resolve host-side while `{{ event.* }}`/`{{ match.* }}` spans survive for
/// the executor's second render pass.
pub(crate) fn render_scoped_strings(value: &Value, scope: &Value) -> Value {
    match value {
        Value::String(source) => Value::String(render_scoped_template(source, scope)),
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| render_scoped_strings(item, scope))
                .collect(),
        ),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, item)| (key.clone(), render_scoped_strings(item, scope)))
                .collect(),
        ),
        _ => value.clone(),
    }
}

fn read_scope_path<'a>(scope: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = scope;
    for part in path.split('.') {
        if part.is_empty() {
            continue;
        }
        current = current.get(part)?;
    }
    Some(current)
}

/// Structured JSON templates never substitute into serialized JSON text.
/// `$number` explicitly converts numeric form values; `$value` preserves types.
pub(super) fn render_json(value: &Value, scope: &Value) -> Result<Value, String> {
    match value {
        Value::Object(map) if map.contains_key("$value") || map.contains_key("$number") => {
            let numeric = map.contains_key("$number");
            let path = map
                .get(if numeric { "$number" } else { "$value" })
                .and_then(Value::as_str)
                .ok_or("JSON binding requires a scope path")?;
            let resolved = read_scope_path(scope, path)
                .filter(|v| !v.is_null() && v.as_str() != Some(""))
                .or_else(|| {
                    map.get("fallback")
                        .and_then(Value::as_str)
                        .and_then(|p| read_scope_path(scope, p))
                })
                .filter(|v| !v.is_null() && v.as_str() != Some(""))
                .ok_or_else(|| format!("Missing required configuration: {path}"))?;
            if numeric {
                let number = resolved
                    .as_f64()
                    .or_else(|| resolved.as_str()?.parse().ok())
                    .filter(|n| n.is_finite())
                    .ok_or_else(|| format!("Invalid number: {path}"))?;
                if map
                    .get("min")
                    .and_then(Value::as_f64)
                    .is_some_and(|min| number < min)
                    || map
                        .get("max")
                        .and_then(Value::as_f64)
                        .is_some_and(|max| number > max)
                    || (map.get("integer").and_then(Value::as_bool) == Some(true)
                        && number.fract() != 0.0)
                {
                    return Err(format!("Number out of range: {path}"));
                }
                return Ok(
                    if map.get("integer").and_then(Value::as_bool) == Some(true) {
                        serde_json::json!(number as i64)
                    } else {
                        serde_json::json!(number)
                    },
                );
            }
            Ok(resolved.clone())
        }
        Value::Object(map) => map
            .iter()
            .map(|(k, v)| Ok((k.clone(), render_json(v, scope)?)))
            .collect::<Result<serde_json::Map<String, Value>, String>>()
            .map(Value::Object),
        Value::Array(items) => items
            .iter()
            .map(|v| render_json(v, scope))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        Value::String(s) => Ok(Value::String(render_scoped_template(s, scope))),
        _ => Ok(value.clone()),
    }
}

#[cfg(test)]
mod json_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn structured_body_escapes_strings_without_recursive_expansion() {
        let text = "quote\"\\\n{{ event.secret }}";
        let rendered = render_json(
            &json!({"id":{"$value":"config.id"}}),
            &json!({"config":{"id":text}}),
        )
        .unwrap();
        let roundtrip: Value = serde_json::from_str(&rendered.to_string()).unwrap();
        assert_eq!(roundtrip["id"], text);
    }

    #[test]
    fn nested_position_and_bounded_quantity() {
        let template = json!({"quantity":{"$number":"config.quantity","min":1,"max":10,"integer":true},
            "position":{"mode":"near_player","playerId":{"$value":"config.playerId"}}});
        let body = render_json(
            &template,
            &json!({"config":{"quantity":"2","playerId":"session-id"}}),
        )
        .unwrap();
        assert_eq!(body["quantity"].as_f64(), Some(2.0));
        assert_eq!(body["position"]["playerId"], "session-id");
        for quantity in ["0", "11", "1.5", "NaN"] {
            assert!(render_json(
                &template,
                &json!({"config":{"quantity":quantity,"playerId":"p"}})
            )
            .is_err());
        }
    }
}
