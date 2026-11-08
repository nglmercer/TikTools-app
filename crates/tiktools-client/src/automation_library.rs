//! Coherent Owner library reads and host-owned, non-replaying transactions.
use crate::{CallResult, ClientError, TikToolsClient};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use tiktools_control_api::modules::automation_library::{
    validate_owner_library_submit, OwnerLibraryEmpty,
};
pub use tiktools_control_api::modules::automation_library::{
    OwnerLibraryAcknowledgement, OwnerLibraryCatalog, OwnerLibraryEntry, OwnerLibraryIntent,
    OwnerLibraryMutation, OwnerLibraryOperation, OwnerLibraryOutcome, OwnerLibraryPack,
    OwnerLibraryProfile, OwnerLibraryReceipt, OwnerLibraryRecord, OwnerLibraryReviewUnknown,
    OwnerLibrarySnapshot, OwnerLibraryState, OwnerLibrarySubmission, OwnerLibrarySubmit,
};

pub(crate) const METHODS: &[&str] = &[
    "automation.library.snapshot",
    "automation.library.get",
    "automation.library.submit",
    "automation.library.resolve",
    "automation.library.review_unknown",
];
const BYTES: usize = 8 * 1024 * 1024;
const STATE_BYTES: usize = 256 * 1024;
const CATALOG_BYTES: usize = 4 * 1024 * 1024;
const RECORD_BYTES: usize = 256 * 1024;

