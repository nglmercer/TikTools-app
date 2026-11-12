//! Saved approval candidates, reusable preflight and pinned Owner rule closures.
use super::*;

impl ApprovalCandidate {
    pub(in crate::services::automation) fn is_reusable(&self) -> bool {
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

    /// Capture a fresh reusable closure for a matched scoped rule, including
    /// Owner definitions. Its complete effect set must be authorized by all
    /// inherited subjects; the matched generation cannot be substituted.
    pub(crate) fn competition_rule_candidate(
        &self,
        record: &MatchedRecord,
    ) -> Option<ApprovalCandidate> {
        let index = recover_rwlock_read(&self.index, "automation index");
        if index.catalog_epoch != Some(record.catalog_epoch) {
            return None;
        }
        let id = record.definition.get("id")?.as_str()?;
        let (review, definitions) =
            capture_review(&index, AutomationFamily::Event, &record.definition)?;
        if record.generation != Some(definitions.first()?.generation)
            || record.definition.get("enabled").and_then(Value::as_bool) != Some(true)
            || record.actions.iter().any(|action| {
                !definitions.iter().any(|fence| {
                    fence.family == AutomationFamily::Action
                        && fence.generation == action.generation
                        && fence.definition == action.definition
                })
            })
        {
            return None;
        }
        Some(ApprovalCandidate {
            index_identity: index.identity.clone(),
            approval_key: ("event".into(), id.into()),
            approval: None,
            review,
            definitions,
            catalog_epoch: record.catalog_epoch,
        })
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
}

pub(in crate::services::automation) fn candidate_for_primary(
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

pub(in crate::services::automation) fn reusable_candidate_for_primary(
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

pub(in crate::services::automation) fn candidate_current(
    index: &BehaviorIndex,
    candidate: &ApprovalCandidate,
) -> bool {
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
