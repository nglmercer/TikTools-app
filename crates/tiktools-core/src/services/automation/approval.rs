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
    authority_key: (String, String),
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
    approval_key: (String, String),
    approval: Option<crate::db::BehaviorApprovalSnapshot>,
    review: Versioned<Value>,
    definitions: Vec<DefinitionFence>,
    catalog_epoch: u64,
}
impl ApprovalCandidate {
    pub(super) fn is_reusable(&self) -> bool {
        self.approval.is_none()
    }
    pub(crate) fn requires_scoped_input(&self) -> bool {
        self.definitions.iter().any(|fence| {
            fence.family == AutomationFamily::Event
                && fence
                    .definition
                    .get("trigger")
                    .and_then(Value::as_str)
                    .is_some_and(|trigger| !trigger.starts_with("tiktok."))
        })
    }
}

/// No public constructor, Serialize or Deserialize: only a trusted saved
/// closure and current saved grants can create this.
#[derive(Clone)]
pub(crate) struct RuntimeApprovalPermit {
    candidate: ApprovalCandidate,
    manifest: manifest::ExecutionManifest,
    configuration_revisions: BTreeMap<(String, String), String>,
    competition: Option<crate::db::CompetitionExecutionFence>,
    competition_winner: Option<(tiktools_competitions::ActionClaim, Value)>,
}

/// Receiving approved scoped input never transfers Owner authority. This
/// private fence pins the saved Owner rule and its entire action closure.
#[derive(Clone)]
pub(crate) struct RuntimeOwnerRuleFence {
    index_identity: std::sync::Arc<()>,
    definitions: Vec<DefinitionFence>,
    catalog_epoch: u64,
}
impl RuntimeOwnerRuleFence {
    fn authorizes_action(&self, action: &Value) -> bool {
        self.definitions
            .iter()
            .any(|fence| fence.family == AutomationFamily::Action && fence.definition == *action)
    }
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
    let authority_key = (family.scope().into(), id.into());
    let authority = index.authorities.as_ref()?.get(&authority_key)?;
    if authority.is_deleted()
        || record.persisted_revision.as_deref() != authority.resource_revision()
    {
        return None;
    }
    Some(DefinitionFence {
        family,
        id: id.into(),
        authority_key,
        definition: record.definition.clone(),
        revision: record.persisted_revision.clone()?,
        generation: record.generation?,
        authority: authority.clone(),
    })
}

/// Borrow the installed record and sidecar during frequent VM authority
/// checks. Capture owns the original snapshots; polling must not clone their
/// JSON, revision/subject strings or composite lookup keys again. Callers keep
/// their existing runtime-identity, catalog and approval-generation fences.
fn fence_current(index: &BehaviorIndex, fence: &DefinitionFence) -> bool {
    if fence.authority_key.0 != fence.family.scope() || fence.authority_key.1 != fence.id {
        return false;
    }
    let record = match fence.family {
        AutomationFamily::Action => index.actions.get(fence.id.as_str()),
        AutomationFamily::Event => index.events.get(fence.id.as_str()),
    };
    let authority = index
        .authorities
        .as_ref()
        .and_then(|authorities| authorities.get(&fence.authority_key));
    match (record, authority) {
        (Some(record), Some(authority)) => {
            !authority.is_deleted()
                && record.persisted_revision.as_deref() == authority.resource_revision()
                && record.persisted_revision.as_deref() == Some(fence.revision.as_str())
                && record.generation == Some(fence.generation)
                && record.definition == fence.definition
                && authority == &fence.authority
        }
        _ => false,
    }
}

fn capture_review(
    index: &BehaviorIndex,
    family: AutomationFamily,
    primary: &Value,
) -> Option<(Versioned<Value>, Vec<DefinitionFence>)> {
    if !index.authority_available {
        return None;
    }
    let id = primary.get("id")?.as_str()?;
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
    Some((review, definitions))
}

