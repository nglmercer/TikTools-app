//! Exact viewer and currency admission for durable point adjustments.
use super::*;

pub(super) fn allowed_points(
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

pub(super) async fn mutate_points(
    shared: &Shared,
    epoch: &str,
    p: PointsCommand,
) -> Result<(), ClientError> {
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
        expected_generation: None,
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
