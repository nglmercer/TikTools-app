//! Host-owned automation metadata.
//!
//! The WebView renders these JSON descriptors, but the Rust host owns the
//! catalog. Runtime plugin descriptors are appended by `AppCore` after the
//! plugin directory scan; no plugin is registered in this module.

use serde_json::{json, Value};

pub fn builtin_action_types() -> Vec<Value> {
    vec![
        json!({
            "id": "core.fetch",
            "version": 1,
            "title": text("Call a URL", "automation.action.core.fetch.title"),
            "description": text("POST or GET event data to a server or webhook.", "automation.action.core.fetch.description"),
            "tag": "fetch",
            "source": {"kind": "builtin"},
            "requiredCapabilities": ["http.request"],
            "fields": [
                field("method", "Method", "select", "POST", "automation.action.core.fetch.field.method.label", json!({
                    "options": [
                        {"value": "GET", "label": text("GET", "automation.action.core.fetch.field.method.option.GET")},
                        {"value": "POST", "label": text("POST", "automation.action.core.fetch.field.method.option.POST")},
                        {"value": "PUT", "label": text("PUT", "automation.action.core.fetch.field.method.option.PUT")},
                        {"value": "DELETE", "label": text("DELETE", "automation.action.core.fetch.field.method.option.DELETE")}
                    ]
                })),
                field("url", "URL", "text", "https://", "automation.action.core.fetch.field.url.label", json!({
                    "placeholder": "https://hooks.example.com/live",
                    "template": true,
                    "hint": text("The host must be literal so it can be allowlisted.", "automation.action.core.fetch.field.url.hint")
                })),
                field("headers", "Headers", "keyvalue", "content-type=application/json", "automation.action.core.fetch.field.headers.label", json!({
                    "template": true,
                    "advanced": true
                })),
                field("body", "Body", "textarea", "{\n  \"viewer\": \"{{ event.user.uniqueId }}\",\n  \"type\": \"{{ event.type }}\"\n}", "automation.action.core.fetch.field.body.label", json!({
                    "template": true,
                    "showIf": {"key": "method", "notEquals": ["GET"]}
                })),
                field("timeoutMs", "Timeout (ms)", "number", "5000", "automation.action.core.fetch.field.timeoutMs.label", json!({"advanced": true})),
                field("emitResponseAs", "Emit the response as", "text", "", "automation.action.core.fetch.field.emitResponseAs.label", json!({
                    "placeholder": "overlay.webhook.done",
                    "advanced": true,
                    "hint": text("Publish the response as an internal event.", "automation.action.core.fetch.field.emitResponseAs.hint")
                })),
                field("allowPrivateNetwork", "Allow local network", "boolean", "false", "automation.action.core.fetch.field.allowPrivateNetwork.label", json!({
                    "advanced": true,
                    "hint": text("Enable only for an explicitly trusted local destination.", "automation.action.core.fetch.field.allowPrivateNetwork.hint")
                }))
            ]
        }),
        json!({
            "id": "core.emit",
            "version": 1,
            "title": text("Emit an internal event", "automation.action.core.emit.title"),
            "description": text("Publish an event that other automations can consume.", "automation.action.core.emit.description"),
            "tag": "emit",
            "source": {"kind": "builtin"},
            "requiredCapabilities": [],
            "fields": [
                field("type", "Internal event", "text", "overlay.alert", "automation.action.core.emit.field.type.label", json!({})),
                field("data", "Data", "keyvalue", "texto={{ event.user.nickname }}", "automation.action.core.emit.field.data.label", json!({"template": true}))
            ]
        }),
        json!({
            "id": "core.points",
            "version": 1,
            "title": text("Give points", "automation.action.core.points.title"),
            "description": text("Add or subtract points for the viewer who triggered the event.", "automation.action.core.points.description"),
            "tag": "points",
            "source": {"kind": "builtin"},
            "requiredCapabilities": ["points.write"],
            "fields": [
                field("uniqueId", "Viewer", "text", "{{ event.user.uniqueId }}", "automation.action.core.points.field.uniqueId.label", json!({"template": true})),
                field("delta", "Points", "number", "10", "automation.action.core.points.field.delta.label", json!({}))
            ]
        }),
        json!({
            "id": "core.points.subtract",
            "version": 1,
            "title": text("Subtract points", "automation.action.core.points.subtract.title"),
            "description": text("Deduct points from the viewer who triggered the event. Balances never drop below zero.", "automation.action.core.points.subtract.description"),
            "tag": "points",
            "source": {"kind": "builtin"},
            "requiredCapabilities": ["points.write"],
            "fields": [
                field("uniqueId", "Viewer", "text", "{{ event.user.uniqueId }}", "automation.action.core.points.subtract.field.uniqueId.label", json!({"template": true})),
                field("amount", "Points to subtract", "number", "10", "automation.action.core.points.subtract.field.amount.label", json!({"min": 0}))
            ]
        }),
        json!({
            "id": "audio.play",
            "version": 1,
            "title": text("Play a sound", "automation.action.audio.play.title"),
            "description": text("Play a local audio file without copying it into TikTools.", "automation.action.audio.play.description"),
            "tag": "audio",
            "source": {"kind": "builtin"},
            "requiredCapabilities": ["audio.play"],
            "fields": [
                field("file", "Audio file", "media", "", "automation.action.audio.play.field.file.label", json!({
                    "hint": text("Select an existing audio file. TikTools stores a path-backed reference and validates it again when it plays.", "automation.action.audio.play.field.file.hint")
                })),
                field("volume", "Volume", "range", "1", "automation.action.audio.play.field.volume.label", json!({
                    "hint": text("A value from 0 (silent) to 1 (full volume).", "automation.action.audio.play.field.volume.hint"),
                    "min": 0, "max": 1, "step": 0.05
                })),
                field("overlap", "If already playing", "select", "allow", "automation.action.audio.play.field.overlap.label", json!({
                    "options": [
                        {"value": "allow", "label": text("Allow overlap", "automation.action.audio.play.field.overlap.option.allow")},
                        {"value": "restart", "label": text("Restart sound", "automation.action.audio.play.field.overlap.option.restart")},
                        {"value": "drop", "label": text("Drop new sound", "automation.action.audio.play.field.overlap.option.drop")}
                    ]
                }))
            ]
        }),
        json!({
            "id": "core.delay",
            "version": 1,
            "title": text("Wait", "automation.action.core.delay.title"),
            "description": text("Delay the remaining actions for this event.", "automation.action.core.delay.description"),
            "tag": "flow",
            "source": {"kind": "builtin"},
            "requiredCapabilities": [],
            "fields": [field("ms", "Duration (ms)", "number", "1000", "automation.action.core.delay.field.ms.label", json!({}))]
        }),
        json!({
            "id": "core.log",
            "version": 1,
            "title": text("Write to the log", "automation.action.core.log.title"),
            "description": text("Leave a line in the automation log.", "automation.action.core.log.description"),
            "tag": "flow",
            "source": {"kind": "builtin"},
            "requiredCapabilities": [],
            "fields": [field("message", "Message", "text", "{{ event.user.uniqueId }} · {{ event.type }}", "automation.action.core.log.field.message.label", json!({"template": true}))]
        }),
        json!({
            "id": "core.code",
            "version": 1,
            "title": text("Code", "automation.action.core.code.title"),
            "description": text("Run bounded JavaScript in napi-vm and return a JSON intent.", "automation.action.core.code.description"),
            "tag": "napi-vm",
            "source": {"kind": "builtin"},
            "requiredCapabilities": [],
            "fields": [field("source", "Script", "code", "return { log: [`${event.user.uniqueId} · ${event.type}`] };", "automation.action.core.code.field.source.label", json!({}))]
        }),
    ]
}

