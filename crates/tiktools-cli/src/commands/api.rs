//! `api` commands: the generic agent-oriented control surface.
//!
//! - `discover` lists every method live from the host registry.
//! - `schema` shows one method's description, flags, and schemas.
//! - `call` resolves the method, validates params, then executes
//!   (unless `--dry-run`); destructive methods need `--confirm`.
//! - `events` streams NDJSON domain events with topic filtering.
//! - `event get` searches the bounded feed journal for events by name/kind/author.
//!
//! `call` output is secret-redacted by default; only `--secrets-visible`
//! opts out. `--explain` prints the call plan (redacted) to stderr and
//! `--format json|human` overrides the global `--json` for one command.

mod calling;
mod dispatch;
mod event_get;
mod events;
mod parsing;
mod verification;

use calling::execute_call;
pub use dispatch::execute;
use event_get::get_event;
use events::stream_events;
#[cfg(test)]
use events::topic_allowed;
pub use parsing::parse;
use verification::execute_verify;

use std::time::Duration;

use serde_json::{json, Value};
use tiktools_client::{
    validate_params, ClientError, ControlEvent, RequestMetadata, TikToolsClient,
};
use tiktools_control_api::modules::rpc::SchemaParams;

use super::args::{flag_value, flag_values, parse_json_value, positional, split_first};
use super::spec::{DomainSpec, VerbSpec};
use super::{redact_output, result_value, CommandError, Output};

pub(crate) const U_DOMAIN: &str = "api <discover|schema|call|events|event|verify> ...";
pub(crate) const U_SCHEMA: &str = "api schema <method> [--format json|human]";
pub(crate) const U_CALL: &str = "api call <method> [params-json] [--timeout SEC] [--dry-run] [--confirm] [--redact|--secrets-visible] [--explain] [--format json|human] [--expected-revision N] [--command-id ID --host-instance-id ID] [--include-meta]";
pub(crate) const U_EVENT_GET: &str =
    "api event get <name> [--type kind]... [--author text] [--limit N]";

pub(crate) const SPEC: DomainSpec = DomainSpec {
    domain: "api",
    summary: "generic agent-oriented control surface over host methods",
    verbs: &[
        VerbSpec::new(
            "discover",
            "api discover [--format json|human]",
            "list every method live from the host registry",
        ),
        VerbSpec::new(
            "schema",
            U_SCHEMA,
            "one method's description, flags and schemas",
        ),
        VerbSpec::new(
            "call",
            U_CALL,
            "validate and execute one method; destructive methods need --confirm",
        ),
        VerbSpec::new(
            "events",
            "api events [--topics t,...] [--timeout SEC] [--max-events N]",
            "stream NDJSON domain events",
        ),
        VerbSpec::new("event get", U_EVENT_GET, "search the bounded feed journal"),
        VerbSpec::new(
            "verify",
            "api verify [--coverage|--smoke] [--sandbox] [--timeout SEC]",
            "verify the control surface against the host",
        ),
    ],
};

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
    GetEvent {
        name: String,
        kinds: Vec<String>,
        author: Option<String>,
        limit: usize,
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
            Self::Events { .. } | Self::GetEvent { .. } => None,
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

#[cfg(test)]
mod command_metadata_tests;
#[cfg(test)]
mod tests;
