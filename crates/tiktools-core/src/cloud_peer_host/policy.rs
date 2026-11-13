//! Closed Cloud-to-native policy conversion, shared by confirmation and import.
use crate::{cloud_teams::PeerAuthorization, control::OperationError};
use tiktools_collaboration::{Capability, ClientKind, Grant};

pub(super) fn grant(item: &PeerAuthorization) -> Result<(Grant, ClientKind), OperationError> {
    let kind = match item.policy.get("nativeClientKind") {
        None => ClientKind::Desktop,
        Some(value) => match value.as_str() {
            Some("desktop") => ClientKind::Desktop,
            Some("agent") => ClientKind::Agent,
            _ => return Err(OperationError::invalid("unsupported native client kind")),
        },
    };
    if let Some(native) = item.policy.get("nativeGrant") {
        if item.role != "custom" {
            return Err(OperationError::invalid(
                "an explicit native grant requires the custom role",
            ));
        }
        let grant: Grant = serde_json::from_value(native.clone())
            .map_err(|_| OperationError::invalid("invalid native collaboration grant"))?;
        grant
            .validate()
            .map_err(|_| OperationError::invalid("invalid native collaboration grant"))?;
        return Ok((grant, kind));
    }
    let mut grant = Grant::default();
    // Legacy operations map only when a capability cannot grant extra writes.
    // Full authoring/effects require the explicit nativeGrant policy above.
    let reads = [
        ("live.status", Capability::LiveRead),
        ("workflow.list", Capability::WorkflowsRead),
        ("points.read", Capability::PointsRead),
        ("competition.list", Capability::CompetitionsRead),
        ("automation.list", Capability::AutomationRead),
        ("command.outcome", Capability::CommandsRead),
    ];
    let writes = [
        ("points.adjust", Capability::PointsAdjust),
        ("workflow.disable", Capability::WorkflowsToggle),
        ("automation.toggle", Capability::AutomationToggle),
        ("competition.control", Capability::CompetitionsManage),
    ];
    match item.role.as_str() {
        "observer" | "moderator" | "automationEditor" => {
            grant
                .capabilities
                .extend(reads.into_iter().map(|(_, cap)| cap));
            if item.role == "moderator" {
                grant
                    .capabilities
                    .extend(writes.into_iter().map(|(_, cap)| cap));
            }
            if item.role == "automationEditor" {
                grant.capabilities.insert(Capability::AutomationEdit);
            }
        }
        "custom" => {
            let operations = item
                .policy
                .get("allowedOperations")
                .and_then(|v| v.as_array())
                .ok_or_else(|| {
                    OperationError::invalid("custom peer policy needs allowed operations")
                })?;
            for (name, cap) in reads.into_iter().chain(writes) {
                if operations.iter().any(|v| v.as_str() == Some(name)) {
                    grant.capabilities.insert(cap);
                }
            }
            if operations
                .iter()
                .any(|v| v.as_str() == Some("automation.draft.submit"))
            {
                grant.capabilities.insert(Capability::AutomationEdit);
            }
        }
        _ => return Err(OperationError::invalid("unsupported peer role")),
    }
    grant
        .validate()
        .map_err(|_| OperationError::invalid("invalid native collaboration grant"))?;
    Ok((grant, kind))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn item(role: &str, policy: serde_json::Value) -> PeerAuthorization {
        PeerAuthorization {
            id: uuid::Uuid::new_v4().to_string(),
            device_id: uuid::Uuid::new_v4().to_string(),
            device_label: "Moderator".into(),
            device_status: "active".into(),
            role: role.into(),
            policy,
            status: "active".into(),
            revision: 1,
            confirmed_revision: Some(1),
            endpoint_id: Some("ab".repeat(32)),
        }
    }
    #[test]
    fn presets_never_infer_effects_or_reusable_automation_authority() {
        for role in ["observer", "moderator", "automationEditor"] {
            let (grant, kind) = grant(&item(role, json!({}))).unwrap();
            assert_eq!(kind, ClientKind::Desktop);
            assert!(grant.effects.is_empty());
            assert!(grant.automation_delegation.is_none());
            assert!(!grant.capabilities.contains(&Capability::ScriptsExecute));
        }
        let (moderator, _) =
            grant(&item("moderator", json!({"nativeClientKind":"agent"}))).unwrap();
        assert!(moderator.capabilities.contains(&Capability::PointsAdjust));
        assert!(!moderator.capabilities.contains(&Capability::AutomationEdit));
    }
    #[test]
    fn explicit_native_authority_preserves_scopes_and_rejects_unknown_or_implicit_authority() {
        let policy = json!({"allowedOperations":[],"nativeClientKind":"agent","nativeGrant":{"capabilities":["points.read","points.adjust"],"resources":{"viewer":["alice"]},"maxPointsDelta":5.0}});
        let (native, kind) = grant(&item("custom", policy.clone())).unwrap();
        assert_eq!(kind, ClientKind::Agent);
        assert!(native.allows_resource(tiktools_collaboration::ResourceKind::Viewer, "alice"));
        assert!(!native.allows_resource(tiktools_collaboration::ResourceKind::Viewer, "bob"));
        assert_eq!(native.max_points_delta, Some(5.0));
        assert!(grant(&item("observer", policy)).is_err());
        for policy in [
            json!({"nativeClientKind":true}),
            json!({"nativeGrant":{"capabilities":["owner"]}}),
            json!({"nativeGrant":{"capabilities":[],"unexpected":true}}),
        ] {
            assert!(grant(&item("custom", policy)).is_err());
        }
    }
}
