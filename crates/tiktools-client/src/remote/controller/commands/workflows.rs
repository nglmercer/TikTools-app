//! Workflow admission and durable intent creation before dispatch.
use super::*;

pub(super) fn allowed(
    shared: &Shared,
    p: &WorkflowCommand,
    remote: &RemoteClient,
) -> Result<(), ClientError> {
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

pub(super) async fn mutate(
    shared: &Shared,
    epoch: &str,
    p: WorkflowCommand,
) -> Result<(), ClientError> {
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
        expected_generation: None,
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