pub fn builtin_translations() -> Value {
    // Host-owned translations ship as data, not code: one JSON document per
    // supported locale set, embedded at compile time so every control-API
    // consumer (WebView, CLI) reads the same catalog. The WebView still
    // overlays its richer host-messages for zh/ja; identical keys agree.
    static CATALOG: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    CATALOG
        .get_or_init(|| {
            serde_json::from_str(include_str!("builtin-translations.json"))
                .expect("builtin translations JSON is valid")
        })
        .clone()
}

pub fn builtin_node_catalog() -> Vec<Value> {
    vec![
        json!({
            "type": "trigger.event", "version": 1, "pluginId": "core", "title": "Event Trigger",
            "category": "Triggers", "kind": "trigger", "inputs": [],
            "outputs": [flow_output(), port("event", "Event", "data", "event", false), port("data", "Data", "data", "json", false), port("user", "User", "data", "json", false)],
            "configSchema": {"type": "object", "properties": {"eventType": {"type": "string"}}, "required": ["eventType"]}
        }),
        json!({
            "type": "condition.compare", "version": 1, "pluginId": "core", "title": "Compare",
            "category": "Conditions", "kind": "condition",
            "inputs": [flow_input(), port("left", "Left", "data", "json", false), port("right", "Right", "data", "json", false)],
            "outputs": [port("true", "True", "flow", "", false), port("false", "False", "flow", "", false), port("result", "Result", "data", "boolean", false)],
            "configSchema": {"type": "object", "properties": {"leftPath": {"type": "string"}, "operator": {"type": "string"}, "right": {} }}
        }),
        json!({
            "type": "transform.template", "version": 1, "pluginId": "core", "title": "Template",
            "category": "Transforms", "kind": "transform",
            "inputs": [flow_input(), port("value", "Value", "data", "json", false)],
            "outputs": [flow_output(), port("value", "Value", "data", "string", false)],
            "configSchema": {"type": "object", "properties": {"template": {"type": "string"}}, "required": ["template"]}
        }),
        json!({
            "type": "transform.script", "version": 1, "pluginId": "core", "title": "Script",
            "category": "Transforms", "kind": "transform",
            "inputs": [flow_input(), port("value", "Value", "data", "json", false)],
            "outputs": [flow_output(), port("value", "Value", "data", "json", false)],
            "configSchema": {"type": "object", "properties": {"source": {"type": "string"}}, "required": ["source"]},
            "requiredCapabilities": ["vm.script"]
        }),
        json!({
            "type": "control.delay", "version": 1, "pluginId": "core", "title": "Delay",
            "category": "Control", "kind": "control", "inputs": [flow_input()], "outputs": [flow_output()],
            "configSchema": {"type": "object", "properties": {"delayMs": {"type": "number"}}, "required": ["delayMs"]}
        }),
        json!({
            "type": "control.cooldown", "version": 1, "pluginId": "core", "title": "Cooldown",
            "category": "Control", "kind": "control", "inputs": [flow_input()], "outputs": [flow_output()],
            "configSchema": {"type": "object", "properties": {"durationMs": {"type": "number"}, "key": {"type": "string"}}}
        }),
        json!({
            "type": "action.log", "version": 1, "pluginId": "core", "title": "Log",
            "category": "Actions", "kind": "action", "inputs": [flow_input()], "outputs": [flow_output()],
            "configSchema": {"type": "object", "properties": {"message": {"type": "string"}}}
        }),
        json!({
            "type": "action.http", "version": 1, "pluginId": "core", "title": "HTTP Request",
            "category": "Actions", "kind": "action", "inputs": [flow_input()], "outputs": [flow_output()],
            "configSchema": {"type": "object", "properties": {"method": {"type": "string"}, "url": {"type": "string"}, "headers": {"type": "object"}, "body": {"type": "string"}}},
            "requiredCapabilities": ["http.request"]
        }),
        json!({
            "type": "action.adjust-points", "version": 1, "pluginId": "core", "title": "Adjust Points",
            "category": "Actions", "kind": "action", "inputs": [flow_input()], "outputs": [flow_output()],
            "configSchema": {"type": "object", "properties": {"uniqueId": {"type": "string"}, "delta": {"type": "number"}}},
            "requiredCapabilities": ["points.write"]
        }),
        json!({
            "type": "action.play-sound", "version": 1, "pluginId": "core", "title": "Play Sound",
            "category": "Actions", "kind": "action", "inputs": [flow_input()], "outputs": [flow_output()],
            "configSchema": {"type": "object", "properties": {
                "filePath": {"type": "string"},
                "volume": {"type": "number", "minimum": 0, "maximum": 1},
                "overlap": {"type": "string", "enum": ["allow", "restart", "drop"]}
            }, "required": ["filePath"]},
            "requiredCapabilities": ["audio.play"]
        }),
    ]
}

