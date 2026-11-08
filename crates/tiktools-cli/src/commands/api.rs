//! `api` commands: the generic agent-oriented control surface.
//!
//! - `discover` lists every method live from the host registry.
//! - `schema` shows one method's description, flags, and schemas.
//! - `call` resolves the method, validates params, then executes
//!   (unless `--dry-run`); destructive methods need `--confirm`.
//! - `events` streams NDJSON domain events with topic filtering.
//!
//! `call` output is secret-redacted by default; only `--secrets-visible`
//! opts out. `--explain` prints the call plan (redacted) to stderr and
//! `--format json|human` overrides the global `--json` for one command.

use std::time::Duration;

use serde_json::{json, Value};
use tiktools_client::{
    validate_params, ClientError, ControlEvent, RequestMetadata, TikToolsClient,
};
use tiktools_control_api::modules::rpc::SchemaParams;

use super::args::{flag_value, parse_json_value, positional, split_first};
use super::{redact_output, result_value, CommandError, Output};

pub enum Command {
    Discover {
        json: Option<bool>,
    },
    Schema {
        method: String,
        json: Option<bool>,
    },
    Call {
        method: String,
        params: Value,
        timeout_secs: Option<u64>,
        metadata: Option<RequestMetadata>,
        include_meta: bool,
        dry_run: bool,
        confirm: bool,
        secrets_visible: bool,
        explain: bool,
        json: Option<bool>,
    },
    Events {
        topics: Vec<String>,
        timeout_secs: Option<u64>,
        max_events: Option<u64>,
    },
    Verify {
        coverage: bool,
        smoke: bool,
        timeout_secs: Option<u64>,
        json: Option<bool>,
    },
}

impl Command {
    pub fn output_json(&self) -> Option<bool> {
        match self {
            Self::Discover { json }
            | Self::Schema { json, .. }
            | Self::Call { json, .. }
            | Self::Verify { json, .. } => *json,
            Self::Events { .. } => None,
        }
    }
}

/// True for `api verify --sandbox`, which the dispatcher routes to an
/// isolated throwaway runtime before any client exists.
pub fn is_sandbox_verify(args: &[String]) -> bool {
    args.len() >= 2
        && args[0] == "api"
        && args[1] == "verify"
        && args.iter().any(|arg| arg == "--sandbox")
}

pub fn parse(args: &[String]) -> Result<Command, CommandError> {
    let (verb, rest) = split_first(args, "api <discover|schema|call|events> ...")?;
    match verb {
        "discover" => Ok(Command::Discover {
            json: parse_format(rest)?,
        }),
        "schema" => Ok(Command::Schema {
            method: positional(rest, 0, "api schema <method> [--format json|human]")?,
            json: parse_format(rest)?,
        }),
        "call" => {
            validate_call_flags(rest)?;
            let positionals = positional_args(rest);
            if positionals.len() > 2 {
                return Err("unexpected api call arguments".to_owned().into());
            }
            let method = positionals
                .first()
                .ok_or_else(|| "api call <method> [params-json] [flags]".to_owned())?
                .to_string();
            let params = positionals
                .get(1)
                .map(|raw| parse_json_value(raw))
                .transpose()?
                .unwrap_or(json!({}));
            let secrets_visible = rest.iter().any(|arg| arg == "--secrets-visible");
            if secrets_visible && rest.iter().any(|arg| arg == "--redact") {
                return Err("--redact and --secrets-visible are mutually exclusive"
                    .to_owned()
                    .into());
            }
            Ok(Command::Call {
                method,
                params,
                timeout_secs: parse_timeout(rest)?,
                metadata: parse_metadata(rest)?,
                include_meta: rest.iter().any(|arg| arg == "--include-meta"),
                dry_run: rest.iter().any(|arg| arg == "--dry-run"),
                confirm: rest.iter().any(|arg| arg == "--confirm"),
                secrets_visible,
                explain: rest.iter().any(|arg| arg == "--explain"),
                json: parse_format(rest)?,
            })
        }
        "events" => Ok(Command::Events {
            topics: event_topics(rest),
            timeout_secs: parse_timeout(rest)?,
            max_events: parse_max_events(rest)?,
        }),
        "verify" => {
            let only_coverage = rest.iter().any(|arg| arg == "--coverage");
            let only_smoke = rest.iter().any(|arg| arg == "--smoke");
            let both = !only_coverage && !only_smoke;
            Ok(Command::Verify {
                coverage: both || only_coverage,
                smoke: both || only_smoke,
                timeout_secs: parse_timeout(rest)?,
                json: parse_format(rest)?,
            })
        }
        other => Err(format!("unknown api verb `{other}`").into()),
    }
}

