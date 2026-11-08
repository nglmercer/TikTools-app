//! Owner manifest previews retain the exact complete version envelope. Digests
//! are opaque host bindings, never execution tokens or candidate reconstruction.
use super::*;
use std::collections::BTreeSet;

const BYTES: usize = 4 * 1024 * 1024;
const DEPENDENCIES: usize = 512;
fn protocol() -> ClientError {
    ClientError::new("protocol", "invalid automation execution manifest")
}
fn object<'a>(
    v: &'a Value,
    keys: &[&str],
) -> Result<&'a serde_json::Map<String, Value>, ClientError> {
    let o = v.as_object().ok_or_else(protocol)?;
    if o.len() != keys.len() || keys.iter().any(|key| !o.contains_key(*key)) {
        return Err(protocol());
    }
    Ok(o)
}
fn text(v: &Value) -> Result<&str, ClientError> {
    v.as_str().ok_or_else(protocol)
}
fn canonical(v: &str, max: usize) -> bool {
    !v.is_empty()
        && v.len() <= max
        && v.trim() == v
        && !v.contains('*')
        && !v.chars().any(char::is_control)
}
fn decimal(v: &Value, positive: bool) -> Result<i64, ClientError> {
    let raw = text(v)?;
    raw.parse::<i64>()
        .ok()
        .filter(|n| *n >= i64::from(positive) && n.to_string() == raw)
        .ok_or_else(protocol)
}
fn digest(v: &Value) -> bool {
    v.as_str().is_some_and(|s| {
        s.len() == 64
            && s.bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    })
}

fn binding(
    v: &Value,
    scope: &str,
    id: &str,
    revision: &str,
    subjects: &mut BTreeSet<(String, String)>,
) -> Result<(), ClientError> {
    let o = object(
        v,
        &[
            "kind",
            "id",
            "revision",
            "exists",
            "contentDigest",
            "provenance",
        ],
    )?;
    if text(&o["kind"])? != scope
        || text(&o["id"])? != id
        || text(&o["revision"])? != revision
        || !canonical(id, 128)
    {
        return Err(protocol());
    }
    let n = decimal(&o["revision"], false)?;
    let exists = o["exists"].as_bool().ok_or_else(protocol)?;
    if n == 0 {
        if exists || !o["contentDigest"].is_null() || !o["provenance"].is_null() {
            return Err(protocol());
        }
        return Ok(());
    }
    if exists {
        if !digest(&o["contentDigest"]) {
            return Err(protocol());
        }
    } else if !o["contentDigest"].is_null() {
        return Err(protocol());
    }
    let provenance = object(&o["provenance"], &["authority", "generation"])?;
    decimal(&provenance["generation"], true)?;
    match provenance["authority"].get("kind").and_then(Value::as_str) {
        Some("owner") => {
            object(&provenance["authority"], &["kind"])?;
        }
        Some("delegated") => {
            let authority = object(
                &provenance["authority"],
                &["kind", "principalId", "sessionId"],
            )?;
            let principal = text(&authority["principalId"])?;
            let session = text(&authority["sessionId"])?;
            if !canonical(principal, 256) || !canonical(session, 256) {
                return Err(protocol());
            }
            subjects.insert((principal.into(), session.into()));
        }
        _ => return Err(protocol()),
    }
    Ok(())
}

fn reason_list(v: &Value) -> Result<(), ClientError> {
    let rows = v
        .as_array()
        .filter(|rows| rows.len() <= 13)
        .ok_or_else(protocol)?;
    let mut seen = BTreeSet::new();
    for row in rows {
        let s = text(row)?;
        if !matches!(
            s,
            "approval-required"
                | "missing-dependency"
                | "missing-capability"
                | "resource-forbidden"
                | "live-restriction"
                | "invalid-configuration"
                | "dynamic-target"
                | "global-access-requires-policy"
                | "unclassifiable-render-field"
                | "opaque-script"
                | "effect-policy-unavailable"
                | "plugin-binding-unavailable"
                | "unknown-action-type"
        ) || !seen.insert(s)
        {
            return Err(protocol());
        }
    }
    Ok(())
}
fn effect(v: &Value, id: &str, exists: bool) -> Result<(), ClientError> {
    let o = object(
        v,
        &[
            "family",
            "id",
            "typeId",
            "kind",
            "enabled",
            "decision",
            "renderFields",
            "dynamicTarget",
            "requiredCapabilities",
            "reasons",
        ],
    )?;
    if o["family"] != "action"
        || text(&o["id"])? != id
        || !matches!(
            o["kind"].as_str(),
            Some(
                "log"
                    | "emit"
                    | "points"
                    | "delay"
                    | "script"
                    | "http"
                    | "audio"
                    | "plugin"
                    | "unknown"
                    | "missing-definition"
            )
        )
        || !matches!(
            o["decision"].as_str(),
            Some("classified" | "review-required" | "unsupported")
        )
        || (!o["enabled"].is_null() && !o["enabled"].is_boolean())
        || !o["dynamicTarget"].is_boolean()
    {
        return Err(protocol());
    }
    if (o["kind"] == "missing-definition") == exists
        || (!exists && (!o["typeId"].is_null() || !o["enabled"].is_null()))
    {
        return Err(protocol());
    }
    if !o["typeId"].is_null() && !canonical(text(&o["typeId"])?, 128) {
        return Err(protocol());
    }
    let fields = o["renderFields"]
        .as_array()
        .filter(|rows| rows.len() <= 256)
        .ok_or_else(protocol)?;
    let mut seen = BTreeSet::new();
    for field in fields {
        let name = text(field)?;
        if !canonical(name, 256) || !seen.insert(name) {
            return Err(protocol());
        }
    }
    let capabilities = o["requiredCapabilities"]
        .as_array()
        .filter(|rows| rows.len() <= 21)
        .ok_or_else(protocol)?;
    let mut seen = BTreeSet::new();
    for cap in capabilities {
        let name = text(cap)?;
        if !matches!(
            name,
            "live.read"
                | "points.read"
                | "points.adjust"
                | "rankings.read"
                | "rankings.manage"
                | "competitions.read"
                | "competitions.manage"
                | "workflows.read"
                | "workflows.edit"
                | "workflows.toggle"
                | "automation.read"
                | "automation.edit"
                | "automation.toggle"
                | "plugins.read"
                | "plugins.toggle"
                | "plugins.settings.read"
                | "plugins.settings.write"
                | "plugins.action.execute"
                | "widgets.read"
                | "widgets.configure"
                | "commands.read"
        ) || !seen.insert(name)
        {
            return Err(protocol());
        }
    }
    reason_list(&o["reasons"])
}

