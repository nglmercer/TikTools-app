//! Remote admission and egress checks, including queued replies and failures.
use crate::{context, policy, ApiError, ControlApi, RemoteCapability, RemoteSession};
use serde_json::Value;

mod commands;
mod current;
mod results;

impl ControlApi {
    /// Reauthorize a queued remote response immediately before network send.
    pub fn authorize_remote(&self, session: &RemoteSession, method: &str) -> Result<(), ApiError> {
        self.router
            .get(method)
            .map(|entry| entry.remote_policy)
            .unwrap_or_default()
            .authorize(session)?;
        // Shared backfill is not filtered by creator. Use a scoped projection
        // before exposing history under a creator-restricted LIVE grant.
        if matches!(method, "ui.feed.backfill" | "ui.feed.browser")
            && session
                .grant()?
                .resources
                .contains_key(&tiktools_core::collaboration::ResourceKind::Live)
        {
            return Err(context::forbidden());
        }
        if matches!(method, "workflows.disable" | "workflows.delete")
            && !session
                .grant()?
                .capabilities
                .contains(&RemoteCapability::WorkflowsRead)
        {
            return Err(context::forbidden());
        }
        let grant = session.grant()?;
        if matches!(
            method,
            "competitions.summary"
                | "competitions.start"
                | "competitions.resume"
                | "competitions.pause"
                | "competitions.cancel"
        ) && grant
            .resources
            .contains_key(&tiktools_core::collaboration::ResourceKind::Live)
        {
            return Err(context::forbidden());
        }
        if matches!(
            method,
            "competitions.start"
                | "competitions.resume"
                | "competitions.pause"
                | "competitions.cancel"
        ) && !grant
            .capabilities
            .contains(&RemoteCapability::CompetitionsRead)
        {
            return Err(context::forbidden());
        }
        if method.starts_with("automation.") && !policy::automation_read_allowed(&grant) {
            return Err(context::forbidden());
        }
        let can_read_domain = grant
            .capabilities
            .contains(&RemoteCapability::WorkflowsRead)
            || grant.capabilities.contains(&RemoteCapability::PointsRead)
                && !grant
                    .resources
                    .contains_key(&tiktools_core::collaboration::ResourceKind::Live)
            || policy::automation_read_allowed(&grant)
            || grant
                .capabilities
                .contains(&RemoteCapability::CompetitionsRead)
                && !grant
                    .resources
                    .contains_key(&tiktools_core::collaboration::ResourceKind::Live);
        if matches!(
            method,
            "collaboration.subscribe" | "collaboration.snapshot" | "collaboration.resume"
        ) && !can_read_domain
        {
            return Err(context::forbidden());
        }
        if method == "collaboration.commands.get"
            && !can_read_domain
            && !(grant
                .capabilities
                .contains(&RemoteCapability::CompetitionsRead)
                && !grant
                    .resources
                    .contains_key(&tiktools_core::collaboration::ResourceKind::Live))
        {
            return Err(context::forbidden());
        }
        if matches!(
            method,
            "points.adjust"
                | "points.viewer.get"
                | "points.leaderboard"
                | "points.info"
                | "points.view"
        ) && (!grant.capabilities.contains(&RemoteCapability::PointsRead)
            || grant
                .resources
                .contains_key(&tiktools_core::collaboration::ResourceKind::Live))
        {
            return Err(context::forbidden());
        }
        Ok(())
    }

    pub fn authorize_remote_response(
        &self,
        session: &RemoteSession,
        method: &str,
        result: &Value,
    ) -> Result<(), ApiError> {
        self.authorize_remote(session, method)?;
        let grant = session.grant()?;
        results::competitions(&grant, method, result)?;
        current::creator_feed(&grant, method, result)?;
        results::points(&grant, method, result)?;
        self.authorize_remote_stream_result(session, method, result)?;
        results::workflows(&grant, method, result)?;
        self.authorize_remote_automation_result(&grant, session, method, result)?;
        commands::authorize(session, &grant, method, result)?;
        self.authorize_remote_live_result(&grant, method, result)?;
        Ok(())
    }

    /// Resource-bound command failures must obey the same egress policy as
    /// successful outcomes, including while queued behind other socket work.
    pub fn authorize_remote_failure(
        &self,
        session: &RemoteSession,
        method: &str,
        data: Option<&Value>,
    ) -> Result<(), ApiError> {
        self.authorize_remote(session, method)?;
        if matches!(
            method,
            "workflows.get"
                | "workflows.list"
                | "workflows.disable"
                | "workflows.delete"
                | "points.viewer.get"
                | "points.leaderboard"
                | "points.view"
                | "points.adjust"
                | "competitions.start"
                | "competitions.resume"
                | "competitions.pause"
                | "competitions.cancel"
                | "automation.list"
                | "automation.get"
                | "automation.source.get"
                | "automation.disable"
                | "automation.enable"
                | "automation.delete"
                | "automation.create"
                | "automation.update"
        ) {
            if let Some(versions) = data
                .and_then(|data| data.get("resourceVersions"))
                .and_then(Value::as_array)
            {
                let grant = session.grant()?;
                let (kind, wire) = if method.starts_with("automation.") {
                    (
                        tiktools_core::collaboration::ResourceKind::Automation,
                        "automation",
                    )
                } else if method.starts_with("competitions.") {
                    (
                        tiktools_core::collaboration::ResourceKind::Competition,
                        "competition",
                    )
                } else if method.starts_with("points.") {
                    (tiktools_core::collaboration::ResourceKind::Viewer, "viewer")
                } else {
                    (
                        tiktools_core::collaboration::ResourceKind::Workflow,
                        "workflow",
                    )
                };
                if versions.iter().any(|v| {
                    v.get("kind").and_then(Value::as_str) != Some(wire)
                        || !v.get("id").and_then(Value::as_str).is_some_and(|id| {
                            v.get("scope").and_then(Value::as_str).is_some_and(|scope| {
                                grant.allows_scoped_resource(kind, scope, id)
                                    && (method != "automation.delete" || scope == "event")
                            })
                        })
                }) {
                    return Err(context::forbidden());
                }
            }
        }
        if let Some(data) = data.filter(|data| {
            method == "collaboration.commands.get" && data.pointer("/command/resource").is_some()
        }) {
            return self.authorize_remote_response(session, method, data);
        }
        Ok(())
    }
}
