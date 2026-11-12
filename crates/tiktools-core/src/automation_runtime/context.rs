//! Trusted in-process execution restrictions. JSON cannot construct, erase or
//! serialize them. Retaining a subject is not an execution-manifest approval.
use crate::{collaboration::ExecutionAuthorizations, AppCore};

#[cfg(feature = "persistence")]
mod admission;
mod approval;
mod authority;
#[cfg(feature = "persistence")]
mod competition;
mod effects;
mod inheritance;

const DEPTH_LIMIT: u8 = 3;

#[derive(Clone)]
enum Authority {
    Owner,
    Restricted(ExecutionAuthorizations),
    #[cfg(feature = "persistence")]
    Approved {
        subjects: ExecutionAuthorizations,
        permits: Vec<crate::services::RuntimeApprovalPermit>,
        owner_rules: Vec<crate::services::RuntimeOwnerRuleFence>,
    },
    Denied,
}
#[derive(Clone)]
pub(crate) struct ExecutionContext {
    authority: Authority,
    depth: u8,
    script_flow: bool,
    // A native event without a verified creator may run the Owner's rules,
    // but must never gain delegated authority through a saved rule or emit.
    allow_delegation: bool,
    #[cfg(feature = "persistence")]
    execution: Option<crate::services::effects::EffectExecution>,
    #[cfg(feature = "persistence")]
    competition: Option<crate::db::CompetitionExecutionFence>,
    #[cfg(feature = "persistence")]
    competition_winner: Option<tiktools_competitions::ActionClaim>,
}

impl ExecutionContext {
    /// Only explicit trusted host/Owner producers use this constructor. Never
    /// infer Owner from source/depth/authority fields in an event body.
    pub(crate) fn owner() -> Self {
        Self {
            authority: Authority::Owner,
            depth: 0,
            script_flow: false,
            allow_delegation: true,
            #[cfg(feature = "persistence")]
            execution: crate::services::effects::EffectExecution::fresh().ok(),
            #[cfg(feature = "persistence")]
            competition: None,
            #[cfg(feature = "persistence")]
            competition_winner: None,
        }
    }

    pub(crate) fn owner_without_delegation() -> Self {
        let mut context = Self::owner();
        context.allow_delegation = false;
        context
    }

    pub(crate) fn denied() -> Self {
        Self {
            authority: Authority::Denied,
            depth: 0,
            script_flow: false,
            allow_delegation: true,
            #[cfg(feature = "persistence")]
            execution: crate::services::effects::EffectExecution::fresh().ok(),
            #[cfg(feature = "persistence")]
            competition: None,
            #[cfg(feature = "persistence")]
            competition_winner: None,
        }
    }

    #[cfg_attr(not(feature = "persistence"), allow(dead_code))]
    pub(crate) fn restricted(subjects: ExecutionAuthorizations) -> Result<Self, &'static str> {
        if subjects.is_empty() {
            return Err("empty subjects do not establish execution authority");
        }
        subjects
            .snapshot()
            .map_err(|_| "saved execution subjects are unavailable")?;
        Ok(Self {
            authority: Authority::Restricted(subjects),
            depth: 0,
            script_flow: false,
            allow_delegation: true,
            #[cfg(feature = "persistence")]
            execution: crate::services::effects::EffectExecution::fresh().ok(),
            #[cfg(feature = "persistence")]
            competition: None,
            #[cfg(feature = "persistence")]
            competition_winner: None,
        })
    }
}

#[cfg(all(test, feature = "persistence"))]
mod tests;
