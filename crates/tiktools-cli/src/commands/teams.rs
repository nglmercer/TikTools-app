//! Headless owner commands. Business rules and schemas remain in the control API.
use super::{redact_output, CommandError, Output};
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::PathBuf, time::Duration};
use tiktools_client::{validate_params, ClientError, TikToolsClient};

pub const USAGE: &str = "teams list --owned | create --name NAME | host enroll --team ID --label NAME | host start [--wait] | host status|stop | people search NICKNAME | invitations list|create|revoke | requests list|approve|deny | grants list|update|revoke | sessions list|revoke | doctor; use --json, --timeout SEC and --confirm for writes; invitations create requires --output-file NEW_PATH";

pub struct Command {
    method: &'static str,
    params: Value,
    timeout: Duration,
    wait: bool,
    output_file: Option<PathBuf>,
}

/// Strict option parsing shared with remote onboarding. Unknown and duplicate flags fail.
pub(crate) fn options(
    args: &[String],
    values: &[&str],
    flags: &[&str],
) -> Result<BTreeMap<String, String>, String> {
    let mut parsed = BTreeMap::new();
    let mut args = args.iter();
    while let Some(key) = args.next() {
        if parsed.contains_key(key) {
            return Err(format!("duplicate option {key}"));
        }
        let value = if flags.contains(&key.as_str()) {
            String::new()
        } else if values.contains(&key.as_str()) {
            args.next()
                .filter(|v| !v.starts_with("--"))
                .ok_or_else(|| format!("{key} needs a value"))?
                .clone()
        } else {
            return Err(format!("unsupported option {key}"));
        };
        parsed.insert(key.clone(), value);
    }
    Ok(parsed)
}

pub(crate) fn timeout(opts: &BTreeMap<String, String>, default: u64) -> Result<Duration, String> {
    let seconds = opts
        .get("--timeout")
        .map(|v| v.parse::<u64>())
        .transpose()
        .map_err(|_| "--timeout must be an integer".to_owned())?
        .unwrap_or(default);
    if !(1..=600).contains(&seconds) {
        return Err("--timeout must be between 1 and 600 seconds".into());
    }
    Ok(Duration::from_secs(seconds))
}

