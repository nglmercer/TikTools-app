use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tiktools_plugin_api::{HostCompat, Incompatibility};

use crate::{Artifact, PackageDocument, Release, ANY_TARGET, CHANNEL_BETA, CHANNEL_STABLE};

#[derive(Debug, Clone)]
pub struct ResolveRequest<'a> {
    pub requirement: Option<&'a semver::VersionReq>,
    pub channel: &'a str,
    pub allow_yanked: bool,
}

impl Default for ResolveRequest<'_> {
    fn default() -> Self {
        Self {
            requirement: None,
            channel: CHANNEL_STABLE,
            allow_yanked: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRelease {
    pub release: Release,
    pub artifact: Artifact,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum RejectionReason {
    Yanked,
    Channel { channel: String },
    Requirement,
    UnknownRuntime { runtime: String },
    Incompatible { message: String },
    NoArtifact { target: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Rejection {
    pub version: String,
    pub reason: RejectionReason,
}

pub fn resolve(
    document: &PackageDocument,
    request: &ResolveRequest<'_>,
    host: &HostCompat,
) -> Result<ResolvedRelease, Vec<Rejection>> {
    let exact = request.requirement.is_some_and(is_exact);
    let mut rejections = Vec::new();
    let mut best: Option<(semver::Version, ResolvedRelease)> = None;
    for release in &document.releases {
        let Some(version) = release.semver() else {
            continue;
        };
        let reject = |reason| Rejection {
            version: release.version.clone(),
            reason,
        };
        if let Some(requirement) = request.requirement {
            if !requirement.matches(&version)
                && !(exact && prerelease_matches(requirement, &version))
            {
                rejections.push(reject(RejectionReason::Requirement));
                continue;
            }
        }
        if release.yanked && !(exact && request.allow_yanked) {
            rejections.push(reject(RejectionReason::Yanked));
            continue;
        }
        if !exact && !channel_allows(request.channel, &release.channel, &version) {
            rejections.push(reject(RejectionReason::Channel {
                channel: release.channel.clone(),
            }));
            continue;
        }
        match compatible_artifact(release, host) {
            Ok(artifact) => {
                if best.as_ref().is_none_or(|(current, _)| version > *current) {
                    best = Some((
                        version,
                        ResolvedRelease {
                            release: release.clone(),
                            artifact,
                        },
                    ));
                }
            }
            Err(reason) => rejections.push(reject(reason)),
        }
    }
    best.map(|(_, resolved)| resolved).ok_or(rejections)
}

pub fn latest_compatible(
    document: &PackageDocument,
    channel: &str,
    host: &HostCompat,
) -> Option<ResolvedRelease> {
    resolve(
        document,
        &ResolveRequest {
            requirement: None,
            channel,
            allow_yanked: false,
        },
        host,
    )
    .ok()
}

pub(crate) fn compatible_artifact(
    release: &Release,
    host: &HostCompat,
) -> Result<Artifact, RejectionReason> {
    let runtime = release
        .runtime_kind()
        .ok_or_else(|| RejectionReason::UnknownRuntime {
            runtime: release.runtime.clone(),
        })?;
    let artifact = release
        .artifacts
        .iter()
        .find(|artifact| artifact.target == host.target)
        .or_else(|| {
            release
                .artifacts
                .iter()
                .find(|artifact| artifact.target == ANY_TARGET)
        })
        .ok_or_else(|| RejectionReason::NoArtifact {
            target: host.target.clone(),
        })?;
    let targets = if artifact.target == ANY_TARGET {
        Vec::new()
    } else {
        vec![artifact.target.clone()]
    };
    host.check(release.requirements(runtime, &targets))
        .map_err(
            |incompatibility: Incompatibility| RejectionReason::Incompatible {
                message: incompatibility.to_string(),
            },
        )?;
    Ok(artifact.clone())
}

fn channel_allows(requested: &str, release_channel: &str, version: &semver::Version) -> bool {
    match requested {
        CHANNEL_BETA => release_channel == CHANNEL_STABLE || release_channel == CHANNEL_BETA,
        _ => release_channel == CHANNEL_STABLE && version.pre.is_empty(),
    }
}

fn is_exact(requirement: &semver::VersionReq) -> bool {
    requirement.comparators.len() == 1
        && requirement.comparators[0].op == semver::Op::Exact
        && requirement.comparators[0].minor.is_some()
        && requirement.comparators[0].patch.is_some()
}

fn prerelease_matches(requirement: &semver::VersionReq, version: &semver::Version) -> bool {
    requirement.comparators.iter().all(|comparator| {
        comparator.major == version.major
            && comparator.minor == Some(version.minor)
            && comparator.patch == Some(version.patch)
            && comparator.pre == version.pre
    })
}
