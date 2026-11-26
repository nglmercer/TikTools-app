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
use super::spec::{DomainSpec, VerbSpec};
use super::{result_value, CommandError};

pub enum Command {
    List,
    Get { key: String },
    Set { key: String, value: String },
    Delete { key: String },
}

const U_GET: &str = "globals get <key>";
const U_SET: &str = "globals set <key> <value>";
const U_DELETE: &str = "globals delete <key>";

pub(crate) const SPEC: DomainSpec = DomainSpec {
    domain: "globals",
    summary: "shared key/value globals across plugins",
    verbs: &[
        VerbSpec::new("list", "globals list", "list every global key"),
        VerbSpec::new("get", U_GET, "read one global"),
        VerbSpec::new("set", U_SET, "write one global"),
        VerbSpec::new("delete", U_DELETE, "delete one global").destructive(),
    ],
};

pub fn parse(args: &[String]) -> Result<Command, CommandError> {
    let (verb, rest) = split_first(args, "globals <verb> ...")?;
    match verb {
        "list" => Ok(Command::List),
        "get" => Ok(Command::Get {
            key: positional(rest, 0, U_GET)?,
        }),
        "set" => Ok(Command::Set {
            key: positional(rest, 0, U_SET)?,
            value: positional(rest, 1, U_SET)?,
        }),
        "delete" => Ok(Command::Delete {
            key: positional(rest, 0, U_DELETE)?,
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
