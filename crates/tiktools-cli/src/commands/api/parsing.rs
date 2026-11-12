//! API arguments, transport metadata and bounded command option validation.
use super::*;

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
