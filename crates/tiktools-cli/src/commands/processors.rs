//! `processor` commands.

use serde_json::{json, Value};
use tiktools_client::{ClientError, TikToolsClient};
use tiktools_control_api::modules::processors::ProcessorTestParams;

use super::args::{parse_json_value, split_first};
use super::spec::{DomainSpec, VerbSpec};
use super::{result_value, CommandError};

pub enum Command {
    List,
    Status,
    Test {
        plugin_id: String,
        processor_id: String,
        event: Value,
    },
}

const U_TEST: &str = "processor test <plugin-id> <processor-id> [event-json]";

pub(crate) const SPEC: DomainSpec = DomainSpec {
    domain: "processor",
    summary: "event processors",
    verbs: &[
        VerbSpec::new("list", "processor list", "list processors"),
        VerbSpec::new("status", "processor status", "processor pipeline status"),
        VerbSpec::new("test", U_TEST, "run one processor against a sample event"),
    ],
};

pub fn parse(args: &[String]) -> Result<Command, CommandError> {
    let (verb, rest) = split_first(args, "processor <verb> ...")?;
    match verb {
        "list" => Ok(Command::List),
        "status" => Ok(Command::Status),
        "test" => {
            if rest.len() < 2 {
                return Err(U_TEST.to_owned().into());
            }
            let event = rest
                .get(2)
                .map(|raw| parse_json_value(raw))
                .transpose()?
                .unwrap_or(json!({}));
            Ok(Command::Test {
                plugin_id: rest[0].clone(),
                processor_id: rest[1].clone(),
                event,
            })
        }
        other => Err(format!("unknown processor verb `{other}`").into()),
    }
}

pub async fn execute(client: &TikToolsClient, command: Command) -> Result<Value, ClientError> {
    match command {
        Command::List => result_value(client.processors_list().await?),
        Command::Status => result_value(client.processors_status().await?),
        Command::Test {
            plugin_id,
            processor_id,
            event,
        } => result_value(
            client
                .processors_test(ProcessorTestParams {
                    plugin_id,
                    processor_id,
                    event,
                })
                .await?,
        ),
    }
}