fn protocol() -> ClientError {
    ClientError::new("protocol", "invalid native owner library response")
}
fn exact<'a>(
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
fn identifier(v: &str) -> bool {
    !v.is_empty()
        && v.len() <= 128
        && v.trim() == v
        && !v.contains('*')
        && !v.chars().any(char::is_control)
}
fn profile_id(v: &str) -> bool {
    !v.is_empty() && v.chars().count() <= 64 && v.trim() == v && !v.chars().any(char::is_control)
}
fn revision(v: &Value) -> Result<i64, ClientError> {
    let s = text(v)?;
    s.parse::<i64>()
        .ok()
        .filter(|n| *n >= 0 && n.to_string() == s)
        .ok_or_else(protocol)
}
fn hash(v: &Value) -> Result<&str, ClientError> {
    let s = text(v)?;
    if s.len() != 64
        || !s
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(protocol());
    }
    Ok(s)
}
fn command_id(v: &str) -> bool {
    v.strip_prefix("owner-library-").is_some_and(|s| {
        s.len() == 32
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}
fn timestamp(v: &Value, positive: bool) -> Result<(), ClientError> {
    v.as_u64()
        .filter(|n| *n >= u64::from(positive) && *n <= 9_007_199_254_740_991)
        .map(|_| ())
        .ok_or_else(protocol)
}
fn within(v: &Value, max: usize) -> Result<(), ClientError> {
    if serde_json::to_vec(v).map_err(|_| protocol())?.len() > max {
        return Err(protocol());
    }
    Ok(())
}
fn source(v: &Value) -> Result<(), ClientError> {
    bounded_source(v, RECORD_BYTES)
}
fn bounded_source(v: &Value, max: usize) -> Result<(), ClientError> {
    let mut stack = vec![(v, 0usize)];
    let mut nodes = 0usize;
    while let Some((node, depth)) = stack.pop() {
        nodes += 1;
        if depth > 64 || nodes > 65_536 {
            return Err(protocol());
        }
        match node {
            Value::Object(values) => stack.extend(values.values().map(|v| (v, depth + 1))),
            Value::Array(values) => stack.extend(values.iter().map(|v| (v, depth + 1))),
            _ => {}
        }
        if nodes + stack.len() > 65_536 {
            return Err(protocol());
        }
    }
    within(v, max)?;
    Ok(())
}
fn canonical(v: &Value, output: &mut String) -> Result<(), ClientError> {
    match v {
        Value::Object(values) => {
            let sorted: std::collections::BTreeMap<_, _> = values.iter().collect();
            output.push('{');
            for (position, (key, value)) in sorted.into_iter().enumerate() {
                if position > 0 {
                    output.push(',');
                }
                output.push_str(&serde_json::to_string(key).map_err(|_| protocol())?);
                output.push(':');
                canonical(value, output)?;
            }
            output.push('}');
        }
        Value::Array(values) => {
            output.push('[');
            for (position, value) in values.iter().enumerate() {
                if position > 0 {
                    output.push(',');
                }
                canonical(value, output)?;
            }
            output.push(']');
        }
        Value::Number(number) => {
            if let Some(value) = number.as_i64() {
                output.push_str(&value.to_string());
            } else if let Some(value) = number.as_u64() {
                output.push_str(&value.to_string());
            } else {
                let value = number
                    .as_f64()
                    .filter(|v| v.is_finite())
                    .ok_or_else(protocol)?;
                let mut buffer = ryu_js::Buffer::new();
                output.push_str(buffer.format_finite(value));
            }
        }
        other => output.push_str(&serde_json::to_string(other).map_err(|_| protocol())?),
    }
    Ok(())
}
fn digest(v: &Value) -> Result<String, ClientError> {
    let mut bytes = String::new();
    canonical(v, &mut bytes)?;
    Ok(
        ring::digest::digest(&ring::digest::SHA256, bytes.as_bytes())
            .as_ref()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
    )
}
fn plain<T>(response: &CallResult<T>) -> Result<(), ClientError> {
    let meta = response.meta.as_ref().ok_or_else(protocol)?;
    if !meta.resource_versions.is_empty()
        || meta.command.is_some()
        || meta.host_instance_id.is_some()
    {
        return Err(protocol());
    }
    Ok(())
}
fn versions(v: &Value, catalog: bool) -> Result<(), ClientError> {
    let values = v.as_array().ok_or_else(protocol)?;
    let mut previous: Option<(&str, &str)> = None;
    let mut primary = 0;
    for (position, value) in values.iter().enumerate() {
        let o = exact(value, &["kind", "scope", "id", "revision"])?;
        let scope = text(&o["scope"])?;
        let id = text(&o["id"])?;
        if o["kind"] != "automation" || !identifier(id) || revision(&o["revision"])? < 1 {
            return Err(protocol());
        }
        if scope == "library" {
            if !catalog || position != 0 || id != "catalog" {
                return Err(protocol());
            }
            primary += 1;
            continue;
        } else if !matches!(scope, "action" | "event") {
            return Err(protocol());
        }
        if previous.is_some_and(|old| old >= (scope, id)) {
            return Err(protocol());
        }
        previous = Some((scope, id));
    }
    if primary != usize::from(catalog) {
        return Err(protocol());
    }
    Ok(())
}
fn catalog(v: &Value) -> Result<(), ClientError> {
    within(v, CATALOG_BYTES)?;
    let o = exact(v, &["packs", "activeId", "templates"])?;
    let active = text(&o["activeId"])?;
    if !profile_id(active) {
        return Err(protocol());
    }
    let mut ids = BTreeSet::new();
    for pack in o["packs"].as_array().ok_or_else(protocol)? {
        let p = exact(
            pack,
            &["id", "name", "description", "eventIds", "actionIds"],
        )?;
        let id = text(&p["id"])?;
        if !profile_id(id)
            || id == "default"
            || !ids.insert(id)
            || text(&p["name"])?.trim().is_empty()
            || text(&p["name"])?.chars().count() > 80
            || text(&p["description"])?.chars().count() > 500
        {
            return Err(protocol());
        }
        for key in ["eventIds", "actionIds"] {
            let mut seen = BTreeSet::new();
            for item in p[key].as_array().ok_or_else(protocol)? {
                let id = text(item)?;
                if !identifier(id) || !seen.insert(id) {
                    return Err(protocol());
                }
            }
        }
    }
    if active != "default" && !ids.contains(active) {
        return Err(protocol());
    }
    let mut templates = BTreeSet::new();
    for template in o["templates"].as_array().ok_or_else(protocol)? {
        bounded_source(template, CATALOG_BYTES)?;
        let id = text(&template["id"])?;
        if !template.is_object() || !profile_id(id) || !templates.insert(id) {
            return Err(protocol());
        }
    }
    Ok(())
}
fn validate_snapshot(response: &CallResult<Value>) -> Result<OwnerLibrarySnapshot, ClientError> {
    plain(response)?;
    within(&response.result, BYTES)?;
    let o = exact(
        &response.result,
        &[
            "hostId",
            "hostInstanceId",
            "revision",
            "definitionDigest",
            "catalog",
            "definitions",
            "resourceVersions",
        ],
    )?;
    if !identifier(text(&o["hostId"])?) || !identifier(text(&o["hostInstanceId"])?) {
        return Err(protocol());
    }
    revision(&o["revision"])?;
    hash(&o["definitionDigest"])?;
    catalog(&o["catalog"])?;
    versions(&o["resourceVersions"], false)?;
    let definitions = o["definitions"].as_object().ok_or_else(protocol)?;
    let mut expected = BTreeSet::new();
    for (key, family) in [("actions", "action"), ("events", "event")] {
        for record in definitions
            .get(key)
            .and_then(Value::as_array)
            .ok_or_else(protocol)?
        {
            bounded_source(record, BYTES)?;
            let id = text(&record["id"])?;
            if !record.is_object() || !identifier(id) || !expected.insert((family, id)) {
                return Err(protocol());
            }
        }
    }
    if expected.len() != o["resourceVersions"].as_array().ok_or_else(protocol)?.len() {
        return Err(protocol());
    }
    for version in o["resourceVersions"].as_array().ok_or_else(protocol)? {
        if !expected.remove(&(text(&version["scope"])?, text(&version["id"])?)) {
            return Err(protocol());
        }
    }
    if !definitions.get("plugins").is_some_and(Value::is_array)
        || !definitions.get("actionTypes").is_some_and(Value::is_array)
        || !definitions
            .get("translations")
            .is_some_and(Value::is_object)
    {
        return Err(protocol());
    }
    let mut actions = definitions["actions"]
        .as_array()
        .ok_or_else(protocol)?
        .clone();
    let mut events = definitions["events"]
        .as_array()
        .ok_or_else(protocol)?
        .clone();
    for records in [&mut actions, &mut events] {
        records.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    }
    if digest(&json!({"actions":actions,"events":events,"resourceVersions":o["resourceVersions"]}))?
        != text(&o["definitionDigest"])?
    {
        return Err(protocol());
    }
    serde_json::from_value(response.result.clone()).map_err(|_| protocol())
}
fn operation(v: &str) -> bool {
    matches!(
        v,
        "create-profile"
            | "switch-profile"
            | "delete-profile"
            | "apply-template"
            | "import-profile"
            | "assign-record"
            | "import-templates"
            | "delete-template"
    )
}
fn intent(v: &Value) -> Result<&str, ClientError> {
    let o = exact(
        v,
        &[
            "commandId",
            "operation",
            "target",
            "expectedRevision",
            "definitionDigest",
            "commandHash",
            "hostInstanceId",
            "createdAt",
        ],
    )?;
    let command = text(&o["commandId"])?;
    let operation = text(&o["operation"])?;
    let target = text(&o["target"])?;
    if !command_id(command)
        || !self::operation(operation)
        || !identifier(text(&o["hostInstanceId"])?)
        || (if operation == "assign-record" {
            !identifier(target)
        } else if operation == "import-templates" {
            target != "templates"
        } else {
            !profile_id(target)
        })
    {
        return Err(protocol());
    }
    revision(&o["expectedRevision"])?;
    hash(&o["definitionDigest"])?;
    hash(&o["commandHash"])?;
    timestamp(&o["createdAt"], false)?;
    Ok(command)
}
fn receipt(v: &Value, i: &Value) -> Result<(), ClientError> {
    let o = exact(
        v,
        &[
            "revision",
            "catalogDigest",
            "definitionDigest",
            "versionsDigest",
            "affectedResources",
        ],
    )?;
    if revision(&o["revision"])?
        != revision(&i["expectedRevision"])?
            .checked_add(1)
            .ok_or_else(protocol)?
        || !o["affectedResources"]
            .as_u64()
            .is_some_and(|n| n > 0 && n <= u64::from(u32::MAX))
    {
        return Err(protocol());
    }
    for key in ["catalogDigest", "definitionDigest", "versionsDigest"] {
        hash(&o[key])?;
    }
    Ok(())
}
fn state_value(v: &Value) -> Result<OwnerLibraryState, ClientError> {
    within(v, STATE_BYTES)?;
    let o = exact(v, &["hostId", "hostInstanceId", "pending", "recent"])?;
    if !identifier(text(&o["hostId"])?) || !identifier(text(&o["hostInstanceId"])?) {
        return Err(protocol());
    }
    let mut ids = BTreeSet::new();
    if !o["pending"].is_null() {
        ids.insert(intent(&o["pending"])?);
    }
    let recent = o["recent"]
        .as_array()
        .filter(|rows| rows.len() <= 16)
        .ok_or_else(protocol)?;
    for record in recent {
        let r = exact(
            record,
            &["intent", "outcome", "receipt", "errorCode", "recordedAt"],
        )?;
        if !ids.insert(intent(&r["intent"])?) {
            return Err(protocol());
        }
        timestamp(&r["recordedAt"], false)?;
        match text(&r["outcome"])? {
            "succeeded" => {
                if !r["errorCode"].is_null() {
                    return Err(protocol());
                }
                receipt(&r["receipt"], &r["intent"])?;
            }
            "rejected" => {
                if !r["receipt"].is_null()
                    || !matches!(
                        text(&r["errorCode"])?,
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
                if !r["receipt"].is_null() || !r["errorCode"].is_null() {
                    return Err(protocol());
                }
            }
            _ => return Err(protocol()),
        }
    }
    serde_json::from_value(v.clone()).map_err(|_| protocol())
}
fn validate_state(response: &CallResult<Value>) -> Result<OwnerLibraryState, ClientError> {
    plain(response)?;
    state_value(&response.result)
}
fn command_hash(input: &OwnerLibrarySubmit) -> Result<String, ClientError> {
    digest(
        &json!({"method":"automation.library.apply","params":{"mutation":input.mutation,"definitionDigest":input.definition_digest},"expectedRevision":input.expected_revision}),
    )
}
fn write_sources(input: &OwnerLibrarySubmit) -> Result<(), ClientError> {
    let entry = |entry: &OwnerLibraryEntry| -> Result<(), ClientError> {
        for record in entry.actions.iter().chain(std::iter::once(&entry.event)) {
            source(record)?;
        }
        Ok(())
    };
    let gallery = |values: Option<&Vec<Value>>| -> Result<(), ClientError> {
        if let Some(values) = values {
            for value in values {
                source(value)?;
            }
        }
        Ok(())
    };
    match &input.mutation {
        OwnerLibraryMutation::ApplyTemplate {
            entry: primary,
            additional_entries,
            templates,
            ..
        } => {
            entry(primary)?;
            if let Some(entries) = additional_entries {
                for value in entries {
                    entry(value)?;
                }
            }
            gallery(templates.as_ref())?;
        }
        OwnerLibraryMutation::ImportProfile {
            entries, templates, ..
        } => {
            for value in entries {
                entry(value)?;
            }
            gallery(templates.as_ref())?;
        }
        OwnerLibraryMutation::ImportTemplates { templates } => gallery(Some(templates))?,
        _ => {}
    }
    Ok(())
}
fn validate_submission(
    response: &CallResult<Value>,
    input: &OwnerLibrarySubmit,
) -> Result<OwnerLibrarySubmission, ClientError> {
    plain(response)?;
    within(&response.result, BYTES)?;
    let o = exact(&response.result, &["state", "acknowledgement"])?;
    state_value(&o["state"])?;
    let state = &o["state"];
    if state["hostId"] != input.host_id || state["hostInstanceId"] != input.host_instance_id {
        return Err(protocol());
    }
    let pending = !state["pending"].is_null();
    let latest = state["recent"].as_array().ok_or_else(protocol)?.first();
    let selected = if pending {
        &state["pending"]
    } else {
        &latest.ok_or_else(protocol)?["intent"]
    };
    let op = serde_json::to_value(input.mutation.operation()).map_err(|_| protocol())?;
    if selected["operation"] != op
        || selected["target"] != input.mutation.target()
        || selected["expectedRevision"] != input.expected_revision
        || selected["definitionDigest"] != input.definition_digest
        || selected["hostInstanceId"] != input.host_instance_id
        || selected["commandHash"] != command_hash(input)?
    {
        return Err(protocol());
    }
    if !o["acknowledgement"].is_null() {
        let a = exact(
            &o["acknowledgement"],
            &["value", "resourceVersions", "command", "hostInstanceId"],
        )?;
        let entry = latest.ok_or_else(protocol)?;
        if pending
            || entry["outcome"] != "succeeded"
            || a["value"] != entry["receipt"]
            || a["hostInstanceId"] != input.host_instance_id
        {
            return Err(protocol());
        }
        versions(&a["resourceVersions"], true)?;
        let rows = a["resourceVersions"].as_array().ok_or_else(protocol)?;
        if rows.len() as u64
            != entry["receipt"]["affectedResources"]
                .as_u64()
                .ok_or_else(protocol)?
            || digest(&a["resourceVersions"])? != text(&entry["receipt"]["versionsDigest"])?
            || !rows
                .iter()
                .any(|v| v["scope"] == "library" && v["revision"] == entry["receipt"]["revision"])
        {
            return Err(protocol());
        }
        let c = exact(
            &a["command"],
            &["commandId", "state", "resource", "committedAt"],
        )?;
        let resource = exact(&c["resource"], &["kind", "scope", "id"])?;
        if c["commandId"] != selected["commandId"]
            || c["state"] != "succeeded"
            || resource["kind"] != "automation"
            || resource["scope"] != "library"
            || resource["id"] != "catalog"
        {
            return Err(protocol());
        }
        timestamp(&c["committedAt"], true)?;
    }
    serde_json::from_value(response.result.clone()).map_err(|_| protocol())
}

impl TikToolsClient {
    pub async fn automation_library_snapshot(&self) -> Result<OwnerLibrarySnapshot, ClientError> {
        validate_snapshot(
            &self
                .call_with_meta(METHODS[0], OwnerLibraryEmpty {}, None)
                .await?,
        )
    }
    pub async fn automation_library_get(&self) -> Result<OwnerLibraryState, ClientError> {
        validate_state(
            &self
                .call_with_meta(METHODS[1], OwnerLibraryEmpty {}, None)
                .await?,
        )
    }
    pub async fn automation_library_submit(
        &self,
        input: OwnerLibrarySubmit,
    ) -> Result<OwnerLibrarySubmission, ClientError> {
        write_sources(&input).map_err(|_| {
            ClientError::new(
                "invalid_params",
                "library source exceeds its bounded structure or size",
            )
        })?;
        validate_owner_library_submit(&input).map_err(|e| ClientError::new(e.code, e.message))?;
        validate_submission(
            &self.call_with_meta(METHODS[2], &input, None).await?,
            &input,
        )
    }
    pub async fn automation_library_resolve(&self) -> Result<OwnerLibraryState, ClientError> {
        validate_state(
            &self
                .call_with_meta(METHODS[3], OwnerLibraryEmpty {}, None)
                .await?,
        )
    }
    pub async fn automation_library_review_unknown(
        &self,
        input: OwnerLibraryReviewUnknown,
    ) -> Result<OwnerLibraryState, ClientError> {
        if !input.acknowledge_unknown || !command_id(&input.command_id) {
            return Err(ClientError::new(
                "invalid_params",
                "review the original library outcome",
            ));
        }
        let state = validate_state(&self.call_with_meta(METHODS[4], &input, None).await?)?;
        if state
            .pending
            .as_ref()
            .is_some_and(|i| i.command_id == input.command_id)
            || !state
                .recent
                .iter()
                .any(|r| r.intent.command_id == input.command_id)
        {
            return Err(protocol());
        }
        Ok(state)
    }
}

#[cfg(test)]
mod tests;
