use std::{fs, io, path::Path};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tiktools_plugin_package::PublicKey;

use crate::{url_policy::checked_url, RegistryError, CHANNEL_STABLE};

const CONFIG_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegistrySource {
    pub name: String,
    pub url: String,
    #[serde(default = "enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub priority: i32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scopes: Vec<String>,
    #[serde(default)]
    #[schemars(with = "Vec<serde_json::Value>")]
    pub root_keys: Vec<PublicKey>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub builtin: bool,
    /// Cloud origin (e.g. a RemakeBackend deployment) whose active snapshot
    /// generation is discovered via `GET {origin}/api/v1/registry/current`.
    /// When set, `url` is resolved to the immutable generation endpoint
    /// `{origin}/api/v1/registry/generations/{n}/files` on every refresh, so
    /// each operation is pinned to a single generation. Older configurations
    /// without this field keep working unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cloud_origin: Option<String>,
}

fn enabled() -> bool {
    true
}

impl RegistrySource {
    pub fn new(name: impl Into<String>, url: impl Into<String>) -> Result<Self, RegistryError> {
        let name = name.into();
        if !is_valid_registry_name(&name) {
            return Err(RegistryError::InvalidName(name));
        }
        let url = url.into();
        checked_url(&url)?;
        Ok(Self {
            name,
            url: url.trim_end_matches('/').to_owned(),
            enabled: true,
            priority: 0,
            scopes: Vec::new(),
            root_keys: Vec::new(),
            builtin: false,
            cloud_origin: None,
        })
    }

    pub fn claims(&self, id: &str) -> bool {
        self.scopes
            .iter()
            .any(|scope| match scope.strip_suffix('*') {
                Some(prefix) => id.starts_with(prefix) && id.len() > prefix.len(),
                None => scope == id,
            })
    }

    pub fn is_verified(&self) -> bool {
        !self.root_keys.is_empty()
    }
}

pub fn is_valid_registry_name(value: &str) -> bool {
    let bytes = value.as_bytes();
    (2..=32).contains(&bytes.len())
        && bytes[0].is_ascii_lowercase()
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum AutoUpdate {
    Off,
    #[default]
    Notify,
    Patch,
    Minor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegistryDefaults {
    #[serde(default = "default_channel")]
    pub channel: String,
    #[serde(default)]
    pub auto_update: AutoUpdate,
    #[serde(default = "default_interval")]
    pub check_interval_hours: u32,
    #[serde(default = "enabled")]
    pub allow_sideload: bool,
}

impl Default for RegistryDefaults {
    fn default() -> Self {
        Self {
            channel: default_channel(),
            auto_update: AutoUpdate::default(),
            check_interval_hours: default_interval(),
            allow_sideload: true,
        }
    }
}

fn default_channel() -> String {
    CHANNEL_STABLE.to_owned()
}

fn default_interval() -> u32 {
    12
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegistriesConfig {
    pub version: u32,
    #[serde(default)]
    pub registries: Vec<RegistrySource>,
    #[serde(default)]
    pub defaults: RegistryDefaults,
}

impl Default for RegistriesConfig {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            registries: Vec::new(),
            defaults: RegistryDefaults::default(),
        }
    }
}

impl RegistriesConfig {
    pub fn load(path: &Path) -> Result<Self, RegistryError> {
        match fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|error| RegistryError::document(&path.display().to_string(), error)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error.into()),
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), RegistryError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temporary = path.with_extension("json.tmp");
        let mut bytes = serde_json::to_vec_pretty(self)
            .map_err(|error| RegistryError::document(&path.display().to_string(), error))?;
        bytes.push(b'\n');
        fs::write(&temporary, bytes)?;
        fs::rename(&temporary, path)?;
        Ok(())
    }

    pub fn merge_builtin(&mut self, builtin: Vec<RegistrySource>) {
        for source in builtin {
            match self
                .registries
                .iter_mut()
                .find(|existing| existing.name == source.name)
            {
                Some(existing) => {
                    if let Some(origin) = source.cloud_origin.clone() {
                        // Keep a previously resolved generation endpoint instead
                        // of resetting it to the bare origin; the next refresh
                        // re-resolves the active generation anyway.
                        if !crate::cloud::is_cloud_url(&existing.url, &origin) {
                            existing.url = source.url;
                        }
                        existing.cloud_origin = Some(origin);
                    } else {
                        existing.url = source.url;
                        existing.cloud_origin = None;
                    }
                    existing.root_keys = source.root_keys;
                    existing.builtin = true;
                }
                None => self.registries.push(source),
            }
        }
    }

    pub fn get(&self, name: &str) -> Option<&RegistrySource> {
        self.registries.iter().find(|source| source.name == name)
    }

    pub fn ordered(&self) -> Vec<&RegistrySource> {
        let mut sources: Vec<_> = self
            .registries
            .iter()
            .filter(|source| source.enabled)
            .collect();
        sources.sort_by(|left, right| {
            right
                .priority
                .cmp(&left.priority)
                .then_with(|| left.name.cmp(&right.name))
        });
        sources
    }

    pub fn candidates_for(&self, id: &str) -> Vec<&RegistrySource> {
        let ordered = self.ordered();
        let claimed: Vec<_> = ordered
            .iter()
            .copied()
            .filter(|source| source.claims(id))
            .collect();
        if claimed.is_empty() {
            ordered
                .into_iter()
                .filter(|source| source.scopes.is_empty())
                .collect()
        } else {
            claimed
        }
    }
}
