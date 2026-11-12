//! Structural and resource-scope checks for competition, point and workflow replies.
use crate::{context, ApiError};
use crate::{policy, RemoteCapability};
use serde_json::Value;
use tiktools_core::collaboration::Grant;

pub(super) fn competitions(grant: &Grant, method: &str, result: &Value) -> Result<(), ApiError> {
    if matches!(
        method,
        "competitions.start" | "competitions.resume" | "competitions.pause" | "competitions.cancel"
    ) && !policy::competition_stop_result_allowed(grant, method, result)
    {
        return Err(context::forbidden());
    }
    if method == "competitions.summary" {
        if !grant
            .capabilities
            .contains(&RemoteCapability::CompetitionsRead)
            || grant
                .resources
                .contains_key(&tiktools_core::collaboration::ResourceKind::Live)
        {
            return Err(context::forbidden());
        }
        if serde_json::from_value::<tiktools_core::control::CompetitionSummaryPage>(result.clone())
            .is_err()
        {
            return Err(context::forbidden());
        }
        let allowed = |id: &str| {
            grant.allows_resource(tiktools_core::collaboration::ResourceKind::Competition, id)
        };
        let rows = result
            .get("competitions")
            .and_then(Value::as_array)
            .ok_or_else(context::forbidden)?;
        if result.as_object().is_none_or(|obj| {
            obj.len() != 2
                || obj
                    .keys()
                    .any(|key| !matches!(key.as_str(), "competitions" | "nextAfter"))
        }) || rows.len() > 50
            || !rows.iter().all(|row| {
                row.as_object().is_some_and(|obj| {
                    (obj.len() == 7
                        || (obj.len() == 8
                            && obj
                                .get("revision")
                                .and_then(Value::as_str)
                                .is_some_and(|s| {
                                    s.parse::<i64>().is_ok_and(|n| n > 0 && n.to_string() == s)
                                })))
                        && obj.keys().all(|key| {
                            matches!(
                                key.as_str(),
                                "id" | "name"
                                    | "kind"
                                    | "lifecycle"
                                    | "generation"
                                    | "revision"
                                    | "deadlineMs"
                                    | "remainingMs"
                            )
                        })
                }) && row.get("id").and_then(Value::as_str).is_some_and(allowed)
            })
            || !result.get("nextAfter").is_some_and(|value| {
                value.is_null()
                    || value.as_str().is_some_and(|id| {
                        allowed(id) && rows.last().and_then(|row| row.get("id")) == Some(value)
                    })
            })
        {
            return Err(context::forbidden());
        }
    }
    Ok(())
}

pub(super) fn points(grant: &Grant, method: &str, result: &Value) -> Result<(), ApiError> {
    if matches!(
        method,
        "points.adjust" | "points.viewer.get" | "points.leaderboard" | "points.view"
    ) {
        let allowed = |row: &Value| {
            row.get("uniqueId")
                .and_then(Value::as_str)
                .is_some_and(|id| {
                    grant.allows_resource(tiktools_core::collaboration::ResourceKind::Viewer, id)
                })
        };
        let permitted = match method {
            "points.view" => {
                result
                    .get("viewers")
                    .and_then(Value::as_array)
                    .is_some_and(|rows| rows.iter().all(allowed))
                    && result
                        .get("selected")
                        .is_some_and(|v| v.is_null() || allowed(v))
                    && result.get("selectedId").is_some_and(|v| {
                        v.is_null()
                            || v.as_str().is_some_and(|id| {
                                grant.allows_resource(
                                    tiktools_core::collaboration::ResourceKind::Viewer,
                                    id,
                                )
                            })
                    })
            }
            "points.adjust" => allowed(result),
            "points.viewer.get" => result.get("viewer").is_some_and(allowed),
            _ => result
                .get("viewers")
                .and_then(Value::as_array)
                .is_some_and(|rows| rows.iter().all(allowed)),
        };
        if !permitted
            || (method == "points.adjust"
                && grant.max_points_delta.is_some_and(|max| {
                    !result
                        .get("delta")
                        .and_then(Value::as_f64)
                        .is_some_and(|n| n.is_finite() && n.abs() <= max)
                }))
        {
            return Err(context::forbidden());
        }
    }
    Ok(())
}

pub(super) fn workflows(grant: &Grant, method: &str, result: &Value) -> Result<(), ApiError> {
    if matches!(
        method,
        "workflows.list" | "workflows.get" | "workflows.disable" | "workflows.delete"
    ) {
        let allowed = |record: &Value| {
            record.get("id").and_then(Value::as_str).is_some_and(|id| {
                grant.allows_resource(tiktools_core::collaboration::ResourceKind::Workflow, id)
            })
        };
        let permitted = if method != "workflows.list" {
            allowed(result)
        } else {
            result
                .get("workflows")
                .and_then(Value::as_array)
                .is_some_and(|rows| rows.iter().all(allowed))
        };
        if !permitted {
            return Err(context::forbidden());
        }
    }
    Ok(())
}