/// Positional arguments (everything that is not a `--flag` or a flag value).
fn positional_args(args: &[String]) -> Vec<&String> {
    const VALUE_FLAGS: &[&str] = &[
        "--timeout",
        "--max-events",
        "--format",
        "--topics",
        "--command-id",
        "--host-instance-id",
        "--expected-revision",
    ];
    let mut positionals = Vec::new();
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if VALUE_FLAGS.contains(&arg.as_str()) {
            index += 2;
            continue;
        }
        if arg.starts_with("--") {
            index += 1;
            continue;
        }
        positionals.push(arg);
        index += 1;
    }
    positionals
}

fn validate_call_flags(args: &[String]) -> Result<(), CommandError> {
    let values = [
        "--timeout",
        "--format",
        "--command-id",
        "--host-instance-id",
        "--expected-revision",
    ];
    let switches = [
        "--dry-run",
        "--confirm",
        "--redact",
        "--secrets-visible",
        "--explain",
        "--include-meta",
    ];
    let mut index = 0;
    while index < args.len() {
        let arg = args[index].as_str();
        if values.contains(&arg) {
            if args
                .get(index + 1)
                .is_none_or(|value| value.starts_with("--"))
            {
                return Err(format!("{arg} requires a value").into());
            }
            index += 2;
        } else if arg.starts_with("--") && !switches.contains(&arg) {
            return Err(format!("unknown api call flag `{arg}`").into());
        } else {
            index += 1;
        }
    }
    Ok(())
}

fn parse_metadata(args: &[String]) -> Result<Option<RequestMetadata>, CommandError> {
    fn read(args: &[String], flag: &str) -> Result<Option<String>, CommandError> {
        let mut positions = args
            .iter()
            .enumerate()
            .filter(|(_, arg)| arg.as_str() == flag);
        let Some((index, _)) = positions.next() else {
            return Ok(None);
        };
        if positions.next().is_some() {
            return Err(format!("{flag} must occur once").into());
        }
        let value = args
            .get(index + 1)
            .filter(|value| !value.starts_with("--"))
            .ok_or_else(|| format!("{flag} requires a value"))?;
        Ok(Some(value.clone()))
    }
    let metadata = RequestMetadata {
        command_id: read(args, "--command-id")?,
        host_instance_id: read(args, "--host-instance-id")?,
        expected_revision: read(args, "--expected-revision")?,
    };
    if metadata.command_id.is_none()
        && metadata.host_instance_id.is_none()
        && metadata.expected_revision.is_none()
    {
        return Ok(None);
    }
    if let Some(revision) = &metadata.expected_revision {
        if revision
            .parse::<i64>()
            .ok()
            .filter(|number| *number >= 0 && number.to_string() == *revision)
            .is_none()
        {
            return Err(
                "--expected-revision requires a canonical nonnegative integer"
                    .to_owned()
                    .into(),
            );
        }
    }
    if metadata.command_id.is_some() || metadata.host_instance_id.is_some() {
        let (Some(id), Some(instance)) = (&metadata.command_id, &metadata.host_instance_id) else {
            return Err("--command-id and --host-instance-id are required together"
                .to_owned()
                .into());
        };
        tiktools_core::collaboration::AdditiveCommandRequest {
            command_id: id.clone(),
            host_instance_id: instance.clone(),
        }
        .validate()
        .map_err(|_| "invalid command metadata".to_owned())?;
    }
    Ok(Some(metadata))
}

fn parse_format(args: &[String]) -> Result<Option<bool>, CommandError> {
    match flag_value(args, "--format") {
        None => Ok(None),
        Some(raw) => match raw.as_str() {
            "json" => Ok(Some(true)),
            "human" => Ok(Some(false)),
            other => Err(format!("--format must be json or human, got `{other}`").into()),
        },
    }
}

