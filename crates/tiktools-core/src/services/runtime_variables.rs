//! Typed local variables over the legacy textual render slice. Type metadata
//! contains no values. Snapshot revisions fence atomic edits across windows.
use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::globals::{storage_key, validate_key, GLOBALS_KEY_PREFIX, MAX_GLOBAL_KEYS};
use crate::control::OperationError;

pub(crate) const TYPE_PREFIX: &str = "runtime.variable.type.";
pub(crate) const REVISION_KEY: &str = "runtime.variable.revision";
pub(crate) const MAX_VARIABLE_STATE_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeVariableKind {
    Text,
    Number,
    Boolean,
    List,
    Secret,
}

impl RuntimeVariableKind {
    pub(crate) fn tag(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Number => "number",
            Self::Boolean => "boolean",
            Self::List => "list",
            Self::Secret => "secret",
        }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeVariable {
    #[serde(rename = "type")]
    pub kind: RuntimeVariableKind,
    pub value: Value,
}

impl std::fmt::Debug for RuntimeVariable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuntimeVariable")
            .field("kind", &self.kind)
            .field("value", &"<redacted>")
            .finish()
    }
}

impl RuntimeVariable {
    pub(crate) fn render(&self) -> Result<String, OperationError> {
        let valid = match self.kind {
            RuntimeVariableKind::Text | RuntimeVariableKind::Secret => {
                self.value.as_str().is_some_and(|s| s.len() <= 4096)
            }
            RuntimeVariableKind::Number => self.value.is_number(),
            RuntimeVariableKind::Boolean => self.value.is_boolean(),
            RuntimeVariableKind::List => self.value.as_array().is_some_and(|items| {
                items.len() <= 100
                    && items
                        .iter()
                        .all(|v| v.as_str().is_some_and(|s| s.chars().count() <= 256))
            }),
        };
        if !valid {
            return Err(OperationError::invalid(
                "variable value does not match its type or size limit",
            ));
        }
        Ok(match self.value.as_str() {
            Some(text) => text.to_owned(),
            None => self.value.to_string(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeVariablesSnapshot {
    pub revision: String,
    pub variables: BTreeMap<String, RuntimeVariable>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeVariablesReplace {
    pub expected_revision: String,
    pub variables: BTreeMap<String, RuntimeVariable>,
}

pub(crate) fn variable_state_key(key: &str) -> bool {
    key.starts_with(GLOBALS_KEY_PREFIX) || key.starts_with(TYPE_PREFIX) || key == REVISION_KEY
}

pub(crate) fn snapshot(
    state: &BTreeMap<String, String>,
) -> Result<RuntimeVariablesSnapshot, OperationError> {
    let relevant: BTreeMap<_, _> = state
        .iter()
        .filter(|(key, _)| variable_state_key(key))
        .collect();
    let bytes = serde_json::to_vec(&relevant)
        .map_err(|_| OperationError::unavailable("runtime variables are unreadable"))?;
    if bytes.len() > MAX_VARIABLE_STATE_BYTES {
        return Err(OperationError::unavailable(
            "runtime variables exceed the storage limit",
        ));
    }
    let revision: String = Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let mut variables = BTreeMap::new();
    for (key, raw) in state {
        let Some(name) = key.strip_prefix(GLOBALS_KEY_PREFIX) else {
            continue;
        };
        if name.is_empty() {
            continue;
        }
        let kind = match state
            .get(&format!("{TYPE_PREFIX}{name}"))
            .map(String::as_str)
        {
            None | Some("text") => RuntimeVariableKind::Text,
            Some("secret") => RuntimeVariableKind::Secret,
            Some("number") => RuntimeVariableKind::Number,
            Some("boolean") => RuntimeVariableKind::Boolean,
            Some("list") => RuntimeVariableKind::List,
            _ => {
                return Err(OperationError::unavailable(
                    "runtime variable type metadata is invalid",
                ))
            }
        };
        let value = match kind {
            RuntimeVariableKind::Text | RuntimeVariableKind::Secret => Value::String(raw.clone()),
            _ => serde_json::from_str(raw)
                .map_err(|_| OperationError::unavailable("runtime variable value needs review"))?,
        };
        let variable = RuntimeVariable { kind, value };
        // Legacy untyped text is displayed without rewriting or inferring a type.
        if !matches!(
            kind,
            RuntimeVariableKind::Text | RuntimeVariableKind::Secret
        ) {
            variable
                .render()
                .map_err(|_| OperationError::unavailable("runtime variable value needs review"))?;
        }
        variables.insert(name.to_owned(), variable);
    }
    Ok(RuntimeVariablesSnapshot {
        revision,
        variables,
    })
}

pub(crate) fn replace(
    state: &mut BTreeMap<String, String>,
    params: RuntimeVariablesReplace,
) -> Result<RuntimeVariablesSnapshot, OperationError> {
    let current = snapshot(state)?;
    if current.revision != params.expected_revision {
        return Err(OperationError::conflict(
            "runtime variables changed; reload before saving",
        ));
    }
    if params.variables.len() > MAX_GLOBAL_KEYS {
        return Err(OperationError::invalid("at most 128 variables"));
    }
    let mut encoded = BTreeMap::new();
    for (name, variable) in &params.variables {
        validate_key(name)?;
        encoded.insert(storage_key(name), variable.render()?);
        encoded.insert(
            format!("{TYPE_PREFIX}{name}"),
            variable.kind.tag().to_owned(),
        );
    }
    if current.variables == params.variables {
        return Ok(current);
    }
    encoded.insert(
        REVISION_KEY.to_owned(),
        format!("{:032x}", fastrand::u128(..)),
    );
    // Validate the complete candidate before replacing any row.
    let next = snapshot(&encoded)?;
    state.retain(|key, _| !variable_state_key(key));
    state.extend(encoded);
    Ok(next)
}
