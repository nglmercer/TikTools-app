//! Explicit, deny-by-default remote method policy. Write capabilities are
//! enabled only for methods with revisions, durable commands and a completed
//! operation-specific policy. Definition activation still requires effect policy.
use crate::{context::forbidden, ApiError, RemoteCapability, RemoteSession};
use tiktools_core::collaboration::{AutomationFamily, Grant, ResourceKind};

pub(crate) fn competition_stop_result_allowed(
    grant: &Grant,
    method: &str,
    value: &serde_json::Value,
) -> bool {
    grant
        .capabilities
        .contains(&RemoteCapability::CompetitionsRead)
        && !grant.resources.contains_key(&ResourceKind::Live)
        && value.as_object().is_some_and(|o| {
            o.len() == 3
                && o.keys()
                    .all(|k| matches!(k.as_str(), "id" | "lifecycle" | "generation"))
        })
        && value
            .get("id")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|id| grant.allows_resource(ResourceKind::Competition, id))
        && value.get("lifecycle").and_then(serde_json::Value::as_str)
            == match method {
                "competitions.start" | "competitions.resume" => Some("active"),
                "competitions.pause" => Some("paused"),
                "competitions.cancel" => Some("cancelled"),
                _ => None,
            }
        && value
            .get("generation")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|s| {
                s.parse::<u64>()
                    .is_ok_and(|n| n <= i64::MAX as u64 && n.to_string() == s)
            })
}

pub(crate) fn automation_read_allowed(grant: &Grant) -> bool {
    grant
        .capabilities
        .contains(&RemoteCapability::AutomationRead)
        && !grant.resources.contains_key(&ResourceKind::Live)
}

pub(crate) fn automation_result_allowed(
    grant: &Grant,
    method: &str,
    result: &serde_json::Value,
) -> bool {
    if !automation_read_allowed(grant) {
        return false;
    }
    let summary_allowed = |value: &serde_json::Value| {
        let Some(object) = value.as_object() else {
            return false;
        };
        let Some(family) = value
            .get("kind")
            .and_then(serde_json::Value::as_str)
            .and_then(AutomationFamily::from_scope)
        else {
            return false;
        };
        object.len() == 5
            && object.keys().all(|k| {
                matches!(
                    k.as_str(),
                    "kind" | "id" | "name" | "enabled" | "definitionRedacted"
                )
            })
            && value.get("definitionRedacted") == Some(&serde_json::Value::Bool(true))
            && value
                .get("enabled")
                .is_some_and(serde_json::Value::is_boolean)
            && value
                .get("name")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|name| name.chars().count() <= 160)
            && value
                .get("id")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|id| grant.allows_automation(family, id))
    };
    match method {
        "automation.list" => {
            result.as_object().is_some_and(|o| o.len() == 2)
                && ["events", "actions"].iter().all(|field| {
                    result
                        .get(field)
                        .and_then(serde_json::Value::as_array)
                        .is_some_and(|rows| {
                            rows.iter().all(|r| {
                                r.get("kind").and_then(serde_json::Value::as_str)
                                    == Some(if *field == "events" {
                                        "event"
                                    } else {
                                        "action"
                                    })
                                    && summary_allowed(r)
                            })
                        })
                })
        }
        "automation.get" => summary_allowed(result),
        "automation.source.get" => {
            grant.automation_delegation.as_ref().is_some_and(|p| {
                (p.allow_owner_source && p.allow_owner_edits)
                    || (p.allow_other_author_source && p.allow_other_author_edits)
            }) && grant
                .capabilities
                .contains(&RemoteCapability::AutomationEdit)
                && result.as_object().is_some_and(|o| {
                    o.len() == 4
                        && o.keys()
                            .all(|k| matches!(k.as_str(), "kind" | "id" | "revision" | "record"))
                })
                && result
                    .get("kind")
                    .and_then(serde_json::Value::as_str)
                    .and_then(AutomationFamily::from_scope)
                    .is_some_and(|family| {
                        result
                            .get("id")
                            .and_then(serde_json::Value::as_str)
                            .is_some_and(|id| {
                                grant.allows_automation(family, id)
                                    && result
                                        .pointer("/record/id")
                                        .and_then(serde_json::Value::as_str)
                                        == Some(id)
                            })
                    })
                && result
                    .get("revision")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|v| {
                        tiktools_core::collaboration::stream_sequence(v).is_ok_and(|n| n > 0)
                    })
        }
        "automation.create" | "automation.update" => {
            result.as_object().is_some_and(|o| {
                o.len() == 4
                    && o.keys()
                        .all(|k| matches!(k.as_str(), "kind" | "id" | "enabled" | "pendingReview"))
            }) && ((result.get("enabled") == Some(&serde_json::Value::Bool(false))
                && result.get("pendingReview") == Some(&serde_json::Value::Bool(true)))
                || (result.get("enabled") == Some(&serde_json::Value::Bool(true))
                    && result.get("pendingReview") == Some(&serde_json::Value::Bool(false))))
                && result
                    .get("kind")
                    .and_then(serde_json::Value::as_str)
                    .and_then(AutomationFamily::from_scope)
                    .is_some_and(|family| {
                        result
                            .get("id")
                            .and_then(serde_json::Value::as_str)
                            .is_some_and(|id| grant.allows_automation(family, id))
                    })
        }
        "automation.enable" => {
            result.as_object().is_some_and(|o| {
                o.len() == 3
                    && o.keys()
                        .all(|k| matches!(k.as_str(), "kind" | "id" | "enabled"))
            }) && result.get("enabled") == Some(&serde_json::Value::Bool(true))
                && result
                    .get("kind")
                    .and_then(serde_json::Value::as_str)
                    .and_then(AutomationFamily::from_scope)
                    .is_some_and(|family| {
                        result
                            .get("id")
                            .and_then(serde_json::Value::as_str)
                            .is_some_and(|id| grant.allows_automation(family, id))
                    })
        }
        "automation.disable" | "automation.delete" => {
            let family = if method == "automation.disable"
                && result.get("kind").and_then(serde_json::Value::as_str) == Some("action")
            {
                AutomationFamily::Action
            } else {
                AutomationFamily::Event
            };
            result.as_object().is_some_and(|o| {
                if family == AutomationFamily::Action {
                    o.len() == 3
                        && o.keys()
                            .all(|k| matches!(k.as_str(), "kind" | "id" | "enabled"))
                } else {
                    o.len() == 2
                }
            }) && result
                .get("id")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|id| grant.allows_automation(family, id))
                && if method == "automation.disable" {
                    result.get("enabled") == Some(&serde_json::Value::Bool(false))
                } else {
                    result
                        .get("deleted")
                        .is_some_and(serde_json::Value::is_boolean)
                }
        }
        _ => false,
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RemotePolicy {
    #[default]
    Deny,
    Authenticated,
    Capability(RemoteCapability),
}

