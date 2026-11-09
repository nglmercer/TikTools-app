use std::{
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc},
};

use tiktools_plugin_api::HostCompat;
use tiktools_plugin_package::Sha256Digest;
use tiktools_plugin_registry::{
    join_url, resolve, DownloadRequest, Downloader, RegistryClient, RegistryError, Release,
    ResolveRequest,
};

use crate::{
    jobs::{JobReporter, JobStage},
    spec::is_exact,
    InstallError, InstallErrorCode, PackageOrigin, PackageSpec, PreparedPackage, SignaturePolicy,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedRelease {
    pub id: String,
    pub version: semver::Version,
    pub runtime: String,
    pub permissions: Vec<String>,
    pub capabilities: Vec<String>,
    /// Fixed Cloud registries bind the signed index publisher and artifact
    /// key to the actual verified archive signature. Generic registries may
    /// omit this additional Cloud-specific requirement.
    pub expected_signer: Option<(String, String)>,
}

impl ExpectedRelease {
    fn from_release(
        id: &str,
        release: &Release,
        expected_signer: Option<(String, String)>,
    ) -> Option<Self> {
        Some(Self {
            id: id.to_owned(),
            version: release.semver()?,
            runtime: release.runtime.clone(),
            permissions: sorted(&release.permissions),
            capabilities: sorted(&release.capabilities),
            expected_signer,
        })
    }

    pub fn check(&self, prepared: &PreparedPackage) -> Result<(), InstallError> {
        let manifest = &prepared.manifest;
        let matches = manifest.id == self.id
            && manifest.semver().as_ref() == Some(&self.version)
            && manifest.runtime.to_string() == self.runtime
            && sorted(&manifest.permissions) == self.permissions
            && sorted(&manifest.capabilities) == self.capabilities;
        let signer_matches = self
            .expected_signer
            .as_ref()
            .is_none_or(|(publisher, key_id)| {
                prepared.signature.is_verified()
                    && prepared.signature.publisher.as_deref() == Some(publisher.as_str())
                    && prepared.signature.key_id.as_deref() == Some(key_id.as_str())
            });
        if matches && signer_matches {
            Ok(())
        } else {
            Err(InstallError::new(
                InstallErrorCode::InvalidPackage,
                format!(
                    "the downloaded package does not match the registry release {}@{}",
                    self.id, self.version
                ),
            ))
        }
    }
}

fn sorted(values: &[String]) -> Vec<String> {
    let mut values = values.to_vec();
    values.sort();
    values.dedup();
    values
}

#[derive(Debug, Clone)]
pub struct ResolvedSource {
    pub path: PathBuf,
    pub origin: PackageOrigin,
    pub policy: SignaturePolicy,
    pub expected: Option<ExpectedRelease>,
    /// Registry artifact target, retained for the final authoritative check.
    pub target: Option<String>,
    /// Keeps the registry generation's trust snapshot stable through package
    /// inspection. The caller must hold this until `Installer::prepare` ends.
    pub trust_guard: Option<Arc<tiktools_plugin_registry::RegistryTrustGuard>>,
}

pub struct PackageSources {
    registry: Arc<RegistryClient>,
    downloader: Downloader,
    fixed_sources: bool,
}

impl PackageSources {
    pub fn new(registry: Arc<RegistryClient>, downloader: Downloader) -> Self {
        Self {
            registry,
            downloader,
            fixed_sources: false,
        }
    }

    /// Restrict artifacts to the configured registry's package tree. Pair
    /// with `Downloader::new_fixed` to prevent off-origin redirects.
    pub fn new_fixed(registry: Arc<RegistryClient>, downloader: Downloader) -> Self {
        Self {
            registry,
            downloader,
            fixed_sources: true,
        }
    }

    pub fn registry(&self) -> &Arc<RegistryClient> {
        &self.registry
    }

    pub fn downloader(&self) -> &Downloader {
        &self.downloader
    }

    pub async fn fetch(
        &self,
        spec: &PackageSpec,
        channel: &str,
        host: &HostCompat,
        reporter: &dyn JobReporter,
        cancel: &AtomicBool,
    ) -> Result<ResolvedSource, InstallError> {
        if self.fixed_sources
            && (!self.registry.enforces_fixed_sources() || !self.downloader.enforces_fixed_origin())
        {
            return Err(InstallError::from(RegistryError::FixedSources));
        }
        match spec {
            PackageSpec::File(path) => Ok(ResolvedSource {
                path: path.clone(),
                origin: PackageOrigin::File,
                policy: SignaturePolicy::AllowWithConsent,
                expected: None,
                target: None,
                trust_guard: None,
            }),
            PackageSpec::Url(url) => {
                if self.fixed_sources {
                    return Err(InstallError::from(RegistryError::FixedSources));
                }
                reporter.stage(JobStage::Download);
                let downloaded = self
                    .downloader
                    .fetch(
                        DownloadRequest {
                            url,
                            expected_digest: None,
                            expected_size: None,
                        },
                        &|done, total| reporter.progress(done, total),
                        cancel,
                    )
                    .await?;
                self.reject_revoked(&downloaded.digest)?;
                Ok(ResolvedSource {
                    path: downloaded.path,
                    origin: PackageOrigin::Url { url: url.clone() },
                    policy: SignaturePolicy::AllowWithConsent,
                    expected: None,
                    target: None,
                    trust_guard: None,
                })
            }
            PackageSpec::Registry {
                registry,
                id,
                requirement,
            } => {
                reporter.stage(JobStage::Resolve);
                let (source, document, trust_guard) = self
                    .registry
                    .package_guarded(id, registry.as_deref())
                    .await?;
                let resolved = resolve_package(
                    &document,
                    requirement.as_ref(),
                    channel,
                    host,
                    self.fixed_sources,
                )
                .map_err(|rejections| {
                    InstallError::from(RegistryError::NoCompatibleRelease {
                        id: id.clone(),
                        rejections,
                    })
                })?;
                let expected_signer = if self.fixed_sources {
                    Some(fixed_cloud_signer(&document, &resolved.artifact)?)
                } else {
                    None
                };
                let expected =
                    ExpectedRelease::from_release(id, &resolved.release, expected_signer)
                        .ok_or_else(|| {
                            InstallError::new(
                                InstallErrorCode::InvalidPackage,
                                "registry release cannot be bound to the downloaded package",
                            )
                        })?;
                reporter.check()?;
                self.reject_revoked(&resolved.artifact.sha256)?;
                if self.fixed_sources && !fixed_artifact_path(id, &resolved.artifact.url) {
                    return Err(InstallError::from(RegistryError::FixedSources));
                }
                let url = join_url(&source.url, &resolved.artifact.url)?;
                reporter.stage(JobStage::Download);
                let downloaded = self
                    .downloader
                    .fetch(
                        DownloadRequest {
                            url: &url,
                            expected_digest: Some(&resolved.artifact.sha256),
                            expected_size: Some(resolved.artifact.size),
                        },
                        &|done, total| reporter.progress(done, total),
                        cancel,
                    )
                    .await?;
                Ok(ResolvedSource {
                    path: downloaded.path,
                    origin: PackageOrigin::Registry {
                        registry: source.name.clone(),
                        url: source.url.clone(),
                    },
                    policy: if source.is_verified() {
                        SignaturePolicy::Require
                    } else {
                        SignaturePolicy::AllowWithConsent
                    },
                    expected: Some(expected),
                    target: Some(resolved.artifact.target.clone()),
                    trust_guard,
                })
            }
        }
    }

    fn reject_revoked(&self, digest: &Sha256Digest) -> Result<(), InstallError> {
        if self.registry.is_revoked(digest) {
            return Err(InstallError::new(
                InstallErrorCode::Revoked,
                format!("package {digest} has been revoked by its registry"),
            ));
        }
        Ok(())
    }
}

/// Cloud's snapshot publisher requires both fields and verifies them against
/// the archive at promotion. Recheck that binding independently on install;
/// a live approval for a different key must not authorize this archive.
fn fixed_cloud_signer(
    document: &tiktools_plugin_registry::PackageDocument,
    artifact: &tiktools_plugin_registry::Artifact,
) -> Result<(String, String), InstallError> {
    let publisher = document
        .publisher
        .as_deref()
        .filter(|value| !value.is_empty());
    let key_id = artifact.key_id.as_deref().filter(|value| !value.is_empty());
    match (publisher, key_id) {
        (Some(publisher), Some(key_id)) => Ok((publisher.to_owned(), key_id.to_owned())),
        _ => Err(InstallError::new(
            InstallErrorCode::InvalidPackage,
            "Cloud release is missing its publisher or artifact signing key",
        )),
    }
}

/// The Cloud publisher emits canonical ASCII package paths. Do not accept URL
/// encodings, URL authority syntax, dot segments or another plugin's tree.
fn fixed_artifact_path(id: &str, path: &str) -> bool {
    if path.len() > 2048 || !path.is_ascii() {
        return false;
    }
    let Some(rest) = path.strip_prefix("packages/") else {
        return false;
    };
    let mut components = rest.split('/');
    if components.next() != Some(id) {
        return false;
    }
    let components: Vec<_> = components.collect();
    !components.is_empty()
        && components.iter().all(|part| {
            !part.is_empty()
                && *part != "."
                && *part != ".."
                && part.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'+')
                })
        })
        && components
            .last()
            .is_some_and(|name| name.ends_with(".plugin"))
}

