//! Host-only review of the complete saved execution closure. This module never
//! manufactures a runtime permit, adopts a definition or loads plugin settings.
//! Inputs must come from the host's atomic review and current session resolver;
//! none of these authority-bearing types implements `Deserialize`.
use crate::collaboration::{AutomationFamily, Capability, Grant, ResourceKind, Versioned};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeSet;

mod classification;
pub(crate) use classification::effect_render_fields;
#[cfg(test)]
mod tests;

pub(crate) const POLICY_VERSION: u32 = 1;
const DIGEST_ALGORITHM: &str = "sha256-canonical-json-v1";
const REVIEW_BYTES: usize = 4 * 1024 * 1024;
const DEFINITION_BYTES: usize = 256 * 1024;
const DEPENDENCIES: usize = 512;

/// Borrowed snapshots from the host's persisted principal/session resolver.
pub(crate) struct ManifestSubject<'a> {
    pub principal_id: &'a str,
    pub session_id: &'a str,
    pub grant_revision: &'a str,
    pub configuration_revision: &'a str,
    pub grant: &'a Grant,
}
pub(crate) struct ManifestSubjects<'a> {
    pub host_id: &'a str,
    pub subjects: &'a [ManifestSubject<'a>],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManifestError {
    InvalidReview,
    TooLarge,
    InvalidSubjects,
    MissingPrimary,
}
impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidReview => "saved automation review is inconsistent",
            Self::TooLarge => "automation manifest exceeds its bounded review budget",
            Self::InvalidSubjects => "automation manifest authorization binding is inconsistent",
            Self::MissingPrimary => "automation definition does not exist",
        })
    }
}
impl std::error::Error for ManifestError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub(crate) enum ManifestAuthority {
    Owner,
    Delegated {
        #[serde(rename = "principalId")]
        principal_id: String,
        #[serde(rename = "sessionId")]
        session_id: String,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ManifestProvenance {
    pub authority: ManifestAuthority,
    pub generation: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DefinitionBinding {
    pub kind: AutomationFamily,
    pub id: String,
    pub revision: String,
    pub exists: bool,
    pub content_digest: Option<String>,
    pub provenance: Option<ManifestProvenance>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SubjectBinding {
    pub principal_id: String,
    pub session_id: String,
    pub grant_revision: String,
    pub configuration_revision: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ManifestReason {
    ApprovalRequired,
    MissingDependency,
    MissingCapability,
    ResourceForbidden,
    LiveRestriction,
    InvalidConfiguration,
    DynamicTarget,
    GlobalAccessRequiresPolicy,
    UnclassifiableRenderField,
    OpaqueScript,
    EffectPolicyUnavailable,
    PluginBindingUnavailable,
    UnknownActionType,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum EffectKind {
    Log,
    Emit,
    Points,
    Delay,
    Script,
    Http,
    Audio,
    Plugin,
    Unknown,
    MissingDefinition,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum EffectDecision {
    Classified,
    ReviewRequired,
    Unsupported,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ManifestEffect {
    pub family: AutomationFamily,
    pub id: String,
    pub type_id: Option<String>,
    pub kind: EffectKind,
    pub enabled: Option<bool>,
    pub decision: EffectDecision,
    pub render_fields: Vec<String>,
    pub dynamic_target: bool,
    pub required_capabilities: Vec<Capability>,
    pub reasons: Vec<ManifestReason>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExecutionManifest {
    pub policy_version: u32,
    pub digest_algorithm: &'static str,
    pub host_id: String,
    pub subjects: Vec<SubjectBinding>,
    pub primary: DefinitionBinding,
    pub dependencies: Vec<DefinitionBinding>,
    pub effects: Vec<ManifestEffect>,
    pub reasons: Vec<ManifestReason>,
    /// This is a review artifact, never a caller-constructible execution grant.
    pub execution_authorized: bool,
    pub manifest_digest: String,
}

struct Entry<'a> {
    binding: DefinitionBinding,
    record: Option<&'a Value>,
}

fn canonical_id(value: &str, limit: usize) -> bool {
    !value.is_empty()
        && value.len() <= limit
        && value.trim() == value
        && !value.contains('*')
        && !value.chars().any(char::is_control)
}
fn decimal(value: &str) -> Option<i64> {
    value
        .parse::<i64>()
        .ok()
        .filter(|n| *n >= 0 && n.to_string() == value)
}
fn exact(value: &Value, fields: usize) -> Result<(), ManifestError> {
    if value.as_object().is_some_and(|o| o.len() == fields) {
        Ok(())
    } else {
        Err(ManifestError::InvalidReview)
    }
}
fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str, ManifestError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or(ManifestError::InvalidReview)
}
fn authority(value: &Value) -> Result<ManifestProvenance, ManifestError> {
    exact(value, 2)?;
    let generation = string(value, "generation")?;
    if !decimal(generation).is_some_and(|n| n > 0) {
        return Err(ManifestError::InvalidReview);
    }
    let a = value.get("authority").ok_or(ManifestError::InvalidReview)?;
    let authority = match string(a, "kind")? {
        "owner" => {
            exact(a, 1)?;
            ManifestAuthority::Owner
        }
        "delegated" => {
            exact(a, 3)?;
            let principal_id = string(a, "principalId")?;
            let session_id = string(a, "sessionId")?;
            if !canonical_id(principal_id, 256) || !canonical_id(session_id, 256) {
                return Err(ManifestError::InvalidReview);
            }
            ManifestAuthority::Delegated {
                principal_id: principal_id.into(),
                session_id: session_id.into(),
            }
        }
        _ => return Err(ManifestError::InvalidReview),
    };
    Ok(ManifestProvenance {
        authority,
        generation: generation.into(),
    })
}

/// Deterministic JSON canonicalization deliberately sorts every object rather
/// than relying on serde_json's optional preserve_order dependency feature.
fn canonical_json(value: &Value, limit: usize) -> Result<String, ManifestError> {
    fn append(out: &mut String, text: &str, limit: usize) -> Result<(), ManifestError> {
        if out.len().checked_add(text.len()).is_none_or(|n| n > limit) {
            return Err(ManifestError::TooLarge);
        }
        out.push_str(text);
        Ok(())
    }
    fn write(
        value: &Value,
        out: &mut String,
        depth: usize,
        limit: usize,
    ) -> Result<(), ManifestError> {
        if depth > 64 {
            return Err(ManifestError::TooLarge);
        }
        match value {
            Value::Object(map) => {
                append(out, "{", limit)?;
                let mut entries: Vec<_> = map.iter().collect();
                entries.sort_unstable_by_key(|(key, _)| *key);
                for (index, (key, value)) in entries.into_iter().enumerate() {
                    if index > 0 {
                        append(out, ",", limit)?;
                    }
                    append(
                        out,
                        &serde_json::to_string(key).map_err(|_| ManifestError::InvalidReview)?,
                        limit,
                    )?;
                    append(out, ":", limit)?;
                    write(value, out, depth + 1, limit)?;
                }
                append(out, "}", limit)
            }
            Value::Array(values) => {
                append(out, "[", limit)?;
                for (index, value) in values.iter().enumerate() {
                    if index > 0 {
                        append(out, ",", limit)?;
                    }
                    write(value, out, depth + 1, limit)?;
                }
                append(out, "]", limit)
            }
            _ => append(
                out,
                &serde_json::to_string(value).map_err(|_| ManifestError::InvalidReview)?,
                limit,
            ),
        }
    }
    let mut out = String::new();
    write(value, &mut out, 0, limit)?;
    Ok(out)
}
fn digest(domain: &str, value: &Value, limit: usize) -> Result<String, ManifestError> {
    let encoded = canonical_json(value, limit)?;
    Ok(crate::collaboration::hash_secret(&format!(
        "tiktools/automation-manifest/v1/{domain}\0{encoded}"
    )))
}

fn entry<'a>(
    kind: AutomationFamily,
    id: &str,
    raw: &'a Value,
    provenance: &Value,
    version: &crate::collaboration::ResourceVersion,
) -> Result<Entry<'a>, ManifestError> {
    if !canonical_id(id, 128)
        || version.kind != ResourceKind::Automation
        || version.scope != kind.scope()
        || version.id != id
    {
        return Err(ManifestError::InvalidReview);
    }
    let revision = decimal(&version.revision).ok_or(ManifestError::InvalidReview)?;
    let record = if raw.is_null() {
        None
    } else {
        if !raw.is_object() || raw.get("id").and_then(Value::as_str) != Some(id) {
            return Err(ManifestError::InvalidReview);
        }
        Some(raw)
    };
    let provenance = if provenance.is_null() {
        None
    } else {
        Some(authority(provenance)?)
    };
    if (revision == 0 && (record.is_some() || provenance.is_some()))
        || (revision > 0 && provenance.is_none())
    {
        return Err(ManifestError::InvalidReview);
    }
    let content_digest = record
        .map(|record| digest("definition", record, DEFINITION_BYTES))
        .transpose()?;
    Ok(Entry {
        binding: DefinitionBinding {
            kind,
            id: id.into(),
            revision: version.revision.clone(),
            exists: record.is_some(),
            content_digest,
            provenance,
        },
        record,
    })
}

fn parse_review(review: &Versioned<Value>) -> Result<Vec<Entry<'_>>, ManifestError> {
    canonical_json(
        &serde_json::to_value(review).map_err(|_| ManifestError::InvalidReview)?,
        REVIEW_BYTES,
    )?;
    if review.command.is_some() {
        return Err(ManifestError::InvalidReview);
    }
    let value = &review.value;
    exact(value, 5)?;
    let kind =
        AutomationFamily::from_scope(string(value, "kind")?).ok_or(ManifestError::InvalidReview)?;
    let id = string(value, "id")?;
    let primary = value.get("record").ok_or(ManifestError::InvalidReview)?;
    let provenance = value
        .get("provenance")
        .ok_or(ManifestError::InvalidReview)?;
    let dependencies = value
        .get("dependencies")
        .and_then(Value::as_array)
        .ok_or(ManifestError::InvalidReview)?;
    if dependencies.len() > DEPENDENCIES {
        return Err(ManifestError::TooLarge);
    }
    let mut expected = Vec::new();
    if kind == AutomationFamily::Event && !primary.is_null() {
        if let Some(ids) = primary.get("actionIds") {
            let ids = ids.as_array().ok_or(ManifestError::InvalidReview)?;
            if ids.len() > DEPENDENCIES {
                return Err(ManifestError::TooLarge);
            }
            let mut seen = BTreeSet::new();
            for id in ids {
                let id = id
                    .as_str()
                    .filter(|id| canonical_id(id, 128))
                    .ok_or(ManifestError::InvalidReview)?;
                if seen.insert(id) {
                    expected.push(id);
                }
            }
        }
    }
    if expected.len() != dependencies.len() || review.resource_versions.len() != expected.len() + 1
    {
        return Err(ManifestError::InvalidReview);
    }
    let mut entries = vec![entry(
        kind,
        id,
        primary,
        provenance,
        &review.resource_versions[0],
    )?];
    for (index, (raw, id)) in dependencies.iter().zip(expected).enumerate() {
        exact(raw, 3)?;
        if string(raw, "id")? != id {
            return Err(ManifestError::InvalidReview);
        }
        entries.push(entry(
            AutomationFamily::Action,
            id,
            raw.get("record").ok_or(ManifestError::InvalidReview)?,
            raw.get("provenance").ok_or(ManifestError::InvalidReview)?,
            &review.resource_versions[index + 1],
        )?);
    }
    Ok(entries)
}

fn identities(entries: &[Entry<'_>]) -> Vec<(String, String)> {
    entries
        .iter()
        .filter_map(
            |entry| match &entry.binding.provenance.as_ref()?.authority {
                ManifestAuthority::Delegated {
                    principal_id,
                    session_id,
                } => Some((principal_id.clone(), session_id.clone())),
                ManifestAuthority::Owner => None,
            },
        )
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
/// The caller resolves exactly these trusted identities, never identity fields
/// from the candidate's editable JSON. Tombstone identities are included.
pub(crate) fn required_subjects(
    review: &Versioned<Value>,
) -> Result<Vec<(String, String)>, ManifestError> {
    Ok(identities(&parse_review(review)?))
}

/// Definition-only comparison for the trusted runtime index. This carries no
/// subject approval or effect authority; admission still rebuilds the complete
/// manifest under the current saved-subject lock.
pub(super) fn definition_bindings(
    review: &Versioned<Value>,
) -> Result<Vec<DefinitionBinding>, ManifestError> {
    Ok(parse_review(review)?
        .into_iter()
        .map(|entry| entry.binding)
        .collect())
}

pub(crate) fn build(
    review: &Versioned<Value>,
    subjects: ManifestSubjects<'_>,
) -> Result<ExecutionManifest, ManifestError> {
    build_with_inherited_subjects(review, subjects, &[])
}

/// Trusted host callers may add inherited principals to the saved authors.
/// This never accepts identities from editable definition JSON. The caller
/// must bind the extra set to a current competition generation separately.
pub(super) fn build_with_inherited_subjects(
    review: &Versioned<Value>,
    subjects: ManifestSubjects<'_>,
    inherited: &[(String, String)],
) -> Result<ExecutionManifest, ManifestError> {
    if !canonical_id(subjects.host_id, 256) {
        return Err(ManifestError::InvalidSubjects);
    }
    let entries = parse_review(review)?;
    if entries[0].record.is_none() {
        return Err(ManifestError::MissingPrimary);
    }
    if inherited.len() > DEPENDENCIES + 1 {
        return Err(ManifestError::InvalidSubjects);
    }
    let mut expected: BTreeSet<_> = identities(&entries).into_iter().collect();
    let mut unique = BTreeSet::new();
    for (principal, session) in inherited {
        if !canonical_id(principal, 256)
            || !canonical_id(session, 256)
            || !unique.insert((principal.clone(), session.clone()))
        {
            return Err(ManifestError::InvalidSubjects);
        }
        expected.insert((principal.clone(), session.clone()));
    }
    let mut actual = BTreeSet::new();
    let mut subject_bindings = Vec::new();
    for subject in subjects.subjects {
        if !canonical_id(subject.principal_id, 256)
            || !canonical_id(subject.session_id, 256)
            || !decimal(subject.grant_revision).is_some_and(|n| n > 0)
            || !decimal(subject.configuration_revision).is_some_and(|n| n > 0)
            || subject.grant.validate().is_err()
            || !actual.insert((
                subject.principal_id.to_owned(),
                subject.session_id.to_owned(),
            ))
        {
            return Err(ManifestError::InvalidSubjects);
        }
        subject_bindings.push(SubjectBinding {
            principal_id: subject.principal_id.into(),
            session_id: subject.session_id.into(),
            grant_revision: subject.grant_revision.into(),
            configuration_revision: subject.configuration_revision.into(),
        });
    }
    if actual != expected {
        return Err(ManifestError::InvalidSubjects);
    }
    subject_bindings
        .sort_by(|a, b| (&a.principal_id, &a.session_id).cmp(&(&b.principal_id, &b.session_id)));
    let mut reasons = BTreeSet::from([ManifestReason::ApprovalRequired]);
    if entries[0].binding.kind == AutomationFamily::Event {
        match entries[0]
            .record
            .and_then(|record| record.get("trigger"))
            .and_then(Value::as_str)
        {
            Some(trigger) if canonical_id(trigger, 128) => {
                // Arbitrary emitted/plugin trigger contracts need their own
                // catalog and propagation policy, not an inferred owner grant.
                if !trigger.starts_with("tiktok.") {
                    if subjects.subjects.is_empty() {
                        reasons.insert(ManifestReason::PluginBindingUnavailable);
                    } else if !crate::collaboration::valid_emit_topic(trigger) {
                        reasons.insert(ManifestReason::InvalidConfiguration);
                    } else {
                        // A custom topic is an incoming scoped event contract,
                        // never permission to trust Owner/plugin-origin input.
                        // Runtime additionally requires an approved incoming
                        // context; this exact source digest binds the trigger.
                        for subject in subjects.subjects {
                            if !subject.grant.capabilities.contains(&Capability::EventsEmit) {
                                reasons.insert(ManifestReason::MissingCapability);
                            }
                            if !subject.grant.allows_emit_topic(trigger) {
                                reasons.insert(ManifestReason::ResourceForbidden);
                            }
                        }
                    }
                }
            }
            _ => {
                reasons.insert(ManifestReason::InvalidConfiguration);
            }
        }
    }
    for subject in subjects.subjects {
        if entries[0].binding.kind == AutomationFamily::Event
            && !subject.grant.capabilities.contains(&Capability::LiveRead)
        {
            reasons.insert(ManifestReason::MissingCapability);
        }
        if subject.grant.resources.contains_key(&ResourceKind::Live) {
            reasons.insert(ManifestReason::LiveRestriction);
        }
        if !subject
            .grant
            .capabilities
            .contains(&Capability::AutomationRead)
        {
            reasons.insert(ManifestReason::MissingCapability);
        }
        for entry in &entries {
            if !subject
                .grant
                .allows_automation(entry.binding.kind, &entry.binding.id)
            {
                reasons.insert(ManifestReason::ResourceForbidden);
            }
        }
    }
    let effects: Vec<_> = entries
        .iter()
        .filter(|entry| entry.binding.kind == AutomationFamily::Action)
        .map(|entry| classification::classify(entry, subjects.subjects))
        .collect();
    for effect in &effects {
        reasons.extend(effect.reasons.iter().copied());
    }
    let mut bindings = entries.into_iter().map(|entry| entry.binding);
    let primary = bindings.next().ok_or(ManifestError::InvalidReview)?;
    let mut manifest = ExecutionManifest {
        policy_version: POLICY_VERSION,
        digest_algorithm: DIGEST_ALGORITHM,
        host_id: subjects.host_id.into(),
        subjects: subject_bindings,
        primary,
        dependencies: bindings.collect(),
        effects,
        reasons: reasons.into_iter().collect(),
        execution_authorized: false,
        manifest_digest: String::new(),
    };
    let mut value = serde_json::to_value(&manifest).map_err(|_| ManifestError::InvalidReview)?;
    value
        .as_object_mut()
        .ok_or(ManifestError::InvalidReview)?
        .remove("manifestDigest");
    manifest.manifest_digest = digest("manifest", &value, REVIEW_BYTES)?;
    Ok(manifest)
}
