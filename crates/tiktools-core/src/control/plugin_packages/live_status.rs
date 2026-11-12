use super::*;

/// Only a receipt created by the verified install path can identify an
/// original target. Never infer it from the current generation or digest.
pub(super) fn installed_live_status(
    receipt: &InstallReceipt,
    source: &RegistrySource,
    document: &PackageDocument,
    live: &BTreeMap<String, LiveArtifactStatus>,
) -> LiveReleaseStatus {
    let unknown = LiveReleaseStatus::Unknown;
    let Some(identity) = receipt.cloud_artifact.as_ref() else {
        return unknown;
    };
    let Some(origin) = source.cloud_origin.as_deref() else {
        return unknown;
    };
    let PackageOrigin::Registry { registry, url } = &receipt.origin else {
        return unknown;
    };
    let Ok(generation) = identity.generation.parse::<u64>() else {
        return unknown;
    };
    if registry != &source.name
        || generation == 0
        || generation.to_string() != identity.generation
        || i64::try_from(generation).is_err()
        || !receipt.signature.is_verified()
        || tiktools_plugin_registry::generation_files_url(origin, generation)
            .ok()
            .as_ref()
            != Some(url)
    {
        return unknown;
    }
    if document.publisher.as_deref().is_none_or(str::is_empty)
        || document.publisher != receipt.signature.publisher
    {
        return unknown;
    }
    let Some(release) = document
        .release(&receipt.version)
        .filter(|release| release.version == receipt.version)
    else {
        return unknown;
    };
    let Some(target) = live
        .get(&release.version)
        .and_then(|entry| entry.targets.get(&identity.target))
    else {
        return unknown;
    };
    if Some(target.digest.as_str()) != receipt.archive_digest.as_deref() {
        return unknown;
    }
    if target.key_id.as_deref().is_none_or(str::is_empty)
        || target.key_id != receipt.signature.key_id
    {
        return unknown;
    }
    target.status
}

pub(super) fn classify_live_releases(
    document: &PackageDocument,
    host: &tiktools_plugin_api::HostCompat,
    records: Option<&BTreeMap<(String, String), cloud_client::CatalogVersion>>,
    revoked: impl Fn(&tiktools_plugin_package::Sha256Digest) -> Result<bool, ()>,
) -> BTreeMap<String, LiveArtifactStatus> {
    let mut counts = BTreeMap::new();
    for release in &document.releases {
        if let Some(version) = release.semver() {
            *counts.entry(version).or_insert(0usize) += 1;
        }
    }
    let mut result = BTreeMap::new();
    for release in &document.releases {
        let mut entry = LiveArtifactStatus {
            status: LiveReleaseStatus::Unknown,
            targets: BTreeMap::new(),
        };
        if release
            .semver()
            .is_some_and(|version| counts.get(&version) == Some(&1))
        {
            for artifact in &release.artifacts {
                let digest = artifact.sha256.to_string();
                let mut status = LiveReleaseStatus::Unknown;
                if release
                    .artifacts
                    .iter()
                    .filter(|a| a.target == artifact.target)
                    .count()
                    == 1
                {
                    if release.yanked {
                        status = LiveReleaseStatus::Yanked;
                    } else if release.advisory.is_some() {
                        status = LiveReleaseStatus::Advisory;
                    } else if let (Some(record), Ok(is_revoked)) = (
                        records.and_then(|items| {
                            items.get(&(release.version.clone(), artifact.target.clone()))
                        }),
                        revoked(&artifact.sha256),
                    ) {
                        // A live approval describes the publisher/key named
                        // in the signed Cloud snapshot. The install path
                        // requires both identities and binds them to the
                        // verified archive, so missing signer metadata must
                        // never be presented as an approved release.
                        if document.publisher.as_deref().is_some_and(|v| !v.is_empty())
                            && artifact.key_id.as_deref().is_some_and(|v| !v.is_empty())
                            && record.sha256 == digest
                            && record.size == artifact.size.to_string()
                        {
                            status = if record.yanked {
                                LiveReleaseStatus::Yanked
                            } else if record.key_revoked || is_revoked {
                                LiveReleaseStatus::KeyRevoked
                            } else if !record.publisher_approved {
                                LiveReleaseStatus::PublisherUnapproved
                            } else if record.advisory.is_some() {
                                LiveReleaseStatus::Advisory
                            } else {
                                LiveReleaseStatus::Approved
                            };
                        }
                    }
                }
                entry.targets.insert(
                    artifact.target.clone(),
                    LiveTargetStatus {
                        status,
                        digest,
                        key_id: artifact.key_id.clone(),
                    },
                );
            }
            match tiktools_plugin_registry::compatible_artifact(release, host) {
                Err(_) => entry.status = LiveReleaseStatus::Incompatible,
                Ok(artifact) => {
                    entry.status = entry
                        .targets
                        .get(&artifact.target)
                        .map_or(LiveReleaseStatus::Unknown, |item| item.status)
                }
            }
        }
        result.insert(release.version.clone(), entry);
    }
    result
}
