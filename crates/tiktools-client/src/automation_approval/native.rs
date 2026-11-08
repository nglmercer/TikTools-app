//! Host-owned approval recovery. Raw closed schemas are checked before typed
//! decoding, so missing nullable fields cannot silently become `None`.
use super::{canonical, decimal, digest, protocol};
use crate::{CallResult, ClientError, TikToolsClient};
use serde_json::Value;
use std::collections::BTreeSet;
use tiktools_control_api::modules::automation_approval::OwnerApprovalEmpty;
pub use tiktools_control_api::modules::automation_approval::{
    OwnerApprovalBlockReason, OwnerApprovalEligibility, OwnerApprovalIntent,
    OwnerApprovalOperation, OwnerApprovalOutcome, OwnerApprovalReceipt, OwnerApprovalRecord,
    OwnerApprovalReviewParams, OwnerApprovalReviewResult, OwnerApprovalReviewUnknown,
    OwnerApprovalState, OwnerApprovalStatus, OwnerApprovalStatusKind, OwnerApprovalSubject,
    OwnerApprovalSubmit,
};

mod review;

fn exact<'a>(
    value: &'a Value,
    keys: &[&str],
) -> Result<&'a serde_json::Map<String, Value>, ClientError> {
    let object = value.as_object().ok_or_else(protocol)?;
    if object.len() != keys.len() || keys.iter().any(|key| !object.contains_key(*key)) {
        return Err(protocol());
    }
    Ok(object)
}
fn text(value: &Value) -> Result<&str, ClientError> {
    value.as_str().ok_or_else(protocol)
}
fn timestamp(value: &Value) -> Result<(), ClientError> {
    if value
        .as_u64()
        .is_some_and(|value| value <= 9_007_199_254_740_991)
    {
        Ok(())
    } else {
        Err(protocol())
    }
}
fn command_id(value: &str) -> bool {
    value.strip_prefix("owner-approval-").is_some_and(|suffix| {
        suffix.len() == 32
            && suffix
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}
fn intent(value: &Value) -> Result<&str, ClientError> {
    let object = exact(
        value,
        &[
            "commandId",
            "operation",
            "kind",
            "id",
            "expectedRevision",
            "hostInstanceId",
            "createdAt",
        ],
    )?;
    let id = text(&object["commandId"])?;
    if !command_id(id)
        || !matches!(text(&object["operation"])?, "activate" | "revoke")
        || !matches!(text(&object["kind"])?, "action" | "event")
        || !canonical(text(&object["id"])?, 128)
        || !canonical(text(&object["hostInstanceId"])?, 256)
    {
        return Err(protocol());
    }
    decimal(text(&object["expectedRevision"])?, true)?;
    timestamp(&object["createdAt"])?;
    Ok(id)
}

fn validate_state(response: &CallResult<Value>) -> Result<OwnerApprovalState, ClientError> {
    let metadata = response.meta.as_ref().ok_or_else(protocol)?;
    if !metadata.resource_versions.is_empty()
        || metadata.command.is_some()
        || metadata.host_instance_id.is_some()
        || serde_json::to_vec(&response.result)
            .map_err(|_| protocol())?
            .len()
            > 256 * 1024
    {
        return Err(protocol());
    }
    let state = exact(
        &response.result,
        &["hostId", "hostInstanceId", "pending", "recent"],
    )?;
    if !canonical(text(&state["hostId"])?, 256) || !canonical(text(&state["hostInstanceId"])?, 256)
    {
        return Err(protocol());
    }
    let mut ids = BTreeSet::new();
    if !state["pending"].is_null() {
        ids.insert(intent(&state["pending"])?);
    }
    let recent = state["recent"]
        .as_array()
        .filter(|recent| recent.len() <= 16)
        .ok_or_else(protocol)?;
    for entry in recent {
        let row = exact(
            entry,
            &["intent", "outcome", "receipt", "errorCode", "recordedAt"],
        )?;
        if !ids.insert(intent(&row["intent"])?) {
            return Err(protocol());
        }
        timestamp(&row["recordedAt"])?;
        match text(&row["outcome"])? {
            "succeeded" => {
                if !row["errorCode"].is_null() {
                    return Err(protocol());
                }
                let receipt = exact(
                    &row["receipt"],
                    &[
                        "revision",
                        "approvalGeneration",
                        "policyVersion",
                        "manifestDigest",
                    ],
                )?;
                if decimal(text(&receipt["revision"])?, true)?
                    != decimal(text(&row["intent"]["expectedRevision"])?, true)?
                        .checked_add(1)
                        .ok_or_else(protocol)?
                {
                    return Err(protocol());
                }
                decimal(text(&receipt["approvalGeneration"])?, true)?;
                if row["intent"]["operation"] == "activate" {
                    if receipt["policyVersion"] != 1 || !digest(text(&receipt["manifestDigest"])?) {
                        return Err(protocol());
                    }
                } else if !receipt["policyVersion"].is_null()
                    || !receipt["manifestDigest"].is_null()
                {
                    return Err(protocol());
                }
            }
            "rejected" => {
                let code = text(&row["errorCode"])?;
                if !row["receipt"].is_null()
                    || !matches!(
                        code,
                        "revision_conflict"
                            | "not_found"
                            | "invalid_params"
                            | "forbidden"
                            | "capability_unavailable"
                            | "host_restarted"
                            | "too_large"
                    )
                {
                    return Err(protocol());
                }
            }
            "reviewed-unknown" => {
                if !row["receipt"].is_null() || !row["errorCode"].is_null() {
                    return Err(protocol());
                }
            }
            _ => return Err(protocol()),
        }
    }
    serde_json::from_value(response.result.clone()).map_err(|_| protocol())
}

fn validate_submission(
    response: &CallResult<Value>,
    params: &OwnerApprovalSubmit,
) -> Result<OwnerApprovalState, ClientError> {
    let state = validate_state(response)?;
    let input = serde_json::to_value(params).map_err(|_| protocol())?;
    if input["operation"] == "activate" && input["acknowledgeEffects"] != true
        || input["operation"] == "revoke" && input["acknowledgeRevocation"] != true
    {
        return Err(protocol());
    }
    if response.result["hostId"] != input["hostId"]
        || response.result["hostInstanceId"] != input["hostInstanceId"]
    {
        return Err(protocol());
    }
    let matches = |candidate: &Value| {
        [
            "operation",
            "kind",
            "id",
            "expectedRevision",
            "hostInstanceId",
        ]
        .iter()
        .all(|key| candidate[*key] == input[*key])
    };
    if !matches(&response.result["pending"])
        && !response.result["recent"]
            .as_array()
            .ok_or_else(protocol)?
            .iter()
            .any(|entry| matches(&entry["intent"]))
    {
        return Err(protocol());
    }
    Ok(state)
}

impl TikToolsClient {
    /// One coherent Owner source/effect/grant review. It never supplies a permit.
    pub async fn automation_approval_review(
        &self,
        params: OwnerApprovalReviewParams,
    ) -> Result<CallResult<OwnerApprovalReviewResult>, ClientError> {
        let response = self
            .call_with_meta("automation.approval.review", &params, None)
            .await?;
        review::validate(response, &params)
    }
    pub async fn automation_approval_get(&self) -> Result<OwnerApprovalState, ClientError> {
        let response = self
            .call_with_meta("automation.approval.get", OwnerApprovalEmpty {}, None)
            .await?;
        validate_state(&response)
    }
    pub async fn automation_approval_submit(
        &self,
        params: OwnerApprovalSubmit,
    ) -> Result<OwnerApprovalState, ClientError> {
        let response = self
            .call_with_meta("automation.approval.submit", &params, None)
            .await?;
        validate_submission(&response, &params)
    }
    /// Resolves the original native identity only; it never sends again.
    pub async fn automation_approval_resolve(&self) -> Result<OwnerApprovalState, ClientError> {
        let response = self
            .call_with_meta("automation.approval.resolve", OwnerApprovalEmpty {}, None)
            .await?;
        validate_state(&response)
    }
    pub async fn automation_approval_review_unknown(
        &self,
        params: OwnerApprovalReviewUnknown,
    ) -> Result<OwnerApprovalState, ClientError> {
        let response = self
            .call_with_meta("automation.approval.review_unknown", &params, None)
            .await?;
        let state = validate_state(&response)?;
        if !params.acknowledge_unknown
            || !command_id(&params.command_id)
            || response.result["pending"]["commandId"] == params.command_id
            || !response.result["recent"]
                .as_array()
                .ok_or_else(protocol)?
                .iter()
                .any(|entry| entry["intent"]["commandId"] == params.command_id)
        {
            return Err(protocol());
        }
        Ok(state)
    }
}

#[cfg(test)]
mod tests;
