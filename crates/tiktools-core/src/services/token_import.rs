//! Host-owned credential setup. Discovery reads metadata; secrets are read only
//! after the user chooses a discovered source or a native file/folder picker.
use serde_json::Value;
use std::{fs::File, io::Read, path::Path};
use tiktools_plugin_api::PluginManifest;
mod discovery;
use discovery::{discover, folder_config, ImportCandidate};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TokenImportCandidate {
    pub id: String,
    pub label: String,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TokenImportResult {
    pub plugin_id: String,
    pub ok: bool,
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidates: Vec<TokenImportCandidate>,
}
struct ImportedSettings {
    token: String,
    endpoint: Option<(String, String)>,
}
const MAX_CONFIG_BYTES: u64 = 64 * 1024;

pub(crate) fn supports_token_import(manifest: &PluginManifest) -> bool {
    let Some(http) = manifest.http.as_ref() else {
        return false;
    };
    let Some(spec) = http.get("tokenImport") else {
        return false;
    };
    let setting = http
        .pointer("/auth/tokenSetting")
        .and_then(Value::as_str)
        .unwrap_or_default();
    spec.get("format").and_then(Value::as_str) == Some("ini")
        && crate::services::capabilities::secret_setting_keys(manifest)
            .iter()
            .any(|key| key == setting)
}
fn ini_value<'a>(text: &'a str, section: &str, key: &str) -> Result<Option<&'a str>, String> {
    let mut current = "";
    let mut value = None;
    for line in text.trim_start_matches('\u{feff}').lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            current = line[1..line.len() - 1].trim();
            continue;
        }
        if current != section {
            continue;
        }
        if let Some((name, entry)) = line.split_once('=') {
            if name.trim() == key {
                if value.is_some() {
                    return Err(
                        "The configuration has duplicate entries; select an unambiguous file."
                            .to_owned(),
                    );
                }
                value = Some(entry.trim());
            }
        }
    }
    Ok(value)
}
fn parse_ini_token(text: &str, spec: &Value) -> Result<String, String> {
    let section = spec["section"]
        .as_str()
        .ok_or("Invalid token import section.")?;
    let key = spec["key"].as_str().ok_or("Invalid token import key.")?;
    let token = ini_value(text, section, key)?.ok_or(
        "Token entry was not found. Start the game/server once to generate its configuration.",
    )?;
    let min = spec.get("minLength").and_then(Value::as_u64).unwrap_or(1) as usize;
    if token.len() < min
        || token.len() > 4096
        || token.chars().any(|c| c.is_whitespace() || c.is_control())
    {
        return Err("The configuration contains an empty or invalid token. Start the game/server to generate one.".to_owned());
    }
    Ok(token.to_owned())
}
fn read_settings(path: &Path, spec: &Value) -> Result<ImportedSettings, String> {
    if !std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.len() <= MAX_CONFIG_BYTES) {
        return Err("Select a regular configuration file smaller than 64 KiB.".to_owned());
    }
    let file = File::open(path).map_err(|_| "Could not open the selected configuration.")?;
    if !file
        .metadata()
        .is_ok_and(|m| m.is_file() && m.len() <= MAX_CONFIG_BYTES)
    {
        return Err("Select a regular configuration file smaller than 64 KiB.".to_owned());
    }
    let mut bytes = Vec::new();
    file.take(MAX_CONFIG_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Could not read the configuration.")?;
    if bytes.len() as u64 > MAX_CONFIG_BYTES {
        return Err("Configuration exceeds 64 KiB.".to_owned());
    }
    let text = std::str::from_utf8(&bytes).map_err(|_| "Configuration must be UTF-8 text.")?;
    let token = parse_ini_token(text, spec)?;
    let endpoint = if let Some(connection) = spec.get("connection") {
        let section = connection["section"]
            .as_str()
            .ok_or("Invalid connection section.")?;
        let host = ini_value(
            text,
            section,
            connection["hostKey"].as_str().ok_or("Invalid host key.")?,
        )?;
        let port = ini_value(
            text,
            section,
            connection["portKey"].as_str().ok_or("Invalid port key.")?,
        )?;
        match (host, port) {
            (None, None) => None,
            (Some(host), Some(port)) => {
                let host = match host {
                    "localhost" | "127.0.0.1" | "0.0.0.0" => "127.0.0.1",
                    "::1" | "::" => "[::1]",
                    _ => return Err("Only loopback or wildcard API listen addresses can be imported automatically.".to_owned()),
                };
                let port = port
                    .parse::<u16>()
                    .ok()
                    .filter(|p| *p != 0)
                    .ok_or("The API port is invalid.")?;
                Some((
                    connection["setting"]
                        .as_str()
                        .ok_or("Invalid URL setting.")?
                        .to_owned(),
                    format!("http://{host}:{port}"),
                ))
            }
            _ => return Err("The configuration must specify both API host and port.".to_owned()),
        }
    } else {
        None
    };
    Ok(ImportedSettings { token, endpoint })
}
#[cfg(test)]
fn read_token(path: &Path, spec: &Value) -> Result<String, String> {
    read_settings(path, spec).map(|s| s.token)
}

