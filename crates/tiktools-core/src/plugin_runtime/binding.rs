//! Host-created plugin review/admission bindings. No type in this module can
//! be deserialized from automation source, provenance, grants or requests.
use crate::collaboration::{Capability, Grant, ResourceKind};
use crate::services::PluginSettingsBinding;
use crate::*;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TrustedPluginBinding {
    plugin_id: String,
    action_type: String,
    runtime: String,
    artifact_digest: String,
    manifest_digest: String,
    descriptor: Value,
    declared_capabilities: Vec<String>,
    declared_permissions: Vec<String>,
    catalog_generation: u64,
    activation_generation: u64,
    running_generation: Option<u64>,
    loaded_artifact_digest: Option<String>,
    settings: PluginSettingsBinding,
}

impl TrustedPluginBinding {
    pub(crate) fn plugin_id(&self) -> &str {
        &self.plugin_id
    }
    pub(crate) fn action_type(&self) -> &str {
        &self.action_type
    }
    pub(crate) fn artifact_digest(&self) -> &str {
        &self.artifact_digest
    }
    pub(crate) fn descriptor(&self) -> &Value {
        &self.descriptor
    }

    /// An absent action map is deny-by-default. Merely having the general
    /// execute capability or a plugin-looking type string is insufficient.
    pub(crate) fn check_grants(&self, grants: &[&Grant]) -> Result<(), String> {
        for grant in grants {
            if !grant
                .capabilities
                .contains(&Capability::PluginActionExecute)
                || !grant.allows_resource(ResourceKind::Plugin, &self.plugin_id)
                || !grant
                    .actions
                    .get(&self.plugin_id)
                    .is_some_and(|actions| actions.contains(&self.action_type))
            {
                return Err("plugin action is outside a current subject grant".into());
            }
        }
        Ok(())
    }

    /// Closed scalar argument projection, checked against the exact reviewed
    /// descriptor. Unsupported field kinds need a separate schema policy.
    pub(crate) fn project_arguments(&self, config: &Value) -> Result<Value, String> {
        let config = config
            .as_object()
            .ok_or("plugin arguments must be an object")?;
        if config.len() > 256 || Value::Object(config.clone()).to_string().len() > 64 * 1024 {
            return Err("plugin arguments exceed the admission budget".into());
        }
        let fields = self
            .descriptor
            .get("fields")
            .and_then(Value::as_array)
            .ok_or("plugin action has no declared argument schema")?;
        let mut schema = BTreeMap::new();
        for field in fields {
            let key = field
                .get("key")
                .and_then(Value::as_str)
                .filter(|key| !key.is_empty() && key.len() <= 256)
                .ok_or("plugin action field has no bounded key")?;
            if schema.insert(key, field).is_some() {
                return Err("plugin action argument schema is ambiguous".into());
            }
            if field.get("required").and_then(Value::as_bool) == Some(true)
                && !config.contains_key(key)
            {
                return Err("plugin action is missing a required argument".into());
            }
        }
        let mut projected = serde_json::Map::new();
        for (key, value) in config {
            let field = schema
                .get(key.as_str())
                .ok_or("plugin action contains an undeclared argument")?;
            let kind = field
                .get("kind")
                .or_else(|| field.get("type"))
                .and_then(Value::as_str)
                .ok_or("plugin action field has no scalar type")?;
            let valid = match kind {
                "text" | "textarea" | "string" | "select" | "radio" | "password" | "color" => {
                    value.as_str().is_some_and(|value| value.len() <= 32 * 1024)
                }
                "number" | "slider" | "range" => value.as_f64().is_some_and(|number| {
                    number.is_finite()
                        && field
                            .get("min")
                            .and_then(Value::as_f64)
                            .is_none_or(|min| number >= min)
                        && field
                            .get("max")
                            .and_then(Value::as_f64)
                            .is_none_or(|max| number <= max)
                }),
                "boolean" | "checkbox" | "toggle" | "switch" => value.is_boolean(),
                _ => false,
            };
            if !valid {
                return Err("plugin argument does not satisfy its declared scalar schema".into());
            }
            if let Some(options) = field.get("options").and_then(Value::as_array) {
                if !options.is_empty()
                    && !options
                        .iter()
                        .any(|option| option.get("value").unwrap_or(option) == value)
                {
                    return Err("plugin argument is outside its declared options".into());
                }
            }
            projected.insert(key.clone(), value.clone());
        }
        Ok(Value::Object(projected))
    }

