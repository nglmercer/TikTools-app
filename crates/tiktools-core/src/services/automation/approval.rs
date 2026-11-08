//! Trusted admission for durable, exact-definition approvals. Stored JSON is
//! evidence to compare, never a deserializable execution permit.
use super::{manifest, *};
use crate::collaboration::{
    AutomationFamily, ExecutionIdentity, ResourceKind, ResourceVersion, Versioned,
};
use serde_json::json;
use std::collections::BTreeSet;

fn approval_generation(approval: &crate::db::BehaviorApprovalSnapshot) -> Option<u64> {
    let generation = approval.generation.parse::<u64>().ok()?;
    (generation > 0
        && generation <= i64::MAX as u64
        && generation.to_string() == approval.generation)
        .then_some(generation)
}

/// Retain the invalidated generation even if its row is absent from the next
/// active snapshot. A delayed read cannot resurrect an invalidated permit.
pub(super) fn clear(index: &mut BehaviorIndex) {
    for (key, approval) in std::mem::take(&mut index.approvals) {
        let generation = approval_generation(&approval).unwrap_or(u64::MAX);
        let retired = index.retired_approvals.entry(key).or_default();
        *retired = (*retired).max(generation);
    }
}

pub(super) fn install(
    index: &mut BehaviorIndex,
    approvals: Vec<crate::db::BehaviorApprovalSnapshot>,
) {
    let mut active = BTreeMap::new();
    for approval in approvals {
        let key = (approval.scope.clone(), approval.id.clone());
        let generation = approval_generation(&approval);
        let previous = index.approvals.get(&key);
        if generation.is_none()
            || generation.is_some_and(|generation| {
                index
                    .retired_approvals
                    .get(&key)
                    .is_some_and(|retired| generation <= *retired)
                    || previous.is_some_and(|old| {
                        old != &approval
                            && approval_generation(old).is_none_or(|old| generation <= old)
                    })
            })
            || active.insert(key, approval).is_some()
        {
            clear(index);
            index.authority_available = false;
            return;
        }
    }
    let previous = std::mem::replace(&mut index.approvals, active);
    for (key, approval) in previous {
        if !index.approvals.contains_key(&key) {
            let generation = approval_generation(&approval).unwrap_or(u64::MAX);
            let retired = index.retired_approvals.entry(key).or_default();
            *retired = (*retired).max(generation);
        }
    }
    let invalid: Vec<_> = index
        .approvals
        .iter()
        .filter_map(|(key, approval)| {
            let valid = AutomationFamily::from_scope(&approval.scope).is_some_and(|family| {
                let primary = match family {
                    AutomationFamily::Action => index.actions.get(&approval.id),
                    AutomationFamily::Event => index.events.get(&approval.id),
                };
                primary.is_some_and(|primary| {
                    candidate_for_primary(index, family, &primary.definition).is_some()
                })
            });
            (!valid).then_some(key.clone())
        })
        .collect();
    for key in invalid {
        if let Some(approval) = index.approvals.remove(&key) {
            let retired = index.retired_approvals.entry(key).or_default();
            *retired = (*retired).max(approval_generation(&approval).unwrap_or(u64::MAX));
        }
    }
}

#[derive(Clone)]
struct DefinitionFence {
    family: AutomationFamily,
    id: String,
    definition: Value,
    revision: String,
    generation: u64,
    authority: RuntimeAuthority,
}

/// Captured only under the host index lock. It still supplies no execution
/// authority until the current subjects and closed policy have been checked.
#[derive(Clone)]
pub(crate) struct ApprovalCandidate {
    index_identity: std::sync::Arc<()>,
    approval: crate::db::BehaviorApprovalSnapshot,
    review: Versioned<Value>,
    definitions: Vec<DefinitionFence>,
    catalog_epoch: u64,
}

/// No public constructor, Serialize or Deserialize: only an exact persisted
/// approval, reconstructed manifest and current saved grants can create this.
#[derive(Clone)]
pub(crate) struct RuntimeApprovalPermit {
    candidate: ApprovalCandidate,
    manifest: manifest::ExecutionManifest,
    configuration_revisions: BTreeMap<(String, String), String>,
}

fn provenance(authority: &RuntimeAuthority) -> Option<Value> {
    let generation = authority.provenance_generation()?.to_string();
    let authority = match authority.subject() {
        Some((principal, session)) => {
            json!({"kind":"delegated","principalId":principal,"sessionId":session})
        }
        None if authority.is_owner() => json!({"kind":"owner"}),
        None => return None,
    };
    Some(json!({"authority":authority,"generation":generation}))
}

