use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tiktools_plugin_api::{manifest::parse_version, HostCompat};
use tiktools_plugin_registry::{
    latest_compatible, Advisory, AutoUpdate, LiveReleaseStatus, PackageDocument, RegistryClient,
    Release,
};

use crate::{InstallReceipt, PackageOrigin, PermissionDiff};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum UpdateKind {
    Patch,
    Minor,
    Major,
    Prerelease,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub id: String,
    pub registry: String,
    pub current: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<UpdateKind>,
    pub channel: String,
    pub pinned: bool,
    pub permission_diff: PermissionDiff,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub advisory: Option<Advisory>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub changelog: Option<String>,
    pub auto_applicable: bool,
    #[serde(default)]
    pub current_status: LiveReleaseStatus,
    #[serde(default)]
    pub latest_status: LiveReleaseStatus,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateReport {
    pub updates: Vec<UpdateInfo>,
    pub checked_at: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
}

pub async fn check_updates(
    receipts: &[InstallReceipt],
    registry: &RegistryClient,
    host: &HostCompat,
) -> UpdateReport {
    let mut report = UpdateReport {
        checked_at: crate::layout::now(),
        ..UpdateReport::default()
    };
    for receipt in receipts {
        let PackageOrigin::Registry { registry: name, .. } = &receipt.origin else {
            continue;
        };
        let document = match registry.package(&receipt.id, Some(name)).await {
            Ok((_, document)) => document,
            Err(error) => {
                report.errors.push(format!("{}: {error}", receipt.id));
                continue;
            }
        };
        if let Some(update) = update_for_document(receipt, name, &document, host) {
            report.updates.push(update);
        }
    }
    report
}

/// Build an update from the exact document already selected by a caller.
/// Cloud product callers can bind its live status to the same generation.
pub fn update_for_document(
    receipt: &InstallReceipt,
    name: &str,
    document: &PackageDocument,
    host: &HostCompat,
) -> Option<UpdateInfo> {
    let advisory = document
        .release(&receipt.version)
        .and_then(|release| release.advisory.clone());
    let latest = latest_compatible(document, &receipt.policy.channel, host)
        .filter(|resolved| is_newer(&resolved.release.version, &receipt.version));
    if latest.is_none() && advisory.is_none() {
        return None;
    }
    let release = latest.as_ref().map(|resolved| &resolved.release);
    let kind = release.and_then(|release| update_kind(&receipt.version, &release.version));
    let permission_diff = release
        .map(|release| permission_diff(receipt, release))
        .unwrap_or_default();
    Some(UpdateInfo {
        id: receipt.id.clone(),
        registry: name.to_owned(),
        current: receipt.version.clone(),
        latest: release.map(|release| release.version.clone()),
        kind,
        channel: receipt.policy.channel.clone(),
        pinned: receipt.policy.pinned,
        auto_applicable: release.is_some()
            && !receipt.policy.pinned
            && !permission_diff.adds_access()
            && policy_allows(receipt.policy.auto_update, kind),
        permission_diff,
        advisory,
        changelog: release.and_then(|release| release.changelog.clone()),
        current_status: LiveReleaseStatus::Unknown,
        latest_status: LiveReleaseStatus::Unknown,
    })
}

fn is_newer(candidate: &str, current: &str) -> bool {
    match (parse_version(candidate), parse_version(current)) {
        (Some(candidate), Some(current)) => candidate > current,
        _ => false,
    }
}

fn update_kind(current: &str, next: &str) -> Option<UpdateKind> {
    let (current, next) = (parse_version(current)?, parse_version(next)?);
    Some(if !next.pre.is_empty() {
        UpdateKind::Prerelease
    } else if next.major != current.major {
        UpdateKind::Major
    } else if next.minor != current.minor {
        UpdateKind::Minor
    } else {
        UpdateKind::Patch
    })
}

fn policy_allows(policy: AutoUpdate, kind: Option<UpdateKind>) -> bool {
    matches!(
        (policy, kind),
        (AutoUpdate::Patch, Some(UpdateKind::Patch))
            | (
                AutoUpdate::Minor,
                Some(UpdateKind::Patch | UpdateKind::Minor)
            )
    )
}

fn permission_diff(receipt: &InstallReceipt, release: &Release) -> PermissionDiff {
    let added = |granted: &[String], next: &[String]| -> Vec<String> {
        next.iter()
            .filter(|value| !granted.contains(value))
            .cloned()
            .collect()
    };
    PermissionDiff {
        added_permissions: added(&receipt.granted_permissions, &release.permissions),
        removed_permissions: added(&release.permissions, &receipt.granted_permissions),
        added_capabilities: added(&receipt.granted_capabilities, &release.capabilities),
        removed_capabilities: added(&release.capabilities, &receipt.granted_capabilities),
    }
}
