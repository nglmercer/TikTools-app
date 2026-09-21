//! Host-settings loader for the backend.
//!
//! The isolated UI persists through the broker into the host's
//! `<plugin-data>/<plugin-id>/settings.json`; the backend reads that same
//! file (never the UI, never the network) so one settings object drives
//! both. The host exports the resolved `TIKTOOLS_PLUGIN_DATA_DIR` at
//! startup and the loader passes `TIKTOOLS_PLUGIN_ID`, so the path below
//! is exact — no search, no guessing. Any failure (missing file,
//! malformed JSON) falls back to defaults, which keep speech disabled.

use std::path::PathBuf;

use super::policy::{sanitize_settings, TtsSettings};

pub const DEFAULT_SERVER_URL: &str = "http://localhost:17842";

#[derive(Debug, Clone)]
pub struct BackendConfig {
    pub server_url: String,
    pub api_token: String,
    pub policy: TtsSettings,
}

impl Default for BackendConfig {
    fn default() -> Self {
        Self {
            server_url: DEFAULT_SERVER_URL.to_owned(),
            api_token: String::new(),
            policy: TtsSettings::default(),
        }
    }
}

/// Exact host settings path from the process-plugin environment contract.
pub fn settings_path() -> Option<PathBuf> {
    let root = std::env::var_os("TIKTOOLS_PLUGIN_DATA_DIR")?;
    let id = std::env::var_os("TIKTOOLS_PLUGIN_ID")?;
    if root.is_empty() || id.is_empty() {
        return None;
    }
    Some(PathBuf::from(root).join(id).join("settings.json"))
}

fn is_http_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}

pub fn config_from_values(values: &serde_json::Value) -> BackendConfig {
    let server_url = values
        .get("serverUrl")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|url| !url.is_empty() && url.len() <= 512 && is_http_url(url))
        .unwrap_or(DEFAULT_SERVER_URL)
        .trim_end_matches('/')
        .to_owned();
    let api_token = values
        .get("apiToken")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|token| !token.is_empty() && token.len() <= 4096)
        .unwrap_or_default()
        .to_owned();
    // The UI nests the speech policy under `tts`; a top-level default
    // voice backfills the policy default so both editors agree.
    let mut policy = sanitize_settings(values.get("tts"));
    if policy.default_voice.is_empty() {
        policy.default_voice = values
            .get("defaultVoice")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|voice| !voice.is_empty())
            .unwrap_or_default()
            .chars()
            .take(128)
            .collect();
    }
    if policy.language == "en" {
        if let Some(language) = values
            .get("defaultLanguage")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|language| !language.is_empty())
        {
            policy.language = language.chars().take(16).collect();
        }
    }
    BackendConfig {
        server_url,
        api_token,
        policy,
    }
}

/// Reloading cache: the file is re-read only when its mtime advances, so
/// the hot chat path pays one stat per event.
pub struct ConfigCache {
    loaded_mtime: Option<std::time::SystemTime>,
    config: BackendConfig,
}

impl ConfigCache {
    pub fn new() -> Self {
        Self {
            loaded_mtime: None,
            config: BackendConfig::default(),
        }
    }

    pub fn get(&mut self) -> &BackendConfig {
        let path = settings_path();
        let mtime = path.as_ref().and_then(|path| {
            std::fs::metadata(path)
                .and_then(|metadata| metadata.modified())
                .ok()
        });
        if mtime != self.loaded_mtime {
            self.loaded_mtime = mtime;
            self.config = path
                .as_ref()
                .and_then(|path| std::fs::read_to_string(path).ok())
                .and_then(|raw| serde_json::from_str(&raw).ok())
                .map(|values| config_from_values(&values))
                .unwrap_or_default();
        }
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn config_defaults_and_backfills() {
        let config = config_from_values(&json!({}));
        assert_eq!(config.server_url, DEFAULT_SERVER_URL);
        assert!(config.api_token.is_empty());
        assert!(!config.policy.enabled);

        let config = config_from_values(&json!({
            "serverUrl": "https://tts.example.com/",
            "apiToken": "secret",
            "defaultVoice": "F1",
            "defaultLanguage": "es",
            "tts": {"enabled": true},
        }));
        assert_eq!(config.server_url, "https://tts.example.com");
        assert_eq!(config.api_token, "secret");
        assert_eq!(config.policy.default_voice, "F1");
        assert_eq!(config.policy.language, "es");
        assert!(config.policy.enabled);
    }

    #[test]
    fn config_rejects_non_http_server_urls() {
        let config = config_from_values(&json!({"serverUrl": "ftp://evil.example/x"}));
        assert_eq!(config.server_url, DEFAULT_SERVER_URL);
        let config = config_from_values(&json!({"serverUrl": "notaurl"}));
        assert_eq!(config.server_url, DEFAULT_SERVER_URL);
    }
}
