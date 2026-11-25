//! Explicit host capability checks for runtime plugins.
//!
//! This broker is the policy boundary for process/WASM plugins. Trusted
//! native plugins can still call OS APIs directly, so their manifest entries
//! document intent and the host API remains the enforceable surface.

use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use tiktools_plugin_api::{
    capabilities::{declares_capability, declares_permission},
    manifest::is_valid_plugin_id,
    PluginManifest,
};
use tiktools_plugin_store::{Collection, Expect, Store, StoreError};

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

/// Overlays schema `default` entries for keys the stored settings omit.
/// Pure: storage is untouched. Only scalar defaults apply (settings are a
/// flat scalar map), and secret keys are never defaulted, so absent secrets
/// stay absent and the UI can tell "not configured" apart from "configured".
pub fn apply_settings_defaults(manifest: &PluginManifest, values: &Value) -> Value {
    let properties = manifest
        .settings_schema
        .as_ref()
        .and_then(|schema| schema.get("properties"))
        .and_then(Value::as_object);
    let Some(properties) = properties else {
        return values.clone();
    };
    let mut merged = values.as_object().cloned().unwrap_or_default();
    for (key, field) in properties {
        if merged.contains_key(key) {
            continue;
        }
        let secret = field
            .get("secret")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if secret {
            continue;
        }
        let Some(default) = field.get("default") else {
            continue;
        };
        if default.is_string() || default.is_number() || default.is_boolean() {
            merged.insert(key.clone(), default.clone());
        }
    }
    Value::Object(merged)
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

/// Projection safe for WebView display. The schema is the UI contract: keys
/// removed by a plugin update stay in storage for compatibility, but they are
/// neither editable nor displayable. This also prevents a formerly secret key
/// from becoming visible when its schema entry is removed.
fn displayable_settings(manifest: &PluginManifest, values: &Value) -> Value {
    let Some(properties) = manifest
        .settings_schema
        .as_ref()
        .and_then(|schema| schema.get("properties"))
        .and_then(Value::as_object)
    else {
        return Value::Object(Default::default());
    };
    let filtered = values
        .as_object()
        .map(|object| {
            object
                .iter()
                .filter(|(key, _)| properties.contains_key(*key))
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect::<serde_json::Map<_, _>>()
        })
        .unwrap_or_default();
    redact_secret_settings(manifest, &Value::Object(filtered))
}

#[derive(Debug, Clone)]
pub struct CapabilityBroker {
    plugin_data_root: PathBuf,
    settings_gate: Arc<Mutex<()>>,
    boot_identity: Option<String>,
}

/// Host-created identity of the settings snapshot. It contains no settings
/// hashes or values, and cannot be reconstructed from plugin-authored JSON.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginSettingsBinding {
    pub(crate) boot_identity: String,
    pub(crate) generation: String,
}

fn new_generation() -> io::Result<String> {
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes).map_err(|error| io::Error::other(error.to_string()))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

/// Persist before publishing the generation or settings. A crash after the
/// generation write but before a settings write invalidates earlier approvals
/// conservatively; it never revives the previous settings identity.
fn atomic_settings_write(directory: &Path, name: &str, bytes: &[u8]) -> io::Result<()> {
    crate::private_file::replace_atomic(&directory.join(name), bytes)
        .map_err(|failure| failure.error)
}

/// Store collection and record holding one plugin's settings.
const SETTINGS_COLLECTION: &str = "plugin-settings";
const SETTINGS_RECORD: &str = "settings";
/// Plain projection of the settings. External plugin processes read this file
/// from their data directory, so the host keeps it equal to the stored values.
const SETTINGS_PROJECTION: &str = "settings.json";
/// Generation file written by hosts before the store; imported once, then removed.
const LEGACY_GENERATION: &str = ".settings-generation";

/// A plugin's settings as stored. The identity that approvals bind to and the
/// values are written as one atomic record, so they can never disagree.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SettingsDocument {
    generation: String,
    values: serde_json::Map<String, Value>,
}

fn store_error(error: StoreError) -> CapabilityError {
    CapabilityError::Io(io::Error::other(error.to_string()))
}

fn settings_collection(directory: &Path) -> Result<Collection<SettingsDocument>, CapabilityError> {
    let store = Store::open(directory.join("store")).map_err(store_error)?;
    store
        .collection::<SettingsDocument>(SETTINGS_COLLECTION)
        .map_err(store_error)
}

