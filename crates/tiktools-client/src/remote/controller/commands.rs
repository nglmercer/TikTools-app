//! Native command coordination. The page cannot choose/reuse durable IDs.
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
enum Action {
    Mutate(WorkflowCommand),
    Points(PointsCommand),
    Automation(AutomationCommand),
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
fn allowed(shared: &Shared, p: &WorkflowCommand, remote: &RemoteClient) -> Result<(), ClientError> {
    let v = shared.view.read().unwrap_or_else(|p| p.into_inner());
    if v.status.epoch != p.epoch || v.status.view_revision != p.view_revision {
        return Err(fail(
            "stale_view",
            "workflow view changed; review the current host state",
        ));
    }
    allowed_row(&v, p, remote)
}
fn allowed_row(v: &View, p: &WorkflowCommand, remote: &RemoteClient) -> Result<(), ClientError> {
    if !matches!(p.operation, Operation::Disable | Operation::Delete) {
        return Err(fail(
            "invalid_params",
            "use the matching command coordinator",
        ));
    }
    let hello = v.status.hello.as_ref().ok_or_else(network)?;
    if v.status.epoch != p.epoch
        || v.status.phase != "ready"
        || remote.is_closed()
        || remote.requires_resync()
        || v.remote
            .as_ref()
            .is_none_or(|r| r.hello().connection_id != remote.hello().connection_id)
    {
        return Err(fail(
            "not_ready",
            "selected connection changed; recover first",
        ));
    }
    if !hello
        .features
        .get(p.operation.feature())
        .copied()
        .unwrap_or(false)
        || !hello.capabilities.contains(&Capability::WorkflowsRead)
        || !hello.capabilities.contains(&p.operation.capability())
    {
        return Err(fail("forbidden", "workflow operation is not available"));
    }
    let row = v
        .rows
        .get(&p.id)
        .ok_or_else(|| fail("stale_view", "workflow is no longer in this view"))?;
    if row.version.revision != p.expected_revision
        || row.version.kind != ResourceKind::Workflow
        || row.version.scope != "host"
    {
        return Err(fail(
            "stale_view",
            "workflow revision changed; review it again",
        ));
    }
    Ok(())
}
fn validate_success(
    intent: &Intent,
    receipt: &CommandReceipt,
    versions: &[ResourceVersion],
    result: &Value,
) -> Result<String, ClientError> {
    intent.validate().map_err(|_| protocol())?;
    let target = receipt.resource.as_ref().ok_or_else(protocol)?;
    if receipt.command_id != intent.command_id
        || receipt.state != CommandState::Succeeded
        || target.kind != intent.operation.kind()
        || target.scope != intent.operation.scope()
        || target.id != intent.resource_id
        || receipt
            .committed_at
            .is_none_or(|t| !(0..=9_007_199_254_740_991).contains(&t))
        || versions.len() != 1
        || versions[0].kind != intent.operation.kind()
        || versions[0].scope != intent.operation.scope()
        || versions[0].id != intent.resource_id
        || result
            .get(if intent.operation == Operation::AdjustPoints {
                "uniqueId"
            } else {
                "id"
            })
            .and_then(Value::as_str)
            != Some(intent.resource_id.as_str())
    {
        return Err(protocol());
    }
    let after =
        tiktools_collaboration::stream_sequence(&versions[0].revision).map_err(|_| protocol())?;
    if intent.operation == Operation::AdjustPoints {
        points_result(intent, result)?;
        return if after > 0 {
            Ok(versions[0].revision.clone())
        } else {
            Err(protocol())
        };
    }
    if intent.operation.is_event()
        && !intent.operation.is_activation()
        && result.as_object().is_none_or(|o| o.len() != 2)
    {
        return Err(protocol());
    }
    let before = tiktools_collaboration::stream_sequence(
        intent.expected_revision.as_deref().ok_or_else(protocol)?,
    )
    .map_err(|_| protocol())?;
    if intent.operation.is_proposal() {
        if result.as_object().is_none_or(|o| o.len() != 4)
            || result.get("kind").and_then(Value::as_str) != Some(intent.operation.scope())
            || result.get("enabled") != Some(&Value::Bool(intent.operation.is_active_proposal()))
            || result.get("pendingReview")
                != Some(&Value::Bool(!intent.operation.is_active_proposal()))
            || after == 0
            || after > i64::MAX as u64
            || before.checked_add(1) != Some(after)
        {
            return Err(protocol());
        }
        return Ok(versions[0].revision.clone());
    }
    let changed = match intent.operation {
        Operation::EnableEvent | Operation::EnableAction => {
            if result.as_object().is_none_or(|o| o.len() != 3)
                || result.get("kind").and_then(Value::as_str) != Some(intent.operation.scope())
                || result.get("enabled") != Some(&Value::Bool(true))
            {
                return Err(protocol());
            }
            true
        }
        Operation::Disable | Operation::DisableEvent => {
            if result.get("enabled") != Some(&Value::Bool(false)) {
                return Err(protocol());
            }
            true
        }
        Operation::Delete | Operation::DeleteEvent => result
            .get("deleted")
            .and_then(Value::as_bool)
            .ok_or_else(protocol)?,
        Operation::AdjustPoints => unreachable!(),
        Operation::CreateEventDraft
        | Operation::UpdateEventDraft
        | Operation::CreateActionDraft
        | Operation::UpdateActionDraft
        | Operation::CreateEventActive
        | Operation::UpdateEventActive
        | Operation::CreateActionActive
        | Operation::UpdateActionActive => unreachable!(),
    };
    if after
        != before
            .checked_add(u64::from(changed))
            .ok_or_else(protocol)?
    {
        return Err(protocol());
    }
    Ok(versions[0].revision.clone())
}
pub(super) async fn finish(
    journal: &Arc<Journal>,
    intent: Intent,
    outcome: Outcome,
    revision: Option<String>,
    error_code: Option<String>,
) -> Result<(), ClientError> {
    finish_record(journal, intent, outcome, revision, error_code, None).await
}
async fn finish_record(
    journal: &Arc<Journal>,
    intent: Intent,
    outcome: Outcome,
    revision: Option<String>,
    error_code: Option<String>,
    points_result: Option<PointsResult>,
) -> Result<(), ClientError> {
    let record = Record {
        intent,
        outcome,
        revision,
        error_code,
        recorded_at: super::super::profiles::commands::timestamp(),
        points_result,
    };
    let j = journal.clone();
    tokio::task::spawn_blocking(move || j.finish(record))
        .await
        .map_err(|_| fail("command_journal_unavailable", "journal task failed"))?
}
async fn mutate(shared: &Shared, epoch: &str, p: WorkflowCommand) -> Result<(), ClientError> {
    let selected = selected(shared, epoch, true)?;
    let remote = selected.remote.ok_or_else(network)?;
    {
        let v = shared.view.read().unwrap_or_else(|p| p.into_inner());
        allowed_row(&v, &p, &remote)?;
    }
    let intent = Intent {
        command_id: uuid::Uuid::new_v4().to_string(),
        operation: p.operation,
        resource_id: p.id.clone(),
        host_instance_id: remote.hello().host_instance_id.clone(),
        expected_revision: Some(p.expected_revision.clone()),
        delta: None,
        currency_name: None,
        proposal_revision: None,
        created_at: profiles::commands::timestamp(),
    };
    let pending = intent.clone();
    let journal = selected.journal.clone();
    tokio::task::spawn_blocking(move || journal.begin(pending))
        .await
        .map_err(|_| fail("command_journal_unavailable", "journal task failed"))??;
    // The synced intent outlives the page/caller. A switch or a changed target
    // during disk I/O is a known unsent rejection, never a send to another host.
    let admission = {
        let v = shared.view.read().unwrap_or_else(|p| p.into_inner());
        allowed_row(&v, &p, &remote)
    };
    if let Err(error) = admission {
        finish(
            &selected.journal,
            intent,
            Outcome::Rejected,
            None,
            Some(code(&error)),
        )
        .await?;
        return Err(error);
    }
    let reply = remote
        .call_value_with_meta(
            intent.operation.method(),
            json!({"id":intent.resource_id}),
            Some(RequestMetadata {
                command_id: Some(intent.command_id.clone()),
                host_instance_id: Some(intent.host_instance_id.clone()),
                expected_revision: intent.expected_revision.clone(),
            }),
        )
        .await;
    confirm_reply(&selected.journal, intent, reply).await
}
fn allowed_points(
    shared: &Shared,
    p: &PointsCommand,
    remote: &RemoteClient,
) -> Result<(), ClientError> {
    let v = shared.view.read().unwrap_or_else(|p| p.into_inner());
    if p.connection_id
        .as_deref()
        .is_some_and(|id| id != remote.hello().connection_id)
    {
        return Err(fail(
            "stale_view",
            "points confirmation belongs to a previous connection",
        ));
    }
    if v.status.epoch != p.epoch
        || v.status.phase != "ready"
        || remote.is_closed()
        || remote.requires_resync()
        || v.remote
            .as_ref()
            .is_none_or(|r| r.hello().connection_id != remote.hello().connection_id)
    {
        return Err(fail(
            "not_ready",
            "selected connection changed; recover first",
        ));
    }
    if p.unique_id.is_empty()
        || p.unique_id.len() > 128
        || p.unique_id.trim() != p.unique_id
        || p.unique_id.starts_with('@')
        || p.unique_id.contains('*')
        || p.unique_id.chars().any(char::is_control)
        || !p.delta.is_finite()
        || p.delta == 0.0
        || p.delta.abs() > 1_000_000_000.0
        || p.currency_name.is_empty()
        || p.currency_name.trim() != p.currency_name
        || p.currency_name.chars().count() > 64
        || p.currency_name.chars().any(char::is_control)
    {
        return Err(fail(
            "invalid_params",
            "invalid viewer, points amount or currency label",
        ));
    }
    let hello = remote.hello();
    if hello.features.get("pointsAdjust") != Some(&true)
        || !hello.capabilities.contains(&Capability::PointsRead)
        || !hello.capabilities.contains(&Capability::PointsAdjust)
        || hello
            .resource_restrictions
            .contains_key(&ResourceKind::Live)
        || hello
            .resource_restrictions
            .get(&ResourceKind::Viewer)
            .is_some_and(|ids| !ids.contains(&p.unique_id))
    {
        return Err(fail("forbidden", "points adjustment is not available"));
    }
    Ok(())
}
async fn mutate_points(shared: &Shared, epoch: &str, p: PointsCommand) -> Result<(), ClientError> {
    let selected = selected(shared, epoch, true)?;
    let remote = selected.remote.ok_or_else(network)?;
    allowed_points(shared, &p, &remote)?;
    #[cfg(test)]
    points_crash_stage("before_intent");
    let intent = Intent {
        command_id: uuid::Uuid::new_v4().to_string(),
        operation: Operation::AdjustPoints,
        resource_id: p.unique_id.clone(),
        host_instance_id: remote.hello().host_instance_id.clone(),
        expected_revision: None,
        delta: Some(p.delta),
        currency_name: Some(p.currency_name.clone()),
        proposal_revision: None,
        created_at: profiles::commands::timestamp(),
    };
    let j = selected.journal.clone();
    let pending = intent.clone();
    tokio::task::spawn_blocking(move || j.begin(pending))
        .await
        .map_err(|_| fail("command_journal_unavailable", "journal task failed"))??;
    #[cfg(test)]
    points_crash_stage("after_intent");
    if let Err(error) = allowed_points(shared, &p, &remote) {
        finish(
            &selected.journal,
            intent,
            Outcome::Rejected,
            None,
            Some(code(&error)),
        )
        .await?;
        return Err(error);
    }
    let reply = remote
        .call_value_with_meta(
            "points.adjust",
            json!({"uniqueId":p.unique_id,"delta":p.delta,"expectedCurrency":p.currency_name}),
            Some(RequestMetadata {
                command_id: Some(intent.command_id.clone()),
                host_instance_id: Some(intent.host_instance_id.clone()),
                expected_revision: None,
            }),
        )
        .await;
    confirm_reply(&selected.journal, intent, reply).await
}
fn points_result(intent: &Intent, value: &Value) -> Result<Option<PointsResult>, ClientError> {
    if intent.operation != Operation::AdjustPoints {
        return Ok(None);
    }
    let award: tiktools_control_api::modules::points::PointAward =
        serde_json::from_value(value.clone()).map_err(|_| protocol())?;
    let requested = intent.delta.ok_or_else(protocol)?;
    let result = PointsResult {
        delta: award.delta,
        total_points: award.total_points,
        level: award.level,
    };
    if award.unique_id != intent.resource_id
        || Some(award.currency_name.as_str()) != intent.currency_name.as_deref()
        || !result.valid_for(requested)
    {
        return Err(protocol());
    }
    Ok(Some(result))
}
pub(super) async fn confirm_reply(
    journal: &Arc<Journal>,
    intent: Intent,
    reply: Result<CallResult<Value>, ClientError>,
) -> Result<(), ClientError> {
    match reply {
        Ok(reply) => {
            let meta = reply.meta.ok_or_else(protocol)?;
            if meta.host_instance_id.as_deref() != Some(intent.host_instance_id.as_str()) {
                return Err(protocol());
            }
            let revision = validate_success(
                &intent,
                meta.command.as_ref().ok_or_else(protocol)?,
                &meta.resource_versions,
                &reply.result,
            )?;
            let points = points_result(&intent, &reply.result)?;
            #[cfg(test)]
            let is_event = intent.operation.is_event();
            #[cfg(test)]
            if is_event {
                automation_crash_stage("after_reply");
            }
            #[cfg(test)]
            if intent.operation == Operation::AdjustPoints {
                points_crash_stage("after_reply");
            }
            #[cfg(test)]
            let is_points = intent.operation == Operation::AdjustPoints;
            finish_record(
                journal,
                intent,
                Outcome::Succeeded,
                Some(revision),
                None,
                points,
            )
            .await?;
            #[cfg(test)]
            if is_points {
                points_crash_stage("after_record");
            }
            #[cfg(test)]
            if is_event {
                automation_crash_stage("after_record");
            }
            Ok(())
        }
        Err(error) => {
            // Forbidden/unauthorized can be egress checks AFTER commit. Internal
            // failures and transport loss likewise cannot prove non-acceptance.
            if matches!(
                error.code.as_str(),
                "invalid_params"
                    | "revision_conflict"
                    | "not_found"
                    | "overloaded"
                    | "currency_changed"
                    | "automation_not_found"
            ) {
                finish(journal, intent, Outcome::Rejected, None, Some(code(&error))).await?;
            }
            Err(error)
        }
    }
}

#[cfg(test)]
fn points_crash_stage(stage: &str) {
    if std::env::var("TIKTOOLS_NATIVE_POINTS_CRASH_STAGE").as_deref() == Ok(stage) {
        std::process::exit(98);
    }
}
async fn resolve(shared: &Shared, epoch: &str, id: Option<&str>) -> Result<(), ClientError> {
    let selected = selected(shared, epoch, true)?;
    let Some(intent) = selected.journal.view().pending else {
        return if id.is_some() {
            Err(fail("stale_view", "pending command changed"))
        } else {
            Ok(())
        };
    };
    if id.is_some_and(|id| id != intent.command_id) {
        return Err(fail("stale_view", "pending command changed"));
    }
    let remote = selected.remote.ok_or_else(network)?;
    let reply = remote
        .call_value_with_meta(
            "collaboration.commands.get",
            json!({"commandId":intent.command_id}),
            None,
        )
        .await?;
    let outcome: CommandOutcome = serde_json::from_value(reply.result).map_err(|_| protocol())?;
    let binding = selected.journal.view().binding;
    if outcome.principal_id != binding.principal_id
        || outcome.method != intent.operation.method()
        || outcome.host_instance_id != intent.host_instance_id
        || outcome.command.command_id != intent.command_id
    {
        return Err(protocol());
    }
    if outcome.command.state != CommandState::Succeeded {
        return Err(fail(
            if matches!(
                outcome.command.state,
                CommandState::Accepted | CommandState::Running
            ) {
                "command_running"
            } else {
                "command_unknown"
            },
            "command outcome is not confirmed",
        ));
    }
    let revision = validate_success(
        &intent,
        &outcome.command,
        &outcome.resource_versions,
        &outcome.result,
    )?;
    let points = points_result(&intent, &outcome.result)?;
    finish_record(
        &selected.journal,
        intent,
        Outcome::Succeeded,
        Some(revision),
        None,
        points,
    )
    .await
}
async fn action(
    shared: Arc<Shared>,
    gate: Arc<Mutex<()>>,
    epoch: String,
    action: Action,
) -> Result<Value, ClientError> {
    let _guard = gate.try_lock_owned().map_err(|_| {
        fail(
            "command_pending",
            "a controller command is already being processed",
        )
    })?;
    selected(&shared, &epoch, false)?;
    if let Action::Mutate(p) = &action {
        let selected = selected(&shared, &epoch, true)?;
        allowed(&shared, p, selected.remote.as_ref().ok_or_else(network)?)?;
    }
    if let Action::Points(p) = &action {
        let selected = selected(&shared, &epoch, true)?;
        allowed_points(&shared, p, selected.remote.as_ref().ok_or_else(network)?)?;
    }
    if let Action::Automation(p) = &action {
        let selected = selected(&shared, &epoch, true)?;
        allowed_automation(&shared, p, selected.remote.as_ref().ok_or_else(network)?)?;
    }
    shared.change(&epoch, |v| {
        v.status.command_busy = true;
        v.status.command_error_code = None;
    });
    let result = async {
        match action {
            Action::Mutate(p) => mutate(&shared, &epoch, p).await,
            Action::Points(p) => mutate_points(&shared, &epoch, p).await,
            Action::Automation(p) => mutate_automation(&shared, &epoch, p).await,
            Action::Proposal(p) => proposals::mutate(&shared, p).await,
            Action::Resolve(id) => resolve(&shared, &epoch, id.as_deref()).await,
            Action::Review(id) => {
                let selected = selected(&shared, &epoch, false)?;
                let intent = selected
                    .journal
                    .view()
                    .pending
                    .ok_or_else(|| fail("stale_view", "pending command changed"))?;
                if intent.command_id != id {
                    Err(fail("stale_view", "pending command changed"))
                } else {
                    finish(
                        &selected.journal,
                        intent,
                        Outcome::ReviewedUnknown,
                        None,
                        Some("command_unknown".into()),
                    )
                    .await
                }
            }
        }
    }
    .await;
    points::dirty(&shared, &epoch);
    shared.change(&epoch, |v| {
        v.status.command_busy = false;
        v.status.command_error_code = result.as_ref().err().map(code);
    });
    selected(&shared, &epoch, false)?;
    result?;
    let view = shared.view.read().unwrap_or_else(|p| p.into_inner());
    if view.status.epoch != epoch {
        return Err(fail("stale_view", "controller selection changed"));
    }
    serde_json::to_value(journal_projection(&view).ok_or_else(protocol)?).map_err(|_| protocol())
}
pub(super) fn schedule_resolve(shared: Arc<Shared>, gate: Arc<Mutex<()>>, epoch: String) {
    if selected(&shared, &epoch, false).is_ok_and(|s| s.journal.view().pending.is_some()) {
        tokio::spawn(async move {
            let _ = action(shared, gate, epoch, Action::Resolve(None)).await;
        });
    }
}
impl Controller {
    pub(super) async fn proposal_command(
        &self,
        p: proposals::Submit,
    ) -> Result<Value, ClientError> {
        let epoch = p.epoch.clone();
        self.command_task(epoch, Action::Proposal(p)).await
    }
    pub async fn automation_command(&self, p: AutomationCommand) -> Result<Value, ClientError> {
        let epoch = p.epoch.clone();
        self.command_task(epoch, Action::Automation(p)).await
    }
    pub async fn points_command(&self, p: PointsCommand) -> Result<Value, ClientError> {
        let epoch = p.epoch.clone();
        self.command_task(epoch, Action::Points(p)).await
    }
    pub(super) async fn open_journal(
        &self,
        profile: &profiles::RemoteProfile,
    ) -> Result<Arc<Journal>, ClientError> {
        if let Some(journal) = self
            .journals
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get(&profile.profile_id)
            .and_then(std::sync::Weak::upgrade)
        {
            return if journal.matches(profile) {
                Ok(journal)
            } else {
                Err(fail("command_journal_invalid", "profile identity changed"))
            };
        }
        let root = self.profiles.directory().to_path_buf();
        let p = profile.clone();
        let journal = Arc::new(
            tokio::task::spawn_blocking(move || Journal::open(&root, &p))
                .await
                .map_err(|_| fail("command_journal_unavailable", "journal task failed"))??,
        );
        let mut cache = self.journals.lock().unwrap_or_else(|p| p.into_inner());
        cache.retain(|_, v| v.strong_count() > 0);
        cache.insert(profile.profile_id.clone(), Arc::downgrade(&journal));
        Ok(journal)
    }
    pub async fn workflow_command(&self, p: WorkflowCommand) -> Result<Value, ClientError> {
        let epoch = p.epoch.clone();
        self.command_task(epoch, Action::Mutate(p)).await
    }
    pub async fn resolve_command(&self, epoch: String, id: String) -> Result<Value, ClientError> {
        self.command_task(epoch, Action::Resolve(Some(id))).await
    }
    pub async fn review_command(&self, epoch: String, id: String) -> Result<Value, ClientError> {
        self.command_task(epoch, Action::Review(id)).await
    }
    async fn command_task(&self, epoch: String, operation: Action) -> Result<Value, ClientError> {
        let shared = self.shared.clone();
        let commands = self.commands.clone();
        // Dropping the WebView request does not cancel known-result persistence.
        tokio::spawn(async move { action(shared, commands, epoch, operation).await })
            .await
            .map_err(|_| {
                fail(
                    "command_unknown",
                    "controller command task ended; check the durable intent",
                )
            })?
    }
}

#[cfg(test)]
mod tests;

fn allowed_automation(
    shared: &Shared,
    p: &AutomationCommand,
    remote: &RemoteClient,
) -> Result<(), ClientError> {
    if !p.operation.is_automation_command() {
        return Err(fail("invalid_params", "unsupported automation operation"));
    }
    let v = shared.view.read().unwrap_or_else(|p| p.into_inner());
    let h = remote.hello();
    if v.status.epoch != p.epoch
        || v.status.phase != "ready"
        || remote.is_closed()
        || remote.requires_resync()
        || v.remote
            .as_ref()
            .is_none_or(|r| r.hello().connection_id != h.connection_id)
    {
        return Err(fail(
            "not_ready",
            "selected connection changed; recover first",
        ));
    }
    if p.automation_revision != v.status.automation_revision
        || p.connection_id != h.connection_id
        || p.host_instance_id != h.host_instance_id
        || p.grant_revision != h.grant_revision
    {
        return Err(fail(
            "stale_view",
            "automation confirmation changed; review again",
        ));
    }
    let binding = v.journal.as_ref().ok_or_else(network)?.view().binding;
    if binding.host_id != h.host_id
        || binding.principal_id != h.principal_id
        || binding.session_id != h.session_id
        || Some(binding.origin.as_str()) != v.status.origin.as_deref()
    {
        return Err(fail("stale_view", "profile identity changed"));
    }
    if !automations::readable(remote)
        || h.features.get(p.operation.feature()) != Some(&true)
        || !h.capabilities.contains(&p.operation.capability())
        || h.resource_restrictions
            .get(&ResourceKind::Automation)
            .is_some_and(|ids| !ids.contains(&format!("{}:{}", p.operation.scope(), p.id)))
    {
        return Err(fail("forbidden", "event operation is unavailable"));
    }
    let family = tiktools_collaboration::AutomationFamily::from_scope(p.operation.scope())
        .ok_or_else(protocol)?;
    let row = automations::automation_row(&v, family, &p.id)
        .ok_or_else(|| fail("stale_view", "event is no longer available"))?;
    if row.version.revision != p.expected_revision
        || row.version.kind != ResourceKind::Automation
        || row.version.scope != p.operation.scope()
        || row.version.id != p.id
    {
        return Err(fail("stale_view", "event revision changed; review again"));
    }
    if p.operation.is_activation() && row.summary.enabled {
        return Err(fail("stale_view", "automation is already enabled"));
    }
    Ok(())
}
async fn mutate_automation(
    shared: &Shared,
    epoch: &str,
    p: AutomationCommand,
) -> Result<(), ClientError> {
    let selected = selected(shared, epoch, true)?;
    let remote = selected.remote.ok_or_else(network)?;
    allowed_automation(shared, &p, &remote)?;
    #[cfg(test)]
    automation_crash_stage("before_intent");
    let intent = Intent {
        command_id: uuid::Uuid::new_v4().to_string(),
        operation: p.operation,
        resource_id: p.id.clone(),
        host_instance_id: remote.hello().host_instance_id.clone(),
        expected_revision: Some(p.expected_revision.clone()),
        delta: None,
        currency_name: None,
        proposal_revision: None,
        created_at: profiles::commands::timestamp(),
    };
    let j = selected.journal.clone();
    let pending = intent.clone();
    tokio::task::spawn_blocking(move || j.begin(pending))
        .await
        .map_err(|_| fail("command_journal_unavailable", "journal task failed"))??;
    #[cfg(test)]
    automation_crash_stage("after_intent");
    if let Err(e) = allowed_automation(shared, &p, &remote) {
        finish(
            &selected.journal,
            intent,
            Outcome::Rejected,
            None,
            Some(code(&e)),
        )
        .await?;
        return Err(e);
    }
    #[cfg(test)]
    automation_crash_stage("before_send");
    let reply = remote
        .call_value_with_meta(
            intent.operation.method(),
            json!({"kind":intent.operation.scope(),"id":intent.resource_id}),
            Some(RequestMetadata {
                command_id: Some(intent.command_id.clone()),
                host_instance_id: Some(intent.host_instance_id.clone()),
                expected_revision: intent.expected_revision.clone(),
            }),
        )
        .await;
    confirm_reply(&selected.journal, intent, reply).await
}

#[cfg(test)]
fn automation_crash_stage(stage: &str) {
    if std::env::var("TIKTOOLS_NATIVE_EVENT_CRASH_STAGE").as_deref() == Ok(stage) {
        std::process::exit(97);
    }
}

#[cfg(test)]
mod event_receipts;