fn text(default: &str, key: &str) -> Value {
    json!({"default": default, "i18key": key})
}

fn field(key: &str, label: &str, kind: &str, value: &str, i18key: &str, extra: Value) -> Value {
    let mut result = json!({
        "key": key,
        "label": text(label, i18key),
        "kind": kind,
        "value": value,
    });
    if let (Some(target), Some(source)) = (result.as_object_mut(), extra.as_object()) {
        target.extend(source.clone());
    }
    result
}

fn port(name: &str, title: &str, kind: &str, value_type: &str, required: bool) -> Value {
    let mut result = json!({"name": name, "title": title, "kind": kind});
    if !value_type.is_empty() {
        result["valueType"] = Value::String(value_type.to_owned());
    }
    if required {
        result["required"] = Value::Bool(true);
    }
    result
}

fn flow_input() -> Value {
    port("flow", "Flow", "flow", "", false)
}

fn flow_output() -> Value {
    port("flow", "Flow", "flow", "", false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_are_runtime_neutral_json() {
        let actions = builtin_action_types();
        assert_eq!(actions.len(), 8);
        assert_eq!(actions[0]["source"]["kind"], "builtin");
        assert_eq!(
            actions
                .iter()
                .find(|action| action["id"] == "audio.play")
                .unwrap()["requiredCapabilities"][0],
            "audio.play"
        );
        assert_eq!(builtin_node_catalog()[0]["type"], "trigger.event");
        assert!(builtin_node_catalog()
            .iter()
            .any(|node| node["type"] == "action.play-sound"));
    }
}

/// Host-owned automation rule templates, shipped as data and validated
/// against the same grammar every client parses. Served in the behavior
/// snapshot so all clients (WebView, CLI, a future cloud registry) read
/// one catalog instead of the WebView hardcoding its own.
pub fn builtin_rule_templates() -> Vec<Value> {
    static CATALOG: std::sync::OnceLock<Vec<Value>> = std::sync::OnceLock::new();
    CATALOG
        .get_or_init(|| {
            serde_json::from_str(include_str!("builtin-templates.json"))
                .expect("builtin rule templates JSON is valid")
        })
        .clone()
}

#[cfg(test)]
mod catalog_tests {
    #[test]
    fn builtin_rule_templates_pass_the_client_grammar() {
        // Host-owned templates ship in the snapshot for every client to
        // instantiate, so each one must pass the same validator the plugin
        // catalog merge applies — a drift here means a template that renders
        // but never applies.
        for template in crate::services::builtin_rule_templates() {
            tiktools_plugin_api::manifest::validate_plugin_template(&template)
                .unwrap_or_else(|error| panic!("builtin template invalid: {error}: {template}"));
        }
    }
}