fn resolve_package(
    document: &tiktools_plugin_registry::PackageDocument,
    requirement: Option<&semver::VersionReq>,
    channel: &str,
    host: &HostCompat,
    fixed_sources: bool,
) -> Result<tiktools_plugin_registry::ResolvedRelease, Vec<tiktools_plugin_registry::Rejection>> {
    resolve(
        document,
        &ResolveRequest {
            requirement,
            channel,
            // The generic installer retains its legacy exact-version policy;
            // the product must never newly install a yanked Cloud release.
            allow_yanked: !fixed_sources && requirement.is_some_and(is_exact),
        },
        host,
    )
}

#[cfg(test)]
mod fixed_source_tests {
    use super::{fixed_artifact_path, fixed_cloud_signer, resolve_package, PackageSources};
    use crate::{NoopReporter, PackageSpec};
    use std::sync::{atomic::AtomicBool, Arc, RwLock};
    use tiktools_plugin_api::HostCompat;
    use tiktools_plugin_package::TrustStore;
    use tiktools_plugin_registry::{Downloader, RegistryClient};

    #[test]
    fn fixed_cloud_requires_publisher_and_exact_artifact_key() {
        let mut document: tiktools_plugin_registry::PackageDocument = serde_json::from_value(serde_json::json!({
            "id": "acme.plugin", "name": "Example", "releases": [{
                "version": "1.2.3", "runtime": "process", "schemaVersion": 3, "protocolVersion": 1,
                "artifacts": [{"target": "any", "url": "packages/acme.plugin/a.plugin", "size": 1, "sha256": "0".repeat(64)}]
            }]
        })).unwrap();
        let mut artifact = document.releases[0].artifacts[0].clone();
        assert!(fixed_cloud_signer(&document, &artifact).is_err());
        document.publisher = Some("acme".into());
        assert!(fixed_cloud_signer(&document, &artifact).is_err());
        artifact.key_id = Some("acme-key".into());
        assert_eq!(
            fixed_cloud_signer(&document, &artifact).unwrap(),
            ("acme".into(), "acme-key".into())
        );
        artifact.key_id = Some(String::new());
        assert!(fixed_cloud_signer(&document, &artifact).is_err());
        document.publisher = Some(String::new());
        artifact.key_id = Some("acme-key".into());
        assert!(fixed_cloud_signer(&document, &artifact).is_err());
    }