fn parse_timeout(args: &[String]) -> Result<Option<u64>, CommandError> {
    match flag_value(args, "--timeout") {
        None => Ok(None),
        Some(raw) => {
            let secs: u64 = raw
                .parse()
                .map_err(|_| "--timeout must be at least 1 second".to_owned())?;
            if secs == 0 {
                return Err("--timeout must be at least 1 second".to_owned().into());
            }
            Ok(Some(secs))
        }
    }
}

fn parse_max_events(args: &[String]) -> Result<Option<u64>, CommandError> {
    match flag_value(args, "--max-events") {
        None => Ok(None),
        Some(raw) => {
            let max: u64 = raw
                .parse()
                .map_err(|_| "--max-events must be at least 1".to_owned())?;
            if max == 0 {
                return Err("--max-events must be at least 1".to_owned().into());
            }
            Ok(Some(max))
        }
    }
}

/// `--topics` filters: repeatable and comma-separated (`--topics live.*`
/// `--topics points.changed`). Empty means no filtering.
fn event_topics(args: &[String]) -> Vec<String> {
    let mut topics = Vec::new();
    let mut index = 0;
    while index < args.len() {
        if args[index] == "--topics" {
            if let Some(raw) = args.get(index + 1) {
                topics.extend(
                    raw.split(',')
                        .map(str::trim)
                        .filter(|topic| !topic.is_empty())
                        .map(str::to_owned),
                );
            }
            index += 2;
        } else {
            index += 1;
        }
    }
    topics
}

pub async fn execute(client: &TikToolsClient, command: Command) -> Result<Output, ClientError> {
    match command {
        Command::Discover { json } => {
            let discovered = client.rpc_discover().await?;
            Ok(Output::new("api-discover", result_value(discovered)?).json_override(json))
        }
        Command::Schema { method, json } => {
            let meta = client.rpc_schema(SchemaParams { method }).await?;
            Ok(Output::new("api-schema", result_value(meta)?).json_override(json))
        }
        Command::Call {
            method,
            params,
            timeout_secs,
            metadata,
            include_meta,
            dry_run,
            confirm,
            secrets_visible,
            explain,
            json,
        } => {
            execute_call(
                client,
                CallArgs {
                    method: &method,
                    params,
                    timeout_secs,
                    metadata,
                    include_meta,
                    dry_run,
                    confirm,
                    secrets_visible,
                    explain,
                    json,
                },
            )
            .await
        }
        Command::Events {
            topics,
            timeout_secs,
            max_events,
        } => {
            stream_events(client, &topics, timeout_secs, max_events).await?;
            Ok(Output::written())
        }
        Command::Verify {
            coverage,
            smoke,
            timeout_secs,
            json,
        } => execute_verify(client, coverage, smoke, timeout_secs, json).await,
    }
}

struct CallArgs<'a> {
    method: &'a str,
    params: Value,
    timeout_secs: Option<u64>,
    metadata: Option<RequestMetadata>,
    include_meta: bool,
    dry_run: bool,
    confirm: bool,
    secrets_visible: bool,
    explain: bool,
    json: Option<bool>,
}

async fn execute_call(client: &TikToolsClient, args: CallArgs<'_>) -> Result<Output, ClientError> {
    // Resolve the method live (unknown names fail here) and validate the
    // params against the registry's own typed shape before executing.
    let meta = client
        .rpc_schema(SchemaParams {
            method: args.method.to_owned(),
        })
        .await?;
    validate_params(args.method, &args.params)?;
    if let Some(metadata) = &args.metadata {
        if metadata.command_id.is_some() && !meta.durable_commands {
            return Err(ClientError::new(
                "unsupported_remote",
                "this method does not support durable commands",
            ));
        }
        if metadata.expected_revision.is_some()
            && meta.concurrency
                != Some(tiktools_control_api::registry::ConcurrencyStrategy::CompareAndSet)
        {
            return Err(ClientError::new(
                "invalid_params",
                "this method does not accept expectedRevision",
            ));
        }
        if metadata.command_id.is_some()
            && meta.concurrency
                == Some(tiktools_control_api::registry::ConcurrencyStrategy::CompareAndSet)
            && metadata.expected_revision.is_none()
        {
            return Err(ClientError::new(
                "invalid_params",
                "compare-and-set commands require expectedRevision",
            ));
        }
    }
    if args.explain {
        explain_call(&meta, &args.params, args.metadata.as_ref());
    }
    if args.dry_run {
        return Ok(Output::new(
            "api-call",
            json!({"ok": true, "dryRun": true, "method": args.method}),
        )
        .json_override(args.json));
    }
    if meta.destructive && !args.confirm {
        return Err(ClientError::new(
            "confirmation_required",
            format!(
                "refusing to run destructive method `{}` without --confirm",
                args.method
            ),
        ));
    }
    let result = match args.timeout_secs {
        Some(secs) => {
            match tokio::time::timeout(
                Duration::from_secs(secs),
                client.call_value_with_meta(args.method, &args.params, args.metadata.clone()),
            )
            .await
            {
                Ok(outcome) => outcome?,
                Err(_) => {
                    let mut error = ClientError::new(
                        "timeout",
                        format!(
                            "call timed out after {secs}s; accepted operation may still complete"
                        ),
                    );
                    if let Some(id) = args
                        .metadata
                        .as_ref()
                        .and_then(|meta| meta.command_id.as_ref())
                    {
                        error.data = Some(json!({"commandId":id,"state":"unknown"}));
                    }
                    return Err(error);
                }
            }
        }
        None => {
            client
                .call_value_with_meta(args.method, &args.params, args.metadata)
                .await?
        }
    };
    let result = if args.include_meta {
        json!({"result":result.result,"meta":result.meta})
    } else {
        result.result
    };
    Ok(
        Output::new("api-call", redact_output(result, args.secrets_visible))
            .json_override(args.json),
    )
}

