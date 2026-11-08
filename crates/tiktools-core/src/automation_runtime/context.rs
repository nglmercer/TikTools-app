//! Trusted in-process execution restrictions. JSON cannot construct, erase or
//! serialize them. Retaining a subject is not an execution-manifest approval.
use crate::{collaboration::ExecutionAuthorizations, AppCore};

const DEPTH_LIMIT: u8 = 3;

#[derive(Clone)]
enum Authority {
    Owner,
    Restricted(ExecutionAuthorizations),
    #[cfg(feature = "persistence")]
    Approved {
        subjects: ExecutionAuthorizations,
        permits: Vec<crate::services::RuntimeApprovalPermit>,
    },
    Denied,
}
#[derive(Clone)]
pub(crate) struct ExecutionContext {
    authority: Authority,
    depth: u8,
    #[cfg(feature = "persistence")]
    execution: Option<crate::services::effects::EffectExecution>,
}
impl ExecutionContext {
    /// Only explicit trusted host/Owner producers use this constructor. Never
    /// infer Owner from source/depth/authority fields in an event body.
    pub(crate) fn owner() -> Self {
        Self {
            authority: Authority::Owner,
            depth: 0,
            #[cfg(feature = "persistence")]
            execution: crate::services::effects::EffectExecution::fresh().ok(),
        }
    }
    pub(crate) fn denied() -> Self {
        Self {
            authority: Authority::Denied,
            depth: 0,
            #[cfg(feature = "persistence")]
            execution: crate::services::effects::EffectExecution::fresh().ok(),
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
            #[cfg(feature = "persistence")]
            execution: crate::services::effects::EffectExecution::fresh().ok(),
        })
    }
    pub(crate) fn is_owner(&self) -> bool {
        matches!(self.authority, Authority::Owner)
    }
    pub(crate) fn is_approved(&self) -> bool {
        #[cfg(feature = "persistence")]
        return matches!(self.authority, Authority::Approved { .. });
        #[cfg(not(feature = "persistence"))]
        false
    }
    pub(crate) fn depth(&self) -> u8 {
        self.depth
    }
    pub(crate) fn child(&self) -> Result<Self, &'static str> {
        let depth = self
            .depth
            .checked_add(1)
            .filter(|depth| *depth < DEPTH_LIMIT)
            .ok_or("automation emit depth limit reached")?;
        Ok(Self {
            authority: self.authority.clone(),
            depth,
            #[cfg(feature = "persistence")]
            execution: self.execution.clone(),
        })
    }
    /// Owner is neutral; denied is absorbing; restricted subjects are combined
    /// conjunctively under one host service. Depth can never reset via nesting.
    pub(crate) fn intersect(&self, other: &Self) -> Result<Self, &'static str> {
        let authority = match (&self.authority, &other.authority) {
            (Authority::Denied, _) | (_, Authority::Denied) => Authority::Denied,
            (Authority::Owner, other) | (other, Authority::Owner) => other.clone(),
            (Authority::Restricted(a), Authority::Restricted(b)) => Authority::Restricted(
                a.intersect(b)
                    .map_err(|_| "execution subject intersection is unavailable")?,
            ),
            #[cfg(feature = "persistence")]
            (Authority::Restricted(a), Authority::Approved { subjects: b, .. })
            | (Authority::Approved { subjects: b, .. }, Authority::Restricted(a)) => {
                // Identity-only restrictions have no approval. Adding a valid
                // permit cannot erase the unapproved incoming restriction.
                Authority::Restricted(
                    a.intersect(b)
                        .map_err(|_| "execution subject intersection is unavailable")?,
                )
            }
            #[cfg(feature = "persistence")]
            (
                Authority::Approved {
                    subjects: a,
                    permits: ap,
                },
                Authority::Approved {
                    subjects: b,
                    permits: bp,
                },
            ) => {
                if ap.len().saturating_add(bp.len()) > 16 {
                    return Err("execution approval intersection exceeds its bound");
                }
                Authority::Approved {
                    subjects: a
                        .intersect(b)
                        .map_err(|_| "execution subject intersection is unavailable")?,
                    permits: ap.iter().chain(bp).cloned().collect(),
                }
            }
        };
        Ok(Self {
            authority,
            depth: self.depth.max(other.depth),
            #[cfg(feature = "persistence")]
            execution: self.execution.clone(),
        })
    }
    /// Revalidates identity/lifetime only. This does not approve definitions,
    /// grant effect capabilities or authorize any currently restricted effect.
    pub(crate) fn check_current(&self) -> Result<(), &'static str> {
        match &self.authority {
            Authority::Owner => Ok(()),
            Authority::Denied => Err("execution authority is unavailable"),
            Authority::Restricted(subjects) => subjects
                .snapshot()
                .map(|_| ())
                .map_err(|_| "saved execution subjects are unavailable"),
            #[cfg(feature = "persistence")]
            Authority::Approved { subjects, permits } => subjects
                .with_current(|identities| {
                    for permit in permits {
                        permit.check_subjects(identities)?;
                    }
                    Ok(())
                })
                .map_err(|_| "saved execution subjects are unavailable")?,
        }
    }

    /// Approved renderers receive only public scalar fields and no globals.
    pub(crate) fn projected_event(
        &self,
        event: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        if self.is_owner() {
            return Ok(event.clone());
        }
        if !self.is_approved() {
            return Err("delegated execution approval is unavailable".into());
        }
        self.check_current().map_err(str::to_owned)?;
        #[cfg(feature = "persistence")]
        return Ok(crate::services::automation_approval_policy::project_event(
            event,
        ));
        #[cfg(not(feature = "persistence"))]
        Err("delegated execution approval is unavailable".into())
    }

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
        if let Authority::Approved { subjects, permits } = &self.authority {
            return subjects
                .with_current(|identities| {
                    for permit in permits {
                        permit.check_subjects(identities).map_err(str::to_owned)?;
                    }
                    automation.with_current_approvals(permits, action, || apply(identities))
                })
                .map_err(|_| "saved execution subjects are unavailable".to_owned())?;
        }
        let _ = (action, automation, apply);
        Err("delegated execution approval is unavailable".into())
    }
    #[cfg(feature = "persistence")]
    pub(crate) fn with_execution(
        mut self,
        execution: crate::services::effects::EffectExecution,
    ) -> Self {
        self.execution = Some(execution);
        self
    }
    #[cfg(feature = "persistence")]
    pub(crate) fn execution(&self) -> Result<&crate::services::effects::EffectExecution, String> {
        self.execution
            .as_ref()
            .ok_or_else(|| "private execution identity is unavailable".into())
    }
    pub(crate) fn effect_named_slot(
        &self,
        event_id: &str,
        action_id: &str,
        occurrence: u32,
    ) -> Result<Self, String> {
        #[cfg(feature = "persistence")]
        {
            let execution = self
                .execution()?
                .named_child("saved-event", event_id)
                .and_then(|execution| execution.named_child("saved-action", action_id))
                .and_then(|execution| execution.child(occurrence))
                .map_err(|error| error.to_string())?;
            Ok(self.clone().with_execution(execution))
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (event_id, action_id, occurrence);
            Ok(self.clone())
        }
    }
    pub(crate) fn require_owner(&self) -> Result<(), &'static str> {
        if self.depth >= DEPTH_LIMIT {
            return Err("automation emit depth limit reached");
        }
        if self.is_owner() {
            Ok(())
        } else {
            Err("delegated execution approval is unavailable")
        }
    }
}

