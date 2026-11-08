use super::*;
use serde_json::json;

fn template(config: Value) -> RuleTemplate {
    parse_rule_template(&json!({"templateVersion":1,"id":"bounded-template","title":"Bounded","actions":[{"name":"A {{ params.number }}","typeId":"core.log","config":config}],"event":{"name":"Event {{ params.number }}","trigger":"tiktok.chat","filters":[{"path":"event.name","operator":"eq","value":"{{ params.filter }}","values":["{{ params.number }}","runtime {{ event.type }}"]}]}})).unwrap()
}
fn serialized_entry(result: &InstantiatedRule) -> usize {
    serde_json::to_vec(&json!({"actions":result.actions,"event":result.event}))
        .unwrap()
        .len()
}

#[test]
fn preflight_exactly_matches_resolved_json_names_filters_and_stable_id_references() {
    let template = template(
        json!({"raw":"{{ params.object }}","inline":"obj={{ params.object }}","number":"{{ params.number }}","runtime":"{{ event.type }}","missing":"{{ params.missing }}","escaped":"ñ😀\n\u{0000}\\\""}),
    );
    let params =
        json!({"object":{"nested":[1,true,"quote\"\\\nñ😀"]},"number":5,"filter":"matched"})
            .as_object()
            .unwrap()
            .clone();
    let mut budget = TemplateInstantiationBudget::new();
    let first = instantiate_template_bounded(
        &template,
        &params,
        Some("  Override  "),
        &["  First action  ".into()],
        &mut budget,
    )
    .unwrap();
    assert_eq!(first.actions[0]["name"], "First action");
    assert_eq!(first.event["name"], "Override");
    assert_eq!(first.actions[0]["config"]["raw"], params["object"]);
    assert_eq!(
        first.actions[0]["config"]["inline"],
        format!("obj={}", params["object"])
    );
    assert_eq!(first.event["filters"][0]["value"], "matched");
    assert_eq!(first.event["filters"][0]["values"][0], 5);
    assert_eq!(first.event["actionIds"][0], first.actions[0]["id"]);
    assert_eq!(budget.used_bytes(), 2 + serialized_entry(&first));
    let second = instantiate_template_bounded(&template, &params, None, &[], &mut budget).unwrap();
    assert_eq!(second.actions[0]["name"], "A 5");
    assert_eq!(second.event["name"], "Event 5");
    assert_ne!(first.actions[0]["id"], second.actions[0]["id"]);
    assert_ne!(first.event["id"], second.event["id"]);
    assert_eq!(
        budget.used_bytes(),
        2 + serialized_entry(&first) + 1 + serialized_entry(&second)
    );
}

#[test]
fn small_repeated_inline_input_rejects_before_materializing_hundreds_of_megabytes() {
    let template = template(json!({"message":"x{{ params.large }}".repeat(4096)}));
    let params = json!({"large":"x".repeat(200_000)})
        .as_object()
        .unwrap()
        .clone();
    let mut budget = TemplateInstantiationBudget::new();
    assert!(matches!(
        instantiate_template_bounded(&template, &params, None, &[], &mut budget),
        Err(TemplateInstantiationError::TooLarge)
    ));
    assert_eq!(budget.used_bytes(), 2);
}

#[test]
fn raw_sole_param_tree_is_bounded_before_clone_and_not_substituted_recursively() {
    let template = template(json!({"message":"{{ params.tree }}"}));
    let mut budget = TemplateInstantiationBudget::new();
    let params = json!({"tree":{"literal":"{{ params.other }}","bool":true},"other":"resolved"})
        .as_object()
        .unwrap()
        .clone();
    let rule = instantiate_template_bounded(&template, &params, None, &[], &mut budget).unwrap();
    assert_eq!(
        rule.actions[0]["config"]["message"]["literal"],
        "{{ params.other }}"
    );
    let used = budget.used_bytes();
    for tree in [
        json!({"large":"x".repeat(RECORD_BYTES)}),
        Value::Array(vec![Value::from(0); MAX_NODES]),
    ] {
        let params = Map::from_iter([("tree".into(), tree)]);
        assert!(matches!(
            instantiate_template_bounded(&template, &params, None, &[], &mut budget),
            Err(TemplateInstantiationError::TooLarge)
        ));
        assert_eq!(budget.used_bytes(), used);
    }
}