/// Prints the call plan to stderr (stdout stays pure result). Params are
/// always redacted here: `--explain` must never leak secrets.
fn explain_call(
    meta: &tiktools_client::MethodMeta,
    params: &Value,
    metadata: Option<&RequestMetadata>,
) {
    if let Some(metadata) = metadata {
        eprintln!(
            "metadata: {}",
            serde_json::to_string(metadata).unwrap_or_default()
        );
    }
    let mut flags = Vec::new();
    if meta.side_effect {
        flags.push("writes");
    }
    if meta.destructive {
        flags.push("destructive");
    }
    if meta.requires_desktop {
        flags.push("desktop-only");
    }
    eprintln!("method: {}", meta.name);
    eprintln!("description: {}", meta.description);
    eprintln!(
        "flags: {}",
        if flags.is_empty() {
            "read-only".to_owned()
        } else {
            flags.join(", ")
        }
    );
    eprintln!(
        "params: {}",
        serde_json::to_string_pretty(&redact_output(params.clone(), false)).unwrap_or_default()
    );
}

/// Streams NDJSON events until `--max-events`, `--timeout`, the host going
/// away, or Ctrl-C. Every line is `{topic, data}`; local or host gaps
/// surface as `event.gap` markers so consumers resync explicitly.
async fn stream_events(
    client: &TikToolsClient,
    topics: &[String],
    timeout_secs: Option<u64>,
    max_events: Option<u64>,
) -> Result<(), ClientError> {
    #[cfg(feature = "remote-client")]
    if let TikToolsClient::Remote(remote) = client {
        return crate::remote::events(remote.clone(), topics, timeout_secs, max_events).await;
    }
    eprintln!("tiktools: listening for events (Ctrl-C to stop)");
    let mut events = client.subscribe();
    let deadline = timeout_secs.map(|secs| tokio::time::Instant::now() + Duration::from_secs(secs));
    let mut printed: u64 = 0;
    loop {
        if max_events.is_some_and(|max| printed >= max) {
            break;
        }
        let event = match deadline {
            Some(deadline) => match tokio::time::timeout_at(deadline, events.recv()).await {
                Ok(outcome) => outcome,
                Err(_) => break,
            },
            None => events.recv().await,
        };
        match event {
            Ok(ControlEvent::Reliable(event)) => {
                if !topic_allowed(topics, &event.topic) {
                    continue;
                }
                println!("{}", serde_json::to_string(&event).unwrap_or_default());
                printed += 1;
            }
            Ok(ControlEvent::Domain(event)) => {
                if !topic_allowed(topics, event.topic()) {
                    continue;
                }
                let line = serde_json::to_value(&event).unwrap_or(Value::Null);
                println!("{}", serde_json::to_string(&line).unwrap_or_default());
                printed += 1;
            }
            Ok(ControlEvent::Gap { lost }) => {
                println!(
                    "{}",
                    json!({"topic": "event.gap", "data": {"lost": lost, "resync": true}})
                );
                printed += 1;
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(lost)) => {
                println!(
                    "{}",
                    json!({"topic": "event.gap", "data": {"lost": lost, "resync": true}})
                );
                printed += 1;
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                eprintln!("tiktools: event stream closed by the host");
                break;
            }
        }
    }
    Ok(())
}

