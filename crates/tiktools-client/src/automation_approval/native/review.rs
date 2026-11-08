//! Cross-bind the source displayed to an Owner with the reviewed manifest. A
//! version envelope alone must not attach one body to another body's digest.
use super::*;
use tiktools_control_api::modules::automation::{
    AutomationGetParams, AutomationManifestParams, AutomationReviewResult,
};
use tiktools_control_api::modules::collaboration::Grant;

const BYTES: usize = 4 * 1024 * 1024;

fn canonical_json(value: &Value, depth: usize) -> Result<Value, ClientError> {
    if depth > 64 {
        return Err(protocol());
    }
    Ok(match value {
        Value::Object(object) => {
            let sorted: std::collections::BTreeMap<_, _> = object.iter().collect();
            Value::Object(
                sorted
                    .into_iter()
                    .map(|(key, value)| Ok((key.clone(), canonical_json(value, depth + 1)?)))
                    .collect::<Result<_, ClientError>>()?,
            )
        }
        Value::Array(values) => Value::Array(
            values
                .iter()
                .map(|value| canonical_json(value, depth + 1))
                .collect::<Result<_, _>>()?,
        ),
        other => other.clone(),
    })
}
fn review_digest(domain: &str, value: &Value) -> Result<String, ClientError> {
    let encoded = serde_json::to_string(&canonical_json(value, 0)?).map_err(|_| protocol())?;
    if encoded.len() > BYTES {
        return Err(protocol());
    }
    let input = format!("tiktools/automation-manifest/v1/{domain}\0{encoded}");
    Ok(
        ring::digest::digest(&ring::digest::SHA256, input.as_bytes())
            .as_ref()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    )
}
fn source_pairs(source: &Value) -> Result<BTreeSet<(String, String)>, ClientError> {
    let mut pairs = BTreeSet::new();
    for provenance in std::iter::once(&source["provenance"]).chain(
        source["dependencies"]
            .as_array()
            .ok_or_else(protocol)?
            .iter()
            .map(|dependency| &dependency["provenance"]),
    ) {
        if provenance["authority"]["kind"] == "delegated" {
            pairs.insert((
                text(&provenance["authority"]["principalId"])?.into(),
                text(&provenance["authority"]["sessionId"])?.into(),
            ));
        }
    }
    Ok(pairs)
}
fn source_binding(source: &Value, binding: &Value) -> Result<(), ClientError> {
    if source["provenance"] != binding["provenance"] {
        return Err(protocol());
    }
    if source["record"].is_null() {
        if binding["exists"] != false || !binding["contentDigest"].is_null() {
            return Err(protocol());
        }
    } else if binding["exists"] != true
        || binding["contentDigest"] != review_digest("definition", &source["record"])?
    {
        return Err(protocol());
    }
    Ok(())
}

