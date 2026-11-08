//! Owner approval receipts describe historical commits. They cannot create a
//! runtime permit or transfer delegated definitions into Owner authority.
use crate::{CallResult, ClientError, RequestMetadata, TikToolsClient};
use serde_json::Value;
use std::collections::BTreeSet;
mod native;
pub use native::*;
pub use tiktools_control_api::modules::automation_approval::{
    ApprovalActivateParams, ApprovalActivateReceipt, ApprovalEnablement, ApprovalRevokeParams,
    ApprovalRevokeReceipt,
};

pub(crate) const METHODS: &[&str] = &[
    ACTIVATE,
    REVOKE,
    "automation.approval.review",
    "automation.approval.get",
    "automation.approval.submit",
    "automation.approval.resolve",
    "automation.approval.review_unknown",
];
const ACTIVATE: &str = "automation.approval.activate";
const REVOKE: &str = "automation.approval.revoke";

fn protocol() -> ClientError {
    ClientError::new("protocol", "invalid automation approval receipt")
}
fn canonical(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value.trim() == value
        && !value.contains('*')
        && !value.chars().any(char::is_control)
}
fn decimal(value: &str, positive: bool) -> Result<i64, ClientError> {
    value
        .parse::<i64>()
        .ok()
        .filter(|n| *n >= i64::from(positive) && n.to_string() == value)
        .ok_or_else(protocol)
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
fn scope(kind: impl serde::Serialize) -> Result<String, ClientError> {
    serde_json::to_value(kind)
        .map_err(|_| protocol())?
        .as_str()
        .filter(|kind| matches!(*kind, "event" | "action"))
        .map(str::to_owned)
        .ok_or_else(protocol)
}
fn is_automation(kind: impl serde::Serialize) -> bool {
    serde_json::to_value(kind).ok() == Some(Value::String("automation".into()))
}
fn envelope<'a, T>(
    response: &'a CallResult<T>,
    kind: &str,
    id: &str,
    metadata: &RequestMetadata,
) -> Result<&'a tiktools_control_api::ResponseMetadata, ClientError> {
    if !canonical(id, 128) {
        return Err(protocol());
    }
    let meta = response.meta.as_ref().ok_or_else(protocol)?;
    let command = meta.command.as_ref().ok_or_else(protocol)?;
    let resource = command.resource.as_ref().ok_or_else(protocol)?;
    if command.command_id != metadata.command_id.as_deref().ok_or_else(protocol)?
        || serde_json::to_value(command.state).map_err(|_| protocol())? != "succeeded"
        || command.committed_at.is_none_or(|time| time <= 0)
        || !is_automation(resource.kind)
        || resource.scope != kind
        || resource.id != id
        || meta.host_instance_id.as_deref()
            != Some(metadata.host_instance_id.as_deref().ok_or_else(protocol)?)
    {
        return Err(protocol());
    }
    let primary = meta.resource_versions.first().ok_or_else(protocol)?;
    if !is_automation(primary.kind)
        || primary.scope != kind
        || primary.id != id
        || decimal(&primary.revision, true)?
            != decimal(
                metadata.expected_revision.as_deref().ok_or_else(protocol)?,
                true,
            )?
            .checked_add(1)
            .ok_or_else(protocol)?
    {
        return Err(protocol());
    }
    Ok(meta)
}

