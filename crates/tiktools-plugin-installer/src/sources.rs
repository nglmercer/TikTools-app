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
}

impl ExpectedRelease {
    fn from_release(id: &str, release: &Release) -> Option<Self> {
        Some(Self {
            id: id.to_owned(),
            version: release.semver()?,
            runtime: release.runtime.clone(),
            permissions: sorted(&release.permissions),
            capabilities: sorted(&release.capabilities),
        })
    }

    pub fn check(&self, prepared: &PreparedPackage) -> Result<(), InstallError> {
        let manifest = &prepared.manifest;
        let matches = manifest.id == self.id
            && manifest.semver().as_ref() == Some(&self.version)
            && manifest.runtime.to_string() == self.runtime
            && sorted(&manifest.permissions) == self.permissions
            && sorted(&manifest.capabilities) == self.capabilities;
        if matches {
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
}

pub struct PackageSources {
    registry: Arc<RegistryClient>,
    downloader: Downloader,
}

impl PackageSources {
    pub fn new(registry: Arc<RegistryClient>, downloader: Downloader) -> Self {
        Self {
            registry,
            downloader,
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
        match spec {
            PackageSpec::File(path) => Ok(ResolvedSource {
                path: path.clone(),
                origin: PackageOrigin::File,
                policy: SignaturePolicy::AllowWithConsent,
                expected: None,
            }),
            PackageSpec::Url(url) => {
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
                })
            }
            PackageSpec::Registry {
                registry,
                id,
                requirement,
            } => {
                reporter.stage(JobStage::Resolve);
                let (source, document) = self.registry.package(id, registry.as_deref()).await?;
                let exact = requirement.as_ref().is_some_and(is_exact);
                let resolved = resolve(
                    &document,
                    &ResolveRequest {
                        requirement: requirement.as_ref(),
                        channel,
                        allow_yanked: exact,
                    },
                    host,
                )
                .map_err(|rejections| {
                    InstallError::from(RegistryError::NoCompatibleRelease {
                        id: id.clone(),
                        rejections,
                    })
                })?;
                reporter.check()?;
                self.reject_revoked(&resolved.artifact.sha256)?;
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
                    expected: ExpectedRelease::from_release(id, &resolved.release),
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