pub(super) fn candidate_for_primary(
    index: &BehaviorIndex,
    family: AutomationFamily,
    primary: &Value,
) -> Option<ApprovalCandidate> {
    let id = primary.get("id")?.as_str()?;
    let approval = index.approvals.get(&(family.scope().into(), id.into()))?;
    if approval.policy_version != manifest::POLICY_VERSION {
        return None;
    }
    let (review, definitions) = capture_review(index, family, primary)?;
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
        approval_key: (approval.scope.clone(), approval.id.clone()),
        approval: Some(approval.clone()),
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
        self.authorize_with_inherited(identities, &[], None, None)
    }

    /// Call inside the competition fence's current-subject transaction. The
    /// permit still needs that fence at each effect sink; it never conveys
    /// competition authority by itself. Persisted approvals cannot be widened.
    #[allow(dead_code)]
    pub(crate) fn authorize_competition(
        self,
        identities: &[ExecutionIdentity],
        fence: &crate::db::CompetitionExecutionFence,
    ) -> Result<RuntimeApprovalPermit, &'static str> {
        let operators = fence.subjects();
        if operators.is_empty()
            || self.approval.is_some()
            || identities
                .iter()
                .any(|identity| identity.host_id() != fence.host_id())
        {
            return Err("competition operator binding is unavailable");
        }
        self.authorize_with_inherited(identities, &operators, Some(fence.clone()), None)
    }

    /// Classify the effective winner action, including the exact saved input
    /// overrides, while the winner outbox and competition fence are held.
    #[allow(dead_code)]
    pub(crate) fn authorize_competition_winner(
        self,
        admission: &crate::db::CompetitionExecutionAdmission<'_>,
    ) -> Result<RuntimeApprovalPermit, &'static str> {
        let claim = admission
            .winner()
            .ok_or("competition winner binding is unavailable")?;
        if self.approval.is_some()
            || self.approval_key != ("action".into(), claim.action.action_id.clone())
            || admission
                .identities()
                .iter()
                .any(|identity| identity.host_id() != admission.fence().host_id())
        {
            return Err("competition winner binding is unavailable");
        }
        let mut effective = self
            .review
            .value
            .get("record")
            .cloned()
            .ok_or("saved winner action is unavailable")?;
        let inputs = match &claim.action.inputs {
            Value::Null => serde_json::Map::new(),
            Value::Object(inputs) => inputs.clone(),
            _ => return Err("competition winner inputs are invalid"),
        };
        let config = effective
            .as_object_mut()
            .ok_or("saved winner action is invalid")?
            .entry("config")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .ok_or("saved winner action is invalid")?;
        config.extend(inputs);
        let operators = admission.fence().subjects();
        if operators.is_empty() {
            return Err("competition operator binding is unavailable");
        }
        self.authorize_with_inherited(
            admission.identities(),
            &operators,
            Some(admission.fence().clone()),
            Some((claim.clone(), effective)),
        )
    }

    fn authorize_with_inherited(
        self,
        identities: &[ExecutionIdentity],
        inherited: &[(String, String)],
        competition: Option<crate::db::CompetitionExecutionFence>,
        competition_winner: Option<(tiktools_competitions::ActionClaim, Value)>,
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
        let mut effective_review = self.review.clone();
        if let Some((_, effective)) = &competition_winner {
            effective_review.value["record"] = effective.clone();
        }
        let manifest = manifest::build_with_inherited_subjects(
            &effective_review,
            manifest::ManifestSubjects {
                host_id: host,
                subjects: &subjects,
            },
            inherited,
        )
        .map_err(|_| "saved approval manifest is unavailable")?;
        if let Some(approval) = &self.approval {
            let policy =
                crate::services::automation_approval_policy::bindings(&manifest, identities)
                    .map_err(|_| "saved approval effect policy is unavailable")?;
            let binding = json!({"manifest":manifest,"bindings":policy});
            if approval.manifest_digest != manifest.manifest_digest || approval.binding != binding {
                return Err("saved approval binding has changed");
            }
        } else {
            crate::services::automation_approval_policy::reusable_preflight(&manifest, identities)
                .map_err(|_| "reusable delegation is unavailable")?;
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
            competition,
            competition_winner,
        };
        permit.check_subjects(identities)?;
        Ok(permit)
    }
}

