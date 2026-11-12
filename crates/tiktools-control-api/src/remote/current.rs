//! Revalidate creator, subscription and saved-source evidence against the current host.
use crate::{context, ApiError};
use crate::{policy, ControlApi, RemoteSession};
use serde_json::Value;
use tiktools_core::collaboration::Grant;

pub(super) fn creator_feed(grant: &Grant, method: &str, result: &Value) -> Result<(), ApiError> {
    if method == "ui.feed.creator"
        && !result
            .get("creatorId")
            .and_then(Value::as_str)
            .is_some_and(|id| {
                grant.allows_resource(tiktools_core::collaboration::ResourceKind::Live, id)
            })
    {
        return Err(context::forbidden());
    }
    Ok(())
}

impl ControlApi {
    pub(super) fn authorize_remote_stream_result(
        &self,
        session: &RemoteSession,
        method: &str,
        result: &Value,
    ) -> Result<(), ApiError> {
        if matches!(
            method,
            "collaboration.subscribe" | "collaboration.snapshot" | "collaboration.resume"
        ) {
            let id = result
                .get("subscriptionId")
                .and_then(Value::as_str)
                .ok_or_else(context::forbidden)?;
            let revision = result
                .get("grantRevision")
                .and_then(Value::as_str)
                .ok_or_else(context::forbidden)?;
            self.core
                .collaboration_stream_validate(
                    &session.authorization()?,
                    session.connection_id().ok_or_else(context::forbidden)?,
                    id,
                    revision,
                    result.get("snapshotId").and_then(Value::as_str),
                )
                .map_err(ApiError::from)?;
        }
        Ok(())
    }

    pub(super) fn authorize_remote_automation_result(
        &self,
        grant: &Grant,
        session: &RemoteSession,
        method: &str,
        result: &Value,
    ) -> Result<(), ApiError> {
        if method.starts_with("automation.")
            && !policy::automation_result_allowed(grant, method, result)
        {
            return Err(context::forbidden());
        }
        if method == "automation.source.get" {
            let kind = result
                .get("kind")
                .and_then(Value::as_str)
                .and_then(|s| s.parse::<tiktools_core::control::AutomationKind>().ok())
                .ok_or_else(context::forbidden)?;
            let id = result
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(context::forbidden)?;
            let revision = result
                .get("revision")
                .and_then(Value::as_str)
                .ok_or_else(context::forbidden)?;
            if !self
                .core
                .automation_shared_source_current(kind, id, revision, &session.authorization()?)
                .unwrap_or(false)
            {
                return Err(context::forbidden());
            }
        }
        Ok(())
    }

    pub(super) fn authorize_remote_live_result(
        &self,
        grant: &Grant,
        method: &str,
        result: &Value,
    ) -> Result<(), ApiError> {
        if matches!(method, "live.status" | "live.monitor.status") {
            if let Some(ids) = grant
                .resources
                .get(&tiktools_core::collaboration::ResourceKind::Live)
            {
                if !result
                    .get("uniqueId")
                    .and_then(Value::as_str)
                    .is_some_and(|id| ids.contains(id))
                {
                    return Err(context::forbidden());
                }
                if method == "live.status" {
                    let verified = result
                        .get("uniqueId")
                        .and_then(Value::as_str)
                        .zip(result.get("roomId").and_then(Value::as_str))
                        .zip(result.get("connectionId").and_then(Value::as_str))
                        .is_some_and(|((creator, room), connection)| {
                            self.core
                                .live_status_matches_verified_scope(creator, room, connection)
                        });
                    if !verified {
                        return Err(context::forbidden());
                    }
                }
            }
        }
        Ok(())
    }
}
