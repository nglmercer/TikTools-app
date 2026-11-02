use std::{fmt, path::PathBuf, str::FromStr};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tiktools_plugin_api::manifest::{is_valid_plugin_id, parse_version};
use tiktools_plugin_registry::is_allowed_url;

use crate::{InstallError, InstallErrorCode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageSpec {
    Registry {
        registry: Option<String>,
        id: String,
        requirement: Option<semver::VersionReq>,
    },
    Url(String),
    File(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum PackageSpecInput {
    File {
        path: String,
    },
    Url {
        url: String,
    },
    Registry {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        version: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        registry: Option<String>,
    },
    Spec {
        spec: String,
    },
}

impl PackageSpec {
    pub fn plugin_id(&self) -> Option<&str> {
        match self {
            Self::Registry { id, .. } => Some(id),
            _ => None,
        }
    }

    pub fn exact(&self) -> bool {
        matches!(self, Self::Registry { requirement: Some(requirement), .. } if is_exact(requirement))
    }
}

impl TryFrom<PackageSpecInput> for PackageSpec {
    type Error = InstallError;

    fn try_from(input: PackageSpecInput) -> Result<Self, Self::Error> {
        match input {
            PackageSpecInput::File { path } => file_spec(&path),
            PackageSpecInput::Url { url } => url_spec(&url),
            PackageSpecInput::Registry {
                id,
                version,
                registry,
            } => registry_spec(registry, &id, version.as_deref()),
            PackageSpecInput::Spec { spec } => spec.parse(),
        }
    }
}

impl FromStr for PackageSpec {
    type Err = InstallError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = value.trim();
        if value.is_empty() || value.len() > 4_096 {
            return Err(invalid("package specs must be 1..=4096 characters"));
        }
        if let Some(path) = value.strip_prefix("file:") {
            return file_spec(path);
        }
        if value.starts_with("https://") || value.starts_with("http://") {
            return url_spec(value);
        }
        let looks_like_path = value.to_ascii_lowercase().ends_with(".plugin")
            || value.contains('/')
            || value.contains('\\');
        if looks_like_path {
            return file_spec(value);
        }
        let (registry, rest) = match value.split_once(':') {
            Some((registry, rest)) => (Some(registry.to_owned()), rest),
            None => (None, value),
        };
        let (id, version) = match rest.split_once('@') {
            Some((id, version)) => (id, Some(version)),
            None => (rest, None),
        };
        registry_spec(registry, id, version)
    }
}

impl fmt::Display for PackageSpec {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Registry {
                registry,
                id,
                requirement,
            } => {
                if let Some(registry) = registry {
                    write!(formatter, "{registry}:")?;
                }
                formatter.write_str(id)?;
                if let Some(requirement) = requirement {
                    write!(formatter, "@{requirement}")?;
                }
                Ok(())
            }
            Self::Url(url) => formatter.write_str(url),
            Self::File(path) => write!(formatter, "{}", path.display()),
        }
    }
}

fn file_spec(path: &str) -> Result<PackageSpec, InstallError> {
    let path = path.trim();
    if path.is_empty() || path.len() > 32_767 {
        return Err(invalid("plugin archive path must be 1..=32767 characters"));
    }
    Ok(PackageSpec::File(PathBuf::from(path)))
}

fn url_spec(url: &str) -> Result<PackageSpec, InstallError> {
    if !is_allowed_url(url) {
        return Err(InstallError::new(
            InstallErrorCode::Unsupported,
            format!("only https URLs are allowed (http is accepted for localhost): {url}"),
        ));
    }
    Ok(PackageSpec::Url(url.to_owned()))
}

fn registry_spec(
    registry: Option<String>,
    id: &str,
    version: Option<&str>,
) -> Result<PackageSpec, InstallError> {
    if !is_valid_plugin_id(id) {
        return Err(invalid(format!("`{id}` is not a valid plugin id")));
    }
    if registry
        .as_deref()
        .is_some_and(|name| name.is_empty() || name.len() > 32)
    {
        return Err(invalid("registry names must be 1..=32 characters"));
    }
    let requirement = match version
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "latest")
    {
        None => None,
        Some(version) => Some(parse_requirement(version)?),
    };
    Ok(PackageSpec::Registry {
        registry,
        id: id.to_owned(),
        requirement,
    })
}

fn parse_requirement(version: &str) -> Result<semver::VersionReq, InstallError> {
    if let Some(exact) = parse_version(version) {
        return semver::VersionReq::parse(&format!("={exact}"))
            .map_err(|error| invalid(error.to_string()));
    }
    semver::VersionReq::parse(version)
        .map_err(|_| invalid(format!("`{version}` is not a valid version range")))
}

pub(crate) fn is_exact(requirement: &semver::VersionReq) -> bool {
    requirement.comparators.len() == 1
        && requirement.comparators[0].op == semver::Op::Exact
        && requirement.comparators[0].patch.is_some()
}

fn invalid(message: impl Into<String>) -> InstallError {
    InstallError::new(InstallErrorCode::Unsupported, message)
}
