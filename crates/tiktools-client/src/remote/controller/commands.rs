//! Native command coordination. The page cannot choose/reuse durable IDs.
mod automation_commands;
mod competition_commands;
mod dispatch;
mod points_commands;
mod receipts;
mod recovery;
mod validation;
mod workflows;

use automation_commands::{allowed_automation, mutate_automation};
use competition_commands::{allowed_competition, mutate_competition};
pub(super) use dispatch::schedule_resolve;
use points_commands::{allowed_points, mutate_points};
use receipts::finish_record;
pub(super) use receipts::{confirm_reply, finish};
use recovery::resolve;
use validation::{points_result, validate_success};
use workflows::{allowed, mutate};

use super::*;
use profiles::commands::{Intent, Operation, Outcome, PointsResult, Record};
use tiktools_collaboration::{CommandOutcome, CommandReceipt, CommandState};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowCommand {
    pub epoch: String,
    pub view_revision: String,
    pub id: String,
    pub expected_revision: String,
    pub operation: Operation,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PointsCommand {
    #[serde(default)]
    pub connection_id: Option<String>,
    pub epoch: String,
    pub unique_id: String,
    pub delta: f64,
    pub currency_name: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AutomationCommand {
    pub epoch: String,
    pub automation_revision: String,
    pub connection_id: String,
    pub host_instance_id: String,
    pub grant_revision: String,
    pub id: String,
    pub expected_revision: String,
    pub operation: Operation,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompetitionCommand {
    pub epoch: String,
    pub connection_id: String,
    pub host_instance_id: String,
    pub grant_revision: String,
    pub id: String,
    pub expected_revision: String,
    pub expected_generation: String,
    pub operation: Operation,
}
enum Action {
    Mutate(WorkflowCommand),
    Points(PointsCommand),
    Automation(AutomationCommand),
    Competition(CompetitionCommand),
    Proposal(proposals::Submit),
    Resolve(Option<String>),
    Review(String),
}
struct Selected {
    journal: Arc<Journal>,
    remote: Option<RemoteClient>,
}

fn code(e: &ClientError) -> String {
    match e.code.as_str() {
        "transport"
        | "proposal_conflict"
        | "proposal_invalid"
        | "proposal_unavailable"
        | "proposal_limit"
        | "currency_changed"
        | "timeout"
        | "cancelled"
        | "protocol"
        | "invalid_params"
        | "revision_conflict"
        | "not_found"
        | "automation_not_found"
        | "forbidden"
        | "unauthorized"
        | "overloaded"
        | "rate_limited"
        | "host_restarted"
        | "host_changed"
        | "command_expired"
        | "command_id_reused"
        | "command_unknown"
        | "command_running"
        | "command_pending"
        | "command_journal_invalid"
        | "command_journal_unavailable"
        | "command_journal_in_use"
        | "stale_view"
        | "not_ready"
        | "unsupported_remote"
        | "internal"
        | "storage"
        | "reauthentication_required" => e.code.clone(),
        _ => "error".into(),
    }
}
fn selected(shared: &Shared, epoch: &str, ready: bool) -> Result<Selected, ClientError> {
    let v = shared.view.read().unwrap_or_else(|p| p.into_inner());
    if v.status.epoch != epoch {
        return Err(fail("stale_view", "controller selection changed"));
    }
    if ready
        && (v.status.phase != "ready"
            || v.remote
                .as_ref()
                .is_none_or(|r| r.is_closed() || r.requires_resync()))
    {
        return Err(fail(
            "not_ready",
            "recover the controller before checking or changing host state",
        ));
    }
    let journal = v
        .journal
        .clone()
        .ok_or_else(|| fail("not_ready", "select a stored profile first"))?;
    Ok(Selected {
        journal,
        remote: v.remote.clone(),
    })
}

#[cfg(test)]
fn points_crash_stage(stage: &str) {
    if std::env::var("TIKTOOLS_NATIVE_POINTS_CRASH_STAGE").as_deref() == Ok(stage) {
        std::process::exit(98);
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
fn automation_crash_stage(stage: &str) {
    if std::env::var("TIKTOOLS_NATIVE_EVENT_CRASH_STAGE").as_deref() == Ok(stage) {
        std::process::exit(97);
    }
}

#[cfg(test)]
mod event_receipts;
