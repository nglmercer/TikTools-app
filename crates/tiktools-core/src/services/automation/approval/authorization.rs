//! Closed manifest authorization and retained subject/grant checks.
use super::*;

impl ApprovalCandidate {
    pub(crate) fn required_subjects(&self) -> Result<Vec<(String, String)>, &'static str> {
        manifest::required_subjects(&self.review).map_err(|_| "saved approval closure is invalid")
    }

    pub(crate) fn authorize(
        self,
        identities: &[ExecutionIdentity],
    ) -> Result<RuntimeApprovalPermit, &'static str> {
        self.authorize_with_inherited(identities, &[], None, None, false)
    }

    pub(super) fn authorize_with_inherited(
        self,
        identities: &[ExecutionIdentity],
        inherited: &[(String, String)],
        competition: Option<crate::db::CompetitionExecutionFence>,
        competition_winner: Option<(tiktools_competitions::ActionClaim, Value)>,
        native_competition_event: bool,
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
            native_competition_event,
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

    pub(crate) fn authorizes_event(&self, event: &Value) -> bool {
        self.candidate
            .definitions
            .iter()
            .any(|fence| fence.family == AutomationFamily::Event && fence.definition == *event)
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
