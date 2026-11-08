//! A review must bind the displayed graph to its complete version envelope.
use super::*;
use std::collections::BTreeSet;
use tiktools_control_api::modules::automation::{
    AutomationReviewAuthority, AutomationReviewProvenance,
};

const MAX_DEPENDENCIES: usize = 512;
const DEFINITION_BYTES: usize = 256 * 1024;
const REVIEW_BYTES: usize = 4 * 1024 * 1024;

fn protocol() -> ClientError {
    ClientError::new("protocol", "invalid automation review snapshot")
}

fn decimal(value: &str) -> Result<i64, ClientError> {
    value
        .parse::<i64>()
        .ok()
        .filter(|number| *number >= 0 && number.to_string() == value)
        .ok_or_else(protocol)
}

fn canonical_id(value: &str, limit: usize) -> bool {
    !value.is_empty()
        && value.len() <= limit
        && value.trim() == value
        && !value.contains('*')
        && !value.chars().any(char::is_control)
}

fn entry(
    id: &str,
    record: Option<&Value>,
    provenance: Option<&AutomationReviewProvenance>,
    revision: &str,
) -> Result<(), ClientError> {
    let revision = decimal(revision)?;
    if revision == 0 {
        if record.is_some() || provenance.is_some() {
            return Err(protocol());
        }
        return Ok(());
    }
    let provenance = provenance.ok_or_else(protocol)?;
    if decimal(&provenance.generation)? == 0 {
        return Err(protocol());
    }
    if let AutomationReviewAuthority::Delegated {
        principal_id,
        session_id,
    } = &provenance.authority
    {
        if !canonical_id(principal_id, 256) || !canonical_id(session_id, 256) {
            return Err(protocol());
        }
    }
    if let Some(record) = record {
        if !record.is_object()
            || record.get("id").and_then(Value::as_str) != Some(id)
            || serde_json::to_vec(record).map_err(|_| protocol())?.len() > DEFINITION_BYTES
        {
            return Err(protocol());
        }
    }
    Ok(())
}

pub(crate) fn validate(
    response: &CallResult<AutomationReviewResult>,
    params: &AutomationGetParams,
) -> Result<(), ClientError> {
    let result = &response.result;
    let kind = match params
        .kind
        .as_deref()
        .unwrap_or("event")
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "event" | "events" => "event",
        "action" | "actions" => "action",
        _ => return Err(protocol()),
    };
    let id = params.id.trim();
    if !canonical_id(id, 128)
        || result.id != id
        || serde_json::to_value(result.kind).map_err(|_| protocol())? != kind
    {
        return Err(protocol());
    }
    let mut expected = Vec::new();
    if kind == "event" {
        if let Some(record) = &result.record {
            if let Some(ids) = record.get("actionIds") {
                let mut seen = BTreeSet::new();
                for id in ids.as_array().ok_or_else(protocol)? {
                    let id = id.as_str().ok_or_else(protocol)?;
                    if !canonical_id(id, 128) {
                        return Err(protocol());
                    }
                    if seen.insert(id) {
                        expected.push(id);
                    }
                    if expected.len() > MAX_DEPENDENCIES {
                        return Err(protocol());
                    }
                }
            }
        }
    }
    if result.dependencies.len() != expected.len()
        || result
            .dependencies
            .iter()
            .zip(&expected)
            .any(|(entry, id)| entry.id != *id)
    {
        return Err(protocol());
    }
    let meta = response.meta.as_ref().ok_or_else(protocol)?;
    if meta.command.is_some() || meta.resource_versions.len() != expected.len() + 1 {
        return Err(protocol());
    }
    if serde_json::to_vec(&serde_json::json!({
        "value": result,
        "resourceVersions": meta.resource_versions,
    }))
    .map_err(|_| protocol())?
    .len()
        > REVIEW_BYTES
    {
        return Err(protocol());
    }
    for (index, version) in meta.resource_versions.iter().enumerate() {
        let (scope, id, record, provenance) = if index == 0 {
            (
                kind,
                result.id.as_str(),
                result.record.as_ref(),
                result.provenance.as_ref(),
            )
        } else {
            let dependency = &result.dependencies[index - 1];
            (
                "action",
                dependency.id.as_str(),
                dependency.record.as_ref(),
                dependency.provenance.as_ref(),
            )
        };
        if version.scope != scope
            || version.id != id
            || serde_json::to_value(version.kind).map_err(|_| protocol())? != "automation"
        {
            return Err(protocol());
        }
        entry(id, record, provenance, &version.revision)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