fn validate_activation(
    response: &CallResult<ApprovalActivateReceipt>,
    params: &ApprovalActivateParams,
    metadata: &RequestMetadata,
) -> Result<(), ClientError> {
    let result = &response.result;
    let kind = scope(params.kind)?;
    if result.kind != params.kind
        || result.id != params.id
        || !result.enabled
        || !result.approved
        || result.policy_version != 1
        || !digest(&result.manifest_digest)
        || !params.acknowledge_effects
    {
        return Err(protocol());
    }
    decimal(&result.approval_generation, true)?;
    let meta = envelope(response, &kind, &params.id, metadata)?;
    if params.reviewed_versions.is_empty()
        || params.reviewed_versions.len() > 513
        || meta.resource_versions.len() != params.reviewed_versions.len()
        || params.enable_plan.is_empty()
        || params.enable_plan.len() > 513
    {
        return Err(protocol());
    }
    let mut plan = BTreeSet::new();
    let mut primary_enabled = false;
    for entry in &params.enable_plan {
        let family = scope(entry.kind)?;
        if !canonical(&entry.id, 128) || !plan.insert((family.clone(), entry.id.as_str())) {
            return Err(protocol());
        }
        if family == kind && entry.id == params.id {
            primary_enabled = entry.enabled;
        }
    }
    if !primary_enabled {
        return Err(protocol());
    }
    let mut seen = BTreeSet::new();
    for (index, (final_version, reviewed)) in meta
        .resource_versions
        .iter()
        .zip(&params.reviewed_versions)
        .enumerate()
    {
        let expected_scope = if index == 0 { kind.as_str() } else { "action" };
        if !is_automation(reviewed.kind)
            || !is_automation(final_version.kind)
            || reviewed.scope != expected_scope
            || final_version.scope != reviewed.scope
            || final_version.id != reviewed.id
            || !canonical(&reviewed.id, 128)
            || !seen.insert((reviewed.scope.as_str(), reviewed.id.as_str()))
            || (index == 0 && reviewed.id != params.id)
            || (kind == "action" && index > 0)
        {
            return Err(protocol());
        }
        let before = decimal(&reviewed.revision, true)?;
        let after = decimal(&final_version.revision, true)?;
        if (index == 0
            && (reviewed.revision != metadata.expected_revision.as_deref().ok_or_else(protocol)?
                || after != before.checked_add(1).ok_or_else(protocol)?))
            || (index > 0
                && (after < before
                    || after > before.checked_add(1).ok_or_else(protocol)?
                    || (after != before
                        && !plan.contains(&(reviewed.scope.clone(), reviewed.id.as_str())))))
        {
            return Err(protocol());
        }
    }
    if plan
        .iter()
        .any(|(family, id)| !seen.contains(&(family.as_str(), *id)))
    {
        return Err(protocol());
    }
    Ok(())
}

fn validate_revocation(
    response: &CallResult<ApprovalRevokeReceipt>,
    params: &ApprovalRevokeParams,
    metadata: &RequestMetadata,
) -> Result<(), ClientError> {
    let result = &response.result;
    if result.kind != params.kind
        || result.id != params.id
        || !result.revoked
        || !params.acknowledge_revocation
    {
        return Err(protocol());
    }
    decimal(&result.approval_generation, true)?;
    let meta = envelope(response, &scope(params.kind)?, &params.id, metadata)?;
    if meta.resource_versions.len() != 1 {
        return Err(protocol());
    }
    Ok(())
}

impl TikToolsClient {
    /// Approves the exact saved review and explicit enable plan atomically.
    /// The returned final digest is a historical receipt, never authorization.
    pub async fn automation_approval_activate(
        &self,
        params: ApprovalActivateParams,
        metadata: RequestMetadata,
    ) -> Result<CallResult<ApprovalActivateReceipt>, ClientError> {
        let response = self
            .call_with_meta(ACTIVATE, &params, Some(metadata.clone()))
            .await?;
        validate_activation(&response, &params, &metadata)?;
        Ok(response)
    }