pub(super) fn validate(
    response: CallResult<Value>,
    params: &OwnerApprovalReviewParams,
) -> Result<CallResult<OwnerApprovalReviewResult>, ClientError> {
    if serde_json::to_vec(&response.result)
        .map_err(|_| protocol())?
        .len()
        > BYTES
    {
        return Err(protocol());
    }
    let result = exact(
        &response.result,
        &[
            "hostId",
            "hostInstanceId",
            "kind",
            "id",
            "source",
            "manifest",
            "subjects",
            "eligibility",
            "approval",
        ],
    )?;
    let kind = serde_json::to_value(params.kind).map_err(|_| protocol())?;
    if !canonical(text(&result["hostId"])?, 256)
        || !canonical(text(&result["hostInstanceId"])?, 256)
        || result["kind"] != kind
        || result["id"] != params.id
        || !canonical(&params.id, 128)
    {
        return Err(protocol());
    }
    let metadata = response.meta.as_ref().ok_or_else(protocol)?;
    if metadata.command.is_some()
        || metadata.host_instance_id.as_deref() != result["hostInstanceId"].as_str()
    {
        return Err(protocol());
    }
    let source = exact(
        &result["source"],
        &["kind", "id", "record", "provenance", "dependencies"],
    )?;
    if source["kind"] != kind || source["id"] != params.id {
        return Err(protocol());
    }
    for dependency in source["dependencies"].as_array().ok_or_else(protocol)? {
        exact(dependency, &["id", "record", "provenance"])?;
    }
    let source_typed: AutomationReviewResult =
        serde_json::from_value(result["source"].clone()).map_err(|_| protocol())?;
    crate::automation::validate_saved_review(
        &CallResult {
            result: source_typed,
            meta: response.meta.clone(),
        },
        &AutomationGetParams {
            kind: Some(text(&kind)?.into()),
            id: params.id.clone(),
        },
    )?;
    let pairs = source_pairs(&result["source"])?;
    let subjects = result["subjects"]
        .as_array()
        .filter(|subjects| subjects.len() <= 513)
        .ok_or_else(protocol)?;
    let eligibility = exact(&result["eligibility"], &["eligible", "reasonCode"])?;
    let eligible = eligibility["eligible"].as_bool().ok_or_else(protocol)?;
    let reason = if eligibility["reasonCode"].is_null() {
        None
    } else {
        Some(text(&eligibility["reasonCode"])?)
    };
    if eligible != reason.is_none()
        || reason.is_some_and(|reason| {
            !matches!(
                reason,
                "definition-missing"
                    | "no-delegated-subjects"
                    | "subjects-unavailable"
                    | "policy-unavailable"
            )
        })
    {
        return Err(protocol());
    }
    let manifest = &result["manifest"];
    if manifest.is_null() {
        if !subjects.is_empty()
            || eligible
            || if source["record"].is_null() {
                reason != Some("definition-missing")
            } else {
                reason != Some("subjects-unavailable") || pairs.is_empty()
            }
        {
            return Err(protocol());
        }
    } else {
        crate::automation::validate_manifest_snapshot(
            &CallResult {
                result: manifest.clone(),
                meta: response.meta.clone(),
            },
            &AutomationManifestParams {
                kind: params.kind,
                id: params.id.clone(),
            },
        )?;
        if manifest["hostId"] != result["hostId"] || subjects.len() != pairs.len() {
            return Err(protocol());
        }
        source_binding(&result["source"], &manifest["primary"])?;
        for (source, binding) in source["dependencies"]
            .as_array()
            .ok_or_else(protocol)?
            .iter()
            .zip(manifest["dependencies"].as_array().ok_or_else(protocol)?)
        {
            source_binding(source, binding)?;
        }
        let mut unsigned = manifest.clone();
        unsigned
            .as_object_mut()
            .ok_or_else(protocol)?
            .remove("manifestDigest");
        if manifest["manifestDigest"] != review_digest("manifest", &unsigned)? {
            return Err(protocol());
        }
        for ((subject, (principal, session)), binding) in subjects
            .iter()
            .zip(pairs.iter())
            .zip(manifest["subjects"].as_array().ok_or_else(protocol)?)
        {
            let subject = exact(
                subject,
                &[
                    "principalId",
                    "sessionId",
                    "grantRevision",
                    "configurationRevision",
                    "grant",
                ],
            )?;
            if text(&subject["principalId"])? != principal
                || text(&subject["sessionId"])? != session
            {
                return Err(protocol());
            }
            for key in [
                "principalId",
                "sessionId",
                "grantRevision",
                "configurationRevision",
            ] {
                if subject[key] != binding[key] {
                    return Err(protocol());
                }
            }
            decimal(text(&subject["grantRevision"])?, true)?;
            decimal(text(&subject["configurationRevision"])?, true)?;
            exact(
                &subject["grant"],
                if subject["grant"].get("effects").is_some() {
                    &[
                        "capabilities",
                        "resources",
                        "settingsFields",
                        "actions",
                        "maxPointsDelta",
                        "effects",
                    ]
                } else {
                    &[
                        "capabilities",
                        "resources",
                        "settingsFields",
                        "actions",
                        "maxPointsDelta",
                    ]
                },
            )?;
            let grant: Grant =
                serde_json::from_value(subject["grant"].clone()).map_err(|_| protocol())?;
            grant.validate().map_err(|_| protocol())?;
            if serde_json::to_value(grant).map_err(|_| protocol())? != subject["grant"] {
                return Err(protocol());
            }
        }
        if pairs.is_empty() {
            if eligible || reason != Some("no-delegated-subjects") {
                return Err(protocol());
            }
        } else if reason.is_some_and(|reason| reason != "policy-unavailable") {
            return Err(protocol());
        }
    }
    let approval = exact(
        &result["approval"],
        &[
            "state",
            "generation",
            "policyVersion",
            "manifestDigest",
            "current",
        ],
    )?;
    let current = approval["current"].as_bool().ok_or_else(protocol)?;
    match text(&approval["state"])? {
        "none" => {
            if current
                || !approval["generation"].is_null()
                || !approval["policyVersion"].is_null()
                || !approval["manifestDigest"].is_null()
            {
                return Err(protocol());
            }
        }
        state @ ("active" | "invalidated" | "revoked") => {
            decimal(text(&approval["generation"])?, true)?;
            if !approval["policyVersion"]
                .as_u64()
                .is_some_and(|version| version > 0 && version <= u64::from(u32::MAX))
                || !digest(text(&approval["manifestDigest"])?)
            {
                return Err(protocol());
            }
            if current
                && (!eligible
                    || state != "active"
                    || manifest.is_null()
                    || approval["manifestDigest"] != manifest["manifestDigest"]
                    || approval["policyVersion"] != manifest["policyVersion"])
            {
                return Err(protocol());
            }
        }
        _ => return Err(protocol()),
    }
    let typed = serde_json::from_value(response.result).map_err(|_| protocol())?;
    Ok(CallResult {
        result: typed,
        meta: response.meta,
    })
}

#[cfg(test)]
mod tests;
