//! Native Owner definition writes. Recovery exposes metadata only and never
//! repeats an operation or installs historical source into a current editor.
use crate::{CallResult, ClientError, TikToolsClient};
use serde_json::Value;
use std::collections::BTreeSet;
use tiktools_control_api::modules::automation_definition::OwnerDefinitionEmpty;
pub use tiktools_control_api::modules::automation_definition::{
    OwnerDefinitionAcknowledgement, OwnerDefinitionIntent, OwnerDefinitionOperation,
    OwnerDefinitionOutcome, OwnerDefinitionReceipt, OwnerDefinitionRecord,
    OwnerDefinitionReviewUnknown, OwnerDefinitionState, OwnerDefinitionSubmission,
    OwnerDefinitionSubmit,
};

const GET: &str = "automation.definition.get";
const SUBMIT: &str = "automation.definition.submit";
const RESOLVE: &str = "automation.definition.resolve";
const REVIEW: &str = "automation.definition.review_unknown";
pub(crate) const METHODS: &[&str] = &[GET, SUBMIT, RESOLVE, REVIEW];
const STATE_BYTES: usize = 256 * 1024;
const RECORD_BYTES: usize = 256 * 1024;
const SUBMISSION_BYTES: usize = 8 * 1024 * 1024;

fn protocol() -> ClientError {
    ClientError::new("protocol", "invalid native owner definition result")
}
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
fn timestamp(value: &Value, positive: bool) -> Result<(), ClientError> {
    value
        .as_u64()
        .filter(|n| *n >= u64::from(positive) && *n <= 9_007_199_254_740_991)
        .map(|_| ())
        .ok_or_else(protocol)
}
fn command_id(value: &str) -> bool {
    value
        .strip_prefix("owner-definition-")
        .is_some_and(|suffix| {
            suffix.len() == 32
                && suffix
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        })
}
fn within(value: &Value, bytes: usize) -> Result<(), ClientError> {
    if serde_json::to_vec(value).map_err(|_| protocol())?.len() > bytes {
        return Err(protocol());
    }
    Ok(())
}
fn canonical_json(value: &Value, depth: usize, nodes: &mut usize) -> Result<Value, ClientError> {
    *nodes += 1;
    if depth > 64 || *nodes > 65_536 {
        return Err(protocol());
    }
    Ok(match value {
        Value::Object(object) => {
            let sorted: std::collections::BTreeMap<_, _> = object.iter().collect();
            Value::Object(
                sorted
                    .into_iter()
                    .map(|(key, value)| Ok((key.clone(), canonical_json(value, depth + 1, nodes)?)))
                    .collect::<Result<_, ClientError>>()?,
            )
        }
        Value::Array(values) => Value::Array(
            values
                .iter()
                .map(|value| canonical_json(value, depth + 1, nodes))
                .collect::<Result<_, _>>()?,
        ),
        other => other.clone(),
    })
}
fn record_digest(value: &Value) -> Result<String, ClientError> {
    within(value, RECORD_BYTES)?;
    let mut nodes = 0;
    let bytes =
        serde_json::to_vec(&canonical_json(value, 0, &mut nodes)?).map_err(|_| protocol())?;
    Ok(ring::digest::digest(&ring::digest::SHA256, &bytes)
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}
fn plain_metadata<T>(response: &CallResult<T>) -> Result<(), ClientError> {
    let meta = response.meta.as_ref().ok_or_else(protocol)?;
    if !meta.resource_versions.is_empty()
        || meta.command.is_some()
        || meta.host_instance_id.is_some()
    {
        return Err(protocol());
    }
    Ok(())
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
            "recordDigest",
            "createdAt",
        ],
    )?;
    let command = text(&object["commandId"])?;
    let operation = text(&object["operation"])?;
    if !command_id(command)
        || !matches!(
            operation,
            "create" | "update" | "delete" | "enable" | "disable"
        )
        || !matches!(text(&object["kind"])?, "action" | "event")
        || !canonical(text(&object["id"])?, 128)
        || !canonical(text(&object["hostInstanceId"])?, 128)
    {
        return Err(protocol());
    }
    decimal(text(&object["expectedRevision"])?, false)?;
    if matches!(operation, "create" | "update") {
        if !digest(text(&object["recordDigest"])?) {
            return Err(protocol());
        }
    } else if !object["recordDigest"].is_null() {
        return Err(protocol());
    }
    timestamp(&object["createdAt"], false)?;
    Ok(command)
}
fn receipt(value: &Value, intent: &Value) -> Result<(), ClientError> {
    let object = exact(
        value,
        &[
            "revision",
            "recordDigest",
            "enabled",
            "deleted",
            "affectedResources",
        ],
    )?;
    let operation = text(&intent["operation"])?;
    let before = decimal(text(&intent["expectedRevision"])?, false)?;
    let affected = object["affectedResources"]
        .as_u64()
        .filter(|n| *n > 0 && *n <= u64::from(u32::MAX))
        .ok_or_else(protocol)?;
    let after = decimal(text(&object["revision"])?, false)?;
    let expected = if operation == "delete" && object["deleted"] == false {
        before
    } else {
        before.checked_add(1).ok_or_else(protocol)?
    };
    if after != expected {
        return Err(protocol());
    }
    match operation {
        "create" | "update" => {
            if object["recordDigest"] != intent["recordDigest"]
                || !object["deleted"].is_null()
                || !(object["enabled"].is_null() || object["enabled"].is_boolean())
                || affected != 1
            {
                return Err(protocol());
            }
        }
        "enable" | "disable" => {
            if !digest(text(&object["recordDigest"])?)
                || object["enabled"] != (operation == "enable")
                || !object["deleted"].is_null()
                || affected != 1
            {
                return Err(protocol());
            }
        }
        "delete" => {
            if !object["recordDigest"].is_null()
                || !object["enabled"].is_null()
                || !object["deleted"].is_boolean()
                || ((intent["kind"] == "event" || object["deleted"] == false) && affected != 1)
            {
                return Err(protocol());
            }
        }
        _ => return Err(protocol()),
    }
    Ok(())
}
fn state_value(value: &Value) -> Result<OwnerDefinitionState, ClientError> {
    within(value, STATE_BYTES)?;
    let object = exact(value, &["hostId", "hostInstanceId", "pending", "recent"])?;
    if !canonical(text(&object["hostId"])?, 128)
        || !canonical(text(&object["hostInstanceId"])?, 128)
    {
        return Err(protocol());
    }
    let mut ids = BTreeSet::new();
    if !object["pending"].is_null() {
        ids.insert(intent(&object["pending"])?);
    }
    let recent = object["recent"]
        .as_array()
        .filter(|rows| rows.len() <= 16)
        .ok_or_else(protocol)?;
    for entry in recent {
        let row = exact(
            entry,
            &["intent", "outcome", "receipt", "errorCode", "recordedAt"],
        )?;
        if !ids.insert(intent(&row["intent"])?) {
            return Err(protocol());
        }
        timestamp(&row["recordedAt"], false)?;
        match text(&row["outcome"])? {
            "succeeded" => {
                if !row["errorCode"].is_null() {
                    return Err(protocol());
                }
                receipt(&row["receipt"], &row["intent"])?;
            }
            "rejected" => {
                if !row["receipt"].is_null()
                    || !matches!(
                        text(&row["errorCode"])?,
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
    serde_json::from_value(value.clone()).map_err(|_| protocol())
}
fn validate_state(response: &CallResult<Value>) -> Result<OwnerDefinitionState, ClientError> {
    plain_metadata(response)?;
    state_value(&response.result)
}

struct Input {
    operation: String,
    kind: String,
    id: String,
    expected: String,
    host: String,
    instance: String,
    record_digest: Option<String>,
}
fn input(params: &OwnerDefinitionSubmit) -> Result<Input, ClientError> {
    let value = serde_json::to_value(params).map_err(|_| protocol())?;
    let operation = text(&value["operation"])?;
    let kind = text(&value["kind"])?;
    let expected = text(&value["expectedRevision"])?;
    let host = text(&value["hostId"])?;
    let instance = text(&value["hostInstanceId"])?;
    if !matches!(
        operation,
        "create" | "update" | "delete" | "enable" | "disable"
    ) || !matches!(kind, "action" | "event")
        || !canonical(host, 128)
        || !canonical(instance, 128)
    {
        return Err(protocol());
    }
    decimal(expected, false)?;
    let id = text(if operation == "create" {
        &value["record"]["id"]
    } else {
        &value["id"]
    })?;
    if !canonical(id, 128) {
        return Err(protocol());
    }
    let record_digest = if matches!(operation, "create" | "update") {
        let mut record = value["record"].clone();
        let object = record.as_object_mut().ok_or_else(protocol)?;
        if object.get("id").and_then(Value::as_str) != Some(id) {
            return Err(protocol());
        }
        if operation == "create"
            && object
                .get("name")
                .and_then(Value::as_str)
                .is_none_or(|name| name.trim().is_empty())
        {
            object.insert(
                "name".into(),
                if kind == "action" {
                    "Untitled action"
                } else {
                    "Untitled event"
                }
                .into(),
            );
        }
        Some(record_digest(&record)?)
    } else {
        None
    };
    Ok(Input {
        operation: operation.into(),
        kind: kind.into(),
        id: id.into(),
        expected: expected.into(),
        host: host.into(),
        instance: instance.into(),
        record_digest,
    })
}
fn matches_input(value: &Value, input: &Input) -> bool {
    value["operation"] == input.operation
        && value["kind"] == input.kind
        && value["id"] == input.id
        && value["expectedRevision"] == input.expected
        && value["hostInstanceId"] == input.instance
        && match &input.record_digest {
            Some(digest) => value["recordDigest"] == *digest,
            None => value["recordDigest"].is_null(),
        }
}
fn acknowledgement(value: &Value, entry: &Value) -> Result<(), ClientError> {
    let ack = exact(
        value,
        &["value", "resourceVersions", "command", "hostInstanceId"],
    )?;
    let intent = &entry["intent"];
    let receipt = &entry["receipt"];
    if entry["outcome"] != "succeeded" || ack["hostInstanceId"] != intent["hostInstanceId"] {
        return Err(protocol());
    }
    let command = exact(
        &ack["command"],
        &["commandId", "state", "resource", "committedAt"],
    )?;
    if command["commandId"] != intent["commandId"] || command["state"] != "succeeded" {
        return Err(protocol());
    }
    timestamp(&command["committedAt"], true)?;
    let resource = exact(&command["resource"], &["kind", "scope", "id"])?;
    if resource["kind"] != "automation"
        || resource["scope"] != intent["kind"]
        || resource["id"] != intent["id"]
    {
        return Err(protocol());
    }
    let versions = ack["resourceVersions"].as_array().ok_or_else(protocol)?;
    if versions.is_empty()
        || versions.len() as u64 != receipt["affectedResources"].as_u64().ok_or_else(protocol)?
    {
        return Err(protocol());
    }
    let mut previous: Option<&str> = None;
    for (position, version) in versions.iter().enumerate() {
        let row = exact(version, &["kind", "scope", "id", "revision"])?;
        let id = text(&row["id"])?;
        decimal(text(&row["revision"])?, position > 0)?;
        if row["kind"] != "automation" || !canonical(id, 128) {
            return Err(protocol());
        }
        if position == 0 {
            if row["scope"] != intent["kind"]
                || row["id"] != intent["id"]
                || row["revision"] != receipt["revision"]
            {
                return Err(protocol());
            }
        } else {
            if intent["operation"] != "delete"
                || intent["kind"] != "action"
                || receipt["deleted"] != true
                || row["scope"] != "event"
                || previous.is_some_and(|previous| previous >= id)
            {
                return Err(protocol());
            }
            previous = Some(id);
        }
    }
    let body = &ack["value"];
    if intent["operation"] == "delete" {
        let body = exact(body, &["id", "deleted"])?;
        if body["id"] != intent["id"] || body["deleted"] != receipt["deleted"] {
            return Err(protocol());
        }
    } else {
        if !body.is_object()
            || body["id"] != intent["id"]
            || record_digest(body)? != text(&receipt["recordDigest"])?
        {
            return Err(protocol());
        }
        let enabled = body.get("enabled").and_then(Value::as_bool);
        if serde_json::to_value(enabled).map_err(|_| protocol())? != receipt["enabled"] {
            return Err(protocol());
        }
    }
    Ok(())
}
fn validate_submission(
    response: &CallResult<Value>,
    params: &OwnerDefinitionSubmit,
) -> Result<OwnerDefinitionSubmission, ClientError> {
    plain_metadata(response)?;
    within(&response.result, SUBMISSION_BYTES)?;
    let response = exact(&response.result, &["state", "acknowledgement"])?;
    state_value(&response["state"])?;
    let expected = input(params)?;
    if response["state"]["hostId"] != expected.host
        || response["state"]["hostInstanceId"] != expected.instance
    {
        return Err(protocol());
    }
    let state = &response["state"];
    let pending = !state["pending"].is_null();
    let latest = state["recent"].as_array().ok_or_else(protocol)?.first();
    let candidate = if pending {
        &state["pending"]
    } else {
        &latest.ok_or_else(protocol)?["intent"]
    };
    if !matches_input(candidate, &expected) {
        return Err(protocol());
    }
    if !response["acknowledgement"].is_null() {
        if pending {
            return Err(protocol());
        }
        acknowledgement(&response["acknowledgement"], latest.ok_or_else(protocol)?)?;
    }
    serde_json::from_value(Value::Object(response.clone())).map_err(|_| protocol())
}

impl TikToolsClient {
    pub async fn automation_definition_get(&self) -> Result<OwnerDefinitionState, ClientError> {
        let response = self
            .call_with_meta(GET, OwnerDefinitionEmpty {}, None)
            .await?;
        validate_state(&response)
    }
    /// Native code creates and persists the command ID before dispatch. The
    /// acknowledgement is available only for this fresh successful submission.
    pub async fn automation_definition_submit(
        &self,
        params: OwnerDefinitionSubmit,
    ) -> Result<OwnerDefinitionSubmission, ClientError> {
        input(&params)
            .map_err(|_| ClientError::new("invalid_params", "invalid owner definition request"))?;
        let response = self.call_with_meta(SUBMIT, &params, None).await?;
        validate_submission(&response, &params)
    }
    /// Reads the immutable original result; never sends the mutation again.
    pub async fn automation_definition_resolve(&self) -> Result<OwnerDefinitionState, ClientError> {
        let response = self
            .call_with_meta(RESOLVE, OwnerDefinitionEmpty {}, None)
            .await?;
        validate_state(&response)
    }
    pub async fn automation_definition_review_unknown(
        &self,
        params: OwnerDefinitionReviewUnknown,
    ) -> Result<OwnerDefinitionState, ClientError> {
        if !params.acknowledge_unknown || !command_id(&params.command_id) {
            return Err(ClientError::new(
                "invalid_params",
                "review the original unresolved definition outcome",
            ));
        }
        let response = self.call_with_meta(REVIEW, &params, None).await?;
        let state = validate_state(&response)?;
        if state
            .pending
            .as_ref()
            .is_some_and(|intent| intent.command_id == params.command_id)
            || !state
                .recent
                .iter()
                .any(|record| record.intent.command_id == params.command_id)
        {
            return Err(protocol());
        }
        Ok(state)
    }
}

#[cfg(test)]
mod tests;
