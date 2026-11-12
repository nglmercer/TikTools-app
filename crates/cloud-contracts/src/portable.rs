//! Portable cloud documents. Validation never executes actions or fetches URLs.
use serde_json::Value;

pub const MAX_DOCUMENT_BYTES: usize = 256 * 1024;
/// The collections every v2 client knows. `GET /api/v2/capabilities` still
/// announces exactly these in `sync.collections`, because deployed clients
/// require that list to match their own; do not add to it.
pub const COLLECTIONS: &[&str] = &[
    "plugin-selections",
    "rule-templates",
    "profiles",
    "plugin-settings",
    "preferences",
];
/// Collections added after v2 shipped. They are announced separately
/// (`sync.optionalCollections`) so older clients keep validating the
/// capability document. A client uses one only when the server announces it.
pub const OPTIONAL_COLLECTIONS: &[&str] = &["variables"];

/// Every collection the server accepts.
pub fn is_collection(collection: &str) -> bool {
    COLLECTIONS.contains(&collection) || OPTIONAL_COLLECTIONS.contains(&collection)
}

/// `variables` value types. `secret` is deliberately not one of them: secrets
/// never sync.
pub const VARIABLE_TYPES: &[&str] = &["text", "number", "boolean", "list"];
pub const MAX_VARIABLE_NAME_BYTES: usize = 64;
pub const MAX_VARIABLE_TEXT_BYTES: usize = 4096;
pub const MAX_VARIABLE_LIST_ITEMS: usize = 100;
pub const MAX_VARIABLE_LIST_ITEM_CHARS: usize = 256;

/// The desktop's runtime-global key grammar: `^[A-Za-z_][A-Za-z0-9_.-]{0,63}$`.
pub fn valid_variable_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    (1..=MAX_VARIABLE_NAME_BYTES).contains(&bytes.len())
        && (bytes[0].is_ascii_alphabetic() || bytes[0] == b'_')
        && bytes[1..]
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(b))
}

fn variable_value(kind: &str, value: &Value) -> bool {
    match kind {
        "text" => value
            .as_str()
            .is_some_and(|s| s.len() <= MAX_VARIABLE_TEXT_BYTES),
        "number" => value.as_f64().is_some_and(f64::is_finite),
        "boolean" => value.is_boolean(),
        "list" => value.as_array().is_some_and(|items| {
            items.len() <= MAX_VARIABLE_LIST_ITEMS
                && items.iter().all(|item| {
                    item.as_str()
                        .is_some_and(|s| s.chars().count() <= MAX_VARIABLE_LIST_ITEM_CHARS)
                })
        }),
        _ => false,
    }
}