    #[test]
    fn cloud_artifact_is_confined_to_its_canonical_package_tree() {
        assert!(fixed_artifact_path(
            "acme.plugin",
            "packages/acme.plugin/acme.plugin-1.2.3+build-linux-x64.plugin"
        ));
        for path in [
            "https://other.example/packages/acme.plugin/x.plugin",
            "//other.example/packages/acme.plugin/x.plugin",
            "packages/acme.plugin/../other/x.plugin",
            "packages/acme.plugin/%2e%2e/other/x.plugin",
            "packages/acme.plugin/%2Fother/x.plugin",
            "packages/acme.plugin\\other/x.plugin",
            "packages/acme.plugin/x.plugin?redirect=other",
            "packages/acme.plugin//x.plugin",
            "packages/other.plugin/x.plugin",
            "packages/acme.plugin/x.txt",
        ] {
            assert!(!fixed_artifact_path("acme.plugin", path), "accepted {path}");
        }
    }

    #[test]
    fn fixed_cloud_cannot_install_a_yanked_exact_version() {
        let document = serde_json::from_value(serde_json::json!({
            "id": "acme.plugin", "name": "Example", "releases": [{
                "version": "1.2.3", "channel": "stable", "runtime": "process",
                "schemaVersion": 3, "protocolVersion": 1, "yanked": true,
                "artifacts": [{"target": "any", "url": "packages/acme.plugin/a.plugin", "size": 1, "sha256": "0".repeat(64)}]
            }]
        })).unwrap();
        let version = semver::VersionReq::parse("=1.2.3").unwrap();
        let host = HostCompat::new(semver::Version::new(4, 0, 2));
        let rejected =
            resolve_package(&document, Some(&version), "stable", &host, true).unwrap_err();
        assert!(
            rejected
                .iter()
                .any(|rejection| rejection.reason
                    == tiktools_plugin_registry::RejectionReason::Yanked)
        );
        assert!(resolve_package(&document, Some(&version), "stable", &host, false).is_ok());
    }

    #[tokio::test]
    async fn fixed_source_refuses_unfenced_registry_or_downloader() {
        let root = tempfile::tempdir().unwrap();
        let registry = RegistryClient::new_fixed(
            root.path().join("registries"),
            root.path().join("registry.json"),
            Vec::new(),
            Arc::new(RwLock::new(TrustStore::new())),
        )
        .unwrap();
        let unsafe_downloader = Downloader::new(root.path().join("cache")).unwrap();
        let sources = PackageSources::new_fixed(Arc::new(registry), unsafe_downloader);
        let spec = PackageSpec::File(root.path().join("test.plugin"));
        let result = sources
            .fetch(
                &spec,
                "stable",
                &HostCompat::new(semver::Version::new(4, 0, 2)),
                &NoopReporter,
                &AtomicBool::new(false),
            )
            .await;
        assert!(result.is_err());

        let registry = RegistryClient::new(
            root.path().join("generic-registries"),
            root.path().join("generic-registry.json"),
            Vec::new(),
            Arc::new(RwLock::new(TrustStore::new())),
        )
        .unwrap();
        let downloader = Downloader::new_fixed(root.path().join("fixed-cache")).unwrap();
        let sources = PackageSources::new_fixed(Arc::new(registry), downloader);
        assert!(sources
            .fetch(
                &spec,
                "stable",
                &HostCompat::new(semver::Version::new(4, 0, 2)),
                &NoopReporter,
                &AtomicBool::new(false),
            )
            .await
            .is_err());
    }
}