impl RemotePolicy {
    pub(crate) fn authorize(self, session: &RemoteSession) -> Result<(), ApiError> {
        session.authorize(None)?;
        match self {
            Self::Deny => Err(forbidden()),
            Self::Authenticated => Ok(()),
            Self::Capability(capability) => session.authorize(Some(capability)),
        }
    }
}

pub(crate) fn redact_remote_failure(error: &mut crate::response::RpcErrorBody) {
    // Storage/implementation diagnostics can embed paths, trigger text or
    // provider values. Structured public domain failures keep their contract.
    if matches!(error.code.as_str(), "internal" | "storage") {
        error.message = "Host could not complete the operation".into();
        error.data = None;
    }
}

pub(crate) fn redact_remote_result(
    method: &str,
    mut result: serde_json::Value,
) -> serde_json::Value {
    // Transport/provider errors may contain request URLs or local paths.
    // A controller needs the failure state, not owner-only diagnostic text.
    if method == "live.monitor.status"
        && result
            .get("lastError")
            .is_some_and(|error| !error.is_null())
    {
        result["lastError"] = serde_json::json!("Host reported a LIVE connection error");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn remote_monitor_errors_keep_the_signal_without_provider_diagnostics() {
        let original = serde_json::json!({"uniqueId":"creator","phase":"error","lastError":"request failed: session_cookie=private-value at /private/path"});
        let redacted = redact_remote_result("live.monitor.status", original);
        assert_eq!(redacted["phase"], "error");
        assert!(redacted["lastError"].is_string());
        assert!(!redacted.to_string().contains("private-value"));
        assert!(!redacted.to_string().contains("/private/path"));
        assert_eq!(
            redact_remote_result("live.monitor.status", serde_json::json!({"lastError":null}))
                ["lastError"],
            serde_json::Value::Null
        );
    }
}
