//! Raw `rpc` passthrough and `host` serving modes.
//!
//! `rpc <method>` is the one intentional untyped path: it calls methods
//! with caller-supplied JSON through [`TikToolsClient::call_value`].
//! Serving modes (`rpc --stdio`, `host --stdio`, `host --ipc`) never
//! create a client; [`serve_plan`] detects them before resolution.

use serde_json::json;

use super::args::{parse_json_value, split_first};
use super::spec::{DomainSpec, VerbSpec};
use super::{CommandError, Plan};

pub(crate) const U_HOST: &str = "host --stdio [--events] | host --ipc";
const U_RPC: &str = "rpc <method> [params-json] | rpc --stdio [--events]";

pub(crate) const SPEC: DomainSpec = DomainSpec {
    domain: "host",
    summary: "headless host runtimes and the raw rpc passthrough",
    verbs: &[
        VerbSpec::new(
            "host --stdio",
            "host --stdio [--events]",
            "headless host on stdio; streams events with --events",
        ),
        VerbSpec::new(
            "host --ipc",
            "host --ipc",
            "headless host on local IPC; refuses when a host is running",
        ),
        VerbSpec::new("rpc", U_RPC, "untyped passthrough to any host method"),
        VerbSpec::new(
            "rpc --stdio",
            "rpc --stdio [--events]",
            "NDJSON request/response loop on stdio",
        ),
    ],
};

pub enum ServeMode {
    Stdio { events: bool },
    Ipc,
}

/// Detects host-serving invocations before any client exists. Returns
/// `None` for normal commands (including raw `rpc <method>` calls).
pub fn serve_plan(args: &[String]) -> Result<Option<ServeMode>, String> {
    if args.is_empty() {
        return Ok(None);
    }
    match args[0].as_str() {
        "rpc" if args.get(1).map(String::as_str) == Some("--stdio") => Ok(Some(ServeMode::Stdio {
            events: args.iter().any(|arg| arg == "--events"),
        })),
        "host" => {
            let (mode, rest) = split_first(&args[1..], U_HOST)?;
            match mode {
                "--stdio" => Ok(Some(ServeMode::Stdio {
                    events: rest.iter().any(|arg| arg == "--events"),
                })),
                "--ipc" => Ok(Some(ServeMode::Ipc)),
                other => Err(format!("unknown host mode `{other}`")),
            }
        }
        _ => Ok(None),
    }
}

pub fn parse_rpc(args: &[String]) -> Result<Plan, CommandError> {
    let (method, rest) = split_first(args, U_RPC)?;
    let params = rest
        .first()
        .map(|raw| parse_json_value(raw))
        .transpose()?
        .unwrap_or(json!({}));
    Ok(Plan::Rpc {
        method: method.to_owned(),
        params,
    })
}