/// Read-only smoke calls: every entry must succeed with fixed safe
/// params on any healthy host. Coverage of mutating methods belongs to
/// the host test suite, never to a CLI that agents run against live data.
const SMOKE_CALLS: &[(&str, &str)] = &[
    ("system.ping", "{}"),
    ("system.info", "{}"),
    ("system.health", "{}"),
    ("system.snapshot", "{}"),
    ("rpc.discover", "{}"),
    ("plugins.list", "{}"),
    ("plugins.diagnostics", "{}"),
    ("automation.list", r#"{"kind":"all"}"#),
    ("automation.context", "{}"),
    ("automation.runs", "{}"),
    ("automation.nodes.list", "{}"),
    ("points.config.get", "{}"),
    ("points.leaderboard", "{}"),
    ("workflows.list", "{}"),
    ("processors.list", "{}"),
    ("processors.status", "{}"),
    ("creators.recent", "{}"),
    ("gifts.list", "{}"),
    ("gifts.debug", "{}"),
    ("live.status", "{}"),
    ("app.state.get", "{}"),
    ("globals.list", "{}"),
];

/// Default per-call budget for verification. The host budget is 150s;
/// verification calls are all cheap reads and fail fast instead.
const VERIFY_TIMEOUT_SECS: u64 = 30;

async fn execute_verify(
    client: &TikToolsClient,
    coverage: bool,
    smoke: bool,
    timeout_secs: Option<u64>,
    json: Option<bool>,
) -> Result<Output, ClientError> {
    let budget = Duration::from_secs(timeout_secs.unwrap_or(VERIFY_TIMEOUT_SECS));
    let mut report = serde_json::Map::new();
    let mut ok = true;
    if coverage {
        let (coverage_ok, value) = verify_coverage(client, budget).await?;
        ok &= coverage_ok;
        report.insert("coverage".to_owned(), value);
    }
    if smoke {
        let (smoke_ok, value) = verify_smoke(client, budget).await;
        ok &= smoke_ok;
        report.insert("smoke".to_owned(), value);
    }
    report.insert("ok".to_owned(), Value::Bool(ok));
    Ok(Output::new("api-verify", Value::Object(report))
        .json_override(json)
        .exit_code(i32::from(!ok)))
}

/// Compares the host's live registry against the typed methods compiled
/// into this CLI. The parity test pins the two at build time; this pins
/// them at deploy time, catching CLI/host version skew.
async fn verify_coverage(
    client: &TikToolsClient,
    budget: Duration,
) -> Result<(bool, Value), ClientError> {
    let discovered: tiktools_control_api::modules::rpc::DiscoverResult =
        call_with_budget(client, "rpc.discover", json!({}), budget).await?;
    let live: std::collections::BTreeSet<&str> = discovered
        .methods
        .iter()
        .map(|meta| meta.name.as_str())
        .collect();
    let compiled: std::collections::BTreeSet<&str> =
        TikToolsClient::covered_methods().into_iter().collect();
    let missing: Vec<&str> = live.difference(&compiled).copied().collect();
    let extra: Vec<&str> = compiled.difference(&live).copied().collect();
    let ok = missing.is_empty() && extra.is_empty();
    Ok((
        ok,
        json!({
            "ok": ok,
            "live": live.len(),
            "compiled": compiled.len(),
            "missing": missing,
            "extra": extra,
        }),
    ))
}

/// Runs every read-only smoke call, recording per-method pass/fail. One
/// failing method never aborts the run: agents get the whole picture.
async fn verify_smoke(client: &TikToolsClient, budget: Duration) -> (bool, Value) {
    let mut results = Vec::with_capacity(SMOKE_CALLS.len());
    let mut failed = 0;
    for (method, params) in SMOKE_CALLS {
        let params: Value = serde_json::from_str(params).unwrap_or(Value::Null);
        match call_with_budget::<Value>(client, method, params, budget).await {
            Ok(_) => results.push(json!({"method": method, "ok": true})),
            Err(error) => {
                failed += 1;
                results.push(json!({
                    "method": method,
                    "ok": false,
                    "code": error.code,
                    "message": error.message,
                }));
            }
        }
    }
    let ok = failed == 0;
    (
        ok,
        json!({
            "ok": ok,
            "passed": results.len() - failed,
            "failed": failed,
            "results": results,
        }),
    )
}

async fn call_with_budget<R>(
    client: &TikToolsClient,
    method: &str,
    params: Value,
    budget: Duration,
) -> Result<R, ClientError>
where
    R: serde::de::DeserializeOwned,
{
    match tokio::time::timeout(budget, client.call(method, params)).await {
        Ok(outcome) => outcome,
        Err(_) => Err(ClientError::new(
            "timeout",
            format!("{method} exceeded the verification budget"),
        )),
    }
}

/// Client-side topic filter: exact topics, the `*` match-all wildcard,
/// and `prefix.*` namespace wildcards covering the bare `prefix` plus
/// every `prefix.`-scoped child.
fn topic_allowed(filters: &[String], topic: &str) -> bool {
    filters.is_empty()
        || filters.iter().any(|filter| {
            filter == "*"
                || filter == topic
                || filter.strip_suffix(".*").is_some_and(|prefix| {
                    !prefix.is_empty()
                        && (topic == prefix
                            || topic
                                .strip_prefix(prefix)
                                .is_some_and(|rest| rest.starts_with('.')))
                })
        })
}

#[cfg(test)]
mod tests {
    use super::topic_allowed;

    #[test]
    fn topics_filter_exact_wildcard_and_namespaces() {
        assert!(topic_allowed(&[], "anything"));
        assert!(topic_allowed(&["*".to_owned()], "anything.at.all"));
        assert!(topic_allowed(&["live.*".to_owned()], "live"));
        assert!(topic_allowed(&["live.*".to_owned()], "live.event"));
        assert!(!topic_allowed(&["live.*".to_owned()], "lively"));
        assert!(!topic_allowed(&["points.changed".to_owned()], "points"));
    }
}

#[cfg(test)]
mod command_metadata_tests {
    use super::*;
    fn args(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|part| part.to_string()).collect()
    }
    #[test]
    fn command_metadata_flags_are_not_domain_params_and_can_precede_the_method() {
        let command = parse(&args(&[
            "call",
            "--expected-revision",
            "7",
            "workflows.save",
            "{}",
            "--command-id",
            "stable-id",
            "--host-instance-id",
            "instance-id",
            "--include-meta",
        ]))
        .unwrap_or_else(|_| panic!("valid metadata"));
        let Command::Call {
            method,
            params,
            metadata,
            include_meta,
            ..
        } = command
        else {
            panic!("call expected");
        };
        assert_eq!(method, "workflows.save");
        assert_eq!(params, json!({}));
        assert!(include_meta);
        let metadata = metadata.unwrap();
        assert_eq!(metadata.expected_revision.as_deref(), Some("7"));
        assert_eq!(metadata.command_id.as_deref(), Some("stable-id"));
    }
    #[test]
    fn malformed_command_flags_fail_before_a_host_can_execute_anything() {
        for flags in [
            vec!["--command-id", "only-id"],
            vec!["--command-id=ignored-before"],
            vec!["--expected-revision", "01"],
            vec!["--expected-revision"],
            vec!["--expected-revision", "1", "--expected-revision", "1"],
            vec!["--unknown"],
        ] {
            let mut input = vec!["call", "workflows.save", "{}"];
            input.extend(flags);
            assert!(matches!(parse(&args(&input)), Err(CommandError::Usage(_))));
        }
        assert!(parse(&args(&[
            "call",
            "workflows.delete",
            "{}",
            "--expected-revision",
            "3"
        ]))
        .is_ok());
    }

    #[test]
    fn additive_metadata_has_no_compare_and_set_precondition() {
        let command = parse(&args(&[
            "call",
            "points.adjust",
            "{\"uniqueId\":\"alice\",\"delta\":2}",
            "--command-id",
            "once",
            "--host-instance-id",
            "host",
            "--include-meta",
        ]))
        .unwrap_or_else(|_| panic!("valid additive metadata"));
        let Command::Call {
            metadata: Some(metadata),
            ..
        } = command
        else {
            panic!("call metadata expected")
        };
        assert_eq!(metadata.command_id.as_deref(), Some("once"));
        assert!(metadata.expected_revision.is_none());
    }
}
