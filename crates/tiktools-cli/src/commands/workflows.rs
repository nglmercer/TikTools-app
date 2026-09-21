//! `workflow` commands.

use serde_json::Value;
use tiktools_client::{ClientError, TikToolsClient};
use tiktools_control_api::modules::workflows::{WorkflowIdParams, WorkflowSaveParams};

use super::args::{one_arg, required_record, split_first};
use super::{result_value, CommandError};

pub enum Command {
    List,
    Get { id: String },
    Save { graph: Value },
    Delete { id: String },
    Enable { id: String },
    Disable { id: String },
}

pub fn parse(args: &[String]) -> Result<Command, CommandError> {
    let (verb, rest) = split_first(args, "workflow <verb> ...")?;
    match verb {
        "list" => Ok(Command::List),
        "get" => Ok(Command::Get {
            id: one_arg(rest, "workflow get <id>")?,
        }),
        "save" => Ok(Command::Save {
            graph: required_record(rest, "workflow save --record json")?,
        }),
        "delete" => Ok(Command::Delete {
            id: one_arg(rest, "workflow delete <id>")?,
        }),
        "enable" => Ok(Command::Enable {
            id: one_arg(rest, "workflow enable <id>")?,
        }),
        "disable" => Ok(Command::Disable {
            id: one_arg(rest, "workflow disable <id>")?,
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
