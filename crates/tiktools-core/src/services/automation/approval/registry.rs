//! Monotonic approval generations, retirement and snapshot installation.
use super::*;

fn approval_generation(approval: &crate::db::BehaviorApprovalSnapshot) -> Option<u64> {
    let generation = approval.generation.parse::<u64>().ok()?;
    (generation > 0
        && generation <= i64::MAX as u64
        && generation.to_string() == approval.generation)
        .then_some(generation)
}

/// Retain the invalidated generation even if its row is absent from the next
/// active snapshot. A delayed read cannot resurrect an invalidated permit.
pub(in crate::services::automation) fn clear(index: &mut BehaviorIndex) {
    for (key, approval) in std::mem::take(&mut index.approvals) {
        let generation = approval_generation(&approval).unwrap_or(u64::MAX);
        let retired = index.retired_approvals.entry(key).or_default();
        *retired = (*retired).max(generation);
    }
}

pub(in crate::services::automation) fn install(
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
