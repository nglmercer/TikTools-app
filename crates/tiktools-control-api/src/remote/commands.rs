//! Retained command outcomes obey the same resource and result bounds as live replies.
use crate::{context, ApiError};
use crate::{policy, RemoteCapability, RemoteSession};
use serde_json::Value;
use tiktools_core::collaboration::Grant;

pub(super) fn authorize(
    session: &RemoteSession,
    grant: &Grant,
    method: &str,
    result: &Value,
) -> Result<(), ApiError> {
    if method == "collaboration.commands.get" {
        if result.get("principalId").and_then(Value::as_str) != Some(session.principal_id()) {
            return Err(context::forbidden());
        }
        let resource = result
            .pointer("/command/resource")
            .ok_or_else(context::forbidden)?;
        let kind = match resource.get("kind").and_then(Value::as_str) {
            Some("workflow")
                if grant
                    .capabilities
                    .contains(&RemoteCapability::WorkflowsRead) =>
            {
                tiktools_core::collaboration::ResourceKind::Workflow
            }
            Some("viewer")
                if grant.capabilities.contains(&RemoteCapability::PointsRead)
                    && !grant
                        .resources
                        .contains_key(&tiktools_core::collaboration::ResourceKind::Live) =>
            {
                tiktools_core::collaboration::ResourceKind::Viewer
            }
            Some("automation") if policy::automation_read_allowed(grant) => {
                tiktools_core::collaboration::ResourceKind::Automation
            }
            Some("competition")
                if grant
                    .capabilities
                    .contains(&RemoteCapability::CompetitionsRead)
                    && !grant
                        .resources
                        .contains_key(&tiktools_core::collaboration::ResourceKind::Live) =>
            {
                tiktools_core::collaboration::ResourceKind::Competition
            }
            _ => return Err(context::forbidden()),
        };
        let scope = resource
            .get("scope")
            .and_then(Value::as_str)
            .ok_or_else(context::forbidden)?;
        if !resource
            .get("id")
            .and_then(Value::as_str)
            .is_some_and(|id| grant.allows_scoped_resource(kind, scope, id))
        {
            return Err(context::forbidden());
        }
        if kind == tiktools_core::collaboration::ResourceKind::Automation {
            if tiktools_core::collaboration::AutomationFamily::from_scope(scope).is_none() {
                return Err(context::forbidden());
            }
            // A retained outcome must be just as narrow as the live
            // command response. Expired-outcome errors carry target only.
            if let Some(method) = result.get("method").and_then(Value::as_str) {
                if !matches!(
                    method,
                    "automation.disable"
                        | "automation.delete"
                        | "automation.create"
                        | "automation.update"
                        | "automation.enable"
                ) || (method == "automation.delete" && scope != "event")
                    || !result.get("result").is_some_and(|v| {
                        policy::automation_result_allowed(grant, method, v)
                            && v.get("id") == resource.get("id")
                            && (!matches!(
                                method,
                                "automation.create" | "automation.update" | "automation.enable"
                            ) || v.get("kind").and_then(Value::as_str) == Some(scope))
                            && (method != "automation.disable"
                                || if scope == "action" {
                                    v.get("kind").and_then(Value::as_str) == Some("action")
                                } else {
                                    scope == "event" && v.get("kind").is_none()
                                })
                    })
                    || !result
                        .get("resourceVersions")
                        .and_then(Value::as_array)
                        .is_some_and(|rows| {
                            rows.len() == 1
                                && rows.iter().all(|v| {
                                    v.get("kind") == resource.get("kind")
                                        && v.get("scope") == resource.get("scope")
                                        && v.get("id") == resource.get("id")
                                })
                        })
                {
                    return Err(context::forbidden());
                }
            }
        }
        if kind == tiktools_core::collaboration::ResourceKind::Competition {
            if let Some(method) = result.get("method").and_then(Value::as_str) {
                if !result.get("result").is_some_and(|value| {
                    policy::competition_stop_result_allowed(grant, method, value)
                        && value.get("id") == resource.get("id")
                }) || !result
                    .get("resourceVersions")
                    .and_then(Value::as_array)
                    .is_some_and(|rows| {
                        rows.len() == 1
                            && rows[0].get("kind") == resource.get("kind")
                            && rows[0].get("scope") == resource.get("scope")
                            && rows[0].get("id") == resource.get("id")
                    })
                {
                    return Err(context::forbidden());
                }
            }
        }
    }
    Ok(())
}
