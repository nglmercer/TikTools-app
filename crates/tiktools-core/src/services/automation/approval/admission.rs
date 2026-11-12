//! Current definition and approval fences held through synchronous effect sinks.
use super::*;

impl RuntimeOwnerRuleFence {
    fn authorizes_action(&self, action: &Value) -> bool {
        self.definitions
            .iter()
            .any(|fence| fence.family == AutomationFamily::Action && fence.definition == *action)
    }
}

impl AutomationService {
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

pub(super) fn execution_chain_current(
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

fn permit_current(index: &BehaviorIndex, permit: &RuntimeApprovalPermit) -> bool {
    candidate_current(index, &permit.candidate)
}