impl AppCore {
    /// Final unsupported-effect guard, before globals/settings, plugins or
    /// effect sinks. Shutdown applies even to otherwise trusted Owner work.
    /// Keep this guard Owner-only. Approved delegated effects need separate,
    /// closed effect/target policies; accepting a valid subject here would
    /// otherwise enable every unsupported transport and unrestricted read.
    pub(crate) fn execution_context_guard(&self, context: &ExecutionContext) -> Result<(), String> {
        if self.is_shutdown() {
            return Err("host is shutting down".into());
        }
        context.require_owner().map_err(str::to_owned)?;
        context.check_current().map_err(str::to_owned)
    }

    /// A queued/manual run keeps its original complete approval fence across
    /// every step. A later approval cannot authorize an older queued clone.
    pub(super) async fn record_execution_context(
        self: &std::sync::Arc<Self>,
        record: &crate::services::MatchedRecord,
        incoming: &ExecutionContext,
    ) -> Result<ExecutionContext, &'static str> {
        #[cfg(feature = "persistence")]
        if let Some(candidate) = record.approval_candidate() {
            return self.approved_candidate_context(candidate, incoming).await;
        }
        let _ = record;
        Ok(incoming.clone())
    }

    #[cfg(feature = "persistence")]
    async fn approved_candidate_context(
        self: &std::sync::Arc<Self>,
        candidate: crate::services::ApprovalCandidate,
        incoming: &ExecutionContext,
    ) -> Result<ExecutionContext, &'static str> {
        let core = self.clone();
        let approved = tokio::task::spawn_blocking(move || {
            let required = candidate.required_subjects()?;
            let service = core
                .collaboration()
                .map_err(|_| "saved execution subjects are unavailable")?;
            let subjects = service
                .execution_authorizations(&required)
                .map_err(|_| "saved execution subjects are unavailable")?;
            let permit = subjects
                .with_current(|identities| candidate.authorize(identities))
                .map_err(|_| "saved execution subjects are unavailable")??;
            Ok::<_, &'static str>(ExecutionContext {
                authority: Authority::Approved {
                    subjects,
                    permits: vec![permit],
                },
                depth: 0,
                #[cfg(feature = "persistence")]
                execution: crate::services::effects::EffectExecution::fresh().ok(),
            })
        })
        .await
        .map_err(|_| "execution approval resolution failed")??;
        incoming.intersect(&approved)
    }

    /// Retain trusted saved authors even on rejected manual/inline paths. A
    /// current saved subject still needs approval; it cannot become Owner by
    /// editing JSON or being invoked from an Owner event.
    pub(super) async fn action_execution_context(
        self: &std::sync::Arc<Self>,
        action: &serde_json::Value,
        origin: Option<&serde_json::Value>,
        incoming: &ExecutionContext,
    ) -> Result<ExecutionContext, &'static str> {
        if self.automation.owner_action_allowed(action, origin) {
            return Ok(incoming.clone());
        }
        #[cfg(feature = "persistence")]
        {
            if let Some(candidate) = self.automation.approval_candidate(action, origin) {
                return self.approved_candidate_context(candidate, incoming).await;
            }
            use crate::collaboration::AutomationFamily;
            let mut subjects = std::collections::BTreeSet::new();
            let mut capture = |family, id: &str| {
                let authority: Option<crate::services::RuntimeAuthority> =
                    self.automation.runtime_authority(family, id);
                if let Some(authority) = authority {
                    if let Some((principal, session)) = authority.subject() {
                        subjects.insert((principal.to_owned(), session.to_owned()));
                    }
                }
            };
            if let Some(id) = action.get("id").and_then(serde_json::Value::as_str) {
                capture(AutomationFamily::Action, id);
            }
            if let Some(event) = origin {
                if let Some(id) = event.get("id").and_then(serde_json::Value::as_str) {
                    capture(AutomationFamily::Event, id);
                }
                if let Some(ids) = event.get("actionIds").and_then(serde_json::Value::as_array) {
                    if ids.len() > 512 {
                        return incoming.intersect(&ExecutionContext::denied());
                    }
                    for id in ids.iter().filter_map(serde_json::Value::as_str) {
                        capture(AutomationFamily::Action, id);
                    }
                }
            }
            if !subjects.is_empty() {
                let core = self.clone();
                let subjects = subjects.into_iter().collect::<Vec<_>>();
                let restricted = tokio::task::spawn_blocking(move || {
                    let service = core
                        .collaboration()
                        .map_err(|_| "saved execution subjects are unavailable")?;
                    let subjects = service
                        .execution_authorizations(&subjects)
                        .map_err(|_| "saved execution subjects are unavailable")?;
                    ExecutionContext::restricted(subjects)
                })
                .await
                .map_err(|_| "execution subject resolution failed")??;
                return incoming.intersect(&restricted);
            }
        }
        incoming.intersect(&ExecutionContext::denied())
    }
}

#[cfg(all(test, feature = "persistence"))]
mod tests;
