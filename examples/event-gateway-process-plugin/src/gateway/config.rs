//! Gateway configuration and settings persistence.

use serde_json::{json, Value};
use std::fs;
use std::io;
use std::net::IpAddr;
use std::path::PathBuf;
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
    pub widgets_dir: Option<PathBuf>,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            port: DEFAULT_PORT,
            bind: DEFAULT_BIND.parse().expect("default gateway bind is valid"),
            allowed_origins: vec![DEFAULT_ORIGIN.to_owned()],
            token: String::new(),
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
            config.token = generated_token()?;
        }
        Ok(config)
    }
}

pub(crate) fn load_config() -> PluginResult<(GatewayConfig, Option<PathBuf>)> {
    let data_directory = data_dir()?;
    fs::create_dir_all(&data_directory).map_err(|error| {
        PluginError::other(format!("could not create plugin data directory: {error}"))
    })?;
    let path = data_directory.join("settings.json");
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

pub(crate) fn persist_generated_token(
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
    if existing
        .get("token")
        .and_then(Value::as_str)
        .is_some_and(|token| !token.trim().is_empty())
    {
        return Ok(());
    }
    let mut settings = existing;
    settings.insert("token".to_owned(), Value::String(config.token.clone()));
    let temporary = path.with_extension("json.tmp");
    fs::write(
        &temporary,
        serde_json::to_vec_pretty(&Value::Object(settings)).unwrap(),
    )
    .map_err(|error| {
        PluginError::other(format!("could not save generated gateway token: {error}"))
    })?;
    fs::rename(&temporary, path).map_err(|error| {
        PluginError::other(format!("could not commit gateway settings: {error}"))
    })?;
    Ok(())
}

const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

/// Generates a 256-bit gateway token from the OS CSPRNG, hex-encoded as
/// `ttk_<64 hex>`. Never uses a non-cryptographic RNG for credentials.
fn generated_token() -> Result<String, String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes)
        .map_err(|error| format!("could not generate gateway token: {error}"))?;
    let mut token = String::with_capacity(4 + 64);
    token.push_str("ttk_");
    for byte in bytes {
        token.push(HEX_DIGITS[(byte >> 4) as usize] as char);
        token.push(HEX_DIGITS[(byte & 15) as usize] as char);
    }
    Ok(token)
}