fn capture(index: &BehaviorIndex, family: AutomationFamily, id: &str) -> Option<DefinitionFence> {
    let record = match family {
        AutomationFamily::Action => index.actions.get(id),
        AutomationFamily::Event => index.events.get(id),
    }?;
    let authority = index
        .authorities
        .as_ref()?
        .get(&(family.scope().into(), id.into()))?;
    if authority.is_deleted()
        || record.persisted_revision.as_deref() != authority.resource_revision()
    {
        return None;
    }
    Some(DefinitionFence {
        family,
        id: id.into(),
        definition: record.definition.clone(),
        revision: record.persisted_revision.clone()?,
        generation: record.generation?,
        authority: authority.clone(),
    })
}

pub(super) fn candidate_for_primary(
    index: &BehaviorIndex,
    family: AutomationFamily,
    primary: &Value,
) -> Option<ApprovalCandidate> {
    if !index.authority_available {
        return None;
    }
    let id = primary.get("id")?.as_str()?;
    let approval = index.approvals.get(&(family.scope().into(), id.into()))?;
    if approval.policy_version != manifest::POLICY_VERSION {
        return None;
    }
    let primary_fence = capture(index, family, id)?;
    // An inline/manual body with the same ID never inherits a saved approval.
    if primary_fence.definition != *primary {
        return None;
    }
    let mut definitions = vec![primary_fence];
    if family == AutomationFamily::Event {
        if let Some(raw) = primary.get("actionIds") {
            let ids = raw.as_array()?;
            if ids.len() > 512 {
                return None;
            }
            let mut seen = BTreeSet::new();
            for raw in ids {
                let id = raw.as_str()?;
                if seen.insert(id) {
                    definitions.push(capture(index, AutomationFamily::Action, id)?);
                }
            }
        }
    }
    let mut dependencies = Vec::with_capacity(definitions.len().saturating_sub(1));
    let mut resource_versions = Vec::with_capacity(definitions.len());
    for (i, fence) in definitions.iter().enumerate() {
        resource_versions.push(ResourceVersion {
            kind: ResourceKind::Automation,
            scope: fence.family.scope().into(),
            id: fence.id.clone(),
            revision: fence.revision.clone(),
        });
        if i > 0 {
            dependencies.push(json!({"id":fence.id,"record":fence.definition,"provenance":provenance(&fence.authority)?}));
        }
    }
    let review = Versioned::new(
        json!({"kind":family.scope(),"id":id,"record":primary,"provenance":provenance(&definitions[0].authority)?,"dependencies":dependencies}),
        resource_versions,
    );
    // Validate closure shape and size before resolving any retained subjects.
    manifest::required_subjects(&review).ok()?;
    let bound_definitions = manifest::definition_bindings(&review).ok()?;
    let stored = approval.binding.get("manifest")?;
    if stored.get("policyVersion")?.as_u64()? != manifest::POLICY_VERSION as u64
        || stored.get("manifestDigest")?.as_str()? != approval.manifest_digest
        || stored.get("primary")? != &serde_json::to_value(&bound_definitions[0]).ok()?
        || stored.get("dependencies")? != &serde_json::to_value(&bound_definitions[1..]).ok()?
    {
        return None;
    }
    Some(ApprovalCandidate {
        index_identity: index.identity.clone(),
        approval: approval.clone(),
        review,
        definitions,
        catalog_epoch: index.catalog_epoch?,
    })
}

