use super::*;
use serde_json::json;

fn minimal_doc() -> Value {
    json!({
        "templateVersion": 1,
        "id": "gift-log",
        "title": "Gift log",
        "actions": [{"name": "Log", "typeId": "core.log", "config": {"message": "hi"}}],
        "event": {"name": "Gift", "trigger": "tiktok.gift"},
    })
}

#[test]
fn minimal_template_normalizes_defaults() {
    let template = parse_rule_template(&minimal_doc()).expect("valid");
    assert_eq!(template.id, "gift-log");
    assert_eq!(template.icon, "plugin");
    assert!(template.actions[0].enabled);
    assert_eq!(template.event.cooldown_scope, "global");
    assert_eq!(template.event.run_mode, "all");
    assert!(template.event.filters.is_empty());
}

#[test]
fn malformed_templates_list_every_problem() {
    assert!(parse_rule_template(&Value::Null).is_err());
    let mut version = minimal_doc();
    version["templateVersion"] = json!(99);
    assert!(parse_rule_template(&version).is_err());
    let mut id = minimal_doc();
    id["id"] = json!("../evil");
    assert!(parse_rule_template(&id).is_err());
    let mut actions = minimal_doc();
    actions["actions"] = json!([]);
    assert!(parse_rule_template(&actions).is_err());
    let mut operator = minimal_doc();
    operator["event"]["filters"] = json!([{"path": "p", "operator": "nope", "value": ""}]);
    let errors = parse_rule_template(&operator).expect_err("bad operator").0;
    assert!(errors.iter().any(|error| error.contains("operator")));
}

#[test]
fn list_import_accepts_single_array_and_wrapped_docs() {
    let (single, _) = parse_rule_template_list(&minimal_doc());
    assert_eq!(single.len(), 1);
    let (pair, _) = parse_rule_template_list(&json!([minimal_doc(), minimal_doc()]));
    assert_eq!(pair.len(), 2);
    let (wrapped, _) = parse_rule_template_list(&json!({"templates": [minimal_doc()]}));
    assert_eq!(wrapped.len(), 1);
    let (mixed, errors) = parse_rule_template_list(&json!([minimal_doc(), {"templateVersion": 1}]));
    assert_eq!(mixed.len(), 1);
    assert!(!errors.is_empty());
}

#[test]
fn params_substitute_raw_for_sole_spans_and_text_otherwise() {
    let mut params = Map::new();
    params.insert("delta".to_owned(), json!(10));
    assert_eq!(
        substitute_params(&json!("{{ params.delta }}"), &params),
        json!(10)
    );
    assert_eq!(
        substitute_params(&json!("n={{ params.delta }}!"), &params),
        json!("n=10!")
    );
    assert_eq!(
        substitute_params(&json!("{{ event.user.uniqueId }}"), &params),
        json!("{{ event.user.uniqueId }}")
    );
    assert_eq!(
        substitute_params(&json!("{{ params.missing }}"), &params),
        json!("")
    );
}

#[test]
fn param_defaults_and_merge_follow_layer_order() {
    let schema = json!({"type": "object", "properties": {"url": {"default": "https://"}}});
    let defaults = param_defaults(schema.as_object());
    assert_eq!(defaults.get("url"), Some(&json!("https://")));
    let mut profile = Map::new();
    profile.insert("port".to_owned(), json!(1));
    let mut entry = Map::new();
    entry.insert("port".to_owned(), json!(2));
    entry.insert("name".to_owned(), json!("dropped"));
    let merged = merge_params(&[&defaults, &profile, &entry]);
    assert_eq!(merged.get("port"), Some(&json!(2)));
    assert!(!merged.contains_key("name"));
}

#[test]
fn instantiate_assigns_fresh_ids_and_keeps_runtime_spans() {
    let template = parse_rule_template(&json!({
        "templateVersion": 1,
        "id": "points",
        "title": "Points",
        "actions": [{"name": "P {{ params.delta }}", "typeId": "core.points",
            "config": {"delta": "{{ params.delta }}", "note": "{{ event.type }}"}}],
        "event": {"name": "Chat", "trigger": "tiktok.chat"},
    }))
    .expect("valid");
    let mut params = Map::new();
    params.insert("delta".to_owned(), json!(5));
    let first = instantiate_template(&template, &params, None, &[]);
    let second = instantiate_template(&template, &params, Some("Custom"), &[]);
    assert_eq!(first.actions[0]["config"]["delta"], json!(5));
    assert_eq!(
        first.actions[0]["config"]["note"],
        json!("{{ event.type }}")
    );
    assert_eq!(first.actions[0]["name"], json!("P 5"));
    assert_eq!(first.event["actionIds"][0], first.actions[0]["id"]);
    assert_ne!(first.event["id"], second.event["id"]);
    assert_ne!(first.actions[0]["id"], second.actions[0]["id"]);
    assert_eq!(second.event["name"], json!("Custom"));
}

