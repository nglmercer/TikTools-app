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
        "template" => {
            if let Some(templates) = result.get("templates").and_then(Value::as_array) {
                if templates.is_empty() {
                    println!("no custom templates");
                    return;
                }
                for template in templates {
                    let id = template.get("id").and_then(Value::as_str).unwrap_or("?");
                    let title = template.get("title").and_then(Value::as_str).unwrap_or("?");
                    let trigger = template
                        .get("trigger")
                        .and_then(Value::as_str)
                        .unwrap_or("?");
                    println!("{id} {title} [{trigger}]");
                }
                return;
            }
            if let Some(imported) = result.get("imported").and_then(Value::as_array) {
                for entry in imported {
                    let event = entry.get("event");
                    let id = event
                        .and_then(|event| event.get("id"))
                        .and_then(Value::as_str)
                        .unwrap_or("?");
                    let name = event
                        .and_then(|event| event.get("name"))
                        .and_then(Value::as_str)
                        .unwrap_or("?");
                    let actions = entry
                        .get("actions")
                        .and_then(Value::as_array)
                        .map(Vec::len)
                        .unwrap_or(0);
                    println!("created event {id} {name} (+{actions} action(s))");
                }
                if result.get("savedToGallery").and_then(Value::as_bool) == Some(true) {
                    println!("saved to template gallery");
                }
                return;
            }
            if let Some(path) = result.get("path").and_then(Value::as_str) {
                println!("wrote {path}");
                return;
            }
            if let Some(deleted) = result.get("deleted").and_then(Value::as_str) {
                println!("deleted {deleted}");
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
        "api-discover" => {
            if let Some(methods) = result.get("methods").and_then(Value::as_array) {
                for method in methods {
                    let name = method.get("name").and_then(Value::as_str).unwrap_or("?");
                    let description = method
                        .get("description")
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    let mut flags = Vec::new();
                    if method.get("sideEffect").and_then(Value::as_bool) == Some(true) {
                        flags.push("writes");
                    }
                    if method.get("destructive").and_then(Value::as_bool) == Some(true) {
                        flags.push("destructive");
                    }
                    if method.get("requiresDesktop").and_then(Value::as_bool) == Some(true) {
                        flags.push("desktop-only");
                    }
                    if flags.is_empty() {
                        println!("{name} - {description}");
                    } else {
                        println!("{name} - {description} [{}]", flags.join(", "));
                    }
                }
                return;
            }
            print_json(&result);
        }
        "api-schema" => {
            let name = result.get("name").and_then(Value::as_str).unwrap_or("?");
            let description = result
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or("");
            let mut flags = Vec::new();
            if result.get("sideEffect").and_then(Value::as_bool) == Some(true) {
                flags.push("writes");
            }
            if result.get("destructive").and_then(Value::as_bool) == Some(true) {
                flags.push("destructive");
            }
            if result.get("requiresDesktop").and_then(Value::as_bool) == Some(true) {
                flags.push("desktop-only");
            }
            if flags.is_empty() {
                println!("{name} - {description}");
            } else {
                println!("{name} - {description} [{}]", flags.join(", "));
            }
            println!("params:");
            print_json(result.get("paramsSchema").unwrap_or(&Value::Null));
            println!("result:");
            print_json(result.get("resultSchema").unwrap_or(&Value::Null));
        }
        "api-call" => {
            print_json(&result);
        }
        "api-verify" => {
            if let Some(coverage) = result.get("coverage") {
                let live = coverage.get("live").and_then(Value::as_u64).unwrap_or(0);
                if coverage.get("ok").and_then(Value::as_bool) == Some(true) {
                    println!("coverage: ok ({live} methods, host == client)");
                } else {
                    let compiled = coverage
                        .get("compiled")
                        .and_then(Value::as_u64)
                        .unwrap_or(0);
                    println!("coverage: MISMATCH (live {live}, compiled {compiled})");
                    print_names("host-only", coverage.get("missing"));
                    print_names("client-only", coverage.get("extra"));
                }
            }
            if let Some(smoke) = result.get("smoke") {
                let passed = smoke.get("passed").and_then(Value::as_u64).unwrap_or(0);
                let failed = smoke.get("failed").and_then(Value::as_u64).unwrap_or(0);
                println!("smoke: {passed} passed, {failed} failed");
                if let Some(results) = smoke.get("results").and_then(Value::as_array) {
                    for entry in results {
                        let method = entry.get("method").and_then(Value::as_str).unwrap_or("?");
                        if entry.get("ok").and_then(Value::as_bool) == Some(true) {
                            println!("  ok {method}");
                        } else {
                            let code = entry.get("code").and_then(Value::as_str).unwrap_or("error");
                            let message = entry
                                .get("message")
                                .and_then(Value::as_str)
                                .unwrap_or("request failed");
                            println!("  FAIL {method} [{code}] {message}");
                        }
                    }
                }
            }
            if result.get("ok").and_then(Value::as_bool) == Some(true) {
                println!("verify: ok");
            } else {
                println!("verify: FAILED");
            }
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

pub fn print_names(label: &str, names: Option<&Value>) {
    let names: Vec<&str> = names
        .and_then(Value::as_array)
        .map(|names| names.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    if !names.is_empty() {
        println!("  {label}: {}", names.join(", "));
    }
}

fn print_json(value: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).unwrap_or_default()
    );
}

/// Placeholder for redacted secrets. Matches the host's
/// `SECRET_SETTING_PLACEHOLDER` so CLI-redacted and host-redacted output
/// look identical.
pub const REDACTED: &str = "••••••••";

/// JSON keys treated as secrets, normalized (lowercase, `-`/`_` removed):
/// `sessionCookie`, `SESSION_COOKIE`, `api-key`, and `apiKey` all match.
const SECRET_FIELDS: &[&str] = &[
    "password",
    "passwd",
    "secret",
    "token",
    "sessioncookie",
    "cookie",
    "authorization",
    "apikey",
    "apitoken",
    "accesstoken",
    "refreshtoken",
    "clientsecret",
    "privatekey",
    "credentials",
];

fn is_secret_key(key: &str) -> bool {
    let normalized: String = key
        .to_ascii_lowercase()
        .chars()
        .filter(|character| *character != '_' && *character != '-')
        .collect();
    SECRET_FIELDS.contains(&normalized.as_str())
}

/// Recursively replaces every secret value with [`REDACTED`]. Objects,
/// arrays, and nested values are all walked; non-string secrets (numbers,
/// nested objects) are replaced wholesale.
pub fn redact_secrets(value: Value) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .into_iter()
                .map(|(key, val)| {
                    if is_secret_key(&key) {
                        (key, Value::String(REDACTED.to_owned()))
                    } else {
                        (key, redact_secrets(val))
                    }
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.into_iter().map(redact_secrets).collect()),
        scalar => scalar,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn redacts_top_level_and_nested_secrets() {
        let value = json!({
            "uniqueId": "someone",
            "sessionCookie": "sessions-stay-secret",
            "nested": {"password": "hunter2", "level": 3},
            "tokens": [{"apiToken": "abc"}, {"kind": "x"}],
        });
        let redacted = redact_secrets(value);
        assert_eq!(redacted["uniqueId"], "someone");
        assert_eq!(redacted["sessionCookie"], REDACTED);
        assert_eq!(redacted["nested"]["password"], REDACTED);
        assert_eq!(redacted["nested"]["level"], 3);
        assert_eq!(redacted["tokens"][0]["apiToken"], REDACTED);
        assert_eq!(redacted["tokens"][1]["kind"], "x");
    }

    #[test]
    fn secret_matching_ignores_case_and_separators() {
        let value = json!({
            "SESSION_COOKIE": "a",
            "Api-Key": "b",
            "accesstoken": "c",
            "nickname": "not-a-secret",
        });
        let redacted = redact_secrets(value);
        assert_eq!(redacted["SESSION_COOKIE"], REDACTED);
        assert_eq!(redacted["Api-Key"], REDACTED);
        assert_eq!(redacted["accesstoken"], REDACTED);
        assert_eq!(redacted["nickname"], "not-a-secret");
    }

    #[test]
    fn replaces_non_string_secrets_wholesale() {
        let value = json!({"credentials": {"user": "u", "pass": "p"}, "count": 2});
        let redacted = redact_secrets(value);
        assert_eq!(redacted["credentials"], REDACTED);
        assert_eq!(redacted["count"], 2);
    }
}
