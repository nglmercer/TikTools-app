use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::validation::{is_safe_relative_path, is_valid_plugin_id, ManifestError};

pub const PLUGIN_TARGETS: [&str; 6] = [
    "win32-x64-msvc",
    "win32-arm64-msvc",
    "linux-x64-gnu",
    "linux-arm64-gnu",
    "darwin-x64-darwin",
    "darwin-arm64-darwin",
];

pub const PLUGIN_CATEGORIES: [&str; 8] = [
    "audio",
    "chat",
    "games",
    "overlays",
    "moderation",
    "integrations",
    "utilities",
    "ai",
];

pub const HOST_ENGINE: &str = "tiktools";

const MAX_AUTHORS: usize = 16;
const MAX_DEPENDENCIES: usize = 32;
const MAX_CATEGORIES: usize = 4;
const MAX_URL_LEN: usize = 512;
const MAX_LICENSE_LEN: usize = 128;
const MAX_AUTHOR_NAME_LEN: usize = 128;
const MAX_EMAIL_LEN: usize = 254;
const MAX_RANGE_LEN: usize = 128;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginAuthor {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authors: Vec<PluginAuthor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub dependencies: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub categories: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub changelog: Option<String>,
}

impl PackageMetadata {
    pub(crate) fn parse(object: &Map<String, Value>, id: &str) -> Result<Self, ManifestError> {
        let publisher = match object.get("publisher") {
            None => None,
            Some(Value::String(value)) if is_valid_publisher(value) => Some(value.clone()),
            Some(_) => return Err(ManifestError::InvalidField("publisher")),
        };
        let engine = match object.get("engines") {
            None => None,
            Some(Value::Object(engines)) => match engines.get(HOST_ENGINE) {
                None => None,
                Some(Value::String(range)) if is_valid_version_req(range) => {
                    Some(range.trim().to_owned())
                }
                Some(_) => return Err(ManifestError::InvalidField("engines")),
            },
            Some(_) => return Err(ManifestError::InvalidField("engines")),
        };
        let dependencies = parse_dependencies(object.get("dependencies"), id)?;
        let changelog = match object.get("changelog") {
            None => None,
            Some(Value::String(path))
                if path.len() <= 256
                    && is_safe_relative_path(path)
                    && path.to_ascii_lowercase().ends_with(".md") =>
            {
                Some(path.clone())
            }
            Some(_) => return Err(ManifestError::InvalidField("changelog")),
        };
        Ok(Self {
            publisher,
            authors: sanitized_authors(object.get("authors")),
            license: object
                .get("license")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| is_valid_license(value))
                .map(ToOwned::to_owned),
            homepage: https_url(object.get("homepage")),
            repository: https_url(object.get("repository")),
            engine,
            dependencies,
            categories: sanitized_categories(object.get("categories")),
            changelog,
        })
    }

    pub fn host_requirement(&self) -> Option<semver::VersionReq> {
        self.engine.as_deref().and_then(parse_version_req)
    }
}

pub fn is_valid_publisher(value: &str) -> bool {
    let bytes = value.as_bytes();
    (2..=39).contains(&bytes.len())
        && bytes[0].is_ascii_lowercase()
        && bytes[1..]
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
        && !value.ends_with('-')
}

pub fn is_plugin_target(value: &str) -> bool {
    PLUGIN_TARGETS.contains(&value)
}

pub fn is_plugin_category(value: &str) -> bool {
    PLUGIN_CATEGORIES.contains(&value)
}

pub fn parse_version(value: &str) -> Option<semver::Version> {
    semver::Version::parse(value.trim().trim_start_matches('v')).ok()
}

pub fn parse_version_req(value: &str) -> Option<semver::VersionReq> {
    let value = value.trim();
    if value.is_empty() || value.len() > MAX_RANGE_LEN {
        return None;
    }
    semver::VersionReq::parse(value).ok()
}

pub fn is_valid_version_req(value: &str) -> bool {
    parse_version_req(value).is_some()
}

pub fn is_https_url(value: &str) -> bool {
    let Some(rest) = value.strip_prefix("https://") else {
        return false;
    };
    value.len() <= MAX_URL_LEN
        && !rest.is_empty()
        && !rest.starts_with('/')
        && !value
            .chars()
            .any(|character| character.is_whitespace() || character.is_control())
}

fn is_valid_license(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_LICENSE_LEN
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric()
                || matches!(character, '.' | '-' | '+' | ' ' | '(' | ')' | ':')
        })
}

fn https_url(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| is_https_url(value))
        .map(ToOwned::to_owned)
}

fn sanitized_authors(value: Option<&Value>) -> Vec<PluginAuthor> {
    let Some(entries) = value.and_then(Value::as_array) else {
        return Vec::new();
    };
    entries
        .iter()
        .filter_map(|entry| match entry {
            Value::String(name) => author_name(name).map(|name| PluginAuthor {
                name,
                email: None,
                url: None,
            }),
            Value::Object(object) => {
                let name = object
                    .get("name")
                    .and_then(Value::as_str)
                    .and_then(author_name)?;
                let email = object
                    .get("email")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|email| {
                        email.len() <= MAX_EMAIL_LEN
                            && email.contains('@')
                            && !email.chars().any(|character| {
                                character.is_whitespace() || character.is_control()
                            })
                    })
                    .map(ToOwned::to_owned);
                Some(PluginAuthor {
                    name,
                    email,
                    url: https_url(object.get("url")),
                })
            }
            _ => None,
        })
        .take(MAX_AUTHORS)
        .collect()
}

