//! Version-bound automation admission and durable command dispatch.
use super::*;

pub(super) fn allowed_automation(
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

pub(super) async fn mutate_automation(
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
        expected_generation: None,
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