impl RuntimeApprovalPermit {
    pub(crate) fn authorizes_action(&self, action: &Value) -> bool {
        if let Some((_, effective)) = &self.competition_winner {
            return effective == action;
        }
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
    /// Candidate only: the saved competition winner reference and generation
    /// must be revalidated under the business writer before it can be used.
    #[allow(dead_code)]
    pub(crate) fn competition_action_candidate(&self, id: &str) -> Option<ApprovalCandidate> {
        let index = recover_rwlock_read(&self.index, "automation index");
        let primary = index.actions.get(id)?;
        let (review, definitions) =
            capture_review(&index, AutomationFamily::Action, &primary.definition)?;
        Some(ApprovalCandidate {
            index_identity: index.identity.clone(),
            approval_key: ("action".into(), id.into()),
            approval: None,
            review,
            definitions,
            catalog_epoch: index.catalog_epoch?,
        })
    }
    /// Capture saved host state only; the caller must still authorize its subjects.
    #[allow(dead_code)]
    pub(crate) fn reusable_candidate(
        &self,
        family: AutomationFamily,
        id: &str,
    ) -> Option<ApprovalCandidate> {
        let index = recover_rwlock_read(&self.index, "automation index");
        reusable_candidate_for_primary(&index, family, id)
    }
    /// Check that a DB review describes the currently installed runtime
    /// closure. This comparison is advisory; it cannot authorize execution.
    pub(crate) fn reusable_preflight_current(
        &self,
        expected: &manifest::ExecutionManifest,
        identities: &[ExecutionIdentity],
    ) -> bool {
        let index = recover_rwlock_read(&self.index, "automation index");
        if !index.authority_available || index.catalog_epoch.is_none() {
            return false;
        }
        let family = expected.primary.kind;
        let primary = match family {
            AutomationFamily::Action => index.actions.get(&expected.primary.id),
            AutomationFamily::Event => index.events.get(&expected.primary.id),
        };
        let Some((review, _)) =
            primary.and_then(|record| capture_review(&index, family, &record.definition))
        else {
            return false;
        };
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
        manifest::build(
            &review,
            manifest::ManifestSubjects {
                host_id: &expected.host_id,
                subjects: &subjects,
            },
        )
        .is_ok_and(|actual| actual == *expected)
    }
    pub(crate) fn owner_rule_fence(&self, record: &MatchedRecord) -> Option<RuntimeOwnerRuleFence> {
        let index = recover_rwlock_read(&self.index, "automation index");
        if !index.authority_available
            || index.catalog_epoch != Some(record.catalog_epoch)
            || record.approval.is_some()
        {
            return None;
        }
        let primary = capture(
            &index,
            AutomationFamily::Event,
            record.definition.get("id")?.as_str()?,
        )?;
        if primary.definition != record.definition
            || record.generation != Some(primary.generation)
            || !primary.authority.is_owner()
            || primary.definition.get("enabled").and_then(Value::as_bool) != Some(true)
        {
            return None;
        }
        let ids = primary.definition.get("actionIds")?.as_array()?;
        if ids.len() > 512 || ids.iter().any(|id| id.as_str().is_none()) {
            return None;
        }
        let ids = ids
            .iter()
            .filter_map(Value::as_str)
            .collect::<BTreeSet<_>>();
        let mut definitions = Vec::with_capacity(ids.len() + 1);
        for id in ids {
            let action = capture(&index, AutomationFamily::Action, id)?;
            if !action.authority.is_owner() {
                return None;
            }
            definitions.push(action);
        }
        if record.actions.iter().any(|action| {
            !definitions.iter().any(|fence| {
                fence.family == AutomationFamily::Action
                    && fence.generation == action.generation
                    && fence.definition == action.definition
            })
        }) {
            return None;
        }
        definitions.push(primary);
        Some(RuntimeOwnerRuleFence {
            index_identity: index.identity.clone(),
            definitions,
            catalog_epoch: record.catalog_epoch,
        })
    }

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
        owner_rules: &[RuntimeOwnerRuleFence],
        action: &Value,
        apply: impl FnOnce() -> Result<R, String>,
    ) -> Result<R, String> {
        let index = recover_rwlock_read(&self.index, "automation index");
        if !execution_chain_current(&index, permits, owner_rules, None)
            || !(permits
                .iter()
                .any(|permit| permit.authorizes_action(action))
                || owner_rules
                    .iter()
                    .any(|rule| rule.authorizes_action(action)))
        {
            return Err("saved approval definition binding has changed".into());
        }
        apply()
    }

