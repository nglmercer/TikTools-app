//! `globals` commands: runtime global values for automations.
//!
//! Globals render at event time (`{{ globals.commandPort }}`), so one
//! `globals set` repoints every rule without re-importing templates.

use serde_json::Value;
use tiktools_client::{ClientError, TikToolsClient};
use tiktools_control_api::modules::globals::{
    GlobalsDeleteParams, GlobalsGetParams, GlobalsSetParams,
};

use super::args::{positional, split_first};
use super::{result_value, CommandError};

pub enum Command {
    List,
    Get { key: String },
    Set { key: String, value: String },
    Delete { key: String },
}

pub fn parse(args: &[String]) -> Result<Command, CommandError> {
    let (verb, rest) = split_first(args, "globals <verb> ...")?;
    match verb {
        "list" => Ok(Command::List),
        "get" => Ok(Command::Get {
            key: positional(rest, 0, "globals get <key>")?,
        }),
        "set" => Ok(Command::Set {
            key: positional(rest, 0, "globals set <key> <value>")?,
            value: positional(rest, 1, "globals set <key> <value>")?,
        }),
        "delete" => Ok(Command::Delete {
            key: positional(rest, 0, "globals delete <key>")?,
        }),
        other => Err(format!("unknown globals verb `{other}`").into()),
    }
}

pub async fn execute(client: &TikToolsClient, command: Command) -> Result<Value, ClientError> {
    match command {
        Command::List => result_value(client.globals_list().await?),
        Command::Get { key } => result_value(client.globals_get(GlobalsGetParams { key }).await?),
        Command::Set { key, value } => {
            result_value(client.globals_set(GlobalsSetParams { key, value }).await?)
        }
        Command::Delete { key } => {
            result_value(client.globals_delete(GlobalsDeleteParams { key }).await?)
        }
    }
}
