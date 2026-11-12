//! Fresh behavior record identities and resolved event/action instantiation.
use super::*;

/// Instantiates behavior records with fresh ids and resolved params.
pub fn instantiate_template(
    template: &RuleTemplate,
    params: &Map<String, Value>,
    event_name: Option<&str>,
    action_names: &[String],
) -> InstantiatedRule {
    let action_ids: Vec<String> = template
        .actions
        .iter()
        .map(|_| fresh_record_id("act"))
        .collect();
    let event_id = fresh_record_id("evt");
    instantiate_template_with_ids(
        template,
        params,
        event_name,
        action_names,
        &action_ids,
        &event_id,
    )
}

fn substituted_name(text: &str, params: &Map<String, Value>) -> String {
    // A non-string sole param has always produced an empty name. Inspect it
    // by reference so that this fallback never clones a discarded object.
    if let Some(path) = sole_params_span(text) {
        return read_param_path_ref(params, &path)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
    }
    substitute_string(text, params)
        .as_str()
        .unwrap_or_default()
        .to_owned()
}

pub(super) fn instantiate_template_with_ids(
    template: &RuleTemplate,
    params: &Map<String, Value>,
    event_name: Option<&str>,
    action_names: &[String],
    ids: &[String],
    event_id: &str,
) -> InstantiatedRule {
    let actions: Vec<Value> = template
        .actions
        .iter()
        .enumerate()
        .map(|(index, action)| {
            let name = action_names
                .get(index)
                .map(|name| name.trim())
                .filter(|name| !name.is_empty())
                .map(str::to_owned)
                .unwrap_or_else(|| substituted_name(&action.name, params));
            serde_json::json!({
                "schemaVersion": 2,
                "id": ids[index],
                "name": name,
                "typeId": action.type_id,
                "enabled": action.enabled,
                "config": substitute_params(&Value::Object(action.config.clone()), params),
            })
        })
        .collect();
    let action_ids: Vec<Value> = actions
        .iter()
        .filter_map(|action| action.get("id").cloned())
        .collect();
    let name = event_name
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| substituted_name(&template.event.name, params));
    let filters = substitute_params(
        &serde_json::to_value(&template.event.filters).unwrap_or(Value::Array(Vec::new())),
        params,
    );
    let event = serde_json::json!({
        "schemaVersion": 1,
        "id": event_id,
        "name": name,
        "enabled": template.event.enabled,
        "trigger": template.event.trigger,
        "filters": filters,
        "cooldownMs": template.event.cooldown_ms,
        "cooldownScope": template.event.cooldown_scope,
        "actionIds": action_ids,
        "runMode": template.event.run_mode,
    });
    InstantiatedRule { actions, event }
}

/// Action type ids plus the trigger a template needs from the host.
pub fn template_requirements(template: &RuleTemplate) -> (Vec<String>, String) {
    let mut type_ids: Vec<String> = template
        .actions
        .iter()
        .map(|action| action.type_id.clone())
        .collect();
    type_ids.sort();
    type_ids.dedup();
    (type_ids, template.event.trigger.clone())
}
