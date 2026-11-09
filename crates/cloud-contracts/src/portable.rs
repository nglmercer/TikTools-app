//! Portable cloud documents. Validation never executes actions or fetches URLs.
use serde_json::Value;

pub const MAX_DOCUMENT_BYTES: usize = 256 * 1024;
pub const COLLECTIONS: &[&str] = &[
    "plugin-selections",
    "rule-templates",
    "profiles",
    "plugin-settings",
    "preferences",
];

pub fn valid_id(id: &str) -> bool {
    (2..=64).contains(&id.len())
        && id.as_bytes()[0].is_ascii_alphanumeric()
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._-".contains(&b))
}

pub fn valid_key(collection: &str, key: &str) -> bool {
    match collection {
        "plugin-selections" | "plugin-settings" => valid_plugin_id(key),
        "rule-templates" | "profiles" => valid_id(key),
        "preferences" => key == "desktop",
        _ => false,
    }
}

fn object(value: &Value, keys: &[&str]) -> bool {
    value
        .as_object()
        .is_some_and(|map| map.keys().all(|k| keys.contains(&k.as_str())))
}
fn text(value: &Value, min: usize, max: usize) -> bool {
    value
        .as_str()
        .is_some_and(|s| (min..=max).contains(&s.len()) && (min == 0 || !s.trim().is_empty()))
}
fn localized(value: &Value, max: usize) -> bool {
    text(value, 1, max)
        || (object(value, &["default", "i18key"])
            && text(&value["default"], 1, max)
            && (value.get("i18key").is_none() || text(&value["i18key"], 1, 128)))
}
fn optional(value: &Value, key: &str, check: impl FnOnce(&Value) -> bool) -> bool {
    value.get(key).is_none_or(check)
}
fn reference(value: &Value) -> bool {
    let identifier = |s: &str| {
        !s.is_empty()
            && s.len() <= 128
            && s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
    };
    (object(value, &["secretRef"]) && value["secretRef"].as_str().is_some_and(identifier))
        || value.as_str().is_some_and(|s| {
            let s = s.trim();
            s.strip_prefix("{{ globals.")
                .and_then(|s| s.strip_suffix(" }}"))
                .is_some_and(identifier)
        })
}
fn sensitive(key: &str) -> bool {
    let key: String = key
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect();
    matches!(
        key.as_str(),
        "password"
            | "passwd"
            | "token"
            | "accesstoken"
            | "refreshtoken"
            | "apikey"
            | "secret"
            | "clientsecret"
            | "authorization"
            | "cookie"
            | "setcookie"
            | "privatekey"
            | "signingkey"
            | "credential"
            | "credentials"
            | "authsecret"
    ) || key.ends_with("password")
        || key.ends_with("token")
        || key.ends_with("secret")
}
fn safe_tree(value: &Value, depth: usize, budget: &mut usize) -> bool {
    if depth > 24 || *budget == 0 {
        return false;
    }
    *budget -= 1;
    match value {
        Value::Object(map) => {
            map.len() <= 256
                && map.iter().all(|(key, value)| {
                    key.len() <= 128
                        && !matches!(
                            key.as_str(),
                            "registryUrl" | "registryOrigin" | "rootKeys" | "registrySources"
                        )
                        && (!sensitive(key) || reference(value))
                        && safe_tree(value, depth + 1, budget)
                })
        }
        Value::Array(items) => {
            items.len() <= 256 && items.iter().all(|v| safe_tree(v, depth + 1, budget))
        }
        Value::String(s) => {
            if s.len() > 8192 || s.contains('\0') {
                return false;
            }
            // Do not resolve template expressions. Reject credentials in concrete URLs.
            if let Ok(url) = url::Url::parse(s) {
                if !url.username().is_empty()
                    || url.password().is_some()
                    || url.query_pairs().any(|(k, _)| sensitive(&k))
                {
                    return false;
                }
            }
            if s.starts_with("-----BEGIN ") || s.starts_with("Bearer ") {
                return false;
            }
            if let Ok(nested) = serde_json::from_str::<Value>(s) {
                if nested.is_object() || nested.is_array() {
                    return safe_tree(&nested, depth + 1, budget);
                }
            }
            true
        }
        _ => true,
    }
}

pub fn safe(value: &Value) -> bool {
    serde_json::to_vec(value).is_ok_and(|v| v.len() <= MAX_DOCUMENT_BYTES)
        && safe_tree(value, 0, &mut 8192)
}

