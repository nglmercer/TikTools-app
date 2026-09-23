//! Gateway configuration and settings persistence.

use serde_json::{json, Value};
use std::fs;
use std::io;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use tiktools_plugin_sdk::data_dir;
use tiktools_plugin_sdk::PluginError;
use tiktools_plugin_sdk::PluginResult;

pub(crate) const DEFAULT_PORT: u16 = 17_452;

const DEFAULT_BIND: &str = "127.0.0.1";

pub(crate) const DEFAULT_ORIGIN: &str = "https://widgets.tiktools.app";

#[derive(Debug, Clone)]
pub struct GatewayConfig {
    pub port: u16,
    pub bind: IpAddr,
    pub allowed_origins: Vec<String>,
    pub token: String,
    pub widget_token: String,
    pub widgets_dir: Option<PathBuf>,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            port: DEFAULT_PORT,
            bind: DEFAULT_BIND.parse().expect("default gateway bind is valid"),
            allowed_origins: vec![DEFAULT_ORIGIN.to_owned()],
            token: String::new(),
            widget_token: String::new(),
            widgets_dir: None,
        }
    }
}

impl GatewayConfig {
    pub(crate) fn from_settings(settings: &Value) -> Result<Self, String> {
        let mut config = Self::default();
        let Some(object) = settings.as_object() else {
            return Err("gateway settings must be a JSON object".to_owned());
        };

        if let Some(port) = object.get("port") {
            let port = port
                .as_u64()
                .and_then(|value| u16::try_from(value).ok())
                .filter(|value| *value != 0)
                .ok_or_else(|| "port must be an integer from 1 to 65535".to_owned())?;
            config.port = port;
        }

        if let Some(bind) = object.get("bind") {
            let bind = bind
                .as_str()
                .ok_or_else(|| "bind must be a loopback IP address".to_owned())?;
            let bind = bind
                .parse::<IpAddr>()
                .map_err(|_| "bind must be a loopback IP address".to_owned())?;
            if !bind.is_loopback() {
                return Err(
                    "remote/LAN gateway binding is disabled; gateway is loopback-only (use 127.0.0.1 or ::1)".to_owned(),
                );
            }
            config.bind = bind;
        }

        if let Some(origins) = object.get("allowedOrigins") {
            let origins = origins
                .as_array()
                .ok_or_else(|| "allowedOrigins must be an array of origins".to_owned())?;
            if origins.len() > 64 {
                return Err("allowedOrigins has too many entries".to_owned());
            }
            config.allowed_origins = origins
                .iter()
                .map(|origin| {
                    let origin = origin
                        .as_str()
                        .map(str::trim)
                        .filter(|origin| !origin.is_empty() && origin.len() <= 2048)
                        .ok_or_else(|| "allowedOrigins contains an invalid origin".to_owned())?;
                    if origin == "*" {
                        return Err(
                            "allowedOrigins cannot use *; list browser origins explicitly"
                                .to_owned(),
                        );
                    }
                    Ok(origin.to_owned())
                })
                .collect::<Result<Vec<_>, String>>()?;
        }

        if let Some(token) = object.get("token") {
            if let Some(token) = token
                .as_str()
                .map(str::trim)
                .filter(|token| !token.is_empty())
            {
                if token.len() > 4096 {
                    return Err("token is too long".to_owned());
                }
                config.token = token.to_owned();
            }
        }
        if let Some(token) = object.get("widgetToken") {
            if let Some(token) = token
                .as_str()
                .map(str::trim)
                .filter(|token| !token.is_empty())
            {
                if token.len() > 4096 {
                    return Err("widgetToken is too long".to_owned());
                }
                config.widget_token = token.to_owned();
            }
        }
        if let Some(dir) = object.get("widgetsDir") {
            // Missing, null, or blank means "probe beside the executable";
            // only a mistyped non-empty value is a startup error.
            if dir.is_null() {
                // Explicit null: keep the default probing behavior.
            } else if let Some(dir) = dir.as_str().map(str::trim).filter(|dir| !dir.is_empty()) {
                if dir.len() > 4096 {
                    return Err("widgetsDir is too long".to_owned());
                }
                config.widgets_dir = Some(PathBuf::from(dir));
            } else if dir.as_str().is_none() {
                return Err("widgetsDir must be a directory path".to_owned());
            }
        }
        if config.token.is_empty() {
            config.token = generated_credential("ttk_")?;
        }
        if config.widget_token.is_empty() {
            config.widget_token = generated_credential("ttw_")?;
        }
        Ok(config)
    }
}

