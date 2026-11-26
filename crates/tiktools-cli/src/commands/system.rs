//! `system` commands.

use serde_json::Value;
use tiktools_client::{ClientError, TikToolsClient};

use super::args::split_first;
use super::spec::{DomainSpec, VerbSpec};
use super::{result_value, CommandError};

const U_DOMAIN: &str = "system <info|health|snapshot|doctor|request-input-access>";

pub(crate) const SPEC: DomainSpec = DomainSpec {
    domain: "system",
    summary: "host information and diagnostics",
    verbs: &[
        VerbSpec::new(
            "info",
            "system info",
            "host version and runtime information",
        ),
        VerbSpec::new("health", "system health", "component health report"),
        VerbSpec::new(
            "snapshot",
            "system snapshot",
            "full state snapshot for debugging",
        ),
        VerbSpec::new("doctor", "system doctor", "diagnose common setup problems"),
        VerbSpec::new(
            "request-input-access",
            "system request-input-access",
            "ask the desktop user for input access",
        ),
    ],
};

pub enum Command {
    Info,
    Health,
    Snapshot,
    Doctor,
    RequestInputAccess,
}

pub fn parse(args: &[String]) -> Result<Command, CommandError> {
    let (verb, _) = split_first(args, U_DOMAIN)?;
    match verb {
        "info" => Ok(Command::Info),
        "health" => Ok(Command::Health),
        "snapshot" => Ok(Command::Snapshot),
        "doctor" => Ok(Command::Doctor),
        "request-input-access" => Ok(Command::RequestInputAccess),
        other => Err(format!("unknown system verb `{other}`").into()),
    }
}

pub async fn execute(client: &TikToolsClient, command: Command) -> Result<Value, ClientError> {
    match command {
        Command::Info => result_value(client.system_info().await?),
        Command::Health => result_value(client.system_health().await?),
        Command::Snapshot => result_value(client.system_snapshot().await?),
        Command::Doctor => result_value(client.system_doctor().await?),
        Command::RequestInputAccess => result_value(client.system_request_input_access().await?),
    }
}
