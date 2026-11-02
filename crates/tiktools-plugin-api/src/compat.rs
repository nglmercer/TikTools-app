use std::{collections::BTreeSet, fmt};

use serde::{Deserialize, Serialize};

use crate::{
    manifest::{current_platform, current_target, parse_version_req},
    PluginManifest, PluginRuntimeKind, TIKTOOLS_PLUGIN_ABI_VERSION,
    TIKTOOLS_PLUGIN_PROTOCOL_VERSION,
};

pub const MIN_SUPPORTED_SCHEMA_VERSION: u32 = 2;
pub const MAX_SUPPORTED_SCHEMA_VERSION: u32 = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostCompat {
    pub host_version: semver::Version,
    pub target: String,
    pub runtimes: BTreeSet<PluginRuntimeKind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompatRequirements<'a> {
    pub schema_version: u32,
    pub protocol_version: u32,
    pub abi_version: Option<u32>,
    pub runtime: PluginRuntimeKind,
    pub engine: Option<&'a str>,
    pub targets: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Incompatibility {
    Schema { version: u32 },
    Protocol { version: u32 },
    Abi { version: u32 },
    HostVersion { required: String, host: String },
    Target { targets: Vec<String>, host: String },
    Runtime { runtime: String },
}

impl fmt::Display for Incompatibility {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Schema { version } => {
                write!(
                    formatter,
                    "plugin manifest has unsupported schema version {version}"
                )
            }
            Self::Protocol { version } => {
                write!(
                    formatter,
                    "plugin protocol version {version} is not supported"
                )
            }
            Self::Abi { version } => {
                write!(formatter, "plugin ABI version {version} is not supported")
            }
            Self::HostVersion { required, host } => {
                write!(
                    formatter,
                    "plugin requires TikTools {required} (this host is {host})"
                )
            }
            Self::Target { host, .. } => {
                write!(formatter, "plugin has no build for this platform ({host})")
            }
            Self::Runtime { runtime } => {
                write!(
                    formatter,
                    "the {runtime} plugin runtime is not available on this host"
                )
            }
        }
    }
}

impl HostCompat {
    pub fn new(host_version: semver::Version) -> Self {
        let mut runtimes = BTreeSet::from([
            PluginRuntimeKind::Process,
            PluginRuntimeKind::NapiVm,
            PluginRuntimeKind::Declarative,
        ]);
        if !cfg!(target_os = "android") {
            runtimes.insert(PluginRuntimeKind::Native);
        }
        Self {
            host_version,
            target: current_target(),
            runtimes,
        }
    }

    pub fn with_runtime(mut self, runtime: PluginRuntimeKind) -> Self {
        self.runtimes.insert(runtime);
        self
    }

    pub fn with_target(mut self, target: impl Into<String>) -> Self {
        self.target = target.into();
        self
    }

    pub fn check_manifest(&self, manifest: &PluginManifest) -> Result<(), Incompatibility> {
        self.check(CompatRequirements {
            schema_version: manifest.schema_version,
            protocol_version: manifest.protocol_version,
            abi_version: manifest.abi_version,
            runtime: manifest.runtime,
            engine: manifest.metadata.engine.as_deref(),
            targets: &manifest.targets,
        })
    }

    pub fn check(&self, requirements: CompatRequirements<'_>) -> Result<(), Incompatibility> {
        if !(MIN_SUPPORTED_SCHEMA_VERSION..=MAX_SUPPORTED_SCHEMA_VERSION)
            .contains(&requirements.schema_version)
        {
            return Err(Incompatibility::Schema {
                version: requirements.schema_version,
            });
        }
        if requirements.protocol_version != TIKTOOLS_PLUGIN_PROTOCOL_VERSION {
            return Err(Incompatibility::Protocol {
                version: requirements.protocol_version,
            });
        }
        if requirements.runtime == PluginRuntimeKind::Native {
            if let Some(version) = requirements
                .abi_version
                .filter(|version| *version != TIKTOOLS_PLUGIN_ABI_VERSION)
            {
                return Err(Incompatibility::Abi { version });
            }
        }
        if !self.runtimes.contains(&requirements.runtime) {
            return Err(Incompatibility::Runtime {
                runtime: requirements.runtime.to_string(),
            });
        }
        if let Some(engine) = requirements.engine {
            let matches = parse_version_req(engine)
                .is_some_and(|requirement| requirement.matches(&self.host_version));
            if !matches {
                return Err(Incompatibility::HostVersion {
                    required: engine.to_owned(),
                    host: self.host_version.to_string(),
                });
            }
        }
        if !self.target_matches(requirements.targets) {
            return Err(Incompatibility::Target {
                targets: requirements.targets.to_vec(),
                host: self.target.clone(),
            });
        }
        Ok(())
    }

    pub fn target_matches(&self, targets: &[String]) -> bool {
        if targets.is_empty() {
            return true;
        }
        let platform = self
            .target
            .split('-')
            .next()
            .map(ToOwned::to_owned)
            .unwrap_or_else(current_platform);
        targets
            .iter()
            .any(|candidate| candidate == &self.target || candidate == &platform)
    }

    pub fn accepts_artifact_target(&self, target: &str) -> bool {
        target == "any" || target == self.target
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host(version: &str) -> HostCompat {
        HostCompat::new(semver::Version::parse(version).unwrap()).with_target("linux-x64-gnu")
    }

    fn manifest(extra: serde_json::Value) -> PluginManifest {
        let mut value = serde_json::json!({
            "schemaVersion": 3,
            "id": "acme.demo",
            "name": "Demo",
            "version": "1.0.0",
            "runtime": "process",
            "entry": "demo",
        });
        value
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        PluginManifest::from_value(value).unwrap()
    }

    #[test]
    fn host_version_range_gates_compatibility() {
        let manifest = manifest(serde_json::json!({"engines": {"tiktools": ">=0.4.0, <0.6.0"}}));
        assert!(host("0.4.2").check_manifest(&manifest).is_ok());
        assert_eq!(
            host("0.3.3").check_manifest(&manifest),
            Err(Incompatibility::HostVersion {
                required: ">=0.4.0, <0.6.0".to_owned(),
                host: "0.3.3".to_owned(),
            })
        );
    }

    #[test]
    fn targets_and_runtimes_gate_compatibility() {
        let foreign = manifest(serde_json::json!({"targets": ["win32-x64-msvc"]}));
        assert!(matches!(
            host("0.4.0").check_manifest(&foreign),
            Err(Incompatibility::Target { .. })
        ));
        let portable = manifest(serde_json::json!({"targets": []}));
        assert!(host("0.4.0").check_manifest(&portable).is_ok());
        let wasm = manifest(serde_json::json!({"runtime": "wasm", "entry": "plugin.wasm"}));
        assert!(matches!(
            host("0.4.0").check_manifest(&wasm),
            Err(Incompatibility::Runtime { .. })
        ));
        assert!(host("0.4.0")
            .with_runtime(PluginRuntimeKind::Wasm)
            .check_manifest(&wasm)
            .is_ok());
    }

    #[test]
    fn artifact_targets_accept_portable_and_exact_matches() {
        let host = host("0.4.0");
        assert!(host.accepts_artifact_target("any"));
        assert!(host.accepts_artifact_target("linux-x64-gnu"));
        assert!(!host.accepts_artifact_target("linux-arm64-gnu"));
    }
}
