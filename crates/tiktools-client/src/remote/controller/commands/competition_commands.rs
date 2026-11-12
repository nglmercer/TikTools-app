//! Competition revision and generation admission before dispatch.
use super::*;

pub(super) fn allowed_competition(
    shared: &Shared,
    p: &CompetitionCommand,
    remote: &RemoteClient,
) -> Result<(), ClientError> {
    if !p.operation.is_competition() {
        return Err(fail("invalid_params", "unsupported competition operation"));
    }
    let view = shared.view.read().unwrap_or_else(|p| p.into_inner());
    let hello = remote.hello();
    let status = view.status.hello.as_ref().ok_or_else(network)?;
    if view.status.epoch != p.epoch
        || view.status.phase != "ready"
        || remote.is_closed()
        || remote.requires_resync()
        || view
            .remote
            .as_ref()
            .is_none_or(|r| r.hello().connection_id != hello.connection_id)
        || p.connection_id != hello.connection_id
        || p.host_instance_id != hello.host_instance_id
        || p.grant_revision != hello.grant_revision
        || status.grant_revision != hello.grant_revision
    {
        return Err(fail("stale_view", "competition connection changed"));
    }
    let binding = view.journal.as_ref().ok_or_else(network)?.view().binding;
    if binding.host_id != hello.host_id
        || binding.principal_id != hello.principal_id
        || binding.session_id != hello.session_id
        || Some(binding.origin.as_str()) != view.status.origin.as_deref()
    {
        return Err(fail("stale_view", "profile identity changed"));
    }
    if hello.features.get(p.operation.feature()) != Some(&true)
        || hello.features.get("competitionSummaryVersions") != Some(&true)
        || !hello.capabilities.contains(&Capability::CompetitionsRead)
        || !hello.capabilities.contains(&Capability::CompetitionsManage)
        || !hello.capabilities.contains(&Capability::CommandsRead)
        || hello
            .resource_restrictions
            .contains_key(&ResourceKind::Live)
        || hello
            .resource_restrictions
            .get(&ResourceKind::Competition)
            .is_some_and(|ids| !ids.contains(&p.id))
    {
        return Err(fail("forbidden", "competition operation unavailable"));
    }
    let observed = view
        .competition_observation
        .as_ref()
        .ok_or_else(|| fail("stale_view", "refresh competition status"))?;
    if observed.epoch != p.epoch
        || observed.connection != p.connection_id
        || observed.host_instance != p.host_instance_id
        || observed.grant_revision != p.grant_revision
    {
        return Err(fail("stale_view", "competition status changed"));
    }
    let row = observed
        .rows
        .get(&p.id)
        .ok_or_else(|| fail("stale_view", "competition no longer visible"))?;
    if row.revision.as_deref() != Some(p.expected_revision.as_str())
        || row.generation != p.expected_generation
        || !(match p.operation {
            Operation::StartCompetition => matches!(
                row.lifecycle,
                tiktools_control_api::modules::competitions::Lifecycle::Draft
            ),
            Operation::ResumeCompetition => matches!(
                row.lifecycle,
                tiktools_control_api::modules::competitions::Lifecycle::Paused
            ),
            Operation::PauseCompetition => matches!(
                row.lifecycle,
                tiktools_control_api::modules::competitions::Lifecycle::Active
            ),
            Operation::CancelCompetition => matches!(
                row.lifecycle,
                tiktools_control_api::modules::competitions::Lifecycle::Draft
                    | tiktools_control_api::modules::competitions::Lifecycle::Active
                    | tiktools_control_api::modules::competitions::Lifecycle::Paused
            ),
            _ => false,
        })
    {
        return Err(fail("stale_view", "competition changed"));
    }
    Ok(())
}

pub(super) async fn mutate_competition(
    shared: &Shared,
    epoch: &str,
    p: CompetitionCommand,
) -> Result<(), ClientError> {
    let selected = selected(shared, epoch, true)?;
    let remote = selected.remote.ok_or_else(network)?;
    allowed_competition(shared, &p, &remote)?;
    let intent = Intent {
        command_id: uuid::Uuid::new_v4().to_string(),
        operation: p.operation,
        resource_id: p.id.clone(),
        host_instance_id: p.host_instance_id.clone(),
        expected_revision: Some(p.expected_revision.clone()),
        delta: None,
        currency_name: None,
        proposal_revision: None,
        expected_generation: Some(p.expected_generation.clone()),
        created_at: profiles::commands::timestamp(),
    };
    let journal = selected.journal.clone();
    let pending = intent.clone();
    tokio::task::spawn_blocking(move || journal.begin(pending))
        .await
        .map_err(|_| fail("command_journal_unavailable", "journal task failed"))??;
    if let Err(error) = allowed_competition(shared, &p, &remote) {
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
            json!({"id":intent.resource_id,"expectedGeneration":intent.expected_generation}),
            Some(RequestMetadata {
                command_id: Some(intent.command_id.clone()),
                host_instance_id: Some(intent.host_instance_id.clone()),
                expected_revision: intent.expected_revision.clone(),
            }),
        )
        .await;
    confirm_reply(&selected.journal, intent, reply).await
}
