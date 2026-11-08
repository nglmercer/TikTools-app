//! Native atomic automation-library transitions. Source is confined to an
//! explicit snapshot/submission; recovery retains only original-result metadata.
use super::{AutomationKind, OperationError};
use crate::collaboration::{CommandReceipt, ResourceVersion};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const OWNER_LIBRARY_MAX_BYTES: usize = 8 * 1024 * 1024;
pub const OWNER_LIBRARY_MAX_CATALOG_BYTES: usize = 4 * 1024 * 1024;
pub const OWNER_LIBRARY_MAX_RECORDS: usize = 512;
#[cfg(feature = "persistence")]
pub(crate) const LIBRARY_STATE_KEYS: [&str; 3] = [
    "behavior.profiles.packs",
    "behavior.profiles.active",
    "behavior.templates.custom",
];

mod runtime;
#[cfg(feature = "persistence")]
pub(crate) use runtime::{library_now, library_rejection};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerLibraryProfile {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerLibraryPack {
    pub id: String,
    pub name: String,
    pub description: String,
    pub event_ids: Vec<String>,
    pub action_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerLibraryCatalog {
    pub packs: Vec<OwnerLibraryPack>,
    pub active_id: String,
    pub templates: Vec<Value>,
}
impl Default for OwnerLibraryCatalog {
    fn default() -> Self {
        Self {
            packs: Vec::new(),
            active_id: "default".into(),
            templates: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerLibraryEntry {
    pub actions: Vec<Value>,
    pub event: Value,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum OwnerLibraryOperation {
    CreateProfile,
    SwitchProfile,
    DeleteProfile,
    ApplyTemplate,
    ImportProfile,
    AssignRecord,
    ImportTemplates,
    DeleteTemplate,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "operation",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum OwnerLibraryMutation {
    CreateProfile {
        id: String,
        name: String,
        description: String,
    },
    SwitchProfile {
        id: String,
    },
    DeleteProfile {
        id: String,
    },
    ApplyTemplate {
        entry: OwnerLibraryEntry,
        profile_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        additional_entries: Option<Vec<OwnerLibraryEntry>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        templates: Option<Vec<Value>>,
    },
    ImportProfile {
        profile: OwnerLibraryProfile,
        entries: Vec<OwnerLibraryEntry>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        templates: Option<Vec<Value>>,
    },
    AssignRecord {
        kind: AutomationKind,
        id: String,
        profile_id: String,
    },
    ImportTemplates {
        templates: Vec<Value>,
    },
    DeleteTemplate {
        id: String,
    },
}
impl OwnerLibraryMutation {
    pub fn operation(&self) -> OwnerLibraryOperation {
        match self {
            Self::CreateProfile { .. } => OwnerLibraryOperation::CreateProfile,
            Self::SwitchProfile { .. } => OwnerLibraryOperation::SwitchProfile,
            Self::DeleteProfile { .. } => OwnerLibraryOperation::DeleteProfile,
            Self::ApplyTemplate { .. } => OwnerLibraryOperation::ApplyTemplate,
            Self::ImportProfile { .. } => OwnerLibraryOperation::ImportProfile,
            Self::AssignRecord { .. } => OwnerLibraryOperation::AssignRecord,
            Self::ImportTemplates { .. } => OwnerLibraryOperation::ImportTemplates,
            Self::DeleteTemplate { .. } => OwnerLibraryOperation::DeleteTemplate,
        }
    }
    pub fn target(&self) -> &str {
        match self {
            Self::CreateProfile { id, .. }
            | Self::SwitchProfile { id }
            | Self::DeleteProfile { id }
            | Self::AssignRecord { id, .. }
            | Self::DeleteTemplate { id } => id,
            Self::ApplyTemplate { profile_id, .. } => profile_id,
            Self::ImportProfile { profile, .. } => &profile.id,
            Self::ImportTemplates { .. } => "templates",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerLibrarySubmit {
    pub mutation: OwnerLibraryMutation,
    pub expected_revision: String,
    pub definition_digest: String,
    pub host_id: String,
    pub host_instance_id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerLibrarySnapshot {
    pub host_id: String,
    pub host_instance_id: String,
    pub revision: String,
    pub definition_digest: String,
    pub catalog: OwnerLibraryCatalog,
    pub definitions: Value,
    pub resource_versions: Vec<ResourceVersion>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerLibraryIntent {
    pub command_id: String,
    pub operation: OwnerLibraryOperation,
    pub target: String,
    pub expected_revision: String,
    pub definition_digest: String,
    pub command_hash: String,
    pub host_instance_id: String,
    pub created_at: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum OwnerLibraryOutcome {
    Succeeded,
    Rejected,
    ReviewedUnknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerLibraryReceipt {
    pub revision: String,
    pub catalog_digest: String,
    pub definition_digest: String,
    pub versions_digest: String,
    pub affected_resources: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerLibraryRecord {
    pub intent: OwnerLibraryIntent,
    pub outcome: OwnerLibraryOutcome,
    pub receipt: Option<OwnerLibraryReceipt>,
    pub error_code: Option<String>,
    pub recorded_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerLibraryState {
    pub host_id: String,
    pub host_instance_id: String,
    pub pending: Option<OwnerLibraryIntent>,
    pub recent: Vec<OwnerLibraryRecord>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerLibraryAcknowledgement {
    pub value: OwnerLibraryReceipt,
    pub resource_versions: Vec<ResourceVersion>,
    pub command: CommandReceipt,
    pub host_instance_id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerLibrarySubmission {
    pub state: OwnerLibraryState,
    pub acknowledgement: Option<OwnerLibraryAcknowledgement>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerLibraryReviewUnknown {
    pub command_id: String,
    pub acknowledge_unknown: bool,
}

/// Cross-language library encoding: keys sort by Unicode scalar value, numeric
/// integers stay exact, and floating-point values use ECMAScript formatting.
/// In particular, a parsed legacy `1.0` has the same encoding as browser `1`.
/// This is private to the new library protocol; other command hashes retain
/// their existing format. Browsers must reject integers they cannot represent.
pub fn owner_library_canonical_json(value: &Value) -> String {
    fn write(value: &Value, output: &mut String) {
        match value {
            Value::Object(values) => {
                let sorted: std::collections::BTreeMap<_, _> = values.iter().collect();
                output.push('{');
                for (index, (key, value)) in sorted.into_iter().enumerate() {
                    if index > 0 {
                        output.push(',');
                    }
                    output.push_str(&Value::String(key.clone()).to_string());
                    output.push(':');
                    write(value, output);
                }
                output.push('}');
            }
            Value::Array(values) => {
                output.push('[');
                for (index, value) in values.iter().enumerate() {
                    if index > 0 {
                        output.push(',');
                    }
                    write(value, output);
                }
                output.push(']');
            }
            Value::Number(number) => {
                if let Some(integer) = number.as_i64() {
                    output.push_str(&integer.to_string());
                } else if let Some(integer) = number.as_u64() {
                    output.push_str(&integer.to_string());
                } else if let Some(float) = number.as_f64() {
                    output.push_str(ryu_js::Buffer::new().format_finite(float));
                } else {
                    output.push_str(&number.to_string());
                }
            }
            other => output.push_str(&other.to_string()),
        }
    }
    let mut output = String::new();
    write(value, &mut output);
    output
}

pub fn owner_library_digest(value: &Value) -> String {
    crate::collaboration::hash_secret(&owner_library_canonical_json(value))
}

pub fn owner_library_versions_digest(versions: &[ResourceVersion]) -> String {
    owner_library_digest(&json!(versions))
}

/// The universe includes source and family-qualified revisions. In particular,
/// a new row invalidates a previously displayed global profile switch.
pub fn owner_library_definition_digest(
    definitions: &Value,
    versions: &[ResourceVersion],
) -> String {
    let rows = |key: &str| {
        let mut rows = definitions
            .get(key)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        rows.sort_by(|left, right| {
            left.get("id")
                .and_then(Value::as_str)
                .cmp(&right.get("id").and_then(Value::as_str))
        });
        rows
    };
    let mut versions = versions.to_vec();
    versions.sort_by(|left, right| (&left.scope, &left.id).cmp(&(&right.scope, &right.id)));
    owner_library_digest(
        &json!({"actions":rows("actions"),"events":rows("events"),"resourceVersions":versions}),
    )
}

pub fn owner_library_command_params(
    mutation: &OwnerLibraryMutation,
    definition_digest: &str,
) -> Value {
    json!({"mutation":mutation,"definitionDigest":definition_digest})
}

pub fn owner_library_command_hash(input: &OwnerLibrarySubmit) -> String {
    owner_library_digest(&json!({
        "method":"automation.library.apply",
        "params":owner_library_command_params(&input.mutation,&input.definition_digest),
        "expectedRevision":input.expected_revision,
    }))
}

pub fn validate_owner_library_submit(input: &OwnerLibrarySubmit) -> Result<(), OperationError> {
    let valid_revision = input
        .expected_revision
        .parse::<i64>()
        .ok()
        .is_some_and(|value| value >= 0 && value.to_string() == input.expected_revision);
    if !valid_revision
        || !library_hash(&input.definition_digest)
        || !crate::collaboration::valid_command_id(&input.host_id)
        || !crate::collaboration::valid_command_id(&input.host_instance_id)
    {
        return Err(OperationError::invalid(
            "invalid reviewed library identity or version",
        ));
    }
    let mut targets = std::collections::BTreeSet::new();
    let mut check_entry = |entry: &OwnerLibraryEntry| -> Result<(), OperationError> {
        if entry.actions.is_empty() || entry.actions.len() > 8 {
            return Err(OperationError::invalid(
                "a template requires one to eight actions",
            ));
        }
        for (kind, record) in entry
            .actions
            .iter()
            .map(|record| (AutomationKind::Action, record))
            .chain(std::iter::once((AutomationKind::Event, &entry.event)))
        {
            super::owner_definition::validate_source(record)?;
            let id = record.get("id").and_then(Value::as_str).unwrap_or_default();
            if !library_record_id(id)
                || !record
                    .get("name")
                    .and_then(Value::as_str)
                    .is_some_and(|name| !name.trim().is_empty())
                || !record.get("enabled").is_some_and(Value::is_boolean)
            {
                return Err(OperationError::invalid(
                    "library graph records require explicit IDs, names and enablement",
                ));
            }
            let family = match kind {
                AutomationKind::Action => "action",
                AutomationKind::Event => "event",
            };
            if !targets.insert((family, id.to_owned())) || targets.len() > OWNER_LIBRARY_MAX_RECORDS
            {
                return Err(OperationError::invalid(
                    "library graph contains duplicate or excessive targets",
                ));
            }
        }
        let actions = entry
            .event
            .get("actionIds")
            .and_then(Value::as_array)
            .ok_or_else(|| OperationError::invalid("template event requires actionIds"))?;
        let expected: Vec<_> = entry
            .actions
            .iter()
            .map(|record| record["id"].clone())
            .collect();
        if *actions != expected {
            return Err(OperationError::invalid(
                "template event must reference its exact ordered actions",
            ));
        }
        Ok(())
    };
    match &input.mutation {
        OwnerLibraryMutation::CreateProfile {
            id,
            name,
            description,
        } => {
            if !library_profile_id(id)
                || id == "default"
                || name.trim().is_empty()
                || name.chars().count() > 80
                || description.chars().count() > 500
            {
                return Err(OperationError::invalid("invalid profile descriptor"));
            }
        }
        OwnerLibraryMutation::SwitchProfile { id } => {
            if !library_profile_id(id) {
                return Err(OperationError::invalid("invalid profile ID"));
            }
        }
        OwnerLibraryMutation::DeleteProfile { id } => {
            if !library_profile_id(id) || id == "default" {
                return Err(OperationError::invalid("invalid profile ID"));
            }
        }
        OwnerLibraryMutation::ApplyTemplate {
            entry,
            profile_id,
            additional_entries,
            ..
        } => {
            if !library_profile_id(profile_id) {
                return Err(OperationError::invalid("invalid profile ID"));
            }
            check_entry(entry)?;
            if let Some(entries) = additional_entries {
                if entries.len() > 31 {
                    return Err(OperationError::invalid(
                        "at most 32 templates per application",
                    ));
                }
                for entry in entries {
                    check_entry(entry)?;
                }
            }
        }
        OwnerLibraryMutation::ImportProfile {
            profile, entries, ..
        } => {
            if !library_profile_id(&profile.id)
                || profile.id == "default"
                || profile.name.trim().is_empty()
                || profile.name.chars().count() > 80
                || profile.description.chars().count() > 500
                || entries.is_empty()
                || entries.len() > 32
            {
                return Err(OperationError::invalid("invalid imported profile"));
            }
            for entry in entries {
                check_entry(entry)?;
            }
        }
        OwnerLibraryMutation::AssignRecord { id, profile_id, .. } => {
            if !library_record_id(id) || !library_profile_id(profile_id) {
                return Err(OperationError::invalid("invalid membership target"));
            }
        }
        OwnerLibraryMutation::ImportTemplates { templates } => {
            if templates.is_empty() || templates.len() > 128 {
                return Err(OperationError::invalid("invalid template import size"));
            }
            for template in templates {
                super::owner_definition::validate_source(template)?;
            }
        }
        OwnerLibraryMutation::DeleteTemplate { id } => {
            if !library_profile_id(id) {
                return Err(OperationError::invalid("invalid template ID"));
            }
        }
    }
    let templates = match &input.mutation {
        OwnerLibraryMutation::ApplyTemplate { templates, .. }
        | OwnerLibraryMutation::ImportProfile { templates, .. } => templates.as_ref(),
        _ => None,
    };
    if let Some(templates) = templates {
        if templates.len() > 128 {
            return Err(OperationError::invalid("invalid template import size"));
        }
        for template in templates {
            super::owner_definition::validate_source(template)?;
        }
    }
    if serde_json::to_vec(input)
        .map_err(|_| OperationError::invalid("invalid library input"))?
        .len()
        > OWNER_LIBRARY_MAX_BYTES
    {
        return Err(OperationError {
            code: "too_large",
            message: "library submission exceeds its byte budget".into(),
            data: None,
        });
    }
    Ok(())
}

pub(crate) fn library_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
pub(crate) fn library_record_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.trim() == value
        && !value.contains('*')
        && !value.chars().any(char::is_control)
}
pub(crate) fn library_profile_id(value: &str) -> bool {
    !value.is_empty()
        && value.chars().count() <= 64
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

#[cfg(all(test, feature = "persistence"))]
mod tests;