pub fn parse(args: &[String]) -> Result<Command, CommandError> {
    let first = args
        .first()
        .map(String::as_str)
        .ok_or_else(|| USAGE.to_owned())?;
    let second = args.get(1).map(String::as_str).unwrap_or("");
    type Spec<'a> = (
        &'static str,
        usize,
        &'a [(&'a str, &'a str)],
        &'a [(&'a str, &'a str)],
        bool,
    );
    let (method, skip, required, optional, write): Spec<'_> = match (first, second) {
        ("list", _) => ("teams.hosts.list", 1, &[], &[], false),
        ("create", _) => ("teams.hosts.create", 1, &[("--name", "name")], &[], true),
        ("host", "enroll") => (
            "teams.devices.enroll",
            2,
            &[("--team", "hostId"), ("--label", "label")],
            &[],
            true,
        ),
        ("host", "start") => ("teams.host.start", 2, &[], &[], true),
        ("host", "status") => ("teams.host.status", 2, &[], &[], false),
        ("host", "stop") => ("teams.host.stop", 2, &[], &[], true),
        ("people", "search") => ("teams.people.search", 3, &[], &[], false),
        ("invitations", "list") => (
            "teams.invitations.list",
            2,
            &[("--team", "hostId")],
            &[],
            false,
        ),
        ("invitations", "create") => (
            "teams.invitations.create",
            2,
            &[("--team", "hostId"), ("--role", "requestedRole")],
            &[("--recipient", "recipientId"), ("--ttl", "ttlSeconds")],
            true,
        ),
        ("invitations", "revoke") => (
            "teams.invitations.revoke",
            2,
            &[("--team", "hostId"), ("--invitation", "invitationId")],
            &[],
            true,
        ),
        ("requests", "list") => (
            "teams.requests.list",
            2,
            &[("--team", "hostId")],
            &[],
            false,
        ),
        ("requests", "approve" | "deny") => (
            "teams.requests.decide",
            2,
            &[("--team", "hostId"), ("--request", "requestId")],
            &[("--role", "role")],
            true,
        ),
        ("grants", "list") => ("teams.grants.list", 2, &[("--team", "hostId")], &[], false),
        ("grants", "update") => (
            "teams.grants.update",
            2,
            &[
                ("--team", "hostId"),
                ("--grant", "grantId"),
                ("--expected-revision", "expectedRevision"),
            ],
            &[("--role", "role"), ("--policy", "policy")],
            true,
        ),
        ("grants", "revoke") => (
            "teams.grants.revoke",
            2,
            &[
                ("--team", "hostId"),
                ("--grant", "grantId"),
                ("--expected-revision", "expectedRevision"),
            ],
            &[],
            true,
        ),
        ("sessions", "list") => (
            "teams.sessions.list",
            2,
            &[("--team", "hostId")],
            &[],
            false,
        ),
        ("sessions", "revoke") => (
            "teams.sessions.revoke",
            2,
            &[("--team", "hostId"), ("--session", "sessionId")],
            &[],
            true,
        ),
        ("doctor", _) => ("doctor", 1, &[], &[], false),
        _ => return Err(USAGE.to_owned().into()),
    };
    let mut values: Vec<_> = required
        .iter()
        .chain(optional)
        .map(|(flag, _)| *flag)
        .collect();
    values.push("--timeout");
    if method == "doctor" {
        values.push("--context");
    }
    if method == "teams.invitations.create" {
        values.push("--output-file");
    }
    let mut flags = vec!["--confirm"];
    if first == "list" {
        flags.push("--owned");
    }
    if method == "teams.host.start" {
        flags.push("--wait");
    }
    if skip > args.len() {
        return Err("teams people search NICKNAME".to_owned().into());
    }
    let opts = options(&args[skip..], &values, &flags)?;
    if opts
        .get("--context")
        .is_some_and(|context| context != "host")
    {
        return Err(
            "use teams doctor --context host or teams doctor --context controller"
                .to_owned()
                .into(),
        );
    }
    if write && !opts.contains_key("--confirm") {
        return Err("Teams writes require --confirm".to_owned().into());
    }
    let mut params = serde_json::Map::new();
    for (flag, key) in required.iter().chain(optional) {
        let Some(value) = opts.get(*flag) else {
            if required.iter().any(|(required, _)| required == flag) {
                return Err(format!("{flag} is required").into());
            }
            continue;
        };
        let value = match *key {
            "ttlSeconds" | "expectedRevision" => json!(value
                .parse::<i64>()
                .map_err(|_| format!("{flag} must be an integer"))?),
            "policy" => super::args::parse_json_value(value)?,
            _ => Value::String(value.clone()),
        };
        params.insert((*key).into(), value);
    }
    if method == "teams.devices.enroll" {
        params.insert("kind".into(), json!("host"));
    }
    if method == "teams.people.search" {
        params.insert("query".into(), json!(args[2]));
    }
    if method == "teams.requests.decide" {
        params.insert("decision".into(), json!(second));
    }
    let output_file = opts.get("--output-file").map(PathBuf::from);
    if method == "teams.invitations.create" && output_file.is_none() {
        return Err("--output-file is required for the private invitation"
            .to_owned()
            .into());
    }
    let params = Value::Object(params);
    if method != "doctor" {
        validate_params(method, &params).map_err(|e| e.message)?;
    }
    Ok(Command {
        method,
        params,
        timeout: timeout(&opts, 30)?,
        wait: opts.contains_key("--wait"),
        output_file,
    })
}

pub async fn execute(client: &TikToolsClient, command: Command) -> Result<Output, ClientError> {
    let deadline = tokio::time::Instant::now() + command.timeout;
    let call = async {
        if command.method == "doctor" {
            return doctor(client).await;
        }
        // Reserve the destination before an effectful request. Never overwrite a private file.
        let mut file = command
            .output_file
            .as_ref()
            .map(|path| {
                let mut options = std::fs::OpenOptions::new();
                options.write(true).create_new(true);
                #[cfg(unix)]
                {
                    use std::os::unix::fs::OpenOptionsExt;
                    options.mode(0o600);
                }
                options.open(path).map_err(|_| {
                    ClientError::new(
                        "output_unavailable",
                        "cannot create private invitation output",
                    )
                })
            })
            .transpose()?;
        let result = client.call_value(command.method, &command.params).await;
        let mut result = match result {
            Ok(value) => value,
            Err(error) => {
                if let Some(path) = &command.output_file {
                    let _ = std::fs::remove_file(path);
                }
                return Err(error);
            }
        };
        if let Some(file) = file.as_mut() {
            use std::io::Write;
            let token = result
                .get_mut("token")
                .and_then(|v| v.take().as_str().map(str::to_owned))
                .ok_or_else(|| ClientError::new("protocol", "invitation response omitted token"))?;
            file.write_all(token.as_bytes()).and_then(|_| file.write_all(b"\n")).and_then(|_| file.sync_all())
                .map_err(|_| ClientError::new("output_unavailable", "invitation was created but private output failed; revoke it before retrying"))?;
            if let Some(value) = result.as_object_mut() {
                value.remove("token");
            }
            result["outputFile"] = json!(command.output_file);
        }
        if command.wait {
            while result.get("state").and_then(Value::as_str) != Some("ready") {
                tokio::time::sleep(Duration::from_millis(200)).await;
                result = client.call_value("teams.host.status", json!({})).await?;
                if result.get("state").and_then(Value::as_str) == Some("stopped") {
                    return Err(ClientError::new(
                        "host_stopped",
                        "host stopped before becoming ready",
                    ));
                }
            }
        }
        Ok(Output::new("teams", redact_output(result, false)))
    };
    tokio::time::timeout_at(deadline, call).await.map_err(|_| {
        ClientError::new(
            "timeout",
            "Teams command deadline exceeded; accepted writes may still complete",
        )
    })?
}