#[test]
fn raw_sole_and_inline_object_depth_are_guarded_before_clone_or_json_serialization() {
    let mut deep = Value::Bool(true);
    for _ in 0..70 {
        deep = Value::Array(vec![deep]);
    }
    let params = Map::from_iter([("tree".into(), deep)]);
    for config in [
        json!({"message":"{{ params.tree }}"}),
        json!({"message":"prefix {{ params.tree }}"}),
    ] {
        let mut budget = TemplateInstantiationBudget::new();
        assert!(matches!(
            instantiate_template_bounded(&template(config), &params, None, &[], &mut budget),
            Err(TemplateInstantiationError::TooLarge)
        ));
        assert_eq!(budget.used_bytes(), 2);
    }
}

#[test]
fn inline_object_serialization_counts_outer_string_escaping_before_allocating_it() {
    let template = template(json!({"message":"prefix {{ params.tree }}"}));
    // The raw object fits a record; escaping its JSON text as an outer string
    // exceeds the record budget and must not reach scalar_text/to_string.
    let params = json!({"tree":{"quote":"\"".repeat(80_000)}})
        .as_object()
        .unwrap()
        .clone();
    assert!(serde_json::to_vec(&params["tree"]).unwrap().len() < RECORD_BYTES);
    let mut budget = TemplateInstantiationBudget::new();
    assert!(matches!(
        instantiate_template_bounded(&template, &params, None, &[], &mut budget),
        Err(TemplateInstantiationError::TooLarge)
    ));
    assert_eq!(budget.used_bytes(), 2);
}

#[test]
fn shared_import_budget_rejects_later_entry_without_charging_failed_expansion() {
    let mut template = template(json!({"message":"{{ params.large }}"}));
    template.actions = vec![template.actions[0].clone(); 8];
    let params = json!({"large":"x".repeat(200_000)})
        .as_object()
        .unwrap()
        .clone();
    let mut budget = TemplateInstantiationBudget::new();
    for _ in 0..5 {
        instantiate_template_bounded(&template, &params, None, &[], &mut budget).unwrap();
    }
    let used = budget.used_bytes();
    assert!(used > 7 * 1024 * 1024 && used < IMPORT_BYTES);
    assert!(matches!(
        instantiate_template_bounded(&template, &params, None, &[], &mut budget),
        Err(TemplateInstantiationError::TooLarge)
    ));
    assert_eq!(budget.used_bytes(), used);
}

#[test]
fn escaped_unicode_is_admitted_at_exact_record_byte_boundary_then_rejects_one_extra_byte() {
    let template = template(json!({"message":"{{ params.text }}"}));
    let mut params = Map::from_iter([("text".into(), Value::String("\"\\\n\u{0000}ñ😀".into()))]);
    let base = measure_action(&template.actions[0], 0, "act-fixed", &params, &[]).unwrap();
    let value = params.get_mut("text").unwrap().as_str().unwrap().to_owned()
        + &"x".repeat(RECORD_BYTES - base);
    params.insert("text".into(), Value::String(value));
    assert_eq!(
        measure_action(&template.actions[0], 0, "act-fixed", &params, &[]).unwrap(),
        RECORD_BYTES
    );
    let result = instantiate_template_with_ids(
        &template,
        &params,
        None,
        &[],
        &["act-fixed".into()],
        "evt-fixed",
    );
    assert_eq!(
        serde_json::to_vec(&result.actions[0]).unwrap().len(),
        RECORD_BYTES
    );
    let larger = params["text"].as_str().unwrap().to_owned() + "x";
    params.insert("text".into(), Value::String(larger));
    assert!(matches!(
        measure_action(&template.actions[0], 0, "act-fixed", &params, &[]),
        Err(TemplateInstantiationError::TooLarge)
    ));
}

#[test]
fn sole_nonstring_names_keep_empty_fallback_without_cloning_discarded_objects() {
    let mut template = template(json!({"message":"safe"}));
    template.actions[0].name = "{{ params.tree }}".into();
    template.event.name = " {{ params.tree }} ".into();
    let params = json!({"tree":{"unused":"x".repeat(IMPORT_BYTES)}})
        .as_object()
        .unwrap()
        .clone();
    let mut budget = TemplateInstantiationBudget::new();
    let result = instantiate_template_bounded(&template, &params, None, &[], &mut budget).unwrap();
    assert_eq!(result.actions[0]["name"], "");
    assert_eq!(result.event["name"], "");
    assert!(budget.used_bytes() < 1024);
}
