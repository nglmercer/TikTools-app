//! Exact binding of successful receipts to durable command intents.
use super::*;

pub(super) fn validate_success(
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
        Operation::DisableAction => {
            if result.as_object().is_none_or(|o| o.len() != 3)
                || result.get("kind").and_then(Value::as_str) != Some("action")
                || result.get("enabled") != Some(&Value::Bool(false))
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
        Operation::StartCompetition
        | Operation::ResumeCompetition
        | Operation::PauseCompetition
        | Operation::CancelCompetition => {
            let expected = intent.expected_generation.as_deref().ok_or_else(protocol)?;
            let generation =
                tiktools_collaboration::stream_sequence(expected).map_err(|_| protocol())?;
            let resulting = if intent.operation == Operation::StartCompetition {
                generation
                    .checked_add(1)
                    .filter(|n| *n <= i64::MAX as u64)
                    .ok_or_else(protocol)?
                    .to_string()
            } else {
                expected.to_owned()
            };
            let lifecycle = match intent.operation {
                Operation::StartCompetition | Operation::ResumeCompetition => "active",
                Operation::PauseCompetition => "paused",
                Operation::CancelCompetition => "cancelled",
                _ => unreachable!(),
            };
            if result.as_object().is_none_or(|o| o.len() != 3)
                || result.get("generation").and_then(Value::as_str) != Some(resulting.as_str())
                || result.get("lifecycle").and_then(Value::as_str) != Some(lifecycle)
            {
                return Err(protocol());
            }
            true
        }
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

pub(super) fn points_result(
    intent: &Intent,
    value: &Value,
) -> Result<Option<PointsResult>, ClientError> {
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
