//! Final admission under current subjects, approvals and business writer gates.
use super::*;

impl ExecutionContext {
    /// Final synchronous admission. Callers hold the business writer and must
    /// commit it inside `apply`; no await or collaboration/index reentry here.
    #[cfg(feature = "persistence")]
    pub(crate) fn with_current_subjects<R>(
        &self,
        action: &serde_json::Value,
        automation: &crate::services::AutomationService,
        apply: impl FnOnce(&[crate::collaboration::ExecutionIdentity]) -> Result<R, String>,
    ) -> Result<R, String> {
        if self.depth >= DEPTH_LIMIT {
            return Err("automation emit depth limit reached".into());
        }
        #[cfg(feature = "persistence")]
        if let Authority::Approved {
            subjects,
            permits,
            owner_rules,
        } = &self.authority
        {
            return subjects
                .with_current(|identities| {
                    for permit in permits {
                        permit.check_subjects(identities).map_err(str::to_owned)?;
                    }
                    automation
                        .with_current_approvals(permits, owner_rules, action, || apply(identities))
                })
                .map_err(|_| "saved execution subjects are unavailable".to_owned())?;
        }
        let _ = (action, automation, apply);
        Err("delegated execution approval is unavailable".into())
    }

    /// Caller holds the automation writer gate. Points transactions also hold
    /// their own writer; acquire the competition writer before grant locks.
    #[cfg(feature = "persistence")]
    pub(crate) fn with_action_admission<R>(
        &self,
        core: &AppCore,
        action: &serde_json::Value,
        apply: impl FnOnce(&[crate::collaboration::ExecutionIdentity]) -> Result<R, String>,
    ) -> Result<R, String> {
        if self.is_competition() {
            self.with_competition_action(core, action, apply)
        } else {
            self.with_current_subjects(action, &core.automation, apply)
        }
    }

    /// Retain every inherited subject/definition fence during scoped event
    /// admission. This supplies no new action cover and cannot promote Owner.
    #[cfg(feature = "persistence")]
    pub(crate) fn with_current_chain<R>(
        &self,
        core: &AppCore,
        apply: impl FnOnce(&[crate::collaboration::ExecutionIdentity]) -> Result<R, String>,
    ) -> Result<R, String> {
        if self.depth >= DEPTH_LIMIT {
            return Err("automation emit depth limit reached".into());
        }
        let Authority::Approved {
            subjects,
            permits,
            owner_rules,
        } = &self.authority
        else {
            return Err("delegated execution approval is unavailable".into());
        };
        if self.is_competition() {
            return self.with_competition_binding(core, |identities, admission| {
                core.automation.with_current_competition_chain(
                    admission,
                    permits,
                    owner_rules,
                    || apply(identities),
                )
            });
        }
        subjects
            .with_current(|identities| {
                for permit in permits {
                    permit.check_subjects(identities).map_err(str::to_owned)?;
                }
                core.automation
                    .with_current_execution_chain(permits, owner_rules, || apply(identities))
            })
            .map_err(|_| "saved execution subjects are unavailable".to_owned())?
    }
}