#[test]
fn profiles_validate_entries_and_share_params() {
    let profile = parse_profile(&json!({
        "profileVersion": 1,
        "id": "minecraft",
        "name": "Minecraft",
        "params": {"commandPort": 8080},
        "templates": [
            {"template": minimal_doc(), "params": {"extra": true}},
            {"template": minimal_doc()},
        ],
    }))
    .expect("valid");
    assert_eq!(profile.entries.len(), 2);
    assert_eq!(profile.params.get("commandPort"), Some(&json!(8080)));
    assert_eq!(
        profile.entries[0].params.get("extra"),
        Some(&Value::Bool(true))
    );
    assert!(parse_profile(&json!({"profileVersion": 99})).is_err());
    assert!(parse_profile(&json!({
        "profileVersion": 1, "id": "x", "name": "X", "templates": []
    }))
    .is_err());
}

#[test]
fn shared_fixtures_agree_with_the_web_engine() {
    let valid: Value = serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/templates/valid-minimal.json"
    ))
    .expect("fixture parses");
    assert!(parse_rule_template(&valid).is_ok());
    for name in ["invalid-version.json", "invalid-operator.json"] {
        let text = match name {
            "invalid-version.json" => {
                include_str!("../../../../../tests/fixtures/templates/invalid-version.json")
            }
            _ => include_str!("../../../../../tests/fixtures/templates/invalid-operator.json"),
        };
        let doc: Value = serde_json::from_str(text).expect("fixture parses");
        assert!(parse_rule_template(&doc).is_err(), "{name} must fail");
    }
}

#[test]
fn requirements_dedupe_action_types() {
    let template = parse_rule_template(&json!({
        "templateVersion": 1,
        "id": "two",
        "title": "Two",
        "actions": [
            {"name": "A", "typeId": "core.log", "config": {}},
            {"name": "B", "typeId": "core.log", "config": {}},
        ],
        "event": {"name": "E", "trigger": "tiktok.chat"},
    }))
    .expect("valid");
    assert_eq!(
        template_requirements(&template),
        (vec!["core.log".to_owned()], "tiktok.chat".to_owned())
    );
}

#[test]
fn minecraft_giftwall_preset_parses_with_titles_on_every_rule() {
    let raw: Value = serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/profiles/minecraft-giftwall.tikprofile.json"
    ))
    .expect("preset must be valid JSON");
    let profile = parse_profile(&raw).expect("preset must parse");
    assert_eq!(profile.id, "minecraft-giftwall");
    assert_eq!(profile.entries.len(), 16);
    for entry in &profile.entries {
        let template = &entry.template;
        // Every rule runs only Minecraft plugin actions.
        assert!(!template.actions.is_empty(), "{}", template.id);
        for action in &template.actions {
            assert!(
                action.type_id.starts_with("minecraft.server."),
                "{} uses {}",
                template.id,
                action.type_id
            );
        }
        // Every gift, follow, and share flashes a title + subtitle.
        let commands: Vec<&str> = template
            .actions
            .iter()
            .filter_map(|action| action.config.get("command"))
            .filter_map(Value::as_str)
            .collect();
        for needle in [
            "/title @a times ",
            "/title @a subtitle ",
            "/title @a title ",
        ] {
            assert!(
                commands.iter().any(|command| command.contains(needle)),
                "{} is missing `{needle}`",
                template.id
            );
        }
    }
    let triggers: Vec<&str> = profile
        .entries
        .iter()
        .map(|entry| entry.template.event.trigger.as_str())
        .collect();
    assert!(triggers.contains(&"tiktok.gift"));
    assert!(triggers.contains(&"tiktok.follow"));
    assert!(triggers.contains(&"tiktok.share"));
}