fn template_shape(doc: &Value) -> bool {
    object(
        doc,
        &[
            "templateVersion",
            "id",
            "title",
            "description",
            "icon",
            "params",
            "actions",
            "event",
        ],
    ) && doc["templateVersion"] == 1
        && doc["id"].as_str().is_some_and(valid_id)
        && localized(&doc["title"], 120)
        && optional(doc, "description", |v| text(v, 0, 500) || localized(v, 500))
        && optional(doc, "icon", |v| text(v, 1, 32))
        && optional(doc, "params", Value::is_object)
        && doc["actions"].as_array().is_some_and(|items| {
            (1..=8).contains(&items.len())
                && items.iter().all(|a| {
                    object(a, &["name", "typeId", "enabled", "config"])
                        && text(&a["name"], 1, 80)
                        && text(&a["typeId"], 1, 128)
                        && optional(a, "enabled", Value::is_boolean)
                        && a["config"].is_object()
                })
        })
        && event(&doc["event"])
}
fn event(doc: &Value) -> bool {
    object(
        doc,
        &[
            "name",
            "enabled",
            "trigger",
            "filters",
            "cooldownMs",
            "cooldownScope",
            "runMode",
        ],
    ) && text(&doc["name"], 1, 80)
        && text(&doc["trigger"], 1, 128)
        && optional(doc, "enabled", Value::is_boolean)
        && optional(doc, "cooldownMs", |v| {
            v.as_u64().is_some_and(|n| n <= 86_400_000)
        })
        && optional(doc, "cooldownScope", |v| {
            matches!(v.as_str(), Some("global" | "user"))
        })
        && optional(doc, "runMode", |v| {
            matches!(v.as_str(), Some("all" | "random"))
        })
        && optional(doc, "filters", |v| {
            v.as_array().is_some_and(|items| {
                items.len() <= 12
                    && items.iter().all(|f| {
                        object(f, &["path", "operator", "value", "values"])
                            && text(&f["path"], 1, 200)
                            && matches!(
                                f["operator"].as_str(),
                                Some(
                                    "gte"
                                        | "gt"
                                        | "lte"
                                        | "lt"
                                        | "eq"
                                        | "neq"
                                        | "contains"
                                        | "starts-with"
                                        | "in"
                                        | "is-true"
                                        | "is-false"
                                )
                            )
                            && optional(f, "value", |v| text(v, 0, 500))
                            && optional(f, "values", |v| {
                                v.as_array().is_some_and(|items| {
                                    items.len() <= 64 && items.iter().all(|v| text(v, 0, 500))
                                })
                            })
                    })
            })
        })
}
fn profile_shape(doc: &Value) -> bool {
    object(
        doc,
        &[
            "profileVersion",
            "id",
            "name",
            "description",
            "params",
            "templates",
        ],
    ) && doc["profileVersion"] == 1
        && doc["id"].as_str().is_some_and(valid_id)
        && text(&doc["name"], 1, 120)
        && optional(doc, "description", |v| text(v, 0, 500))
        && optional(doc, "params", Value::is_object)
        && doc["templates"].as_array().is_some_and(|items| {
            (1..=32).contains(&items.len())
                && items.iter().all(|entry| {
                    object(entry, &["template", "params"])
                        && template_shape(&entry["template"])
                        && optional(entry, "params", Value::is_object)
                })
        })
}
pub fn document(kind: &str, doc: &Value) -> bool {
    safe(doc)
        && match kind {
            "template" => template_shape(doc),
            "profile" => profile_shape(doc),
            _ => false,
        }
}
pub fn validate(collection: &str, key: &str, value: &Value) -> bool {
    if !valid_key(collection, key) || !safe(value) {
        return false;
    }
    match collection {
        "plugin-selections" => plugin_selection(key, value),
        "rule-templates" => template_shape(value) && value["id"] == key,
        "profiles" => profile_shape(value) && value["id"] == key,
        "plugin-settings" => {
            object(value, &["schemaVersion", "pluginId", "settings"])
                && value["schemaVersion"] == 1
                && value["pluginId"] == key
                && value["settings"].is_object()
        }
        "preferences" => {
            object(
                value,
                &[
                    "schemaVersion",
                    "theme",
                    "locale",
                    "reducedMotion",
                    "updateChannel",
                ],
            ) && value["schemaVersion"] == 1
                && optional(value, "theme", |v| {
                    matches!(v.as_str(), Some("system" | "light" | "dark"))
                })
                && optional(value, "locale", |v| {
                    v.as_str().is_some_and(|s| {
                        (2..=35).contains(&s.len())
                            && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
                    })
                })
                && optional(value, "reducedMotion", Value::is_boolean)
                && optional(value, "updateChannel", |v| {
                    matches!(v.as_str(), Some("stable" | "beta"))
                })
        }
        _ => false,
    }
}

fn valid_plugin_id(id: &str) -> bool {
    id.len() <= 128
        && id.split_once('.').is_some()
        && id.split('.').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"-_".contains(&b))
        })
}
fn plugin_selection(key: &str, value: &Value) -> bool {
    object(value, &["schemaVersion", "pluginId", "version", "enabled"])
        && value["schemaVersion"] == 1
        && value["pluginId"] == key
        && value["enabled"].is_boolean()
        && value["version"]
            .as_str()
            .is_some_and(|s| s.len() <= 64 && semver::Version::parse(s).is_ok())
}

