//! Ranking commands using the shared control API.
use super::{
    args::{one_arg, required_record, split_first},
    result_value,
    spec::{DomainSpec, VerbSpec},
    typed_params, CommandError,
};
use serde_json::Value;
use tiktools_client::{ClientError, TikToolsClient};
use tiktools_control_api::modules::rankings::{
    Definition, ExternalContribution, RankingId, RankingUpdate,
};

pub enum Command {
    List,
    Metrics,
    Create(Definition),
    Update(RankingUpdate),
    Get(String),
    Snapshot(String),
    Delete(String),
    Reset(String),
    Pause(String),
    Resume(String),
    Contribute(ExternalContribution),
}
const U_DOMAIN: &str =
    "ranking <list|metrics|create|update|get|snapshot|delete|reset|pause|resume|contribute>";
const U_CREATE: &str = "ranking create --record <json|@file>";
const U_UPDATE: &str = "ranking update --record <json|@file>";
const U_CONTRIBUTE: &str = "ranking contribute --record <json|@file>";
const U_GET: &str = "ranking get <id>";
const U_SNAPSHOT: &str = "ranking snapshot <id>";
const U_DELETE: &str = "ranking delete <id>";
const U_RESET: &str = "ranking reset <id>";
const U_PAUSE: &str = "ranking pause <id>";
const U_RESUME: &str = "ranking resume <id>";

pub(crate) const SPEC: DomainSpec = DomainSpec {
    domain: "ranking",
    summary: "viewer ranking definitions and state",
    verbs: &[
        VerbSpec::new("list", "ranking list", "list ranking definitions"),
        VerbSpec::new(
            "metrics",
            "ranking metrics",
            "metrics available to ranking definitions",
        ),
        VerbSpec::new("create", U_CREATE, "create a ranking definition"),
        VerbSpec::new("update", U_UPDATE, "update a ranking definition"),
        VerbSpec::new("get", U_GET, "read one ranking"),
        VerbSpec::new("snapshot", U_SNAPSHOT, "current standings of one ranking"),
        VerbSpec::new("delete", U_DELETE, "delete a ranking").destructive(),
        VerbSpec::new("reset", U_RESET, "reset accumulated state").destructive(),
        VerbSpec::new("pause", U_PAUSE, "pause accumulation"),
        VerbSpec::new("resume", U_RESUME, "resume accumulation"),
        VerbSpec::new("contribute", U_CONTRIBUTE, "add an external contribution"),
    ],
};

pub fn parse(args: &[String]) -> Result<Command, CommandError> {
    let (verb, rest) = split_first(args, U_DOMAIN)?;
    Ok(match verb {
        "list" if rest.is_empty() => Command::List,
        "metrics" if rest.is_empty() => Command::Metrics,
        "create" => Command::Create(typed_params(required_record(rest, U_CREATE)?)?),
        "update" => Command::Update(typed_params(required_record(rest, U_UPDATE)?)?),
        "get" => Command::Get(one_arg(rest, U_GET)?),
        "snapshot" => Command::Snapshot(one_arg(rest, U_SNAPSHOT)?),
        "delete" => Command::Delete(one_arg(rest, U_DELETE)?),
        "reset" => Command::Reset(one_arg(rest, U_RESET)?),
        "pause" => Command::Pause(one_arg(rest, U_PAUSE)?),
        "resume" => Command::Resume(one_arg(rest, U_RESUME)?),
        "contribute" => Command::Contribute(typed_params(required_record(rest, U_CONTRIBUTE)?)?),
        _ => return Err(format!("invalid ranking command `{verb}`").into()),
    })
}
pub async fn execute(client: &TikToolsClient, command: Command) -> Result<Value, ClientError> {
    match command {
        Command::List => result_value(client.rankings_list().await?),
        Command::Metrics => result_value(client.rankings_metrics().await?),
        Command::Create(d) => result_value(client.rankings_create(d).await?),
        Command::Update(update) => result_value(client.rankings_update(update).await?),
        Command::Get(id) => result_value(client.rankings_get(RankingId { id }).await?),
        Command::Snapshot(id) => result_value(client.rankings_snapshot(RankingId { id }).await?),
        Command::Delete(id) => result_value(client.rankings_delete(RankingId { id }).await?),
        Command::Reset(id) => result_value(client.rankings_reset(RankingId { id }).await?),
        Command::Pause(id) => result_value(client.rankings_pause(RankingId { id }).await?),
        Command::Resume(id) => result_value(client.rankings_resume(RankingId { id }).await?),
        Command::Contribute(c) => result_value(client.rankings_contribute(c).await?),
    }
}
