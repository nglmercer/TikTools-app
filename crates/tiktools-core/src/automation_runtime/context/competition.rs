//! Competition generation and winner fences retained through execution admission.
use super::*;

impl ExecutionContext {
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

    /// Keep the competition generation, operators and saved authors locked
    /// through admission, including the immutable winner when applicable.
    #[cfg(feature = "persistence")]
    pub(super) fn with_competition_binding<R>(
        &self,
        core: &AppCore,
        apply: impl FnOnce(
            &[crate::collaboration::ExecutionIdentity],
            &crate::db::CompetitionExecutionAdmission<'_>,
        ) -> Result<R, String>,
    ) -> Result<R, String> {
        let (Some(fence), Authority::Approved { subjects, .. }) =
            (&self.competition, &self.authority)
        else {
            return Err("competition authority is unavailable".into());
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
        if let Some(claim) = &self.competition_winner {
            core.db
                .with_competition_winner_admission(
                    &service,
                    fence,
                    claim,
                    &additional,
                    |_, identities, admission, _| {
                        apply(identities, admission).map_err(crate::db::DatabaseError::Invalid)
                    },
                )
                .map_err(|error| error.to_string())
        } else {
            core.db
                .with_competition_execution_fence_and_subjects(
                    &service,
                    fence,
                    &additional,
                    |_, identities, admission| {
                        apply(identities, admission).map_err(crate::db::DatabaseError::Invalid)
                    },
                )
                .map_err(|error| error.to_string())
        }
    }

    #[cfg(feature = "persistence")]
    pub(crate) fn with_competition_action<R>(
        &self,
        core: &AppCore,
        action: &serde_json::Value,
        apply: impl FnOnce(&[crate::collaboration::ExecutionIdentity]) -> Result<R, String>,
    ) -> Result<R, String> {
        let Authority::Approved {
            permits,
            owner_rules,
            ..
        } = &self.authority
        else {
            return Err("competition authority is unavailable".into());
        };
        self.with_competition_binding(core, |identities, admission| {
            core.automation.with_current_competition_approvals(
                admission,
                permits,
                owner_rules,
                action,
                || apply(identities),
            )
        })
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
            Authority::Approved {
                subjects,
                permits,
                owner_rules,
            },
        ) = (&self.competition, &self.authority)
        else {
            return Err("competition authority is unavailable".into());
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
            .with_competition_journal_admission(
                &service,
                fence,
                self.competition_winner.as_ref(),
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

    #[cfg(feature = "persistence")]
    pub(crate) fn is_competition(&self) -> bool {
        self.competition.is_some()
    }
}

impl AppCore {
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
}