    /// The admission proof is available only while the matching competition
    /// transaction and every current subject are locked by the database.
    #[allow(dead_code)]
    pub(crate) fn with_current_competition_approvals<R>(
        &self,
        admission: &crate::db::CompetitionExecutionAdmission<'_>,
        permits: &[RuntimeApprovalPermit],
        owner_rules: &[RuntimeOwnerRuleFence],
        action: &Value,
        apply: impl FnOnce() -> Result<R, String>,
    ) -> Result<R, String> {
        let index = recover_rwlock_read(&self.index, "automation index");
        if !execution_chain_current(&index, permits, owner_rules, Some(admission.fence()))
            || permits.iter().any(|permit| {
                permit.check_subjects(admission.identities()).is_err()
                    || permit
                        .competition_winner
                        .as_ref()
                        .is_some_and(|(claim, _)| admission.winner() != Some(claim))
            })
            || !(permits
                .iter()
                .any(|permit| permit.authorizes_action(action))
                || owner_rules
                    .iter()
                    .any(|rule| rule.authorizes_action(action)))
        {
            return Err("saved competition approval definition binding has changed".into());
        }
        apply()
    }

    pub(crate) fn with_current_execution_chain<R>(
        &self,
        permits: &[RuntimeApprovalPermit],
        owner_rules: &[RuntimeOwnerRuleFence],
        apply: impl FnOnce() -> Result<R, String>,
    ) -> Result<R, String> {
        let index = recover_rwlock_read(&self.index, "automation index");
        if !execution_chain_current(&index, permits, owner_rules, None) {
            return Err("inherited saved execution binding has changed".into());
        }
        apply()
    }
}

fn execution_chain_current(
    index: &BehaviorIndex,
    permits: &[RuntimeApprovalPermit],
    owner_rules: &[RuntimeOwnerRuleFence],
    competition: Option<&crate::db::CompetitionExecutionFence>,
) -> bool {
    !permits.is_empty()
        && permits.iter().all(|permit| {
            permit
                .competition
                .as_ref()
                .is_none_or(|fence| Some(fence) == competition)
                && permit_current(index, permit)
        })
        && (competition.is_none()
            || permits
                .iter()
                .any(|permit| permit.competition.as_ref() == competition))
        && owner_rules.iter().all(|rule| {
            std::sync::Arc::ptr_eq(&index.identity, &rule.index_identity)
                && index.authority_available
                && index.catalog_epoch == Some(rule.catalog_epoch)
                && rule
                    .definitions
                    .iter()
                    .all(|fence| fence.authority.is_owner() && fence_current(index, fence))
        })
}

pub(super) fn reusable_candidate_for_primary(
    index: &BehaviorIndex,
    family: AutomationFamily,
    id: &str,
) -> Option<ApprovalCandidate> {
    let primary = match family {
        AutomationFamily::Action => index.actions.get(id),
        AutomationFamily::Event => index.events.get(id),
    }?;
    let (review, definitions) = capture_review(index, family, &primary.definition)?;
    if manifest::required_subjects(&review).ok()?.is_empty() {
        return None;
    }
    Some(ApprovalCandidate {
        index_identity: index.identity.clone(),
        approval_key: (family.scope().into(), id.into()),
        approval: None,
        review,
        definitions,
        catalog_epoch: index.catalog_epoch?,
    })
}

pub(super) fn candidate_current(index: &BehaviorIndex, candidate: &ApprovalCandidate) -> bool {
    if !std::sync::Arc::ptr_eq(&index.identity, &candidate.index_identity)
        || !index.authority_available
        || index.catalog_epoch != Some(candidate.catalog_epoch)
    {
        return false;
    }
    if let Some(approval) = &candidate.approval {
        if candidate.approval_key.0 != approval.scope
            || candidate.approval_key.1 != approval.id
            || index.approvals.get(&candidate.approval_key) != Some(approval)
        {
            return false;
        }
    }
    candidate
        .definitions
        .iter()
        .all(|fence| fence_current(index, fence))
}

fn permit_current(index: &BehaviorIndex, permit: &RuntimeApprovalPermit) -> bool {
    candidate_current(index, &permit.candidate)
}

#[cfg(test)]
mod tests;
