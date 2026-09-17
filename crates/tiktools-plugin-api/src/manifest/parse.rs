use serde_json::{Map, Value};

use super::{
    types::{PluginManifest, PluginRuntimeKind, PluginSecurityModel, PluginTrust},
    validation::{
        current_platform, current_target, is_safe_relative_path, is_supported_schema,
        is_valid_plugin_id, validate_action_type, validate_declarative_action,
        validate_http_config, ManifestError, MAX_DESCRIPTOR_BYTES, MAX_LIST_ENTRIES,
        MAX_MANIFEST_BYTES,
    },
};
use crate::{TIKTOOLS_PLUGIN_ABI_VERSION, TIKTOOLS_PLUGIN_PROTOCOL_VERSION};

impl PluginManifest {
    pub const fn security_model(&self) -> PluginSecurityModel {
        self.runtime.security_model()
    }

    pub fn from_json_str(input: &str) -> Result<Self, ManifestError> {
        if input.len() > MAX_MANIFEST_BYTES {
            return Err(ManifestError::TooLarge);
        }
        let value: Value = serde_json::from_str(input)?;
        Self::from_value(value)
    }

    pub fn from_value(value: Value) -> Result<Self, ManifestError> {
        let object = value.as_object().ok_or(ManifestError::NotAnObject)?;
        let schema_version =
            number(object, "schemaVersion").ok_or(ManifestError::MissingField("schemaVersion"))?;
        if !is_supported_schema(schema_version) {
            return Err(ManifestError::UnsupportedSchema(schema_version));
        }
        let is_v3 = schema_version > 2;

        let id = required_string(object, "id")?;
        if !is_valid_plugin_id(&id) {
            return Err(ManifestError::InvalidField("id"));
        }
        let name = required_string(object, "name")?;
        if name.trim().is_empty() || name.len() > 256 {
            return Err(ManifestError::InvalidField("name"));
        }
        let version = required_string(object, "version")?;
        if version.trim().is_empty()
            || version.len() > 128
            || version.chars().any(char::is_whitespace)
        {
            return Err(ManifestError::InvalidField("version"));
        }
        let description = optional_string(object, "description");
        if description
            .as_deref()
            .is_some_and(|value| value.len() > 4_096)
        {
            return Err(ManifestError::InvalidField("description"));
        }

        let runtime = optional_string(object, "runtime")
            .and_then(|value| PluginRuntimeKind::parse(&value))
            .ok_or(ManifestError::MissingField("runtime"))?;
        // Declarative packages are interpreted data with nothing to execute,
        // so the entry is optional and defaults to empty. Every other runtime
        // keeps the historical required-entry rule.
        let entry = match optional_string(object, "entry") {
            Some(entry) => entry,
            None if runtime == PluginRuntimeKind::Declarative => String::new(),
            None => return Err(ManifestError::MissingField("entry")),
        };
        if (!entry.is_empty() || runtime != PluginRuntimeKind::Declarative)
            && !is_safe_relative_path(&entry)
        {
            return Err(ManifestError::UnsafeEntry);
        }

        let default_trust = PluginTrust::default_for_runtime(runtime);
        let trust = match optional_string(object, "trust") {
            None => default_trust,
            Some(value) => match value.as_str() {
                "trusted" => PluginTrust::Trusted,
                "sandboxed" => PluginTrust::Sandboxed,
                "untrusted" => PluginTrust::Untrusted,
                _ => return Err(ManifestError::InvalidField("trust")),
            },
        };

        let capabilities = string_list(object, "capabilities")?;
        let permissions = string_list(object, "permissions")?;
        let targets = string_list(object, "targets")?;
        let protocol_version =
            number(object, "protocolVersion").unwrap_or(TIKTOOLS_PLUGIN_PROTOCOL_VERSION);
        let abi_version = number(object, "abiVersion");
        if protocol_version == 0 {
            return Err(ManifestError::InvalidField("protocolVersion"));
        }
        if runtime == PluginRuntimeKind::Native && abi_version.is_some_and(|version| version == 0) {
            return Err(ManifestError::InvalidField("abiVersion"));
        }

        let action_types = json_list(object, "actionTypes")?;
        for action_type in &action_types {
            validate_action_type(action_type)?;
            // Declarative action blocks are only meaningful on schema v3; on
            // v2 the same keys keep their historical pass-through meaning for
            // process plugins, so they are neither validated nor interpreted.
            if is_v3
                && action_type.as_object().is_some_and(|descriptor| {
                    descriptor.contains_key("http") || descriptor.contains_key("optionSources")
                })
            {
                validate_declarative_action(action_type)?;
            }
        }
        let event_types = json_list(object, "eventTypes")?;
        let processor_types = json_list(object, "processorTypes")?;
        let (settings_schema, settings_ui_hints) = settings(object)?;
        // Declarative integration blocks exist only on schema v3. A v2
        // manifest carrying the same keys keeps today's behavior: the keys
        // are ignored instead of validated.
        let http = if is_v3 {
            let http = object.get("http").cloned();
            if let Some(http) = http.as_ref() {
                validate_http_config(http)?;
            }
            http
        } else {
            None
        };
        let templates = if is_v3 {
            json_list(object, "templates")?
        } else {
            Vec::new()
        };
        let pages = if is_v3 {
            json_list(object, "pages")?
        } else {
            Vec::new()
        };

        Ok(Self {
            schema_version,
            id,
            name,
            version,
            description,
            runtime,
            entry,
            trust,
            capabilities,
            permissions,
            protocol_version,
            abi_version,
            targets,
            action_types,
            event_types,
            processor_types,
            settings_schema,
            settings_ui_hints,
            http,
            templates,
            pages,
        })
    }

