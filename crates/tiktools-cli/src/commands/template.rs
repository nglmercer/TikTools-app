//! `template` commands: JSON rule templates and profiles for behavior.
//!
//! Templates instantiate actions+events from data files; profiles bundle
//! several templates with shared params (e.g. the Minecraft CommandAPI
//! host/port). Imported documents are also stored in the `behavior.
//! templates.custom` app.state gallery shared with the web template modal,
//! so CLI and UI stay in one library.

use std::collections::BTreeSet;

use serde_json::{Map, Value};
use tiktools_client::TikToolsClient;
use tiktools_control_api::modules::app::{AppStateGetParams, AppStateSetParams};
use tiktools_control_api::modules::automation::AutomationCreateParams;
use tiktools_control_api::ClientError;
use tiktools_core::services::templates::{
    instantiate_template, merge_params, param_defaults, parse_profile, parse_rule_template,
    parse_rule_template_list, template_requirements, RuleTemplate, CUSTOM_TEMPLATES_KEY,
};

use super::args::{flag_value, flag_values, positional, split_first};
use super::{result_value, CommandError};

/// Built-in triggers mirrored from the web `BUILTIN_EVENT_TYPES` contract.
/// Advisory only: the host owns trigger validation at create time.
const BUILTIN_TRIGGERS: &[&str] = &[
    "tiktok.chat",
    "tiktok.gift",
    "tiktok.like",
    "tiktok.follow",
    "tiktok.share",
    "tiktok.join",
    "tiktok.social",
    "tiktok.room_stats",
    "tiktok.connected",
    "tiktok.disconnected",
    "points.awarded",
    "plugin.emit",
];

const MAX_TEMPLATE_FILE_BYTES: u64 = 262_144;

pub enum Command {
    Import {
        path: String,
        params: Map<String, Value>,
        event_name: Option<String>,
        action_name: Option<String>,
        dry_run: bool,
        save: bool,
        disabled: bool,
    },
    Export {
        event_id: String,
        out: String,
        title: Option<String>,
        id: Option<String>,
    },
    List,
    Delete {
        template_id: String,
    },
    ProfileImport {
        path: String,
        params: Map<String, Value>,
        dry_run: bool,
        save: bool,
        disabled: bool,
    },
    ProfileExport {
        out: String,
        name: Option<String>,
    },
}

fn collect_params(rest: &[String]) -> Result<Map<String, Value>, CommandError> {
    let mut params = Map::new();
    for raw in flag_values(rest, "--param") {
        let (key, value) = raw
            .split_once('=')
            .ok_or_else(|| CommandError::Usage(format!("--param needs k=v, got `{raw}`")))?;
        if key.trim().is_empty() {
            return Err(CommandError::Usage(format!(
                "--param needs k=v, got `{raw}`"
            )));
        }
        let value: Value =
            serde_json::from_str(value).unwrap_or_else(|_| Value::String(value.to_owned()));
        params.insert(key.to_owned(), value);
    }
    Ok(params)
}

pub fn parse(args: &[String]) -> Result<Command, CommandError> {
    let (verb, rest) = split_first(args, "template <verb> ...")?;
    let dry_run = rest.iter().any(|arg| arg == "--dry-run");
    let save = !rest.iter().any(|arg| arg == "--no-save");
    let disabled = rest.iter().any(|arg| arg == "--disabled");
    match verb {
        "import" => Ok(Command::Import {
            path: positional(
                rest,
                0,
                "template import <file> [--param k=v]... [--dry-run] [--no-save]",
            )?,
            params: collect_params(rest)?,
            event_name: flag_value(rest, "--event-name"),
            action_name: flag_value(rest, "--action-name"),
            dry_run,
            save,
            disabled,
        }),
        "export" => Ok(Command::Export {
            event_id: flag_value(rest, "--event")
                .ok_or_else(|| "template export needs --event <id> --out <file>".to_owned())?,
            out: flag_value(rest, "--out")
                .ok_or_else(|| "template export needs --event <id> --out <file>".to_owned())?,
            title: flag_value(rest, "--title"),
            id: flag_value(rest, "--id"),
        }),
        "list" => Ok(Command::List),
        "delete" => Ok(Command::Delete {
            template_id: positional(rest, 0, "template delete <template-id>")?,
        }),
        "profile-import" => Ok(Command::ProfileImport {
            path: positional(
                rest,
                0,
                "template profile-import <file> [--param k=v]... [--dry-run] [--no-save]",
            )?,
            params: collect_params(rest)?,
            dry_run,
            save,
            disabled,
        }),
        "profile-export" => Ok(Command::ProfileExport {
            out: flag_value(rest, "--out")
                .ok_or_else(|| "template profile-export needs --out <file>".to_owned())?,
            name: flag_value(rest, "--name"),
        }),
        other => Err(format!("unknown template verb `{other}`").into()),
    }
}