fn author_name(value: &str) -> Option<String> {
    let name = value.trim();
    (!name.is_empty() && name.len() <= MAX_AUTHOR_NAME_LEN && !name.chars().any(char::is_control))
        .then(|| name.to_owned())
}

fn sanitized_categories(value: Option<&Value>) -> Vec<String> {
    let Some(entries) = value.and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut categories = Vec::new();
    for category in entries.iter().filter_map(Value::as_str) {
        let category = category.trim().to_ascii_lowercase();
        if is_plugin_category(&category) && !categories.contains(&category) {
            categories.push(category);
        }
        if categories.len() >= MAX_CATEGORIES {
            break;
        }
    }
    categories
}

fn parse_dependencies(
    value: Option<&Value>,
    own_id: &str,
) -> Result<BTreeMap<String, String>, ManifestError> {
    const FIELD: &str = "dependencies";
    let Some(value) = value else {
        return Ok(BTreeMap::new());
    };
    let entries = value
        .as_object()
        .ok_or(ManifestError::InvalidField(FIELD))?;
    if entries.len() > MAX_DEPENDENCIES {
        return Err(ManifestError::InvalidField(FIELD));
    }
    entries
        .iter()
        .map(|(id, range)| {
            let range = range.as_str().ok_or(ManifestError::InvalidField(FIELD))?;
            if !is_valid_plugin_id(id) || id == own_id || !is_valid_version_req(range) {
                return Err(ManifestError::InvalidField(FIELD));
            }
            Ok((id.clone(), range.trim().to_owned()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::PluginManifest;

    fn parse(extra: serde_json::Value) -> Result<PluginManifest, super::ManifestError> {
        let mut value = serde_json::json!({
            "schemaVersion": 3,
            "id": "acme.tts",
            "name": "Acme TTS",
            "version": "1.2.0",
            "runtime": "process",
            "entry": "tts",
        });
        value
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        PluginManifest::from_value(value)
    }

    #[test]
    fn parses_publishing_metadata() {
        let manifest = parse(serde_json::json!({
            "publisher": "acme",
            "authors": ["Acme Labs", {"name": "Ada", "email": "ada@acme.dev", "url": "https://acme.dev"}],
            "license": "MIT OR Apache-2.0",
            "homepage": "https://acme.dev/tts",
            "repository": "https://github.com/acme/tts",
            "engines": {"tiktools": ">=0.4.0", "node": ">=20"},
            "dependencies": {"acme.voices": "^2.0"},
            "categories": ["Audio", "chat", "unknown", "audio"],
            "changelog": "CHANGELOG.md",
        }))
        .unwrap();
        let metadata = &manifest.metadata;
        assert_eq!(metadata.publisher.as_deref(), Some("acme"));
        assert_eq!(metadata.authors.len(), 2);
        assert_eq!(metadata.authors[1].email.as_deref(), Some("ada@acme.dev"));
        assert_eq!(metadata.license.as_deref(), Some("MIT OR Apache-2.0"));
        assert_eq!(metadata.engine.as_deref(), Some(">=0.4.0"));
        assert_eq!(metadata.dependencies["acme.voices"], "^2.0");
        assert_eq!(metadata.categories, ["audio", "chat"]);
        assert_eq!(metadata.changelog.as_deref(), Some("CHANGELOG.md"));
        assert_eq!(manifest.semver().unwrap().to_string(), "1.2.0");
    }

    #[test]
    fn display_metadata_is_sanitized_not_fatal() {
        let manifest = parse(serde_json::json!({
            "authors": [42, {"name": ""}],
            "license": "<script>",
            "homepage": "http://insecure.example",
            "repository": "javascript:alert(1)",
            "categories": "audio",
        }))
        .unwrap();
        assert_eq!(manifest.metadata, super::PackageMetadata::default());
    }

    #[test]
    fn identity_and_compatibility_metadata_is_strict() {
        for extra in [
            serde_json::json!({"publisher": "Acme"}),
            serde_json::json!({"publisher": "a"}),
            serde_json::json!({"engines": {"tiktools": "not a range"}}),
            serde_json::json!({"engines": ">=0.4"}),
            serde_json::json!({"dependencies": {"acme.tts": "^1"}}),
            serde_json::json!({"dependencies": {"Bad Id": "^1"}}),
            serde_json::json!({"dependencies": {"acme.voices": 2}}),
            serde_json::json!({"changelog": "../CHANGELOG.md"}),
            serde_json::json!({"changelog": "CHANGELOG.txt"}),
        ] {
            assert!(parse(extra.clone()).is_err(), "{extra} should be rejected");
        }
    }

    #[test]
    fn plugin_targets_and_publishers_validate() {
        assert!(super::is_plugin_target("linux-x64-gnu"));
        assert!(!super::is_plugin_target("linux64"));
        assert!(super::is_valid_publisher("tiktools"));
        assert!(super::is_valid_publisher("acme-labs"));
        assert!(!super::is_valid_publisher("acme-"));
        assert!(!super::is_valid_publisher("9acme"));
    }
}
