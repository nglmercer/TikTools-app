//! Outcome lookup for an existing durable intent without replaying it.
use super::*;

pub(super) async fn resolve(
    shared: &Shared,
    epoch: &str,
    id: Option<&str>,
) -> Result<(), ClientError> {
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
