//! Human output (one line per item for lists; pretty JSON otherwise).

use serde_json::Value;

pub fn print_human(command: &str, result: Value) {
    match command {
        "plugin" => {
            if let Some(plugins) = result.get("plugins").and_then(Value::as_array) {
                if plugins.is_empty() {
                    println!("no plugins installed");
                    return;
                }
                for plugin in plugins {
                    let descriptor = plugin.get("descriptor");
                    let id = descriptor
                        .and_then(|descriptor| descriptor.get("id"))
                        .and_then(Value::as_str)
                        .or_else(|| plugin.get("id").and_then(Value::as_str))
                        .unwrap_or("?");
                    let version = descriptor
                        .and_then(|descriptor| descriptor.get("version"))
                        .and_then(Value::as_str)
                        .unwrap_or("?");
                    let mut flags = Vec::new();
                    for (key, label) in [
                        ("installed", "installed"),
                        ("enabled", "enabled"),
                        ("running", "running"),
                        ("available", "available"),
                    ] {
                        if plugin.get(key).and_then(Value::as_bool) == Some(true) {
                            flags.push(label);
                        }
                    }
                    println!("{id} {version} [{}]", flags.join(","));
                }
                return;
            }
            print_json(&result);
        }
        "automation" => {
            let events = result
                .get("events")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let actions = result
                .get("actions")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            if result.get("events").is_some() || result.get("actions").is_some() {
                for record in events.iter().chain(actions.iter()) {
                    let id = record.get("id").and_then(Value::as_str).unwrap_or("?");
                    let name = record.get("name").and_then(Value::as_str).unwrap_or("?");
                    let enabled = record.get("enabled").and_then(Value::as_bool) == Some(true);
                    println!(
                        "{} {} [{}]",
                        id,
                        name,
                        if enabled { "enabled" } else { "disabled" }
                    );
                }
                if events.is_empty() && actions.is_empty() {
                    println!("no automations");
                }
                return;
            }
            print_json(&result);
        }
        "points" => {
            if let Some(viewers) = result.get("viewers").and_then(Value::as_array) {
                if viewers.is_empty() {
                    println!("no viewers yet");
                    return;
                }
                for viewer in viewers {
                    let id = viewer
                        .get("uniqueId")
                        .and_then(Value::as_str)
                        .unwrap_or("?");
                    let points = viewer
                        .get("points")
                        .and_then(Value::as_f64)
                        .unwrap_or_default();
                    let level = viewer.get("level").and_then(Value::as_u64).unwrap_or(1);
                    println!("{id}: {points} (level {level})");
                }
                return;
            }
            print_json(&result);
        }
        "system" => {
            if let Some(checks) = result.get("checks").and_then(Value::as_array) {
                println!(
                    "doctor ok: {}",
                    result.get("ok").and_then(Value::as_bool) == Some(true)
                );
                for check in checks {
                    let id = check.get("id").and_then(Value::as_str).unwrap_or("?");
                    let status = check.get("status").and_then(Value::as_str).unwrap_or("?");
                    let message = check.get("message").and_then(Value::as_str).unwrap_or("");
                    if message.is_empty() {
                        println!("  [{status}] {id}");
                    } else {
                        println!("  [{status}] {id}: {message}");
                    }
                }
                return;
            }
            print_json(&result);
        }
        "workflow" => {
            if let Some(workflows) = result.get("workflows").and_then(Value::as_array) {
                if workflows.is_empty() {
                    println!("no workflows");
                    return;
                }
                for workflow in workflows {
                    let id = workflow.get("id").and_then(Value::as_str).unwrap_or("?");
                    let name = workflow.get("name").and_then(Value::as_str).unwrap_or("?");
                    let enabled = workflow.get("enabled").and_then(Value::as_bool) == Some(true);
                    println!(
                        "{} {} [{}]",
                        id,
                        name,
                        if enabled { "enabled" } else { "disabled" }
                    );
                }
                return;
            }
            print_json(&result);
        }
        _ => print_json(&result),
    }
}

pub fn print_json(value: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).unwrap_or_default()
    );
}
