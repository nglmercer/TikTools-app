//! `workflow` commands.

use serde_json::Value;
use tiktools_client::{ClientError, TikToolsClient};
use tiktools_control_api::modules::workflows::{WorkflowIdParams, WorkflowSaveParams};

use super::args::{one_arg, required_record, split_first};
use super::spec::{DomainSpec, VerbSpec};
use super::{result_value, CommandError};

pub enum Command {
    List,
    Get { id: String },
    Save { graph: Value },
    Delete { id: String },
    Enable { id: String },
    Disable { id: String },
}

const U_GET: &str = "workflow get <id>";
const U_SAVE: &str = "workflow save --record json";
const U_DELETE: &str = "workflow delete <id>";
const U_ENABLE: &str = "workflow enable <id>";
const U_DISABLE: &str = "workflow disable <id>";

pub(crate) const SPEC: DomainSpec = DomainSpec {
    domain: "workflow",
    summary: "visual workflow graphs",
    verbs: &[
        VerbSpec::new("list", "workflow list", "list saved workflow graphs"),
        VerbSpec::new("get", U_GET, "read one graph"),
        VerbSpec::new("save", U_SAVE, "create or replace a graph"),
        VerbSpec::new("delete", U_DELETE, "delete a graph").destructive(),
        VerbSpec::new("enable", U_ENABLE, "enable a graph"),
        VerbSpec::new("disable", U_DISABLE, "disable a graph"),
    ],
};

pub fn parse(args: &[String]) -> Result<Command, CommandError> {
    let (verb, rest) = split_first(args, "workflow <verb> ...")?;
    match verb {
        "list" => Ok(Command::List),
        "get" => Ok(Command::Get {
            id: one_arg(rest, U_GET)?,
        }),
        "save" => Ok(Command::Save {
            graph: required_record(rest, U_SAVE)?,
        }),
        "delete" => Ok(Command::Delete {
            id: one_arg(rest, U_DELETE)?,
        }),
        "enable" => Ok(Command::Enable {
            id: one_arg(rest, U_ENABLE)?,
        }),
        "disable" => Ok(Command::Disable {
            id: one_arg(rest, U_DISABLE)?,
        }),
        other => Err(format!("unknown workflow verb `{other}`").into()),
    }
}

pub async fn execute(client: &TikToolsClient, command: Command) -> Result<Value, ClientError> {
    match command {
        Command::List => result_value(client.workflows_list().await?),
        Command::Get { id } => result_value(client.workflows_get(WorkflowIdParams { id }).await?),
        Command::Save { graph } => {
            result_value(client.workflows_save(WorkflowSaveParams { graph }).await?)
        }
        Command::Delete { id } => {
            result_value(client.workflows_delete(WorkflowIdParams { id }).await?)
        }
        Command::Enable { id } => {
            result_value(client.workflows_enable(WorkflowIdParams { id }).await?)
        }
        Command::Disable { id } => {
            result_value(client.workflows_disable(WorkflowIdParams { id }).await?)
        }
    }
}