fn read_doc(path: &str) -> Result<Value, ClientError> {
    let meta = std::fs::metadata(path).map_err(|error| {
        ClientError::new("invalid_params", format!("could not read {path}: {error}"))
    })?;
    if meta.len() > MAX_TEMPLATE_FILE_BYTES {
        return Err(ClientError::new(
            "invalid_params",
            format!("{path} exceeds 256 KB"),
        ));
    }
    let text = std::fs::read_to_string(path).map_err(|error| {
        ClientError::new("invalid_params", format!("could not read {path}: {error}"))
    })?;
    serde_json::from_str(&text).map_err(|error| {
        ClientError::new(
            "invalid_params",
            format!("{path} is not valid JSON: {error}"),
        )
    })
}

fn invalid(message: String) -> ClientError {
    ClientError::new("invalid_params", message)
}

/// Action type ids the snapshot reports as usable, mirroring the web gallery.
fn available_action_types(snapshot: &Value) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    let types = snapshot.get("actionTypes").and_then(Value::as_array);
    let plugins = snapshot.get("plugins").and_then(Value::as_array);
    let enabled_plugins: BTreeSet<&str> = plugins
        .into_iter()
        .flatten()
        .filter(|plugin| {
            plugin.get("installed").and_then(Value::as_bool) == Some(true)
                && plugin.get("enabled").and_then(Value::as_bool) == Some(true)
        })
        .filter_map(|plugin| plugin.get("descriptor")?.get("id")?.as_str())
        .collect();
    for entry in types.into_iter().flatten() {
        let Some(id) = entry.get("id").and_then(Value::as_str) else {
            continue;
        };
        let source = entry.get("source");
        let kind = source
            .and_then(|source| source.get("kind"))
            .and_then(Value::as_str);
        match kind {
            Some("builtin") => {
                ids.insert(id.to_owned());
            }
            Some("plugin") => {
                let plugin_id = source
                    .and_then(|source| source.get("pluginId"))
                    .and_then(Value::as_str);
                if plugin_id.is_some_and(|plugin_id| enabled_plugins.contains(plugin_id)) {
                    ids.insert(id.to_owned());
                }
            }
            _ => {}
        }
    }
    ids
}

fn known_triggers(snapshot: &Value) -> BTreeSet<String> {
    let mut triggers: BTreeSet<String> = BUILTIN_TRIGGERS.iter().map(ToString::to_string).collect();
    if let Some(types) = snapshot.get("eventTypes").and_then(Value::as_array) {
        for entry in types {
            if let Some(name) = entry.get("type").and_then(Value::as_str) {
                triggers.insert(name.to_owned());
            }
        }
    }
    triggers
}