/// Resolves the settings file for one plugin id. The host stores
/// per-plugin settings at `<data-root>/<plugin-id>/settings.json`, so the
/// gateway must read and write the same file — otherwise the generated
/// credentials never reach the host UI and saved OBS URLs go stale.
/// A missing or path-unsafe id falls back to the legacy shared-root file
/// so out-of-host runs keep working; the host always passes a valid id.
pub(crate) fn settings_path_for(data_directory: &Path, plugin_id: &str) -> PathBuf {
    if is_safe_plugin_id(plugin_id) {
        data_directory.join(plugin_id).join("settings.json")
    } else {
        data_directory.join("settings.json")
    }
}

fn is_safe_plugin_id(plugin_id: &str) -> bool {
    !plugin_id.is_empty()
        && plugin_id.len() <= 128
        && !plugin_id.contains('/')
        && !plugin_id.contains('\\')
        && !plugin_id.contains("..")
        && !plugin_id.contains('\0')
}

pub(crate) fn load_config(plugin_id: &str) -> PluginResult<(GatewayConfig, Option<PathBuf>)> {
    let data_directory = data_dir()?;
    let path = settings_path_for(&data_directory, plugin_id);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            PluginError::other(format!("could not create plugin data directory: {error}"))
        })?;
    }
    let settings = match fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str::<Value>(&contents).map_err(|error| {
            PluginError::other(format!("gateway settings are not valid JSON: {error}"))
        })?,
        Err(error) if error.kind() == io::ErrorKind::NotFound => json!({}),
        Err(error) => {
            return Err(PluginError::other(format!(
                "could not read gateway settings: {error}"
            )))
        }
    };
    GatewayConfig::from_settings(&settings)
        .map(|config| (config, Some(path)))
        .map_err(PluginError::other)
}

pub(crate) fn persist_generated_credentials(
    config: &GatewayConfig,
    path: Option<&PathBuf>,
) -> PluginResult<()> {
    let Some(path) = path else {
        return Ok(());
    };
    let existing = fs::read_to_string(path)
        .ok()
        .and_then(|contents| serde_json::from_str::<Value>(&contents).ok())
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();
    let mut settings = existing;
    let mut changed = false;
    for (key, value) in [
        ("token", &config.token),
        ("widgetToken", &config.widget_token),
    ] {
        let present = settings
            .get(key)
            .and_then(Value::as_str)
            .is_some_and(|token| !token.trim().is_empty());
        if !present && !value.trim().is_empty() {
            settings.insert(key.to_owned(), Value::String(value.clone()));
            changed = true;
        }
    }
    if !changed {
        return Ok(());
    }
    let temporary = path.with_extension("json.tmp");
    fs::write(
        &temporary,
        serde_json::to_vec_pretty(&Value::Object(settings)).unwrap(),
    )
    .map_err(|error| {
        PluginError::other(format!(
            "could not save generated gateway credentials: {error}"
        ))
    })?;
    fs::rename(&temporary, path).map_err(|error| {
        PluginError::other(format!("could not commit gateway settings: {error}"))
    })?;
    Ok(())
}

const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

/// Generates a 256-bit credential from the OS CSPRNG, hex-encoded with the
/// given prefix (`ttk_` for the full gateway token, `ttw_` for the
/// widget-scoped token). Never uses a non-cryptographic RNG for
/// credentials.
fn generated_credential(prefix: &str) -> Result<String, String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes)
        .map_err(|error| format!("could not generate gateway credential: {error}"))?;
    let mut token = String::with_capacity(prefix.len() + 64);
    token.push_str(prefix);
    for byte in bytes {
        token.push(HEX_DIGITS[(byte >> 4) as usize] as char);
        token.push(HEX_DIGITS[(byte & 15) as usize] as char);
    }
    Ok(token)
}