impl ApprovalCandidate {
    pub(crate) fn required_subjects(&self) -> Result<Vec<(String, String)>, &'static str> {
        manifest::required_subjects(&self.review).map_err(|_| "saved approval closure is invalid")
    }

    pub(crate) fn authorize(
        self,
        identities: &[ExecutionIdentity],
    ) -> Result<RuntimeApprovalPermit, &'static str> {
        let host = identities
            .first()
            .ok_or("saved approval subjects are unavailable")?
            .host_id();
        if identities.iter().any(|identity| identity.host_id() != host) {
            return Err("saved approval host binding is unavailable");
        }
        let subjects: Vec<_> = identities
            .iter()
            .map(|identity| manifest::ManifestSubject {
                principal_id: identity.principal_id(),
                session_id: identity.session_id(),
                grant_revision: identity.grant_revision(),
                configuration_revision: identity.configuration_revision(),
                grant: identity.grant(),
            })
            .collect();
        let manifest = manifest::build(
            &self.review,
            manifest::ManifestSubjects {
                host_id: host,
                subjects: &subjects,
            },
        )
        .map_err(|_| "saved approval manifest is unavailable")?;
        let policy = crate::services::automation_approval_policy::bindings(&manifest, identities)
            .map_err(|_| "saved approval effect policy is unavailable")?;
        let binding = json!({"manifest":manifest,"bindings":policy});
        if self.approval.manifest_digest != manifest.manifest_digest
            || self.approval.binding != binding
        {
            return Err("saved approval binding has changed");
        }
        let configuration_revisions = identities
            .iter()
            .map(|identity| {
                (
                    (
                        identity.principal_id().to_owned(),
                        identity.session_id().to_owned(),
                    ),
                    identity.configuration_revision().to_owned(),
                )
            })
            .collect();
        let permit = RuntimeApprovalPermit {
            candidate: self,
            manifest,
            configuration_revisions,
        };
        permit.check_subjects(identities)?;
        Ok(permit)
    }
}

impl RuntimeApprovalPermit {
    pub(crate) fn authorizes_action(&self, action: &Value) -> bool {
        self.candidate
            .definitions
            .iter()
            .any(|fence| fence.family == AutomationFamily::Action && fence.definition == *action)
    }

    /// Extra subjects from a conjoined context are allowed; every subject
    /// pinned by this exact approval must still have its approved grant.
    pub(crate) fn check_subjects(
        &self,
        identities: &[ExecutionIdentity],
    ) -> Result<(), &'static str> {
        for subject in &self.manifest.subjects {
            let identity = identities
                .iter()
                .find(|identity| {
                    identity.host_id() == self.manifest.host_id
                        && identity.principal_id() == subject.principal_id
                        && identity.session_id() == subject.session_id
                        && identity.grant_revision() == subject.grant_revision
                        && self
                            .configuration_revisions
                            .get(&(subject.principal_id.clone(), subject.session_id.clone()))
                            .is_some_and(|revision| revision == identity.configuration_revision())
                })
                .ok_or("saved approval grant binding has changed")?;
            identity
                .check_deadline()
                .map_err(|_| "saved approval subject has expired")?;
        }
        Ok(())
    }
}

impl AutomationService {
    pub(crate) fn approval_candidate(
        &self,
        action: &Value,
        origin: Option<&Value>,
    ) -> Option<ApprovalCandidate> {
        let index = recover_rwlock_read(&self.index, "automation index");
        let candidate = match origin {
            Some(primary) => candidate_for_primary(&index, AutomationFamily::Event, primary),
            None => candidate_for_primary(&index, AutomationFamily::Action, action),
        }?;
        candidate
            .definitions
            .iter()
            .any(|fence| fence.family == AutomationFamily::Action && fence.definition == *action)
            .then_some(candidate)
    }

    /// Hold the exact definition and approval fences through a synchronous
    /// sink. The closure must not reenter an automation index writer.
    pub(crate) fn with_current_approvals<R>(
        &self,
        permits: &[RuntimeApprovalPermit],
        action: &Value,
        apply: impl FnOnce() -> Result<R, String>,
    ) -> Result<R, String> {
        let index = recover_rwlock_read(&self.index, "automation index");
        if permits.is_empty()
            || permits
                .iter()
                .any(|permit| !permit.authorizes_action(action) || !permit_current(&index, permit))
        {
            return Err("saved approval definition binding has changed".into());
        }
        apply()
    }
}

pub(super) fn candidate_current(index: &BehaviorIndex, candidate: &ApprovalCandidate) -> bool {
    if !std::sync::Arc::ptr_eq(&index.identity, &candidate.index_identity)
        || !index.authority_available
        || index.catalog_epoch != Some(candidate.catalog_epoch)
    {
        return false;
    }
    let approval = &candidate.approval;
    if index
        .approvals
        .get(&(approval.scope.clone(), approval.id.clone()))
        != Some(approval)
    {
        return false;
    }
    candidate.definitions.iter().all(|fence| {
        capture(index, fence.family, &fence.id).is_some_and(|current| {
            current.generation == fence.generation
                && current.revision == fence.revision
                && current.definition == fence.definition
                && current.authority == fence.authority
        })
    })
}

fn permit_current(index: &BehaviorIndex, permit: &RuntimeApprovalPermit) -> bool {
    candidate_current(index, &permit.candidate)
}

#[cfg(test)]
mod tests;