fn variable(key: &str, value: &Value) -> bool {
    object(value, &["schemaVersion", "name", "type", "value"])
        && value.as_object().is_some_and(|map| map.len() == 4)
        && value["schemaVersion"] == 1
        && value["name"].as_str().is_some_and(|name| name == key)
        && valid_variable_name(key)
        && value["type"].as_str().is_some_and(|kind| {
            VARIABLE_TYPES.contains(&kind) && variable_value(kind, &value["value"])
        })
}

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
        "variables" => valid_variable_name(key),
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
        "variables" => variable(key, value),
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
    // Retain all previously valid v2 keys while supporting the manifest ID
    // grammar, including official legacy IDs such as `hotkeys` and `textintel`.
    let legacy = id.len() <= 128
        && id.split_once('.').is_some()
        && id.split('.').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"-_".contains(&b))
        });
    let bytes = id.as_bytes();
    let manifest = (2..=128).contains(&bytes.len())
        && bytes[0].is_ascii_lowercase()
        && bytes[1..].iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        });
    legacy || manifest
}
fn plugin_selection(key: &str, value: &Value) -> bool {
    object(value, &["schemaVersion", "pluginId", "version", "enabled"])
        && value["schemaVersion"] == 1
        && value["pluginId"] == key
        && value["enabled"].is_boolean()
        && value["version"]
            .as_str()
            .is_some_and(|s| s.len() <= 128 && semver::Version::parse(s).is_ok())
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
    fn workspace_accepts_manifest_ids_and_existing_keys_without_truncating_versions() {
        let version = format!("1.0.0+{}", "a".repeat(122));
        assert_eq!(version.len(), 128);
        for id in ["hotkeys", "textintel", "ab..foo", "1.old"] {
            assert!(valid_key("plugin-selections", id));
            assert!(valid_key("plugin-settings", id));
            assert!(validate(
                "plugin-selections",
                id,
                &json!({
                    "schemaVersion":1,"pluginId":id,"version":version,"enabled":true
                })
            ));
        }
        assert!(!valid_key("plugin-selections", "A.bad"));
        assert!(!valid_key("plugin-selections", "../outside"));
        let overlong = format!("{version}a");
        assert!(!validate(
            "plugin-selections",
            "hotkeys",
            &json!({
                "schemaVersion":1,"pluginId":"hotkeys","version":overlong,"enabled":true
            })
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
    fn variable(name: &str, kind: &str, value: Value) -> Value {
        json!({"schemaVersion":1,"name":name,"type":kind,"value":value})
    }
    #[test]
    fn variables_accept_each_type_with_a_matching_value() {
        for (name, kind, value) in [
            ("commandPort", "number", json!(46665)),
            ("ratio", "number", json!(0.25)),
            ("negative", "number", json!(-3)),
            ("_flag", "boolean", json!(false)),
            ("greeting.text-1_a", "text", json!("hola")),
            ("empty", "text", json!("")),
            ("tags", "list", json!(["a", "b"])),
            ("none", "list", json!([])),
            ("A1._-b", "text", json!("x")),
        ] {
            let doc = variable(name, kind, value);
            assert!(validate("variables", name, &doc), "{doc}");
            assert!(valid_key("variables", name));
        }
        assert!(is_collection("variables") && is_collection("preferences"));
        assert!(!is_collection("secrets"));
        assert!(
            !COLLECTIONS.contains(&"variables"),
            "the legacy list announced to deployed clients must not grow"
        );
    }
    #[test]
    fn variables_refuse_secrets_wrong_types_bad_names_and_oversize_values() {
        let ok = |n: &str, k: &str, v: Value| validate("variables", n, &variable(n, k, v));
        // Secrets never sync, whatever the value looks like.
        assert!(!ok("apiKey", "secret", json!("hunter2")));
        assert!(!ok("apiKey", "password", json!("x")));
        assert!(!ok("x", "Text", json!("x")));
        // The value must match the declared type.
        assert!(!ok("a", "text", json!(1)));
        assert!(!ok("a", "number", json!("1")));
        assert!(!ok("a", "number", json!(null)));
        assert!(!ok("a", "boolean", json!("true")));
        assert!(!ok("a", "list", json!("a,b")));
        assert!(!ok("a", "list", json!([1])));
        assert!(!ok("a", "list", json!([["nested"]])));
        // Bounds.
        assert!(ok("a", "text", json!("a".repeat(4096))));
        assert!(!ok("a", "text", json!("a".repeat(4097))));
        assert!(ok("a", "list", json!(vec!["a"; 100])));
        assert!(!ok("a", "list", json!(vec!["a"; 101])));
        assert!(ok("a", "list", json!(["ñ".repeat(256)])));
        assert!(!ok("a", "list", json!(["a".repeat(257)])));
        // Key grammar.
        for name in [
            "",
            "9lives",
            "has space",
            "semi;colon",
            "slash/a",
            "{{ x }}",
            "é",
        ] {
            assert!(!valid_key("variables", name), "{name:?}");
        }
        assert!(valid_key("variables", &"k".repeat(64)));
        assert!(!valid_key("variables", &"k".repeat(65)));
        // Key and name must agree; fields are closed and complete.
        assert!(!validate(
            "variables",
            "a",
            &variable("b", "text", json!("x"))
        ));
        let mut extra = variable("a", "text", json!("x"));
        extra["secret"] = json!(true);
        assert!(!validate("variables", "a", &extra));
        let mut version = variable("a", "text", json!("x"));
        version["schemaVersion"] = json!(2);
        assert!(!validate("variables", "a", &version));
        assert!(!validate(
            "variables",
            "a",
            &json!({"schemaVersion":1,"name":"a","type":"text"})
        ));
        assert!(!validate(
            "variables",
            "a",
            &json!({"schemaVersion":1,"name":"a","value":"x"})
        ));
        assert!(!validate("variables", "a", &json!(["a"])));
        // The general credential guards still apply to values.
        assert!(!ok("a", "text", json!("Bearer abcdef")));
        assert!(!ok("a", "text", json!("https://user:pw@example.test/")));
        assert!(!ok("a", "text", json!("-----BEGIN PRIVATE KEY-----")));
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