    /// Durably revokes future delegated work under the primary revision fence.
    pub async fn automation_approval_revoke(
        &self,
        params: ApprovalRevokeParams,
        metadata: RequestMetadata,
    ) -> Result<CallResult<ApprovalRevokeReceipt>, ClientError> {
        let response = self
            .call_with_meta(REVOKE, &params, Some(metadata.clone()))
            .await?;
        validate_revocation(&response, &params, &metadata)?;
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn metadata() -> RequestMetadata {
        RequestMetadata {
            command_id: Some("approval-original".into()),
            host_instance_id: Some("instance-original".into()),
            expected_revision: Some("4".into()),
        }
    }
    fn params() -> ApprovalActivateParams {
        serde_json::from_value(json!({
            "kind":"event","id":"draft","manifestDigest":"a".repeat(64),
            "reviewedVersions":[
                {"kind":"automation","scope":"event","id":"draft","revision":"4"},
                {"kind":"automation","scope":"action","id":"draft","revision":"2"},
                {"kind":"automation","scope":"action","id":"inactive","revision":"3"}
            ],
            "enablePlan":[{"kind":"event","id":"draft","enabled":true},
                {"kind":"action","id":"draft","enabled":true}],
            "acknowledgeEffects":true
        }))
        .unwrap()
    }
    fn activation() -> CallResult<ApprovalActivateReceipt> {
        CallResult {
            result: serde_json::from_value(json!({
                "kind":"event","id":"draft","enabled":true,"approved":true,
                "policyVersion":1,"manifestDigest":"b".repeat(64),"approvalGeneration":"7"
            }))
            .unwrap(),
            meta: Some(serde_json::from_value(json!({
                "hostInstanceId":"instance-original",
                "command":{"commandId":"approval-original","state":"succeeded",
                    "committedAt":42,"resource":{"kind":"automation","scope":"event","id":"draft"}},
                "resourceVersions":[
                    {"kind":"automation","scope":"event","id":"draft","revision":"5"},
                    {"kind":"automation","scope":"action","id":"draft","revision":"3"},
                    {"kind":"automation","scope":"action","id":"inactive","revision":"3"}
                ]
            })).unwrap()),
        }
    }

    #[test]
    fn receipt_binds_complete_post_transition_versions_and_final_digest() {
        let response = activation();
        let request = params();
        assert_ne!(response.result.manifest_digest, request.manifest_digest);
        assert!(validate_activation(&response, &request, &metadata()).is_ok());
        let mut incomplete = response.clone();
        incomplete.meta.as_mut().unwrap().resource_versions.pop();
        assert!(validate_activation(&incomplete, &request, &metadata()).is_err());
        let mut reordered = response;
        reordered
            .meta
            .as_mut()
            .unwrap()
            .resource_versions
            .swap(1, 2);
        assert!(validate_activation(&reordered, &request, &metadata()).is_err());
    }

    #[test]
    fn receipt_rejects_unplanned_or_repeated_definition_advancement() {
        let request = params();
        for (index, revision) in [(0, "4"), (0, "6"), (1, "4"), (2, "4")] {
            let mut response = activation();
            response.meta.as_mut().unwrap().resource_versions[index].revision = revision.into();
            assert!(validate_activation(&response, &request, &metadata()).is_err());
        }
        let mut duplicate = request.clone();
        duplicate.enable_plan.push(duplicate.enable_plan[0].clone());
        assert!(validate_activation(&activation(), &duplicate, &metadata()).is_err());
        let mut missing_primary = request;
        missing_primary.enable_plan.remove(0);
        assert!(validate_activation(&activation(), &missing_primary, &metadata()).is_err());
    }

    #[test]
    fn receipt_rejects_boolean_digest_generation_and_command_confusion() {
        let request = params();
        let mut cases = Vec::new();
        let mut response = activation();
        response.result.enabled = false;
        cases.push(response);
        let mut response = activation();
        response.result.approved = false;
        cases.push(response);
        let mut response = activation();
        response.result.policy_version = 2;
        cases.push(response);
        let mut response = activation();
        response.result.manifest_digest = "B".repeat(64);
        cases.push(response);
        let mut response = activation();
        response.result.approval_generation = "07".into();
        cases.push(response);
        let mut response = activation();
        response
            .meta
            .as_mut()
            .unwrap()
            .command
            .as_mut()
            .unwrap()
            .command_id = "new-command".into();
        cases.push(response);
        let mut response = activation();
        response.meta.as_mut().unwrap().host_instance_id = Some("instance-current".into());
        cases.push(response);
        for response in cases {
            assert!(validate_activation(&response, &request, &metadata()).is_err());
        }
        // A later current instance does not alter a retained historical receipt.
        assert!(validate_activation(&activation(), &request, &metadata()).is_ok());
    }

    #[test]
    fn revocation_requires_a_primary_generation_fence_and_original_command() {
        let request: ApprovalRevokeParams = serde_json::from_value(json!({
            "kind":"event","id":"draft","acknowledgeRevocation":true
        }))
        .unwrap();
        let activation = activation();
        let mut response = CallResult {
            result: ApprovalRevokeReceipt {
                kind: request.kind,
                id: request.id.clone(),
                revoked: true,
                approval_generation: "8".into(),
            },
            meta: activation.meta,
        };
        response
            .meta
            .as_mut()
            .unwrap()
            .resource_versions
            .truncate(1);
        assert!(validate_revocation(&response, &request, &metadata()).is_ok());
        response.result.revoked = false;
        assert!(validate_revocation(&response, &request, &metadata()).is_err());
        response.result.revoked = true;
        response.meta.as_mut().unwrap().resource_versions[0].revision = "4".into();
        assert!(validate_revocation(&response, &request, &metadata()).is_err());
    }

    #[test]
    fn receipt_shapes_reject_additional_authority_or_source_data() {
        let original = serde_json::to_value(activation().result).unwrap();
        for field in [
            "record",
            "source",
            "authority",
            "provenance",
            "grant",
            "permit",
        ] {
            let mut extra = original.clone();
            extra[field] = json!({});
            assert!(serde_json::from_value::<ApprovalActivateReceipt>(extra).is_err());
        }
    }
}
