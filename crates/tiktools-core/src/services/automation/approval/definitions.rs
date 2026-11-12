//! Exact saved definition capture, provenance and borrowed current-state checks.
use super::*;

pub(super) fn provenance(authority: &RuntimeAuthority) -> Option<Value> {
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

pub(super) fn capture(
    index: &BehaviorIndex,
    family: AutomationFamily,
    id: &str,
) -> Option<DefinitionFence> {
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
pub(super) fn fence_current(index: &BehaviorIndex, fence: &DefinitionFence) -> bool {
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

pub(super) fn capture_review(
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