    pub fn validate_compatibility(&self) -> Result<(), ManifestError> {
        if !is_supported_schema(self.schema_version) {
            return Err(ManifestError::UnsupportedSchema(self.schema_version));
        }
        if self.protocol_version != TIKTOOLS_PLUGIN_PROTOCOL_VERSION {
            return Err(ManifestError::InvalidField("protocolVersion"));
        }
        if self.runtime == PluginRuntimeKind::Native
            && self
                .abi_version
                .is_some_and(|version| version != TIKTOOLS_PLUGIN_ABI_VERSION)
        {
            return Err(ManifestError::InvalidField("abiVersion"));
        }
        for action_type in &self.action_types {
            validate_action_type(action_type)?;
        }
        Ok(())
    }

    pub fn target_matches_current_platform(&self) -> bool {
        if self.targets.is_empty() {
            return true;
        }
        let target = current_target();
        let platform = current_platform();
        self.targets
            .iter()
            .any(|candidate| candidate == &target || candidate == &platform)
    }
}
fn required_string(
    object: &Map<String, Value>,
    key: &'static str,
) -> Result<String, ManifestError> {
    optional_string(object, key).ok_or(ManifestError::MissingField(key))
}

fn optional_string(object: &Map<String, Value>, key: &str) -> Option<String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn number(object: &Map<String, Value>, key: &str) -> Option<u32> {
    object
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
}

fn string_list(
    object: &Map<String, Value>,
    key: &'static str,
) -> Result<Vec<String>, ManifestError> {
    object
        .get(key)
        .map(|value| string_list_value(value, key))
        .transpose()
        .map(|value| value.unwrap_or_default())
}

fn string_list_value(value: &Value, key: &'static str) -> Result<Vec<String>, ManifestError> {
    let entries = value.as_array().ok_or(ManifestError::InvalidField(key))?;
    if entries.len() > MAX_LIST_ENTRIES {
        return Err(ManifestError::InvalidField(key));
    }
    entries
        .iter()
        .map(|entry| {
            let value = entry.as_str().ok_or(ManifestError::InvalidField(key))?;
            if value.is_empty() || value.len() > 256 || value.chars().any(char::is_whitespace) {
                return Err(ManifestError::InvalidField(key));
            }
            Ok(value.to_owned())
        })
        .collect()
}

fn json_list(object: &Map<String, Value>, key: &'static str) -> Result<Vec<Value>, ManifestError> {
    let Some(value) = object.get(key) else {
        return Ok(Vec::new());
    };
    let entries = value.as_array().ok_or(ManifestError::InvalidField(key))?;
    if entries.len() > MAX_LIST_ENTRIES
        || entries.iter().any(|entry| {
            serde_json::to_vec(entry)
                .map(|bytes| bytes.len() > MAX_DESCRIPTOR_BYTES)
                .unwrap_or(true)
        })
    {
        return Err(ManifestError::InvalidField(key));
    }
    Ok(entries.clone())
}

fn settings(object: &Map<String, Value>) -> Result<(Option<Value>, Option<Value>), ManifestError> {
    let Some(settings) = object.get("settings") else {
        return Ok((None, None));
    };
    let settings = settings
        .as_object()
        .ok_or(ManifestError::InvalidField("settings"))?;
    let schema = settings.get("schema").cloned();
    let ui_hints = settings.get("uiHints").cloned();
    if schema.as_ref().is_some_and(|value| !value.is_object())
        || ui_hints.as_ref().is_some_and(|value| !value.is_object())
    {
        return Err(ManifestError::InvalidField("settings"));
    }
    for value in [schema.as_ref(), ui_hints.as_ref()].into_iter().flatten() {
        if serde_json::to_vec(value)
            .map(|bytes| bytes.len() > MAX_DESCRIPTOR_BYTES)
            .unwrap_or(true)
        {
            return Err(ManifestError::InvalidField("settings"));
        }
    }
    Ok((schema, ui_hints))
}