pub(crate) fn validate(
    response: &CallResult<Value>,
    params: &AutomationManifestParams,
) -> Result<(), ClientError> {
    if serde_json::to_vec(&response.result)
        .map_err(|_| protocol())?
        .len()
        > BYTES
    {
        return Err(protocol());
    }
    let result = object(
        &response.result,
        &[
            "policyVersion",
            "digestAlgorithm",
            "hostId",
            "subjects",
            "primary",
            "dependencies",
            "effects",
            "reasons",
            "executionAuthorized",
            "manifestDigest",
        ],
    )?;
    if result["policyVersion"] != 1
        || result["digestAlgorithm"] != "sha256-canonical-json-v1"
        || result["executionAuthorized"] != false
        || !canonical(text(&result["hostId"])?, 256)
        || !digest(&result["manifestDigest"])
    {
        return Err(protocol());
    }
    let kind = serde_json::to_value(params.kind).map_err(|_| protocol())?;
    let scope = text(&kind)?;
    let id = params.id.trim();
    if !canonical(id, 128) {
        return Err(protocol());
    }
    if result["primary"].get("exists") != Some(&Value::Bool(true)) {
        return Err(protocol());
    }
    let dependencies = result["dependencies"]
        .as_array()
        .filter(|rows| rows.len() <= DEPENDENCIES)
        .ok_or_else(protocol)?;
    if scope == "action" && !dependencies.is_empty() {
        return Err(protocol());
    }
    let meta = response.meta.as_ref().ok_or_else(protocol)?;
    if meta.command.is_some() || meta.resource_versions.len() != dependencies.len() + 1 {
        return Err(protocol());
    }
    let mut subjects = BTreeSet::new();
    let mut seen = BTreeSet::new();
    let mut action_ids = Vec::new();
    for (index, version) in meta.resource_versions.iter().enumerate() {
        let (entry, expected_scope, expected_id) = if index == 0 {
            (&result["primary"], scope, id)
        } else {
            let v = &dependencies[index - 1];
            (v, "action", text(v.get("id").ok_or_else(protocol)?)?)
        };
        if serde_json::to_value(version.kind).map_err(|_| protocol())? != "automation"
            || version.scope != expected_scope
            || version.id != expected_id
            || !seen.insert((expected_scope, expected_id))
        {
            return Err(protocol());
        }
        binding(
            entry,
            expected_scope,
            expected_id,
            &version.revision,
            &mut subjects,
        )?;
        if expected_scope == "action" {
            action_ids.push((
                expected_id,
                entry
                    .get("exists")
                    .and_then(Value::as_bool)
                    .ok_or_else(protocol)?,
            ));
        }
    }
    let rows = result["subjects"]
        .as_array()
        .filter(|rows| rows.len() <= DEPENDENCIES + 1)
        .ok_or_else(protocol)?;
    if rows.len() != subjects.len() {
        return Err(protocol());
    }
    for (row, (principal, session)) in rows.iter().zip(subjects) {
        let o = object(
            row,
            &[
                "principalId",
                "sessionId",
                "grantRevision",
                "configurationRevision",
            ],
        )?;
        if text(&o["principalId"])? != principal || text(&o["sessionId"])? != session {
            return Err(protocol());
        }
        decimal(&o["grantRevision"], true)?;
        decimal(&o["configurationRevision"], true)?;
    }
    let effects = result["effects"].as_array().ok_or_else(protocol)?;
    if effects.len() != action_ids.len() {
        return Err(protocol());
    }
    for (entry, (id, exists)) in effects.iter().zip(action_ids) {
        effect(entry, id, exists)?;
    }
    reason_list(&result["reasons"])
}

#[cfg(test)]
mod tests;
