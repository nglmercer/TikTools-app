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
    /// Records an inherited competition binding without making it an effect
    /// approval. Restricted contexts continue to fail closed at dispatch.
    #[cfg(feature = "persistence")]
    pub(crate) fn restricted_competition(
        subjects: ExecutionAuthorizations,
        fence: crate::db::CompetitionExecutionFence,
    ) -> Result<Self, &'static str> {
        let mut context = Self::restricted(subjects)?;
        context.competition = Some(fence);
        Ok(context)
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
            script_flow: self.script_flow,
            allow_delegation: self.allow_delegation,
            #[cfg(feature = "persistence")]
            execution: self.execution.clone(),
            #[cfg(feature = "persistence")]
            competition: self.competition.clone(),
            #[cfg(feature = "persistence")]
            competition_winner: self.competition_winner.clone(),
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
                    owner_rules: ar,
                },
                Authority::Approved {
                    subjects: b,
                    permits: bp,
                    owner_rules: br,
                },
            ) => {
                if ap.len().saturating_add(bp.len()) > 16 || ar.len().saturating_add(br.len()) > 16
                {
                    return Err("execution approval intersection exceeds its bound");
                }
                Authority::Approved {
                    subjects: a
                        .intersect(b)
                        .map_err(|_| "execution subject intersection is unavailable")?,
                    permits: ap.iter().chain(bp).cloned().collect(),
                    owner_rules: ar.iter().chain(br).cloned().collect(),
                }
            }
        };
        let allow_delegation = self.allow_delegation && other.allow_delegation;
        if !allow_delegation && !matches!(authority, Authority::Owner | Authority::Denied) {
            return Err("unverified LIVE input cannot authorize delegated execution");
        }
        #[cfg(feature = "persistence")]
        let competition = match (&self.competition, &other.competition) {
            (Some(a), Some(b)) if a != b => return Err("competition execution bindings conflict"),
            (Some(fence), _) | (_, Some(fence)) => Some(fence.clone()),
            _ => None,
        };
        #[cfg(feature = "persistence")]
        let competition_winner = match (&self.competition_winner, &other.competition_winner) {
            (Some(a), Some(b)) if a != b => return Err("competition winner bindings conflict"),
            (Some(claim), _) | (_, Some(claim)) => Some(claim.clone()),
            _ => None,
        };
        Ok(Self {
            authority,
            depth: self.depth.max(other.depth),
            script_flow: self.script_flow || other.script_flow,
            allow_delegation,
            #[cfg(feature = "persistence")]
            execution: self.execution.clone(),
            #[cfg(feature = "persistence")]
            competition,
            #[cfg(feature = "persistence")]
            competition_winner,
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
            Authority::Approved {
                subjects, permits, ..
            } => subjects
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

    /// Revalidate the immutable winner, current operators and every saved
    /// automation author at the effect boundary under one business writer.
    #[cfg(feature = "persistence")]
    pub(crate) fn with_competition_action<R>(
        &self,
        core: &AppCore,
        action: &serde_json::Value,
        apply: impl FnOnce(&[crate::collaboration::ExecutionIdentity]) -> Result<R, String>,
    ) -> Result<R, String> {
        let (
            Some(fence),
            Some(claim),
            Authority::Approved {
                subjects,
                permits,
                owner_rules,
            },
        ) = (&self.competition, &self.competition_winner, &self.authority)
        else {
            return Err("competition winner authority is unavailable".into());
        };
        let additional = subjects
            .snapshot()
            .map_err(|_| "saved execution subjects are unavailable")?
            .iter()
            .map(|id| (id.principal_id().to_owned(), id.session_id().to_owned()))
            .collect::<Vec<_>>();
        let service = core
            .collaboration()
            .map_err(|_| "collaboration is unavailable")?;
        core.db
            .with_competition_winner_admission(
                &service,
                fence,
                claim,
                &additional,
                |_, identities, admission, _| {
                    core.automation
                        .with_current_competition_approvals(
                            admission,
                            permits,
                            owner_rules,
                            action,
                            || apply(identities),
                        )
                        .map_err(crate::db::DatabaseError::Invalid)
                },
            )
            .map_err(|error| error.to_string())
    }
    /// Only invoked by the effect journal while its SQLite writer is held.
    #[cfg(feature = "persistence")]
    pub(crate) fn with_competition_journal_admission<R>(
        &self,
        core: &AppCore,
        action: &serde_json::Value,
        apply: impl FnOnce(&[crate::collaboration::ExecutionIdentity]) -> Result<R, String>,
    ) -> Result<R, String> {
        let (
            Some(fence),
            Some(claim),
            Authority::Approved {
                subjects,
                permits,
                owner_rules,
            },
        ) = (&self.competition, &self.competition_winner, &self.authority)
        else {
            return Err("competition winner authority is unavailable".into());
        };
        let additional = subjects
            .snapshot()
            .map_err(|_| "saved execution subjects are unavailable")?
            .iter()
            .map(|id| (id.principal_id().to_owned(), id.session_id().to_owned()))
            .collect::<Vec<_>>();
        let service = core
            .collaboration()
            .map_err(|_| "collaboration is unavailable")?;
        core.db
            .with_competition_winner_journal_admission(
                &service,
                fence,
                claim,
                &additional,
                |identities, admission| {
                    core.automation
                        .with_current_competition_approvals(
                            admission,
                            permits,
                            owner_rules,
                            action,
                            || apply(identities),
                        )
                        .map_err(crate::db::DatabaseError::Invalid)
                },
            )
            .map_err(|error| error.to_string())
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
        if self.is_competition_winner() {
            self.with_competition_action(core, action, apply)
        } else {
            self.with_current_subjects(action, &core.automation, apply)
        }
    }

    #[cfg(feature = "persistence")]
    pub(crate) fn is_competition_winner(&self) -> bool {
        self.competition_winner.is_some()
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
        if let (Some(fence), Some(claim)) = (&self.competition, &self.competition_winner) {
            let additional = subjects
                .snapshot()
                .map_err(|_| "saved execution subjects are unavailable")?
                .iter()
                .map(|id| (id.principal_id().to_owned(), id.session_id().to_owned()))
                .collect::<Vec<_>>();
            let service = core
                .collaboration()
                .map_err(|_| "collaboration is unavailable")?;
            return core
                .db
                .with_competition_winner_admission(
                    &service,
                    fence,
                    claim,
                    &additional,
                    |_, identities, admission, _| {
                        core.automation
                            .with_current_competition_chain(admission, permits, owner_rules, || {
                                apply(identities)
                            })
                            .map_err(crate::db::DatabaseError::Invalid)
                    },
                )
                .map_err(|error| error.to_string());
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

    #[cfg_attr(not(feature = "persistence"), allow(dead_code))]
    pub(crate) fn mark_script_flow(mut self) -> Self {
        self.script_flow = true;
        self
    }
    // Currently consumed only by the optional HTTP sink and scoped tests.
    #[cfg_attr(not(feature = "http"), allow(dead_code))]
    pub(crate) fn has_script_flow(&self) -> bool {
        self.script_flow
    }

    #[cfg_attr(not(feature = "persistence"), allow(dead_code))]
    pub(crate) fn effect_child_slot(&self, namespace: &str, slot: u32) -> Result<Self, String> {
        #[cfg(feature = "persistence")]
        {
            let execution = self
                .execution()?
                .named_child("derived-output", namespace)
                .and_then(|execution| execution.child(slot))
                .map_err(|error| error.to_string())?;
            Ok(self.clone().with_execution(execution))
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (namespace, slot);
            Ok(self.clone())
        }
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
    /// Dispatch admits only trusted Owner or an approved retained chain. This
    /// guard is separate from the Owner-only generic transport/read guard.
    pub(crate) fn automation_dispatch_guard(
        &self,
        context: &ExecutionContext,
    ) -> Result<(), String> {
        if context.is_owner() {
            return self.execution_context_guard(context);
        }
        if self.is_shutdown() {
            return Err("host is shutting down".into());
        }
        #[cfg(feature = "persistence")]
        {
            let _writer = self.automation_gate().map_err(|error| error.to_string())?;
            context.with_current_chain(self, |_| Ok(()))
        }
        #[cfg(not(feature = "persistence"))]
        {
            Err("delegated execution requires persistence".into())
        }
    }
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
        {
            let context = if incoming.is_competition_winner() {
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
        if candidate.requires_scoped_input() && !incoming.is_approved() {
            return Err("this saved approval requires approved scoped input");
        }
        let inherited_competition = match (
            &incoming.competition,
            &incoming.competition_winner,
            &incoming.authority,
        ) {
            (Some(fence), Some(claim), Authority::Approved { subjects, .. }) => {
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
                let permit = core
                    .db
                    .with_competition_winner_admission(
                        &service,
                        &fence,
                        &claim,
                        &all,
                        |_, _, admission, _| {
                            candidate
                                .authorize_competition_chain(admission, &inherited)
                                .map_err(|error| crate::db::DatabaseError::Invalid(error.into()))
                        },
                    )
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
        incoming.intersect(&approved)
    }

    #[cfg(feature = "persistence")]
    pub(crate) async fn competition_winner_execution_context(
        self: &std::sync::Arc<Self>,
        claim: tiktools_competitions::ActionClaim,
        action: serde_json::Value,
    ) -> Result<ExecutionContext, &'static str> {
        let core = self.clone();
        tokio::task::spawn_blocking(move || {
            let service = core
                .collaboration()
                .map_err(|_| "collaboration is unavailable")?;
            let fence = core
                .db
                .capture_competition_execution_fence(
                    &service,
                    &claim.competition_id,
                    claim.generation,
                )
                .map_err(|_| "competition authority is unavailable")?;
            let candidate = core
                .automation
                .competition_action_candidate(&claim.action.action_id)
                .ok_or("winner action is unavailable")?;
            let required = candidate.required_subjects()?;
            let mut all = fence.subjects();
            for subject in &required {
                if !all.contains(subject) {
                    all.push(subject.clone());
                }
            }
            let subjects = service
                .execution_authorizations(&all)
                .map_err(|_| "saved execution subjects are unavailable")?;
            let permit = core
                .db
                .with_competition_winner_admission(
                    &service,
                    &fence,
                    &claim,
                    &required,
                    |_, _, admission, _| {
                        let permit = candidate
                            .authorize_competition_winner(admission)
                            .map_err(|error| crate::db::DatabaseError::Invalid(error.into()))?;
                        if !permit.authorizes_action(&action) {
                            return Err(crate::db::DatabaseError::Invalid(
                                "winner action has changed".into(),
                            ));
                        }
                        Ok(permit)
                    },
                )
                .map_err(|_| "winner action authorization is unavailable")?;
            Ok(ExecutionContext {
                authority: Authority::Approved {
                    subjects,
                    permits: vec![permit],
                    owner_rules: Vec::new(),
                },
                depth: 0,
                script_flow: false,
                allow_delegation: true,
                execution: crate::services::effects::EffectExecution::fresh().ok(),
                competition: Some(fence),
                competition_winner: Some(claim),
            })
        })
        .await
        .map_err(|_| "winner authorization task failed")?
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
            // A retained permit may already cover this exact saved action as
            // part of its pinned closure. Recheck the full chain before reuse.
            if incoming.is_competition_winner() {
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

#[cfg(all(test, feature = "persistence"))]
mod tests;
