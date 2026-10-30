//! Optional L4D2api settings. Old RCON credentials are ignored, not required.
use std::path::{Path, PathBuf};
use std::time::Duration;
use serde_json::Value;
use tiktools_plugin_sdk::{PluginError, PluginResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    pub game_dir: Option<PathBuf>,
    pub timeout: Duration,
    pub max_per_minute: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Self { game_dir: None, timeout: Duration::from_secs(5), max_per_minute: 30 }
    }
}

fn settings_path() -> Option<PathBuf> {
    let root = std::env::var_os("TIKTOOLS_PLUGIN_DATA_DIR")?;
    let plugin_id = std::env::var_os("TIKTOOLS_PLUGIN_ID")
        .unwrap_or_else(|| "l4d2.interactive".into());
    Some(PathBuf::from(root).join(plugin_id).join("settings.json"))
}

impl Settings {
    pub fn parse(raw: &str) -> PluginResult<Self> {
        let value: Value = serde_json::from_str(raw)
            .map_err(|_| PluginError::invalid_request("L4D2 settings are not valid JSON"))?;
        if !value.is_object() {
            return Err(PluginError::invalid_request("L4D2 settings must be an object"));
        }
        let mut settings = Self::default();
        if let Some(value) = value.get("gameDir") {
            let dir = value.as_str().ok_or_else(|| PluginError::invalid_request("gameDir must be a string"))?.trim();
            if !dir.is_empty() {
                settings.game_dir = Some(PathBuf::from(dir));
            }
        }
        if let Some(value) = value.get("timeoutMs") {
            let ms = value.as_u64().filter(|n| (1000..=12000).contains(n))
                .ok_or_else(|| PluginError::invalid_request("timeoutMs must be 1000..12000"))?;
            settings.timeout = Duration::from_millis(ms);
        }
        if let Some(value) = value.get("maxPerMinute") {
            settings.max_per_minute = value.as_u64().filter(|n| (1..=120).contains(n))
                .ok_or_else(|| PluginError::invalid_request("maxPerMinute must be 1..120"))?;
        }
        Ok(settings)
    }

    pub fn load() -> PluginResult<Self> {
        let Some(path) = settings_path() else { return Ok(Self::default()); };
        match std::fs::read_to_string(path) {
            Ok(raw) => Self::parse(&raw),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(_) => Err(PluginError::other("Unable to read L4D2 plugin settings")),
        }
    }

    /// Read-only Steam discovery from upstream, never installation or repair.
    pub fn resolve_game_dir(&self) -> PluginResult<PathBuf> {
        if let Some(dir) = &self.game_dir {
            l4d2api_installer::validate_game_dir(dir).map_err(|_| {
                PluginError::invalid_request("Invalid L4D2 gameDir; select the directory containing left4dead2.exe")
            })?;
            return Ok(dir.clone());
        }
        let roots = l4d2api_installer::candidate_steam_roots(None);
        let installs = l4d2api_installer::find_l4d2_installs(&roots);
        match installs.as_slice() {
            [] => Err(PluginError::other("Left 4 Dead 2 not found in Steam. Set gameDir in plugin settings. Install the addon with the upstream L4D2api installer.")),
            [only] => Ok(only.clone()),
            many => {
                let active: Vec<&PathBuf> = many.iter()
                    .filter(|dir| Self::ipc_dir(dir).join(l4d2api_client::HEARTBEAT_FILE).is_file())
                    .collect();
                match active.as_slice() {
                    [only] => Ok((**only).clone()),
                    _ => Err(PluginError::invalid_request("Multiple Left 4 Dead 2 installations detected; set gameDir explicitly")),
                }
            }
        }
    }

    pub fn ipc_dir(game_dir: &Path) -> PathBuf {
        game_dir.join("left4dead2").join("ems").join("tiktools_l4d2")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_settings_use_defaults() {
        assert_eq!(Settings::parse("{}").unwrap(), Settings::default());
    }

    #[test]
    fn old_rcon_settings_do_not_block_ipc() {
        let settings = Settings::parse(r#"{"host":"127.0.0.1","port":27015,"password":"old","timeoutMs":5000,"maxPerMinute":30}"#).unwrap();
        assert_eq!(settings, Settings::default());
    }

    #[test]
    fn rejects_invalid_limits() {
        for raw in [r#"{"timeoutMs":0}"#, r#"{"timeoutMs":12001}"#, r#"{"maxPerMinute":0}"#, r#"{"gameDir":12}"#] {
            assert!(Settings::parse(raw).is_err(), "{raw}");
        }
    }

    #[test]
    fn custom_game_dir_must_be_valid() {
        let config = Settings::parse(r#"{"gameDir":"not-a-game-install"}"#).unwrap();
        assert!(config.resolve_game_dir().is_err());
    }
}