impl crate::AppCore {
    /// Backward-compatible file import operation.
    pub async fn plugin_token_import(
        self: &std::sync::Arc<Self>,
        id: &str,
    ) -> Result<crate::control::PluginProvisionResult, crate::control::OperationError> {
        let result = self.plugin_token_setup(id, "file", None).await?;
        Ok(crate::control::PluginProvisionResult {
            plugin_id: result.plugin_id,
            ok: result.ok,
            error: result.error,
        })
    }
    /// Auto returns source descriptors only. Candidate imports re-discover and
    /// verify the opaque ID; callers cannot provide arbitrary filesystem paths.
    pub async fn plugin_token_setup(
        self: &std::sync::Arc<Self>,
        id: &str,
        mode: &str,
        candidate_id: Option<&str>,
    ) -> Result<TokenImportResult, crate::control::OperationError> {
        use crate::control::OperationError;
        use tiktools_plugin_api::{MediaKind, MediaPickerMode, MediaPickerOptions};
        let plugin = self.require_discovered(id)?;
        let id = plugin.manifest.id.clone();
        let result = |ok, error| TokenImportResult {
            plugin_id: id.clone(),
            ok,
            error,
            candidates: Vec::new(),
        };
        let fail = |message: &str| result(false, Some(message.to_owned()));
        if !self.plugin_ready(&id) || !supports_token_import(&plugin.manifest) {
            return Ok(fail(
                "Enable a plugin declaring supported secret token import first.",
            ));
        }
        if !matches!(mode, "file" | "folder" | "auto" | "candidate") {
            return Err(OperationError::invalid("Unknown token setup mode."));
        }
        if mode == "candidate"
            && candidate_id
                .is_none_or(|s| s.len() != 16 || !s.bytes().all(|b| b.is_ascii_hexdigit()))
        {
            return Err(OperationError::invalid(
                "Invalid configuration candidate ID.",
            ));
        }
        static PICKER: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);
        let Ok(_permit) = PICKER.try_acquire() else {
            return Ok(fail("Another token import is already in progress."));
        };
        let spec = plugin.manifest.http.as_ref().unwrap()["tokenImport"].clone();
        if mode != "file" && spec.get("discovery").is_none() {
            return Ok(fail(
                "This plugin does not support automatic or folder setup.",
            ));
        }
        let selected = if mode == "auto" || mode == "candidate" {
            let scan_spec = spec.clone();
            let found =
                match tokio::task::spawn_blocking(move || discover(&scan_spec)).await {
                    Ok(Ok(found)) => found,
                    _ => return Ok(fail(
                        "Could not search known game locations. Select the game folder instead.",
                    )),
                };
            if mode == "auto" {
                if found.is_empty() {
                    return Ok(fail("No configuration found in known locations. Start the game once, then select its game or mod profile folder."));
                }
                return Ok(TokenImportResult {
                    plugin_id: id,
                    ok: false,
                    error: None,
                    candidates: found.iter().map(ImportCandidate::descriptor).collect(),
                });
            }
            match found
                .into_iter()
                .find(|candidate| Some(candidate.id.as_str()) == candidate_id)
            {
                Some(candidate) => candidate.path,
                None => return Ok(fail(
                    "This configuration is no longer available. Search again or select its folder.",
                )),
            }
        } else {
            let folder = mode == "folder";
            let name = spec["fileName"].as_str().unwrap_or("configuration");
            let selected=match self.media.open_picker(MediaPickerOptions {
                mode:if folder {MediaPickerMode::Directory} else {MediaPickerMode::File}, kind:MediaKind::Other,
                title:Some(if folder {"Select the game installation or active mod profile folder".to_owned()} else {format!("Import token — select {name}")}),
                extensions:if folder {Vec::new()} else {vec!["cfg".to_owned(),"ini".to_owned(),"conf".to_owned()]}, ..Default::default()
            }).await {
                Ok(Some(path))=>path,
                Ok(None)=>return Ok(result(false,None)),
                Err(_)=>return Ok(fail("The native file picker is unavailable. Use the desktop app or enter the token manually.")),
            };
            if folder {
                match folder_config(&selected,&spec) {Some(path)=>path, None=>return Ok(fail("No configuration found in that folder. Select the game installation or active mod profile; start the game once to generate its configuration."))}
            } else {
                selected
            }
        };
        let read_spec = spec.clone();
        let imported =
            match tokio::task::spawn_blocking(move || read_settings(&selected, &read_spec)).await {
                Ok(Ok(settings)) => settings,
                Ok(Err(error)) => return Ok(fail(&error)),
                Err(_) => return Ok(fail("Could not read the selected configuration.")),
            };
        let current = self.require_discovered(&id)?;
        if !self.plugin_ready(&id)
            || current.manifest.http != plugin.manifest.http
            || !supports_token_import(&current.manifest)
        {
            return Ok(fail(
                "Plugin configuration changed while selecting the file. Try importing again.",
            ));
        }
        let setting = current
            .manifest
            .http
            .as_ref()
            .unwrap()
            .pointer("/auth/tokenSetting")
            .and_then(Value::as_str)
            .unwrap();
        let settings = self
            .capabilities
            .load_plugin_settings_raw(&current.manifest)
            .map_err(|_| OperationError::internal("Could not load plugin settings."))?;
        let mut values: std::collections::BTreeMap<String, Value> = settings
            .as_object()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect();
        values.insert(setting.to_owned(), Value::String(imported.token));
        if let Some((setting, url)) = imported.endpoint {
            let property = current
                .manifest
                .settings_schema
                .as_ref()
                .and_then(|s| s.get("properties"))
                .and_then(|p| p.get(&setting));
            if property
                .and_then(|p| p.get("format"))
                .and_then(Value::as_str)
                != Some("uri")
            {
                return Ok(fail("The imported URL must target a declared URI setting."));
            }
            values.insert(setting, Value::String(url));
        }
        self.plugin_settings_save(&id, values).map_err(|_| {
            OperationError::internal("Could not securely save the imported settings.")
        })?;
        Ok(result(true, None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn spec() -> Value {
        json!({"section":"HTTP", "key":"AuthenticationToken", "minLength":43})
    }
    #[test]
    fn bepinex_ini_preserves_base64_padding_and_ignores_other_sections() {
        let token = "x".repeat(43) + "=";
        let text = format!("\u{feff}# BepInEx\r\n[Other]\r\nAuthenticationToken = ignored\r\n[HTTP]\r\n## secret\r\nAuthenticationToken = {token}\r\n");
        assert_eq!(parse_ini_token(&text, &spec()).unwrap(), token);
    }
    #[test]
    fn errors_never_echo_configuration_or_credentials() {
        for text in [
            "[HTTP]\nAuthenticationToken=secret",
            "[HTTP]\nAuthenticationToken=",
            "[HTTP]\nAuthenticationToken=private one",
            "[Other]\nAuthenticationToken=private",
            "[HTTP]\nAuthenticationToken=private\nAuthenticationToken=private",
        ] {
            let error = parse_ini_token(text, &spec()).unwrap_err();
            assert!(!error.contains("private") && !error.contains("secret"));
        }
    }
    #[test]
    fn only_repo_opts_in_and_requires_a_secret_destination() {
        let mut doc: Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/plugins/repo-game.plugin.json"
        ))
        .unwrap();
        let manifest = PluginManifest::from_json_str(&doc.to_string()).unwrap();
        assert!(supports_token_import(&manifest));
        doc["settings"]["schema"]["properties"]["apiToken"]["secret"] = json!(false);
        assert!(!supports_token_import(
            &PluginManifest::from_json_str(&doc.to_string()).unwrap()
        ));
        for source in [
            include_str!("../../../../tests/fixtures/plugins/sonicboom-server.plugin.json"),
            include_str!("../../../../tests/fixtures/plugins/minecraft-server.plugin.json"),
        ] {
            assert!(!supports_token_import(
                &PluginManifest::from_json_str(source).unwrap()
            ));
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::{MediaHost, MediaHostError, MediaHostFuture};
    use std::{
        path::PathBuf,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc, Mutex,
        },
    };
    use tiktools_plugin_api::{
        AudioPlayOptions, AudioPlaybackResult, MediaFileRef, MediaPickerOptions,
    };

    struct Picker {
        path: Mutex<Option<PathBuf>>,
        calls: AtomicUsize,
    }
    impl MediaHost for Picker {
        fn open_picker(&self, options: MediaPickerOptions) -> MediaHostFuture<Option<PathBuf>> {
            assert_eq!(options.kind, tiktools_plugin_api::MediaKind::Other);
            if options.mode == tiktools_plugin_api::MediaPickerMode::File {
                assert!(options.title.unwrap().contains("org.repo.rest-api.cfg"));
            }
            self.calls.fetch_add(1, Ordering::SeqCst);
            let path = self.path.lock().unwrap().clone();
            Box::pin(async move { Ok(path) })
        }
        fn play_audio(
            &self,
            _: MediaFileRef,
            _: AudioPlayOptions,
        ) -> MediaHostFuture<AudioPlaybackResult> {
            Box::pin(async { Err(MediaHostError::Failed("not used".to_owned())) })
        }
    }
    struct Emitter;
    impl crate::HostEmitter for Emitter {
        fn emit(&self, _: crate::ipc::messages::HostMessage) {}
    }

    #[tokio::test]
    async fn native_import_persists_only_secret_and_preserves_settings_on_cancel_or_error() {
        let root = std::env::temp_dir().join(format!(
            "tiktools-import-{}-{}",
            std::process::id(),
            fastrand::u64(..)
        ));
        let package = root.join("packages/repo.game");
        std::fs::create_dir_all(&package).unwrap();
        std::fs::write(
            package.join("plugin.json"),
            include_str!("../../../../tests/fixtures/plugins/repo-game.plugin.json"),
        )
        .unwrap();
        let config = root.join("org.repo.rest-api.cfg");
        let token = "fixture".repeat(7) + "==";
        std::fs::write(&config, format!("[HTTP]\nAuthenticationToken = {token}\n")).unwrap();
        let picker = Arc::new(Picker {
            path: Mutex::new(Some(config.clone())),
            calls: AtomicUsize::new(0),
        });
        let manager =
            tiktools_plugin_loader::PluginManager::new(vec![tiktools_plugin_loader::PluginRoot {
                path: root.join("packages"),
                source: tiktools_plugin_loader::PluginSource::Development,
            }]);
        manager.scan().unwrap();
        let mut core = crate::AppCore::with_media_host(Arc::new(Emitter), picker.clone());
        core.plugins = Arc::new(manager);
        core.capabilities = Arc::new(crate::services::capabilities::CapabilityBroker::new(
            root.join("data"),
        ));
        let core = Arc::new(core);
        core.set_plugin_activation("repo.game", true, true);
        core.plugin_settings_save(
            "repo.game",
            [("quantity".to_owned(), serde_json::json!(3))].into(),
        )
        .unwrap();
        let result = core.plugin_token_import("repo.game").await.unwrap();
        assert!(result.ok, "{result:?}");
        assert!(!serde_json::to_string(&result).unwrap().contains(&token));
        let display = core.plugin_settings("repo.game").unwrap();
        assert_eq!(display.values["quantity"], 3);
        assert_eq!(
            display.values["apiToken"],
            crate::services::capabilities::SECRET_SETTING_PLACEHOLDER
        );
        let manifest = &core.plugins.get("repo.game").unwrap().manifest;
        assert_eq!(
            core.capabilities
                .load_plugin_settings_raw(manifest)
                .unwrap()["apiToken"],
            token
        );
        // Autosaving a masked value after import must keep the credential.
        core.plugin_settings_save("repo.game", display.values)
            .unwrap();
        assert_eq!(
            core.capabilities
                .load_plugin_settings_raw(manifest)
                .unwrap()["apiToken"],
            token
        );
        *picker.path.lock().unwrap() = None;
        let cancelled = core.plugin_token_import("repo.game").await.unwrap();
        assert!(!cancelled.ok && cancelled.error.is_none());
        *picker.path.lock().unwrap() = Some(config.clone());
        std::fs::write(
            &config,
            "[HTTP]\nAuthenticationToken = invalid-private-value\n",
        )
        .unwrap();
        let invalid = core.plugin_token_import("repo.game").await.unwrap();
        assert!(!invalid.ok);
        assert!(!invalid.error.unwrap().contains("invalid-private-value"));
        assert_eq!(
            core.capabilities
                .load_plugin_settings_raw(manifest)
                .unwrap()["apiToken"],
            token
        );
        std::fs::write(
            &config,
            format!("[HTTP]\nAuthenticationToken = {token}\nHost = 0.0.0.0\nPort = 18053\n"),
        )
        .unwrap();
        *picker.path.lock().unwrap() = Some(root.clone());
        let folder = core
            .plugin_token_setup("repo.game", "folder", None)
            .await
            .unwrap();
        assert!(folder.ok, "{folder:?}");
        assert_eq!(
            core.plugin_settings("repo.game").unwrap().values["serverUrl"],
            "http://127.0.0.1:18053"
        );
        assert!(core
            .plugin_token_setup("repo.game", "candidate", Some("../../secret"))
            .await
            .is_err());
        core.set_plugin_activation("repo.game", true, false);
        let before = picker.calls.load(Ordering::SeqCst);
        assert!(!core.plugin_token_import("repo.game").await.unwrap().ok);
        assert_eq!(picker.calls.load(Ordering::SeqCst), before);
        // Oversized and missing files fail without echoing file contents/paths.
        std::fs::write(&config, vec![b'x'; MAX_CONFIG_BYTES as usize + 1]).unwrap();
        assert!(read_token(
            &config,
            &serde_json::json!({"section":"HTTP","key":"AuthenticationToken"})
        )
        .is_err());
        let _ = std::fs::remove_dir_all(root);
    }
}

#[cfg(test)]
mod endpoint_tests {
    use super::*;
    #[test]
    fn imports_loopback_ports_and_rejects_remote_endpoints_without_echoing_credentials() {
        let root = std::env::temp_dir().join(format!("connection-import-{}", fastrand::u64(..)));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("config.cfg");
        let spec = serde_json::json!({"section":"HTTP","key":"Token","connection":{"setting":"serverUrl","section":"HTTP","hostKey":"Host","portKey":"Port"}});
        for (host, expected) in [
            ("127.0.0.1", "http://127.0.0.1:19000"),
            ("localhost", "http://127.0.0.1:19000"),
            ("0.0.0.0", "http://127.0.0.1:19000"),
            ("::", "http://[::1]:19000"),
        ] {
            std::fs::write(
                &path,
                format!("[HTTP]\nToken=test\nHost={host}\nPort=19000"),
            )
            .unwrap();
            assert_eq!(
                read_settings(&path, &spec).unwrap().endpoint.unwrap().1,
                expected
            );
        }
        for (host, port) in [
            ("remote.example", "19000"),
            ("127.0.0.1", "0"),
            ("127.0.0.1", "65536"),
            ("127.0.0.1", "abc"),
        ] {
            std::fs::write(
                &path,
                format!("[HTTP]\nToken=private-fixture\nHost={host}\nPort={port}"),
            )
            .unwrap();
            let error = read_settings(&path, &spec).err().unwrap();
            assert!(!error.contains("private-fixture"));
        }
        let _ = std::fs::remove_dir_all(root);
    }
}
