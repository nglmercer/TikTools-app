//! Durable receipt recording and known-result reply classification.
use super::*;

pub(in crate::remote::controller) async fn finish(
    journal: &Arc<Journal>,
    intent: Intent,
    outcome: Outcome,
    revision: Option<String>,
    error_code: Option<String>,
) -> Result<(), ClientError> {
    finish_record(journal, intent, outcome, revision, error_code, None).await
}

pub(super) async fn finish_record(
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
        recorded_at: crate::remote::profiles::commands::timestamp(),
        points_result,
    };
    let j = journal.clone();
    tokio::task::spawn_blocking(move || j.finish(record))
        .await
        .map_err(|_| fail("command_journal_unavailable", "journal task failed"))?
}

pub(in crate::remote::controller) async fn confirm_reply(
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
