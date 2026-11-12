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

mod calling;
mod dispatch;
mod events;
mod parsing;
mod verification;

use calling::execute_call;
pub use dispatch::execute;
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
