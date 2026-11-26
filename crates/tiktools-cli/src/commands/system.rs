//! `system` commands.

use serde_json::Value;
use tiktools_client::{ClientError, TikToolsClient};

use super::args::split_first;
use super::spec::{DomainSpec, VerbSpec};
use super::{result_value, CommandError};

const U_DOMAIN: &str = "system <info|health|snapshot|doctor|request-input-access|shutdown>";

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
        // Typing the verb is the intent confirmation, consistent with every
        // other destructive first-class form (plugin uninstall, points reset).
        VerbSpec::new("shutdown", "system shutdown", "shut down the running host").destructive(),
    ],
};

pub enum Command {
    Info,
    Health,
    Snapshot,
    Doctor,
    RequestInputAccess,
    Shutdown,
}

pub fn parse(args: &[String]) -> Result<Command, CommandError> {
    let (verb, _) = split_first(args, U_DOMAIN)?;
    match verb {
        "info" => Ok(Command::Info),
        "health" => Ok(Command::Health),
        "snapshot" => Ok(Command::Snapshot),
        "doctor" => Ok(Command::Doctor),
        "request-input-access" => Ok(Command::RequestInputAccess),
        "shutdown" => Ok(Command::Shutdown),
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
        Command::Shutdown => result_value(client.system_shutdown().await?),
    }
}
