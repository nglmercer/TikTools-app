//! Resolution of saved approvals and inherited authors for queued rules and actions.
use super::*;

impl AppCore {
    /// A queued/manual run keeps its original complete approval fence across
    /// every step. A later approval cannot authorize an older queued clone.
    pub(in crate::automation_runtime) async fn record_execution_context(
        self: &std::sync::Arc<Self>,
        record: &crate::services::MatchedRecord,
        incoming: &ExecutionContext,
    ) -> Result<ExecutionContext, &'static str> {
        #[cfg(feature = "persistence")]
        {
            let context = if incoming.is_competition()
                && matches!(&incoming.authority, Authority::Approved { permits, .. } if permits.iter().any(|permit| permit.authorizes_event(&record.definition)))
            {
                incoming.clone()
            } else if incoming.is_competition() {
                let candidate = self
                    .automation
                    .competition_rule_candidate(record)
                    .ok_or("saved competition rule is unavailable")?;
                self.approved_candidate_context(candidate, incoming).await?
            } else if let Some(candidate) = record.approval_candidate() {
                self.approved_candidate_context(candidate, incoming).await?
            } else if incoming.is_approved() {
                let fence = self
                    .automation
                    .owner_rule_fence(record)
                    .ok_or("saved Owner rule binding is unavailable")?;
                let mut context = incoming.clone();
                let Authority::Approved { owner_rules, .. } = &mut context.authority else {
                    return Err("scoped rule authority is unavailable");
                };
                if owner_rules.len() >= 16 {
                    return Err("execution Owner rule intersection exceeds its bound");
                }
                owner_rules.push(fence);
                context
            } else {
                incoming.clone()
            };
            if incoming.is_approved() {
                let id = record
                    .definition
                    .get("id")
                    .and_then(serde_json::Value::as_str)
                    .ok_or("saved child rule identity is unavailable")?;
                context
                    .with_current_chain(self, |identities| {
                        use crate::collaboration::{AutomationFamily, Capability, ResourceKind};
                        if identities.iter().any(|identity| {
                            let grant = identity.grant();
                            !grant.capabilities.contains(&Capability::AutomationRead)
                                || !grant.capabilities.contains(&Capability::LiveRead)
                                || !grant.allows_automation(AutomationFamily::Event, id)
                                || grant.resources.contains_key(&ResourceKind::Live)
                        }) {
                            Err(
                                "inherited execution subject cannot observe the child rule input"
                                    .into(),
                            )
                        } else {
                            Ok(())
                        }
                    })
                    .map_err(|_| {
                        "inherited execution subject cannot observe the child rule input"
                    })?;
            }
            Ok(context)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = record;
            Ok(incoming.clone())
        }
    }

    #[cfg(feature = "persistence")]
    async fn approved_candidate_context(
        self: &std::sync::Arc<Self>,
        candidate: crate::services::ApprovalCandidate,
        incoming: &ExecutionContext,
    ) -> Result<ExecutionContext, &'static str> {
        if candidate.requires_scoped_input()
            && !incoming.is_approved()
            && !incoming.is_competition()
        {
            return Err("this saved approval requires approved scoped input");
        }
        let native_competition_event =
            incoming.is_competition() && matches!(incoming.authority, Authority::Restricted(_));
        let inherited_competition = match (
            &incoming.competition,
            &incoming.competition_winner,
            &incoming.authority,
        ) {
            (
                Some(fence),
                claim,
                Authority::Approved { subjects, .. } | Authority::Restricted(subjects),
            ) => {
                let inherited = subjects
                    .snapshot()
                    .map_err(|_| "saved execution subjects are unavailable")?
                    .iter()
                    .map(|identity| {
                        (
                            identity.principal_id().to_owned(),
                            identity.session_id().to_owned(),
                        )
                    })
                    .collect::<Vec<_>>();
                Some((fence.clone(), claim.clone(), inherited))
            }
            _ => None,
        };
        let core = self.clone();
        let approved = tokio::task::spawn_blocking(move || {
            let required = candidate.required_subjects()?;
            let service = core
                .collaboration()
                .map_err(|_| "saved execution subjects are unavailable")?;
            let (subjects, permit) = if let Some((fence, claim, inherited)) = inherited_competition
            {
                let mut all = inherited.clone();
                for subject in &required {
                    if !all.contains(subject) {
                        all.push(subject.clone());
                    }
                }
                let subjects = service
                    .execution_authorizations(&all)
                    .map_err(|_| "saved execution subjects are unavailable")?;
                let permit = if let Some(claim) = claim {
                    core.db.with_competition_winner_admission(
                        &service,
                        &fence,
                        &claim,
                        &all,
                        |_, _, admission, _| {
                            candidate
                                .authorize_competition_chain(
                                    admission,
                                    &inherited,
                                    native_competition_event,
                                )
                                .map_err(|error| crate::db::DatabaseError::Invalid(error.into()))
                        },
                    )
                } else {
                    core.db.with_competition_execution_fence_and_subjects(
                        &service,
                        &fence,
                        &all,
                        |_, _, admission| {
                            candidate
                                .authorize_competition_chain(
                                    admission,
                                    &inherited,
                                    native_competition_event,
                                )
                                .map_err(|error| crate::db::DatabaseError::Invalid(error.into()))
                        },
                    )
                }
                .map_err(|_| "competition chain authorization is unavailable")?;
                (subjects, permit)
            } else {
                let subjects = service
                    .execution_authorizations(&required)
                    .map_err(|_| "saved execution subjects are unavailable")?;
                let permit = subjects
                    .with_current(|identities| candidate.authorize(identities))
                    .map_err(|_| "saved execution subjects are unavailable")??;
                (subjects, permit)
            };
            Ok::<_, &'static str>(ExecutionContext {
                authority: Authority::Approved {
                    subjects,
                    permits: vec![permit],
                    owner_rules: Vec::new(),
                },
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
        })
        .await
        .map_err(|_| "execution approval resolution failed")??;
        if incoming.is_competition() && matches!(incoming.authority, Authority::Restricted(_)) {
            let mut context = incoming.clone();
            context.authority = approved.authority;
            return Ok(context);
        }
        incoming.intersect(&approved)
    }

    /// Retain trusted saved authors even on rejected manual/inline paths. A
    /// current saved subject still needs approval; it cannot become Owner by
    /// editing JSON or being invoked from an Owner event.
    pub(in crate::automation_runtime) async fn action_execution_context(
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
            // A retained permit may already cover this exact saved action as
            // part of its pinned closure. Recheck the full chain before reuse.
            if incoming.is_competition() {
                // The core effect guard validates the exact winner and permit
                // under the business writer before any effect is admitted.
                return Ok(incoming.clone());
            }
            if incoming.is_approved()
                && incoming
                    .with_current_subjects(action, &self.automation, |_| Ok(()))
                    .is_ok()
            {
                return Ok(incoming.clone());
            }
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