async fn check_requirements(
    client: &TikToolsClient,
    templates: &[RuleTemplate],
) -> Result<(), ClientError> {
    let snapshot = client.automation_snapshot().await?;
    let available = available_action_types(&snapshot);
    let triggers = known_triggers(&snapshot);
    let mut problems = Vec::new();
    for template in templates {
        let (type_ids, trigger) = template_requirements(template);
        for id in type_ids {
            if !available.contains(&id) {
                problems.push(format!(
                    "template {} requires unavailable action type {id}",
                    template.id
                ));
            }
        }
        if !triggers.contains(&trigger) {
            problems.push(format!(
                "template {} has unknown trigger {trigger}",
                template.id
            ));
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(invalid(problems.join("; ")))
    }
}

async fn read_gallery(client: &TikToolsClient) -> Result<Vec<RuleTemplate>, ClientError> {
    let result = client
        .app_state_get(AppStateGetParams {
            keys: Some(vec![CUSTOM_TEMPLATES_KEY.to_owned()]),
        })
        .await?;
    let Some(raw) = result.state.get(CUSTOM_TEMPLATES_KEY) else {
        return Ok(Vec::new());
    };
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    let value: Value = serde_json::from_str(raw)
        .map_err(|error| invalid(format!("stored template gallery is corrupt: {error}")))?;
    let (templates, _) = parse_rule_template_list(&value);
    Ok(templates)
}

async fn write_gallery(
    client: &TikToolsClient,
    templates: &[RuleTemplate],
) -> Result<(), ClientError> {
    let value = serde_json::to_value(templates)
        .map_err(|error| invalid(format!("could not encode gallery: {error}")))?;
    client
        .app_state_set(AppStateSetParams {
            key: CUSTOM_TEMPLATES_KEY.to_owned(),
            value: serde_json::to_string(&value)
                .map_err(|error| invalid(format!("could not encode gallery: {error}")))?,
        })
        .await?;
    Ok(())
}

fn apply_disabled(actions: &mut [Value], event: &mut Value) {
    for action in actions.iter_mut() {
        if let Some(object) = action.as_object_mut() {
            object.insert("enabled".to_owned(), Value::Bool(false));
        }
    }
    if let Some(object) = event.as_object_mut() {
        object.insert("enabled".to_owned(), Value::Bool(false));
    }
}

async fn create_rule(
    client: &TikToolsClient,
    mut actions: Vec<Value>,
    mut event: Value,
    disabled: bool,
) -> Result<Value, ClientError> {
    if disabled {
        apply_disabled(&mut actions, &mut event);
    }
    let mut created_actions = Vec::with_capacity(actions.len());
    for action in actions {
        let created = client
            .automation_create(AutomationCreateParams {
                kind: Some("action".to_owned()),
                record: action,
            })
            .await?;
        created_actions.push(created);
    }
    let created_event = client
        .automation_create(AutomationCreateParams {
            kind: Some("event".to_owned()),
            record: event,
        })
        .await?;
    result_value(serde_json::json!({"actions": created_actions, "event": created_event}))
}

fn plan_value(template: &RuleTemplate, params: &Map<String, Value>) -> Value {
    let defaults = param_defaults(template.params.as_ref());
    let merged = merge_params(&[&defaults, params]);
    let instantiated = instantiate_template(template, &merged, None, &[]);
    let (type_ids, trigger) = template_requirements(template);
    serde_json::json!({
        "template": template.id,
        "params": merged,
        "actions": instantiated.actions,
        "event": instantiated.event,
        "requirements": {"actionTypeIds": type_ids, "trigger": trigger},
    })
}

pub async fn execute(client: &TikToolsClient, command: Command) -> Result<Value, ClientError> {
    match command {
        Command::Import {
            path,
            params,
            event_name,
            action_name,
            dry_run,
            save,
            disabled,
        } => {
            let doc = read_doc(&path)?;
            if doc.get("profileVersion").is_some() {
                return Err(invalid(
                    "this is a profile document; use `template profile-import`".to_owned(),
                ));
            }
            let (templates, errors) = parse_rule_template_list(&doc);
            if !errors.is_empty() {
                return Err(invalid(errors.join("; ")));
            }
            if templates.is_empty() {
                return Err(invalid("no templates found".to_owned()));
            }
            if templates.len() > 1 && (event_name.is_some() || action_name.is_some()) {
                return Err(invalid(
                    "--event-name/--action-name apply to single-template imports only".to_owned(),
                ));
            }
            if dry_run {
                let plans: Vec<Value> = templates
                    .iter()
                    .map(|template| plan_value(template, &params))
                    .collect();
                return result_value(serde_json::json!({"dryRun": true, "plans": plans}));
            }
            check_requirements(client, &templates).await?;
            if save {
                let mut gallery = read_gallery(client).await?;
                for template in &templates {
                    gallery.retain(|entry| entry.id != template.id);
                    gallery.push(template.clone());
                }
                write_gallery(client, &gallery).await?;
            }
            let mut imported = Vec::with_capacity(templates.len());
            for template in &templates {
                let defaults = param_defaults(template.params.as_ref());
                let merged = merge_params(&[&defaults, &params]);
                let action_names = action_name
                    .clone()
                    .map(|name| vec![name])
                    .unwrap_or_default();
                let instantiated =
                    instantiate_template(template, &merged, event_name.as_deref(), &action_names);
                let created =
                    create_rule(client, instantiated.actions, instantiated.event, disabled).await?;
                imported.push(serde_json::json!({
                    "template": template.id,
                    "actions": created.get("actions"),
                    "event": created.get("event"),
                }));
            }
            result_value(serde_json::json!({"imported": imported, "savedToGallery": save}))
        }
        Command::Export {
            event_id,
            out,
            title,
            id,
        } => {
            let snapshot = client.automation_snapshot().await?;
            let events = snapshot
                .get("events")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let event = events
                .iter()
                .find(|entry| entry.get("id").and_then(Value::as_str) == Some(event_id.as_str()))
                .ok_or_else(|| invalid(format!("event {event_id} not found")))?;
            let actions = snapshot
                .get("actions")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let action_ids: Vec<&str> = event
                .get("actionIds")
                .and_then(Value::as_array)
                .map(|ids| ids.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default();
            let mut exported = Vec::new();
            for action_id in &action_ids {
                let action = actions
                    .iter()
                    .find(|entry| entry.get("id").and_then(Value::as_str) == Some(*action_id))
                    .ok_or_else(|| invalid(format!("action {action_id} not found")))?;
                exported.push(serde_json::json!({
                    "name": action.get("name"),
                    "typeId": action.get("typeId"),
                    "enabled": action.get("enabled"),
                    "config": action.get("config").cloned().unwrap_or(Value::Object(Map::new())),
                }));
            }
            if exported.is_empty() {
                return Err(invalid(format!(
                    "event {event_id} has no actions to export"
                )));
            }
            let doc = serde_json::json!({
                "templateVersion": 1,
                "id": id.unwrap_or_else(|| format!("export-{event_id}")),
                "title": title.or_else(|| event.get("name").and_then(Value::as_str).map(str::to_owned)).unwrap_or_else(|| "Exported template".to_owned()),
                "description": "",
                "icon": "plugin",
                "actions": exported,
                "event": {
                    "name": event.get("name"),
                    "enabled": event.get("enabled"),
                    "trigger": event.get("trigger"),
                    "filters": event.get("filters").cloned().unwrap_or(Value::Array(Vec::new())),
                    "cooldownMs": event.get("cooldownMs"),
                    "cooldownScope": event.get("cooldownScope"),
                    "runMode": event.get("runMode"),
                },
            });
            let parsed = parse_rule_template(&doc).map_err(|errors| invalid(errors.to_string()))?;
            std::fs::write(
                &out,
                serde_json::to_string_pretty(&parsed).unwrap_or_default(),
            )
            .map_err(|error| invalid(format!("could not write {out}: {error}")))?;
            result_value(serde_json::json!({"path": out, "template": parsed.id}))
        }
        Command::List => {
            let gallery = read_gallery(client).await?;
            let templates: Vec<Value> = gallery
                .iter()
                .map(|template| {
                    serde_json::json!({
                        "id": template.id,
                        "title": template.title.default,
                        "trigger": template.event.trigger,
                        "actions": template.actions.len(),
                        "custom": true,
                    })
                })
                .collect();
            result_value(serde_json::json!({"templates": templates}))
        }
        Command::Delete { template_id } => {
            let mut gallery = read_gallery(client).await?;
            let before = gallery.len();
            gallery.retain(|entry| entry.id != template_id);
            if gallery.len() == before {
                return Err(invalid(format!("template {template_id} not in gallery")));
            }
            write_gallery(client, &gallery).await?;
            result_value(serde_json::json!({"deleted": template_id}))
        }
        Command::ProfileImport {
            path,
            params,
            dry_run,
            save,
            disabled,
        } => {
            let doc = read_doc(&path)?;
            if doc.get("templateVersion").is_some() {
                return Err(invalid(
                    "this is a template document; use `template import`".to_owned(),
                ));
            }
            let profile = parse_profile(&doc).map_err(|errors| invalid(errors.to_string()))?;
            let templates: Vec<RuleTemplate> = profile
                .entries
                .iter()
                .map(|entry| entry.template.clone())
                .collect();
            if dry_run {
                let plans: Vec<Value> = profile
                    .entries
                    .iter()
                    .map(|entry| {
                        let defaults = param_defaults(entry.template.params.as_ref());
                        let merged =
                            merge_params(&[&defaults, &profile.params, &entry.params, &params]);
                        let instantiated =
                            instantiate_template(&entry.template, &merged, None, &[]);
                        let (type_ids, trigger) = template_requirements(&entry.template);
                        serde_json::json!({
                            "template": entry.template.id,
                            "params": merged,
                            "actions": instantiated.actions,
                            "event": instantiated.event,
                            "requirements": {"actionTypeIds": type_ids, "trigger": trigger},
                        })
                    })
                    .collect();
                return result_value(
                    serde_json::json!({"dryRun": true, "profile": profile.id, "plans": plans}),
                );
            }
            check_requirements(client, &templates).await?;
            if save {
                let mut gallery = read_gallery(client).await?;
                for template in &templates {
                    gallery.retain(|entry| entry.id != template.id);
                    gallery.push(template.clone());
                }
                write_gallery(client, &gallery).await?;
            }
            let mut imported = Vec::with_capacity(profile.entries.len());
            for entry in &profile.entries {
                let defaults = param_defaults(entry.template.params.as_ref());
                let merged = merge_params(&[&defaults, &profile.params, &entry.params, &params]);
                let instantiated = instantiate_template(&entry.template, &merged, None, &[]);
                let created =
                    create_rule(client, instantiated.actions, instantiated.event, disabled).await?;
                imported.push(serde_json::json!({
                    "template": entry.template.id,
                    "actions": created.get("actions"),
                    "event": created.get("event"),
                }));
            }
            result_value(
                serde_json::json!({"profile": profile.id, "imported": imported, "savedToGallery": save}),
            )
        }
        Command::ProfileExport { out, name } => {
            let snapshot = client.automation_snapshot().await?;
            let events = snapshot
                .get("events")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let actions = snapshot
                .get("actions")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let mut entries = Vec::new();
            let mut skipped = 0u64;
            for event in &events {
                let Some(event_id) = event.get("id").and_then(Value::as_str) else {
                    continue;
                };
                let action_ids: Vec<&str> = event
                    .get("actionIds")
                    .and_then(Value::as_array)
                    .map(|ids| ids.iter().filter_map(Value::as_str).collect())
                    .unwrap_or_default();
                let mut exported = Vec::new();
                for action_id in &action_ids {
                    if let Some(action) = actions
                        .iter()
                        .find(|entry| entry.get("id").and_then(Value::as_str) == Some(*action_id))
                    {
                        exported.push(serde_json::json!({
                            "name": action.get("name"),
                            "typeId": action.get("typeId"),
                            "enabled": action.get("enabled"),
                            "config": action.get("config").cloned().unwrap_or(Value::Object(Map::new())),
                        }));
                    }
                }
                if exported.is_empty() {
                    skipped += 1;
                    continue;
                }
                entries.push(serde_json::json!({
                    "template": {
                        "templateVersion": 1,
                        "id": format!("export-{event_id}"),
                        "title": event.get("name"),
                        "description": "",
                        "icon": "plugin",
                        "actions": exported,
                        "event": {
                            "name": event.get("name"),
                            "enabled": event.get("enabled"),
                            "trigger": event.get("trigger"),
                            "filters": event.get("filters").cloned().unwrap_or(Value::Array(Vec::new())),
                            "cooldownMs": event.get("cooldownMs"),
                            "cooldownScope": event.get("cooldownScope"),
                            "runMode": event.get("runMode"),
                        },
                    },
                    "params": {},
                }));
            }
            let doc = serde_json::json!({
                "profileVersion": 1,
                "id": "exported-profile",
                "name": name.unwrap_or_else(|| "Exported profile".to_owned()),
                "description": "",
                "params": {},
                "templates": entries,
            });
            parse_profile(&doc).map_err(|errors| invalid(errors.to_string()))?;
            std::fs::write(&out, serde_json::to_string_pretty(&doc).unwrap_or_default())
                .map_err(|error| invalid(format!("could not write {out}: {error}")))?;
            result_value(
                serde_json::json!({"path": out, "entries": entries.len(), "skipped": skipped}),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse;
    use super::Command;

    fn args(words: &[&str]) -> Vec<String> {
        words.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn import_parses_params_and_flags() {
        // `CommandError` has no Debug (it prints usage instead), so match
        // rather than unwrap.
        let parsed = parse(&args(&[
            "import",
            "rules.json",
            "--param",
            "port=8080",
            "--param",
            "host=127.0.0.1",
            "--dry-run",
            "--no-save",
        ]));
        assert!(matches!(
            parsed,
            Ok(Command::Import { path, params, dry_run: true, save: false, .. })
                if path == "rules.json"
                    && params.get("port") == Some(&serde_json::json!(8080))
                    && params.get("host") == Some(&serde_json::json!("127.0.0.1"))
        ));
    }

    #[test]
    fn import_rejects_malformed_params_and_missing_file() {
        assert!(parse(&args(&["import", "rules.json", "--param", "nope"])).is_err());
        assert!(parse(&args(&["import"])).is_err());
    }

    #[test]
    fn export_requires_event_and_out() {
        assert!(matches!(
            parse(&args(&["export", "--event", "e1", "--out", "t.json"])),
            Ok(Command::Export { .. })
        ));
        assert!(parse(&args(&["export", "--event", "e1"])).is_err());
    }

    #[test]
    fn profile_verbs_parse() {
        assert!(matches!(
            parse(&args(&["profile-import", "p.json"])),
            Ok(Command::ProfileImport { .. })
        ));
        assert!(matches!(
            parse(&args(&["profile-export", "--out", "p.json"])),
            Ok(Command::ProfileExport { .. })
        ));
        assert!(parse(&args(&["profile-export"])).is_err());
        assert!(parse(&args(&["nope"])).is_err());
    }
}