fn valid_generation(generation: &str) -> bool {
    generation.len() == 64 && generation.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// Writes the settings projection to the plugin's data directory.
fn project_settings(
    directory: &Path,
    values: &serde_json::Map<String, Value>,
) -> Result<(), CapabilityError> {
    let payload = serde_json::to_vec(values)
        .map_err(|error| CapabilityError::Io(io::Error::new(io::ErrorKind::InvalidData, error)))?;
    atomic_settings_write(directory, SETTINGS_PROJECTION, &payload)?;
    Ok(())
}

/// Values from a `settings.json` written before the store existed. Anything
/// that is not a JSON object reads as empty, as it always has.
fn legacy_settings_values(
    directory: &Path,
) -> Result<serde_json::Map<String, Value>, CapabilityError> {
    match fs::read_to_string(directory.join(SETTINGS_PROJECTION)) {
        Ok(text) => Ok(match serde_json::from_str::<Value>(&text) {
            Ok(Value::Object(values)) => values,
            _ => serde_json::Map::new(),
        }),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(serde_json::Map::new()),
        Err(error) => Err(error.into()),
    }
}

impl CapabilityBroker {
    pub fn new(plugin_data_root: PathBuf) -> Self {
        Self {
            plugin_data_root,
            settings_gate: Arc::new(Mutex::new(())),
            boot_identity: new_generation().ok(),
        }
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
        let _lease = self
            .settings_gate
            .lock()
            .map_err(|_| io::Error::other("plugin settings lease is unavailable"))?;
        self.load_plugin_settings_unlocked(manifest)
    }

    /// The plugin's settings document. On first access it is created from the
    /// legacy `settings.json` when one exists, under a fresh generation.
    fn ensure_settings_document(
        &self,
        manifest: &PluginManifest,
    ) -> Result<SettingsDocument, CapabilityError> {
        let directory = self.ensure_plugin_data_dir(manifest)?;
        let collection = settings_collection(&directory)?;
        if let Some(record) = collection.get(SETTINGS_RECORD).map_err(store_error)? {
            return Ok(record.value);
        }
        // Keep the legacy generation when there is one, so approvals that were
        // valid before the migration stay valid after it.
        let legacy_generation = match fs::read_to_string(directory.join(LEGACY_GENERATION)) {
            Ok(text) if valid_generation(&text) => Some(text),
            _ => None,
        };
        let document = SettingsDocument {
            generation: match legacy_generation {
                Some(generation) => generation,
                None => new_generation()?,
            },
            values: legacy_settings_values(&directory)?,
        };
        collection
            .put(SETTINGS_RECORD, &document, Expect::Absent)
            .map_err(store_error)?;
        match fs::remove_file(directory.join(LEGACY_GENERATION)) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        Ok(document)
    }

    fn load_plugin_settings_unlocked(
        &self,
        manifest: &PluginManifest,
    ) -> Result<Value, CapabilityError> {
        let document = self.ensure_settings_document(manifest)?;
        // Schema defaults apply to every consumer (probes, actions, option
        // sources, display) so first-run behavior matches a saved config.
        Ok(apply_settings_defaults(
            manifest,
            &Value::Object(document.values),
        ))
    }

    /// Hold this through final effect admission. Every host settings writer,
    /// including reset and token provisioning, uses the same lease. Callers
    /// must not await or reenter settings operations in `apply`.
    pub(crate) fn with_plugin_settings_binding<R>(
        &self,
        manifest: &PluginManifest,
        apply: impl FnOnce(PluginSettingsBinding, Value) -> Result<R, String>,
    ) -> Result<R, String> {
        let _lease = self
            .settings_gate
            .lock()
            .map_err(|_| "plugin settings lease is unavailable".to_owned())?;
        let boot_identity = self
            .boot_identity
            .clone()
            .ok_or_else(|| "plugin settings identity is unavailable".to_owned())?;
        let directory = self
            .ensure_plugin_data_dir(manifest)
            .map_err(|error| error.to_string())?;
        let document = self
            .ensure_settings_document(manifest)
            .map_err(|error| error.to_string())?;
        if !valid_generation(&document.generation) {
            return Err("plugin settings generation is invalid".into());
        }
        // External plugin processes read settings.json, so the projection must
        // match the stored values before any effect is admitted.
        project_settings(&directory, &document.values).map_err(|error| error.to_string())?;
        let settings = apply_settings_defaults(manifest, &Value::Object(document.values));
        apply(
            PluginSettingsBinding {
                boot_identity,
                generation: document.generation,
            },
            settings,
        )
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
            .map(|values| displayable_settings(manifest, &values))
    }

    pub fn save_plugin_settings(
        &self,
        manifest: &PluginManifest,
        values: &BTreeMap<String, Value>,
    ) -> Result<Value, CapabilityError> {
        let _lease = self
            .settings_gate
            .lock()
            .map_err(|_| io::Error::other("plugin settings lease is unavailable"))?;
        // A secret that round-trips as the placeholder keeps its stored
        // value; only a changed value overwrites it. Placeholder text is
        // never persisted, so the file cannot fill with bullets.
        let mut merged = values.clone();
        let stored = self
            .load_plugin_settings_unlocked(manifest)
            .unwrap_or_else(|_| Value::Object(Default::default()));
        let stored = stored.as_object().cloned().unwrap_or_default();
        let secrets = secret_setting_keys(manifest);
        for key in &secrets {
            let is_placeholder =
                merged.get(key).and_then(Value::as_str) == Some(SECRET_SETTING_PLACEHOLDER);
            if !is_placeholder {
                continue;
            }
            match stored.get(key) {
                Some(previous) => {
                    merged.insert(key.clone(), previous.clone());
                }
                None => {
                    merged.remove(key);
                }
            }
        }
        // Keys hidden because an update removed them from the schema are not
        // editable, but they remain compatibility storage. Carry them through
        // unrelated saves; declared keys still follow the editor's complete map.
        let declared = manifest
            .settings_schema
            .as_ref()
            .and_then(|schema| schema.get("properties"))
            .and_then(Value::as_object);
        for (key, value) in stored {
            if !merged.contains_key(&key)
                && declared.is_some_and(|properties| !properties.contains_key(&key))
            {
                merged.insert(key, value);
            }
        }
        let directory = self.ensure_plugin_data_dir(manifest)?;
        // One atomic record carries the new identity and the values; the
        // projection follows it. A crash in between leaves the store authoritative.
        let stored_values: serde_json::Map<String, Value> = merged.into_iter().collect();
        let document = SettingsDocument {
            generation: new_generation()?,
            values: stored_values.clone(),
        };
        settings_collection(&directory)?
            .put(SETTINGS_RECORD, &document, Expect::Any)
            .map_err(store_error)?;
        project_settings(&directory, &stored_values)?;
        // The echo overlays defaults so it matches what a later load returns.
        let stored = apply_settings_defaults(manifest, &Value::Object(stored_values));
        Ok(displayable_settings(manifest, &stored))
    }

    pub(crate) fn reset_plugin_settings(
        &self,
        manifest: &PluginManifest,
    ) -> Result<(), CapabilityError> {
        let _lease = self
            .settings_gate
            .lock()
            .map_err(|_| io::Error::other("plugin settings lease is unavailable"))?;
        let directory = self.ensure_plugin_data_dir(manifest)?;
        let document = SettingsDocument {
            generation: new_generation()?,
            values: serde_json::Map::new(),
        };
        settings_collection(&directory)?
            .put(SETTINGS_RECORD, &document, Expect::Any)
            .map_err(store_error)?;
        match fs::remove_file(directory.join(SETTINGS_PROJECTION)) {
            Ok(()) => {
                #[cfg(unix)]
                fs::File::open(directory)?.sync_all()?;
                Ok(())
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
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
    fn settings_identity_is_durable_rotates_on_equal_writes_and_cannot_revive_after_restart() {
        let root =
            std::env::temp_dir().join(format!("tiktools-settings-binding-{}", fastrand::u128(..)));
        let broker = CapabilityBroker::new(root.clone());
        let manifest = manifest();
        let binding = |broker: &CapabilityBroker| {
            broker
                .with_plugin_settings_binding(&manifest, |binding, _| Ok(binding))
                .unwrap()
        };
        let initial = binding(&broker);
        assert_eq!(initial, binding(&broker.clone()));
        let values = BTreeMap::from([("value".to_owned(), Value::from("a"))]);
        broker.save_plugin_settings(&manifest, &values).unwrap();
        let first = binding(&broker);
        broker.save_plugin_settings(&manifest, &values).unwrap();
        let second = binding(&broker);
        assert_ne!(initial.generation, first.generation);
        assert_ne!(first.generation, second.generation);
        let reloaded = binding(&CapabilityBroker::new(root.clone()));
        assert_eq!(second.generation, reloaded.generation);
        assert_ne!(second.boot_identity, reloaded.boot_identity);
        broker.reset_plugin_settings(&manifest).unwrap();
        assert_ne!(second.generation, binding(&broker).generation);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn settings_writer_cannot_cross_final_binding_lease() {
        let root =
            std::env::temp_dir().join(format!("tiktools-settings-lease-{}", fastrand::u128(..)));
        let broker = CapabilityBroker::new(root.clone());
        let manifest = manifest();
        let (started, start_rx) = std::sync::mpsc::channel();
        let (finished, finish_rx) = std::sync::mpsc::channel();
        let mut worker = None;
        broker
            .with_plugin_settings_binding(&manifest, |_, _| {
                let writer = broker.clone();
                let manifest = manifest.clone();
                worker = Some(std::thread::spawn(move || {
                    started.send(()).unwrap();
                    writer
                        .save_plugin_settings(&manifest, &BTreeMap::new())
                        .unwrap();
                    finished.send(()).unwrap();
                }));
                start_rx.recv().unwrap();
                assert!(finish_rx.try_recv().is_err());
                Ok(())
            })
            .unwrap();
        worker.unwrap().join().unwrap();
        finish_rx.recv().unwrap();
        let _ = std::fs::remove_dir_all(root);
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
    fn display_hides_stored_keys_removed_from_the_schema() {
        let root =
            std::env::temp_dir().join(format!("tiktools-legacy-secret-{}", fastrand::u128(..)));
        let broker = CapabilityBroker::new(root.clone());
        let old = secret_manifest();
        broker
            .save_plugin_settings(
                &old,
                &BTreeMap::from([("apiToken".to_owned(), Value::String("tok-123".to_owned()))]),
            )
            .unwrap();
        let updated = PluginManifest::from_json_str(
            r#"{
                "schemaVersion": 3,
                "id": "secret.plugin",
                "name": "Secret",
                "version": "1.0.1",
                "runtime": "declarative",
                "settings": {"schema": {"type": "object", "properties": {
                    "serverUrl": {"type": "string"}
                }}}
            }"#,
        )
        .unwrap();
        let display = broker.load_plugin_settings_for_display(&updated).unwrap();
        assert!(display.get("apiToken").is_none());
        broker
            .save_plugin_settings(
                &updated,
                &BTreeMap::from([(
                    "serverUrl".to_owned(),
                    Value::String("http://localhost:17842".to_owned()),
                )]),
            )
            .unwrap();
        let raw = broker.load_plugin_settings_raw(&updated).unwrap();
        assert_eq!(raw.get("apiToken").and_then(Value::as_str), Some("tok-123"));
        let display = broker.load_plugin_settings_for_display(&updated).unwrap();
        assert!(display.get("apiToken").is_none());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn redacts_only_flagged_secret_keys() {
        let manifest = secret_manifest();
        assert_eq!(secret_setting_keys(&manifest), vec!["apiToken".to_owned()]);
        let redacted = redact_secret_settings(
            &manifest,
            &serde_json::json!({"serverUrl": "http://localhost:17842", "apiToken": "tok-123"}),
        );
        assert_eq!(
            redacted.get("serverUrl").and_then(Value::as_str),
            Some("http://localhost:17842")
        );
        assert_eq!(
            redacted.get("apiToken").and_then(Value::as_str),
            Some(SECRET_SETTING_PLACEHOLDER)
        );
        // Absent secrets stay absent so the UI can show "not configured".
        let redacted = redact_secret_settings(
            &manifest,
            &serde_json::json!({"serverUrl": "http://localhost:17842"}),
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
                        Value::String("http://localhost:17842".to_owned()),
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
                        Value::String("http://127.0.0.1:17842".to_owned()),
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
            Some("http://127.0.0.1:17842")
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

    fn defaulted_manifest() -> PluginManifest {
        PluginManifest::from_json_str(
            r#"{
                "schemaVersion": 3,
                "id": "defaults.plugin",
                "name": "Defaults",
                "version": "1.0.0",
                "runtime": "declarative",
                "settings": {"schema": {"type": "object", "properties": {
                    "serverUrl": {"type": "string", "default": "http://localhost:17842"},
                    "defaultLanguage": {"type": "string", "default": "en"},
                    "playNow": {"type": "boolean", "default": false},
                    "nested": {"type": "object", "default": {"ignored": true}},
                    "apiToken": {"type": "string", "secret": true, "default": "must-not-apply"}
                }}}
            }"#,
        )
        .unwrap()
    }

    #[test]
    fn schema_defaults_fill_missing_settings_but_never_secrets() {
        let root = std::env::temp_dir().join(format!(
            "tiktools-defaults-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let broker = CapabilityBroker::new(root.clone());
        let manifest = defaulted_manifest();
        // First run: nothing stored, defaults surface for display and execution.
        let display = broker.load_plugin_settings_for_display(&manifest).unwrap();
        assert_eq!(
            display.get("serverUrl").and_then(Value::as_str),
            Some("http://localhost:17842")
        );
        assert_eq!(
            display.get("defaultLanguage").and_then(Value::as_str),
            Some("en")
        );
        assert_eq!(display.get("playNow").and_then(Value::as_bool), Some(false));
        assert!(display.get("nested").is_none());
        assert!(display.get("apiToken").is_none());
        // Stored values win; the save echo matches a later load.
        let saved = broker
            .save_plugin_settings(
                &manifest,
                &BTreeMap::from([(
                    "serverUrl".to_owned(),
                    Value::String("http://127.0.0.1:4000".to_owned()),
                )]),
            )
            .unwrap();
        assert_eq!(
            saved.get("serverUrl").and_then(Value::as_str),
            Some("http://127.0.0.1:4000")
        );
        assert_eq!(
            saved.get("defaultLanguage").and_then(Value::as_str),
            Some("en")
        );
        let raw = broker.load_plugin_settings_raw(&manifest).unwrap();
        assert_eq!(
            raw.get("serverUrl").and_then(Value::as_str),
            Some("http://127.0.0.1:4000")
        );
        assert_eq!(
            raw.get("defaultLanguage").and_then(Value::as_str),
            Some("en")
        );
        let _ = std::fs::remove_dir_all(root);
    }
}

#[cfg(test)]
mod settings_store_tests {
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

    fn scratch(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "tiktools-settings-store-{name}-{}",
            fastrand::u128(..)
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn legacy_settings_json_is_imported_into_the_store_with_its_generation() {
        let root = scratch("legacy");
        let plugin_dir = root.join("demo.plugin");
        fs::create_dir_all(&plugin_dir).unwrap();
        let legacy_generation = "ab".repeat(32);
        fs::write(plugin_dir.join(LEGACY_GENERATION), &legacy_generation).unwrap();
        fs::write(
            plugin_dir.join(SETTINGS_PROJECTION),
            br#"{"value":"legacy"}"#,
        )
        .unwrap();
        let broker = CapabilityBroker::new(root.clone());
        let manifest = manifest();
        let binding = broker
            .with_plugin_settings_binding(&manifest, |binding, settings| {
                assert_eq!(settings["value"], "legacy");
                Ok(binding)
            })
            .unwrap();
        assert_eq!(binding.generation, legacy_generation);
        assert!(!plugin_dir.join(LEGACY_GENERATION).exists());
        let projected: Value =
            serde_json::from_slice(&fs::read(plugin_dir.join(SETTINGS_PROJECTION)).unwrap())
                .unwrap();
        assert_eq!(projected, serde_json::json!({"value": "legacy"}));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn saved_settings_live_in_the_store_and_survive_a_restart() {
        let root = scratch("restart");
        let manifest = manifest();
        let values = BTreeMap::from([("value".to_owned(), Value::from("stored"))]);
        CapabilityBroker::new(root.clone())
            .save_plugin_settings(&manifest, &values)
            .unwrap();
        let store = Store::open(root.join("demo.plugin").join("store")).unwrap();
        let record = store
            .collection::<SettingsDocument>(SETTINGS_COLLECTION)
            .unwrap()
            .get(SETTINGS_RECORD)
            .unwrap()
            .expect("record in store");
        assert_eq!(record.value.values["value"], "stored");
        let reloaded = CapabilityBroker::new(root.clone())
            .load_plugin_settings(&manifest)
            .unwrap();
        assert_eq!(reloaded["value"], "stored");
        let projected: Value = serde_json::from_slice(
            &fs::read(root.join("demo.plugin").join(SETTINGS_PROJECTION)).unwrap(),
        )
        .unwrap();
        assert_eq!(projected, serde_json::json!({"value": "stored"}));
        let _ = fs::remove_dir_all(root);
    }
}
