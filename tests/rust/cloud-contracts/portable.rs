use cloud_contracts::portable::{
    document, is_collection, safe, valid_key, validate, values_equivalent, COLLECTIONS,
};
use serde_json::{json, Value};

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
