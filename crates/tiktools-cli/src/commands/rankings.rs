//! Ranking commands using the shared control API.
use super::{
    args::{one_arg, required_record, split_first},
    result_value, typed_params, CommandError,
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
pub fn parse(args: &[String]) -> Result<Command, CommandError> {
    let (verb, rest) = split_first(
        args,
        "ranking <list|metrics|create|update|get|snapshot|delete|reset|pause|resume|contribute>",
    )?;
    Ok(match verb {
        "list" if rest.is_empty() => Command::List,
        "metrics" if rest.is_empty() => Command::Metrics,
        "create" => Command::Create(typed_params(required_record(
            rest,
            "ranking create --record <json|@file>",
        )?)?),
        "update" => Command::Update(typed_params(required_record(
            rest,
            "ranking update --record <json|@file>",
        )?)?),
        "get" => Command::Get(one_arg(rest, "ranking get <id>")?),
        "snapshot" => Command::Snapshot(one_arg(rest, "ranking snapshot <id>")?),
        "delete" => Command::Delete(one_arg(rest, "ranking delete <id>")?),
        "reset" => Command::Reset(one_arg(rest, "ranking reset <id>")?),
        "pause" => Command::Pause(one_arg(rest, "ranking pause <id>")?),
        "resume" => Command::Resume(one_arg(rest, "ranking resume <id>")?),
        "contribute" => Command::Contribute(typed_params(required_record(
            rest,
            "ranking contribute --record <json|@file>",
        )?)?),
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
