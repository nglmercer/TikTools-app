//! Explicit host capability checks for runtime plugins.
//!
//! This broker is the policy boundary for process/WASM plugins. Trusted
//! native plugins can still call OS APIs directly, so their manifest entries
//! document intent and the host API remains the enforceable surface.

use std::{collections::BTreeMap, fs, io, path::PathBuf};

use serde_json::Value;
use thiserror::Error;
use tiktools_plugin_api::{
    capabilities::{declares_capability, declares_permission},
    manifest::is_valid_plugin_id,
    PluginManifest,
};

#[derive(Debug, Error)]
pub enum CapabilityError {
    #[error("plugin `{plugin_id}` is not permitted to use `{requested}`")]
    PermissionDenied {
        plugin_id: String,
        requested: String,
    },
    #[error("plugin `{plugin_id}` does not declare capability `{requested}`")]
    CapabilityUndeclared {
        plugin_id: String,
        requested: String,
    },
    #[error("invalid plugin id for capability storage: {0}")]
    InvalidPluginId(String),
    #[error("could not create plugin capability directory: {0}")]
    Io(#[from] io::Error),
}

/// Placeholder the host emits for secret settings instead of the stored
/// value. The WebView never sees a token: saving the placeholder back
/// preserves the stored secret, and only a changed value overwrites it.
pub const SECRET_SETTING_PLACEHOLDER: &str = "••••••••";

/// Settings keys flagged `"secret": true` in the manifest settings schema.
/// Unknown or schemaless keys are never treated as secrets.
pub fn secret_setting_keys(manifest: &PluginManifest) -> Vec<String> {
    manifest
        .settings_schema
        .as_ref()
        .and_then(|schema| schema.get("properties"))
        .and_then(Value::as_object)
        .map(|properties| {
            properties
                .iter()
                .filter(|(_, field)| {
                    field
                        .get("secret")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                })
                .map(|(key, _)| key.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// Replaces every present secret value with the placeholder. Pure: the
/// stored settings are untouched, and absent secrets stay absent so the UI
/// can tell "not configured" apart from "configured".
pub fn redact_secret_settings(manifest: &PluginManifest, values: &Value) -> Value {
    let secrets = secret_setting_keys(manifest);
    if secrets.is_empty() {
        return values.clone();
    }
    let mut redacted = values.as_object().cloned().unwrap_or_default();
    for key in secrets {
        if let Some(slot) = redacted.get_mut(&key) {
            if !slot.is_null() {
                *slot = Value::String(SECRET_SETTING_PLACEHOLDER.to_owned());
            }
        }
    }
    Value::Object(redacted)
}

#[derive(Debug, Clone)]
pub struct CapabilityBroker {
    plugin_data_root: PathBuf,
}

impl CapabilityBroker {
    pub fn new(plugin_data_root: PathBuf) -> Self {
        Self { plugin_data_root }
    }

    pub fn plugin_data_root(&self) -> &PathBuf {
        &self.plugin_data_root
    }

    pub fn require_permission(
        &self,
        manifest: &PluginManifest,
        requested: &str,
    ) -> Result<(), CapabilityError> {
        if declares_permission(manifest, requested) {
            return Ok(());
        }
        Err(CapabilityError::PermissionDenied {
            plugin_id: manifest.id.clone(),
            requested: requested.to_owned(),
        })
    }

    pub fn require_capability(
        &self,
        manifest: &PluginManifest,
        requested: &str,
    ) -> Result<(), CapabilityError> {
        if declares_capability(manifest, requested) {
            return Ok(());
        }
        Err(CapabilityError::CapabilityUndeclared {
            plugin_id: manifest.id.clone(),
            requested: requested.to_owned(),
        })
    }

    pub fn ensure_plugin_data_dir(
        &self,
        manifest: &PluginManifest,
    ) -> Result<PathBuf, CapabilityError> {
        if !is_valid_plugin_id(&manifest.id) {
            return Err(CapabilityError::InvalidPluginId(manifest.id.clone()));
        }
        let directory = self.plugin_data_root.join(&manifest.id);
        fs::create_dir_all(&directory)?;
        Ok(directory)
    }

    pub fn load_plugin_settings(
        &self,
        manifest: &PluginManifest,
    ) -> Result<Value, CapabilityError> {
        let path = self.ensure_plugin_data_dir(manifest)?.join("settings.json");
        match fs::read_to_string(path) {
            Ok(value) => {
                Ok(serde_json::from_str(&value)
                    .unwrap_or_else(|_| Value::Object(Default::default())))
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                Ok(Value::Object(Default::default()))
            }
            Err(error) => Err(CapabilityError::Io(error)),
        }
    }

    /// Raw stored values for host-internal use (execution, option fetch,
    /// processor enrichment). Never emit the result to the WebView; use
    /// [`load_plugin_settings_for_display`](Self::load_plugin_settings_for_display).
    pub fn load_plugin_settings_raw(
        &self,
        manifest: &PluginManifest,
    ) -> Result<Value, CapabilityError> {
        self.load_plugin_settings(manifest)
    }

    /// Stored values with secrets replaced by the placeholder. This is the
    /// only settings payload the WebView may receive.
    pub fn load_plugin_settings_for_display(
        &self,
        manifest: &PluginManifest,
    ) -> Result<Value, CapabilityError> {
        self.load_plugin_settings(manifest)
            .map(|values| redact_secret_settings(manifest, &values))
    }

    pub fn save_plugin_settings(
        &self,
        manifest: &PluginManifest,
        values: &BTreeMap<String, Value>,
    ) -> Result<Value, CapabilityError> {
        // A secret that round-trips as the placeholder keeps its stored
        // value; only a changed value overwrites it. Placeholder text is
        // never persisted, so the file cannot fill with bullets.
        let mut merged = values.clone();
        let secrets = secret_setting_keys(manifest);
        if !secrets.is_empty() {
            let stored = self
                .load_plugin_settings(manifest)
                .unwrap_or_else(|_| Value::Object(Default::default()));
            let stored = stored.as_object();
            for key in &secrets {
                let is_placeholder =
                    merged.get(key).and_then(Value::as_str) == Some(SECRET_SETTING_PLACEHOLDER);
                if !is_placeholder {
                    continue;
                }
                match stored.and_then(|object| object.get(key)) {
                    Some(previous) => {
                        merged.insert(key.clone(), previous.clone());
                    }
                    None => {
                        merged.remove(key);
                    }
                }
            }
        }
        let directory = self.ensure_plugin_data_dir(manifest)?;
        let path = directory.join("settings.json");
        let temporary = directory.join("settings.json.tmp");
        let payload = serde_json::to_vec(&merged).map_err(|error| {
            CapabilityError::Io(io::Error::new(io::ErrorKind::InvalidData, error))
        })?;
        fs::write(&temporary, payload)?;
        fs::rename(&temporary, &path)?;
        let stored = Value::Object(merged.into_iter().collect());
        Ok(redact_secret_settings(manifest, &stored))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> PluginManifest {
        PluginManifest::from_json_str(
            r#"{
                "schemaVersion": 2,
                "id": "demo.plugin",
                "name": "Demo",
                "version": "1.0.0",
                "runtime": "process",
                "entry": "index.js",
                "capabilities": ["audio.play"],
                "permissions": ["audio.output"]
            }"#,
        )
        .unwrap()
    }

    #[test]
    fn enforces_declared_capabilities_and_permissions() {
        let broker = CapabilityBroker::new(std::env::temp_dir());
        let manifest = manifest();
        assert!(broker.require_capability(&manifest, "audio.play").is_ok());
        assert!(broker.require_permission(&manifest, "audio.output").is_ok());
        assert!(broker.require_permission(&manifest, "http").is_err());
    }

    fn secret_manifest() -> PluginManifest {
        PluginManifest::from_json_str(
            r#"{
                "schemaVersion": 3,
                "id": "secret.plugin",
                "name": "Secret",
                "version": "1.0.0",
                "runtime": "declarative",
                "settings": {"schema": {"type": "object", "properties": {
                    "serverUrl": {"type": "string"},
                    "apiToken": {"type": "string", "secret": true}
                }}}
            }"#,
        )
        .unwrap()
    }

    #[test]
    fn redacts_only_flagged_secret_keys() {
        let manifest = secret_manifest();
        assert_eq!(secret_setting_keys(&manifest), vec!["apiToken".to_owned()]);
        let redacted = redact_secret_settings(
            &manifest,
            &serde_json::json!({"serverUrl": "http://localhost:3000", "apiToken": "tok-123"}),
        );
        assert_eq!(
            redacted.get("serverUrl").and_then(Value::as_str),
            Some("http://localhost:3000")
        );
        assert_eq!(
            redacted.get("apiToken").and_then(Value::as_str),
            Some(SECRET_SETTING_PLACEHOLDER)
        );
        // Absent secrets stay absent so the UI can show "not configured".
        let redacted = redact_secret_settings(
            &manifest,
            &serde_json::json!({"serverUrl": "http://localhost:3000"}),
        );
        assert!(redacted.get("apiToken").is_none());
    }

    #[test]
    fn saving_a_placeholder_preserves_the_stored_secret() {
        let root = std::env::temp_dir().join(format!(
            "tiktools-secret-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let broker = CapabilityBroker::new(root.clone());
        let manifest = secret_manifest();
        let saved = broker
            .save_plugin_settings(
                &manifest,
                &BTreeMap::from([
                    (
                        "serverUrl".to_owned(),
                        Value::String("http://localhost:3000".to_owned()),
                    ),
                    ("apiToken".to_owned(), Value::String("tok-123".to_owned())),
                ]),
            )
            .unwrap();
        // The save response is already display-safe.
        assert_eq!(
            saved.get("apiToken").and_then(Value::as_str),
            Some(SECRET_SETTING_PLACEHOLDER)
        );
        // Saving the placeholder round-trip keeps the stored token.
        let saved = broker
            .save_plugin_settings(
                &manifest,
                &BTreeMap::from([
                    (
                        "serverUrl".to_owned(),
                        Value::String("http://127.0.0.1:3000".to_owned()),
                    ),
                    (
                        "apiToken".to_owned(),
                        Value::String(SECRET_SETTING_PLACEHOLDER.to_owned()),
                    ),
                ]),
            )
            .unwrap();
        assert_eq!(
            saved.get("apiToken").and_then(Value::as_str),
            Some(SECRET_SETTING_PLACEHOLDER)
        );
        let raw = broker.load_plugin_settings_raw(&manifest).unwrap();
        assert_eq!(raw.get("apiToken").and_then(Value::as_str), Some("tok-123"));
        assert_eq!(
            raw.get("serverUrl").and_then(Value::as_str),
            Some("http://127.0.0.1:3000")
        );
        // A changed value overwrites the secret.
        broker
            .save_plugin_settings(
                &manifest,
                &BTreeMap::from([("apiToken".to_owned(), Value::String("tok-456".to_owned()))]),
            )
            .unwrap();
        let raw = broker.load_plugin_settings_raw(&manifest).unwrap();
        assert_eq!(raw.get("apiToken").and_then(Value::as_str), Some("tok-456"));
        let _ = std::fs::remove_dir_all(root);
    }
}
