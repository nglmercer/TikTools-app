//! Competition definition commands.
use super::{
    args::{one_arg, required_record, split_first},
    result_value,
    spec::{DomainSpec, VerbSpec},
    typed_params, CommandError,
};
use serde_json::Value;
use tiktools_client::{ClientError, TikToolsClient};
use tiktools_control_api::modules::competitions::{
    CompetitionActionId, CompetitionId, CompetitionResultsRequest, Definition,
    ResolveCompetitionAction, SelectWinner,
};
pub enum Command {
    List,
    Get(String),
    Create(Definition),
    Update(Definition),
    Delete(String),
    State(String),
    Results(String),
    Action(CompetitionActionId),
    ResolveAction(ResolveCompetitionAction),
    MarkActionUncertain(CompetitionActionId),
    Start(String),
    Finish(String),
    Candidates(String),
    SelectWinner(SelectWinner),
    Pause(String),
    Resume(String),
    Cancel(String),
}
const U_DOMAIN: &str = "competition <list|get|create|update|delete|state|results|action|mark-action-uncertain|resolve-action|start|finish|candidates|select-winner|pause|resume|cancel>";
const U_CREATE: &str = "competition create --record <json|@file>";
const U_UPDATE: &str = "competition update --record <json|@file>";
const U_ACTION: &str = "competition action --record <json|@file>";
const U_RESOLVE: &str = "competition resolve-action --record <json|@file>";
const U_UNCERTAIN: &str = "competition mark-action-uncertain --record <json|@file>";
const U_WINNER: &str = "competition select-winner --record <json|@file>";
const U_GET: &str = "competition get <id>";
const U_RESULTS: &str = "competition results <id>";
const U_STATE: &str = "competition state <id>";
const U_CANDIDATES: &str = "competition candidates <id>";
const U_FINISH: &str = "competition finish <id>";
const U_START: &str = "competition start <id>";
const U_PAUSE: &str = "competition pause <id>";
const U_RESUME: &str = "competition resume <id>";
const U_CANCEL: &str = "competition cancel <id>";
const U_DELETE: &str = "competition delete <id>";

pub(crate) const SPEC: DomainSpec = DomainSpec {
    domain: "competition",
    summary: "timed viewer competitions",
    verbs: &[
        VerbSpec::new("list", "competition list", "list competitions"),
        VerbSpec::new("get", U_GET, "read one competition"),
        VerbSpec::new("create", U_CREATE, "create a competition"),
        VerbSpec::new("update", U_UPDATE, "update a competition"),
        VerbSpec::new("delete", U_DELETE, "delete a competition").destructive(),
        VerbSpec::new("state", U_STATE, "current competition state"),
        VerbSpec::new("results", U_RESULTS, "final or intermediate results"),
        VerbSpec::new("action", U_ACTION, "record a competition action"),
        VerbSpec::new(
            "mark-action-uncertain",
            U_UNCERTAIN,
            "flag an action as uncertain",
        ),
        VerbSpec::new("resolve-action", U_RESOLVE, "resolve an uncertain action"),
        VerbSpec::new("start", U_START, "start a competition"),
        VerbSpec::new("finish", U_FINISH, "finish a competition").destructive(),
        VerbSpec::new("candidates", U_CANDIDATES, "winner candidates"),
        VerbSpec::new("select-winner", U_WINNER, "select the winner"),
        VerbSpec::new("pause", U_PAUSE, "pause a competition"),
        VerbSpec::new("resume", U_RESUME, "resume a competition"),
        VerbSpec::new("cancel", U_CANCEL, "cancel a competition").destructive(),
    ],
};

pub fn parse(args: &[String]) -> Result<Command, CommandError> {
    let (verb, rest) = split_first(args, U_DOMAIN)?;
    Ok(match verb {
        "list" if rest.is_empty() => Command::List,
        "get" => Command::Get(one_arg(rest, U_GET)?),
        "mark-action-uncertain" => {
            Command::MarkActionUncertain(typed_params(required_record(rest, U_UNCERTAIN)?)?)
        }
        "action" => Command::Action(typed_params(required_record(rest, U_ACTION)?)?),
        "resolve-action" => {
            Command::ResolveAction(typed_params(required_record(rest, U_RESOLVE)?)?)
        }
        "results" => Command::Results(one_arg(rest, U_RESULTS)?),
        "state" => Command::State(one_arg(rest, U_STATE)?),
        "candidates" => Command::Candidates(one_arg(rest, U_CANDIDATES)?),
        "select-winner" => Command::SelectWinner(typed_params(required_record(rest, U_WINNER)?)?),
        "finish" => Command::Finish(one_arg(rest, U_FINISH)?),
        "start" => Command::Start(one_arg(rest, U_START)?),
        "pause" => Command::Pause(one_arg(rest, U_PAUSE)?),
        "resume" => Command::Resume(one_arg(rest, U_RESUME)?),
        "cancel" => Command::Cancel(one_arg(rest, U_CANCEL)?),
        "delete" => Command::Delete(one_arg(rest, U_DELETE)?),
        "update" => Command::Update(typed_params(required_record(rest, U_UPDATE)?)?),
        "create" => Command::Create(typed_params(required_record(rest, U_CREATE)?)?),
        _ => return Err(format!("invalid competition command `{verb}`").into()),
    })
}
pub async fn execute(client: &TikToolsClient, command: Command) -> Result<Value, ClientError> {
    match command {
        Command::List => result_value(client.competitions_list().await?),
        Command::Get(id) => result_value(client.competitions_get(CompetitionId { id }).await?),
        Command::MarkActionUncertain(params) => {
            result_value(client.competitions_mark_action_uncertain(params).await?)
        }
        Command::Action(params) => result_value(client.competitions_action(params).await?),
        Command::ResolveAction(params) => {
            result_value(client.competitions_resolve_action(params).await?)
        }
        Command::Results(id) => result_value(
            client
                .competitions_results(CompetitionResultsRequest {
                    id,
                    before_generation: None,
                    limit: None,
                })
                .await?,
        ),
        Command::State(id) => result_value(client.competitions_state(CompetitionId { id }).await?),
        Command::Candidates(id) => {
            result_value(client.competitions_candidates(CompetitionId { id }).await?)
        }
        Command::SelectWinner(params) => {
            result_value(client.competitions_select_winner(params).await?)
        }
        Command::Finish(id) => {
            result_value(client.competitions_finish(CompetitionId { id }).await?)
        }
        Command::Start(id) => result_value(client.competitions_start(CompetitionId { id }).await?),
        Command::Pause(id) => result_value(client.competitions_pause(CompetitionId { id }).await?),
        Command::Resume(id) => {
            result_value(client.competitions_resume(CompetitionId { id }).await?)
        }
        Command::Cancel(id) => {
            result_value(client.competitions_cancel(CompetitionId { id }).await?)
        }
        Command::Update(definition) => result_value(client.competitions_update(definition).await?),
        Command::Delete(id) => {
            result_value(client.competitions_delete(CompetitionId { id }).await?)
        }
        Command::Create(definition) => result_value(client.competitions_create(definition).await?),
    }
}
