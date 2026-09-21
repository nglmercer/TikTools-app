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
use tiktools_client::{validate_params, ClientError, ControlEvent, TikToolsClient};
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
            let method = positional(rest, 0, "api call <method> [params-json] [flags]")?;
            let params = positional_args(rest)
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
        other => Err(format!("unknown api verb `{other}`").into()),
    }
}

/// Positional arguments (everything that is not a `--flag` or a flag value).
fn positional_args(args: &[String]) -> Vec<&String> {
    const VALUE_FLAGS: &[&str] = &["--timeout", "--max-events", "--format", "--topics"];
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
    }
}

struct CallArgs<'a> {
    method: &'a str,
    params: Value,
    timeout_secs: Option<u64>,
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
    if args.explain {
        explain_call(&meta, &args.params);
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
                client.call_value(args.method, &args.params),
            )
            .await
            {
                Ok(outcome) => outcome?,
                Err(_) => {
                    return Err(ClientError::new(
                        "timeout",
                        format!("call timed out after {secs}s"),
                    ));
                }
            }
        }
        None => client.call_value(args.method, &args.params).await?,
    };
    Ok(
        Output::new("api-call", redact_output(result, args.secrets_visible))
            .json_override(args.json),
    )
}

/// Prints the call plan to stderr (stdout stays pure result). Params are
/// always redacted here: `--explain` must never leak secrets.
fn explain_call(meta: &tiktools_client::MethodMeta, params: &Value) {
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