async fn doctor(client: &TikToolsClient) -> Result<Output, ClientError> {
    let mut checks = serde_json::Map::new();
    let mut ok = true;
    for method in [
        "system.info",
        "cloud.status",
        "teams.devices.list",
        "teams.hosts.list",
        "teams.host.status",
    ] {
        match client.call_value(method, json!({})).await {
            Ok(value) => {
                checks.insert(method.into(), redact_output(value, false));
            }
            Err(error) => {
                ok = false;
                checks.insert(
                    method.into(),
                    json!({"error": {"code":error.code,"message":error.message,"data":error.data}}),
                );
            }
        }
    }
    let mut next_steps = Vec::new();
    if checks
        .get("cloud.status")
        .and_then(|v| v.get("state"))
        .and_then(Value::as_str)
        != Some("signed-in")
    {
        next_steps
            .push(json!({"reason":"authentication_required","command":"tiktools cloud login"}));
    }
    if checks
        .get("teams.hosts.list")
        .and_then(|v| v.get("items"))
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty)
    {
        next_steps.push(json!({"reason":"team_missing","command":"tiktools teams create --name NAME --confirm"}));
    }
    let enrolled = checks
        .get("teams.devices.list")
        .and_then(|v| v.get("items"))
        .and_then(Value::as_array)
        .is_some_and(|items| {
            items.iter().any(|item| {
                item["kind"] == "host" && item["isThisDevice"] == true && item["status"] == "active"
            })
        });
    if !enrolled {
        next_steps.push(json!({"reason":"host_not_enrolled","command":"tiktools teams host enroll --team ID --label NAME --confirm"}));
    }
    if checks
        .get("teams.host.status")
        .and_then(|v| v.get("state"))
        .and_then(Value::as_str)
        != Some("ready")
    {
        next_steps.push(json!({"reason":"host_not_serving","command":"tiktools teams host start --wait --timeout 30 --confirm"}));
    }
    let ready = ok && next_steps.is_empty();
    Ok(Output::new(
        "teams-doctor",
        json!({"ok":ok,"ready":ready,"context":"host","checks":checks,"nextSteps":next_steps}),
    )
    .exit_code(i32::from(!ok)))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse_args(args: &[&str]) -> Result<Command, CommandError> {
        parse(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }
    #[test]
    fn rejects_unconfirmed_writes_and_unknown_or_duplicate_options() {
        assert!(parse_args(&["create", "--name", "Test"]).is_err());
        assert!(parse_args(&["create", "--name", "A", "--name", "B", "--confirm"]).is_err());
        assert!(parse_args(&["list", "--token", "secret"]).is_err());
        assert!(parse_args(&["host", "start", "--confirm", "--timeout", "0"]).is_err());
    }
    #[test]
    fn requires_private_output_for_invitations() {
        assert!(parse_args(&[
            "invitations",
            "create",
            "--team",
            "11111111-1111-4111-8111-111111111111",
            "--role",
            "observer",
            "--confirm"
        ])
        .is_err());
    }
    #[test]
    fn maps_owner_commands_to_existing_contracts() {
        let create = parse_args(&["create", "--name", "Lab", "--confirm"])
            .unwrap_or_else(|_| panic!("valid create"));
        assert_eq!(create.method, "teams.hosts.create");
        assert_eq!(create.params, json!({"name":"Lab"}));
        let host = parse_args(&["host", "start", "--wait", "--confirm", "--timeout", "2"])
            .unwrap_or_else(|_| panic!("valid start"));
        assert!(host.wait);
        assert_eq!(host.timeout, Duration::from_secs(2));
    }
}
