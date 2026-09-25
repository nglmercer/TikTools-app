//! `system` commands.

use serde_json::Value;
use tiktools_client::{ClientError, TikToolsClient};

use super::args::split_first;
use super::{result_value, CommandError};

pub enum Command {
    Info,
    Health,
    Snapshot,
    Doctor,
    RequestInputAccess,
}

pub fn parse(args: &[String]) -> Result<Command, CommandError> {
    let (verb, _) = split_first(args, "system <verb> ...")?;
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
