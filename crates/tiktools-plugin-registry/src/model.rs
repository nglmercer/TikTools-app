use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tiktools_plugin_api::{
    manifest::{is_valid_plugin_id, parse_version, HOST_ENGINE},
    CompatRequirements, PluginRuntimeKind,
};
use tiktools_plugin_package::{PublicKey, Sha256Digest};

pub const REGISTRY_PROTOCOL_VERSION: u32 = 1;
pub const ANY_TARGET: &str = "any";
pub const CHANNEL_STABLE: &str = "stable";
pub const CHANNEL_BETA: &str = "beta";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RootDocument {
    pub version: u32,
    #[schemars(with = "Vec<serde_json::Value>")]
    pub keys: Vec<PublicKey>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegistryConfigDocument {
    pub protocol: u32,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mirrors: Vec<String>,
    #[serde(default)]
    pub catalog_pages: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub revoked_digests: Vec<String>,
    #[serde(default)]
    pub generated_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum AdvisorySeverity {
    Low,
    Moderate,
    High,
    Critical,
}

/// Current Cloud status of the exact host artifact, separate from the
/// immutable signed snapshot. `Unknown` never authorizes installation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum LiveReleaseStatus {
    #[default]
    Unknown,
    Approved,
    Yanked,
    KeyRevoked,
    PublisherUnapproved,
    Advisory,
    Incompatible,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Advisory {
    pub severity: AdvisorySeverity,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
    pub target: String,
    pub url: String,
    pub size: u64,
    #[schemars(with = "String")]
    pub sha256: Sha256Digest,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Release {
    pub version: String,
    #[serde(default = "default_channel")]
    pub channel: String,
    #[serde(default)]
    pub published_at: u64,
    #[serde(default)]
    pub yanked: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub advisory: Option<Advisory>,
    pub runtime: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub engines: BTreeMap<String, String>,
    pub schema_version: u32,
    pub protocol_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub abi_version: Option<u32>,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub dependencies: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub changelog: Option<String>,
    pub artifacts: Vec<Artifact>,
}

impl Release {
    pub fn semver(&self) -> Option<semver::Version> {
        parse_version(&self.version)
    }

    pub fn runtime_kind(&self) -> Option<PluginRuntimeKind> {
        PluginRuntimeKind::parse(&self.runtime)
    }

    pub fn engine(&self) -> Option<&str> {
        self.engines.get(HOST_ENGINE).map(String::as_str)
    }

    pub fn requirements<'a>(
        &'a self,
        runtime: PluginRuntimeKind,
        targets: &'a [String],
    ) -> CompatRequirements<'a> {
        CompatRequirements {
            schema_version: self.schema_version,
            protocol_version: self.protocol_version,
            abi_version: self.abi_version,
            runtime,
            engine: self.engine(),
            targets,
        }
    }
}

fn default_channel() -> String {
    CHANNEL_STABLE.to_owned()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PackageDocument {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub categories: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub screenshots: Vec<String>,
    #[serde(
        default,
        rename = "dist-tags",
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    pub dist_tags: BTreeMap<String, String>,
    #[serde(default)]
    pub releases: Vec<Release>,
}

impl PackageDocument {
    pub fn validate(&self, expected_id: &str) -> Result<(), String> {
        if self.id != expected_id || !is_valid_plugin_id(&self.id) {
            return Err(format!(
                "document describes `{}`, expected `{expected_id}`",
                self.id
            ));
        }
        if self.releases.len() > 4_096 {
            return Err("too many releases".to_owned());
        }
        for release in &self.releases {
            if release.semver().is_none() {
                return Err(format!(
                    "release `{}` is not a semver version",
                    release.version
                ));
            }
            if release.artifacts.is_empty() || release.artifacts.len() > 32 {
                return Err(format!(
                    "release `{}` has an invalid artifact list",
                    release.version
                ));
            }
        }
        Ok(())
    }

    pub fn release(&self, version: &str) -> Option<&Release> {
        let wanted = parse_version(version)?;
        self.releases
            .iter()
            .find(|release| release.semver().as_ref() == Some(&wanted))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CatalogEntry {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub categories: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    pub latest: String,
    pub runtime: String,
    #[serde(default)]
    pub updated_at: u64,
}

impl CatalogEntry {
    pub fn matches(&self, words: &[String], category: Option<&str>) -> bool {
        if category.is_some_and(|category| !self.categories.iter().any(|entry| entry == category)) {
            return false;
        }
        let haystack = [
            self.id.as_str(),
            self.name.as_str(),
            self.publisher.as_deref().unwrap_or_default(),
            self.description.as_deref().unwrap_or_default(),
        ]
        .join(" ")
        .to_lowercase();
        let tags = self.tags.join(" ");
        words
            .iter()
            .all(|word| haystack.contains(word.as_str()) || tags.contains(word.as_str()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CatalogPage {
    pub page: u32,
    pub pages: u32,
    pub total: u32,
    pub entries: Vec<CatalogEntry>,
}

pub fn shard_path(id: &str) -> String {
    let characters: Vec<char> = id.chars().collect();
    let pad = |range: std::ops::Range<usize>| -> String {
        range
            .map(|index| characters.get(index).copied().unwrap_or('_'))
            .collect()
    };
    let first = pad(0..2);
    let second = pad(2..4);
    // A literal parent component is resolved by URL and filesystem clients.
    // Use its canonical location beneath index/ so writers and readers sign
    // and fetch the same bytes; other identifiers retain their legacy paths.
    if second == ".." {
        format!("index/{id}.json")
    } else {
        format!("index/{first}/{second}/{id}.json")
    }
}