    /// Host-supplied settings are the conjunctive intersection of explicit
    /// per-plugin field grants and the installed settings schema. Secrets have
    /// no ambient/default access; an explicitly named secret field also needs
    /// plugins.settings.read from every subject. Native OS access is separate.
    pub(crate) fn project_settings(
        &self,
        manifest: &tiktools_plugin_api::PluginManifest,
        settings: &Value,
        grants: &[&Grant],
    ) -> Value {
        let mut projected = serde_json::Map::new();
        let Some(properties) = manifest
            .settings_schema
            .as_ref()
            .and_then(|schema| schema.get("properties"))
            .and_then(Value::as_object)
        else {
            return Value::Object(projected);
        };
        if grants.is_empty() {
            return Value::Object(projected);
        }
        for key in properties.keys() {
            if grants.iter().all(|grant| {
                grant.capabilities.contains(&Capability::PluginSettingsRead)
                    && grant
                        .settings_fields
                        .get(&self.plugin_id)
                        .is_some_and(|fields| fields.contains(key))
            }) {
                if let Some(value) = settings.get(key).filter(|value| {
                    matches!(value, Value::Null | Value::Bool(_) | Value::Number(_))
                        || value.as_str().is_some_and(|value| value.len() <= 32 * 1024)
                }) {
                    projected.insert(key.clone(), value.clone());
                }
            }
        }
        Value::Object(projected)
    }
}

impl AppCore {
    /// Lock order: plugin invocation gate -> loader lifecycle/catalog lease ->
    /// settings writer lease -> activation read. The caller may then acquire
    /// its automation/business writer and final subject/index fence. No await,
    /// settings reentry or plugin lifecycle call may occur inside `apply`.
    pub(crate) fn with_trusted_plugin_binding<R>(
        &self,
        action_type: &str,
        apply: impl FnOnce(
            TrustedPluginBinding,
            tiktools_plugin_loader::DiscoveredPlugin,
            Value,
        ) -> Result<R, String>,
    ) -> Result<R, String> {
        let _invocation = self
            .plugin_state
            .invocation_gate
            .lock()
            .map_err(|_| "plugin invocation lease is unavailable".to_owned())?;
        self.plugins
            .with_catalog_lease(|manager| {
                let mut matches = manager.list().into_iter().flat_map(|plugin| {
                    let descriptors: Vec<_> = plugin
                        .manifest
                        .action_types
                        .iter()
                        .filter(|descriptor| {
                            descriptor.get("id").and_then(Value::as_str) == Some(action_type)
                        })
                        .cloned()
                        .collect();
                    descriptors
                        .into_iter()
                        .map(move |descriptor| (plugin.clone(), descriptor))
                });
                let (plugin, descriptor) =
                    matches.next().ok_or("plugin action is not installed")?;
                if matches.next().is_some() {
                    return Err("plugin action ownership is ambiguous".into());
                }
                if !plugin.available || !self.plugin_ready(&plugin.manifest.id) {
                    return Err("plugin action is not installed, enabled and available".into());
                }
                let disk_manifest = std::fs::read(plugin.directory.join("plugin.json"))
                    .ok()
                    .filter(|bytes| bytes.len() <= 256 * 1024)
                    .and_then(|bytes| String::from_utf8(bytes).ok())
                    .and_then(|text| tiktools_plugin_api::PluginManifest::from_json_str(&text).ok())
                    .and_then(|manifest| serde_json::to_value(manifest).ok())
                    .ok_or("plugin package manifest could not be verified")?;
                let manifest = serde_json::to_value(&plugin.manifest)
                    .map_err(|_| "plugin manifest binding could not be encoded".to_owned())?;
                if disk_manifest != manifest {
                    return Err("plugin manifest changed without a trusted catalog refresh".into());
                }
                let parsed: super::PluginActionDescriptor =
                    serde_json::from_value(descriptor.clone())
                        .map_err(|_| "plugin action descriptor is invalid".to_owned())?;
                for capability in &parsed.required_capabilities {
                    self.capabilities
                        .require_capability(&plugin.manifest, capability)
                        .map_err(|error| error.to_string())?;
                }
                let artifact_digest = tiktools_plugin_loader::plugin_artifact_fingerprint(&plugin)
                    .map_err(|error| error.to_string())?;
                let manifest_digest = crate::collaboration::hash_secret(&format!(
                    "tiktools/plugin-manifest/v1\0{manifest}"
                ));
                let settings_manifest = plugin.manifest.clone();
                self.capabilities.with_plugin_settings_binding(
                    &settings_manifest,
                    |settings, values| {
                        let activation = self
                            .plugin_state
                            .activation
                            .read()
                            .map_err(|_| "plugin activation lease is unavailable".to_owned())?;
                        if activation
                            .get(&plugin.manifest.id)
                            .is_some_and(|state| !state.installed || !state.enabled)
                        {
                            return Err("plugin action activation changed".into());
                        }
                        let binding = TrustedPluginBinding {
                            plugin_id: plugin.manifest.id.clone(),
                            action_type: action_type.to_owned(),
                            runtime: plugin.manifest.runtime.to_string(),
                            artifact_digest,
                            manifest_digest,
                            descriptor,
                            declared_capabilities: plugin.manifest.capabilities.clone(),
                            declared_permissions: plugin.manifest.permissions.clone(),
                            catalog_generation: manager.registry_revision(),
                            activation_generation: self
                                .plugin_state
                                .activation_revision
                                .load(Ordering::Acquire),
                            running_generation: manager.running_generation(&plugin.manifest.id),
                            loaded_artifact_digest: manager
                                .running_artifact_fingerprint(&plugin.manifest.id),
                            settings,
                        };
                        apply(binding, plugin, values)
                    },
                )
            })
            .map_err(|error| error.to_string())?
    }
}

#[cfg(test)]
mod tests;