/// Compare JSON data across PostgreSQL JSONB numeric normalization. Numbers
/// retain their exact decimal wire value; converting them all to f64 would
/// collapse distinct large integers and misread shortest float encodings.
/// Object keys, strings, array order and every other JSON type stay exact.
pub fn values_equivalent(left: &Value, right: &Value) -> bool {
    fn decimal(number: &serde_json::Number) -> Option<(bool, String, i64)> {
        let token = number.to_string();
        let (negative, token) = match token.strip_prefix('-') {
            Some(token) => (true, token),
            None => (false, token.as_str()),
        };
        let (mantissa, exponent) = match token.find(['e', 'E']) {
            Some(index) => (&token[..index], token[index + 1..].parse::<i64>().ok()?),
            None => (token, 0),
        };
        let fractional = mantissa
            .split_once('.')
            .map_or(0, |(_, fraction)| fraction.len());
        let digits: String = mantissa
            .chars()
            .filter(|character| *character != '.')
            .collect();
        if digits.is_empty() || !digits.bytes().all(|digit| digit.is_ascii_digit()) {
            return None;
        }
        let significant = digits.trim_start_matches('0');
        if significant.is_empty() {
            return Some((false, "0".into(), 0));
        }
        let reduced = significant.trim_end_matches('0');
        let exponent = exponent
            .checked_sub(i64::try_from(fractional).ok()?)?
            .checked_add(i64::try_from(significant.len() - reduced.len()).ok()?)?;
        Some((negative, reduced.into(), exponent))
    }
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => match (decimal(left), decimal(right)) {
            (Some(left), Some(right)) => left == right,
            _ => false,
        },
        (Value::Array(left), Value::Array(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(left, right)| values_equivalent(left, right))
        }
        (Value::Object(left), Value::Object(right)) => {
            left.len() == right.len()
                && left.iter().all(|(key, left)| {
                    right
                        .get(key)
                        .is_some_and(|right| values_equivalent(left, right))
                })
        }
        _ => left == right,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    pub fn template() -> Value {
        json!({"templateVersion":1,"id":"gift-rule","title":"Gift rule","actions":[{"name":"Notify","typeId":"core.notify","config":{}}],"event":{"name":"Gift","trigger":"tiktok.gift"}})
    }
    #[test]
    fn desktop_v1_interchange_fixture_is_accepted() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/templates/minecraft-gift-command.tiktemplate.json"
        ))
        .unwrap();
        assert!(document("template", &fixture));
    }
    #[test]
    fn documents_and_portable_collections_validate() {
        let template = template();
        assert!(document("template", &template));
        assert!(validate("rule-templates", "gift-rule", &template));
        assert!(!validate("rule-templates", "another-rule", &template));
        assert!(document(
            "profile",
            &json!({"profileVersion":1,"id":"gift-pack","name":"Gift pack","templates":[{"template":template}]})
        ));
        assert!(validate(
            "plugin-settings",
            "acme.plugin",
            &json!({"schemaVersion":1,"pluginId":"acme.plugin","settings":{"volume":50}})
        ));
        assert!(validate(
            "preferences",
            "desktop",
            &json!({"schemaVersion":1,"locale":"es-PE","theme":"dark"})
        ));
    }
    #[test]
    fn secrets_sources_and_unbounded_documents_are_refused() {
        for value in [
            json!({"token":"secret"}),
            json!({"headers":{"Authorization":"Bearer secret"}}),
            json!({"url":"https://user:pass@example.test"}),
            json!({"url":"https://example.test/?api_key=secret"}),
            json!({"registryUrl":"https://other.test"}),
            json!({"large":"a".repeat(8193)}),
        ] {
            assert!(!safe(&value));
        }
        assert!(safe(&json!({"password":{"secretRef":"minecraft"}})));
        assert!(safe(
            &json!({"headers":{"Authorization":"{{ globals.authHeader }}"}})
        ));
        let mut bad = template();
        bad["actions"][0]["script"] = json!("unknown");
        assert!(!document("template", &bad));
        let mut deep = json!(0);
        for _ in 0..26 {
            deep = json!([deep]);
        }
        assert!(!safe(&deep));
    }
    #[test]
    fn jsonb_numeric_equivalence_preserves_exact_decimal_wire_values() {
        let parse = |text| serde_json::from_str::<Value>(text).unwrap();
        for (left, right) in [
            ("1e18", "1000000000000000000"),
            ("1.0000000000000001e18", "1000000000000000100"),
            ("1.0", "1"),
            ("-0.0", "0"),
            ("1e-6", "0.000001"),
            (
                r#"{"data":[1e18,-0.0,{"n":1.0}],"text":"ñ"}"#,
                r#"{"text":"ñ","data":[1000000000000000000,0,{"n":1}]}"#,
            ),
        ] {
            let (left, right) = (parse(left), parse(right));
            assert!(values_equivalent(&left, &right), "{left} != {right}");
            assert!(values_equivalent(&right, &left));
        }
        for (left, right) in [
            ("18446744073709551615", "18446744073709551614"),
            ("18446744073709551615", "1.8446744073709552e19"),
            ("1.0000000000000001e18", "1000000000000000128"),
            ("1e18", "1000000000000000001"),
            (r#"[1,2]"#, r#"[2,1]"#),
            (r#"{"n":1}"#, r#"{"n":"1"}"#),
            (r#"{"n":1}"#, r#"{"n":1,"extra":null}"#),
        ] {
            assert!(
                !values_equivalent(&parse(left), &parse(right)),
                "{left} == {right}"
            );
        }
    }
}
