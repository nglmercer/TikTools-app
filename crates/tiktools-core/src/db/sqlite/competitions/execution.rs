//! Generation fences and synchronous admission for queued competition work.
use super::validate_competition_winner_claim;
use super::{CompetitionExecutionAdmission, CompetitionExecutionFence};
use super::{DatabaseError, DatabaseManager};
use rusqlite::{params, OptionalExtension};
use tiktools_competitions::{ActionClaim, Lifecycle, LifecycleState, WinnerAction};

fn competition_execution_fence(
    tx: &rusqlite::Connection,
    id: &str,
    generation: u64,
    identities: &[crate::collaboration::ExecutionIdentity],
) -> Result<CompetitionExecutionFence, DatabaseError> {
    let denied = || DatabaseError::CommandAuthorization(crate::collaboration::Error::Forbidden);
    let row: Option<(String,i64)> = tx.query_row(
        "SELECT s.state_json,r.revision FROM competition_state s JOIN competition_control_revisions r ON r.competition_id=s.competition_id AND r.deleted=0 WHERE s.competition_id=?1",
        [id], |row| Ok((row.get(0)?,row.get(1)?)),
    ).optional()?;
    let (state, revision) = row.ok_or_else(denied)?;
    let state: LifecycleState = serde_json::from_str(&state).map_err(|_| denied())?;
    if state.generation != generation
        || revision <= 0
        || matches!(state.lifecycle, Lifecycle::Draft | Lifecycle::Failed)
    {
        return Err(denied());
    }
    let host_id = identities.first().ok_or_else(denied)?.host_id().to_owned();
    if identities
        .iter()
        .any(|identity| identity.host_id() != host_id)
    {
        return Err(denied());
    }
    let subjects = identities
        .iter()
        .map(|identity| {
            (
                identity.principal_id().to_owned(),
                identity.session_id().to_owned(),
                identity.grant_revision().to_owned(),
                identity.configuration_revision().to_owned(),
            )
        })
        .collect();
    Ok(CompetitionExecutionFence {
        id: id.into(),
        generation,
        revision,
        host_id,
        subjects,
    })
}

impl DatabaseManager {
    /// Capture a generation binding before queueing work. Grant changes and
    /// any later lifecycle/definition change invalidate this exact binding.
    #[allow(dead_code)]
    pub(crate) fn capture_competition_execution_fence(
        &self,
        service: &std::sync::Arc<crate::collaboration::CollaborationService>,
        id: &str,
        generation: u64,
    ) -> Result<CompetitionExecutionFence, DatabaseError> {
        self.with_competition_delegation(service, id, generation, |tx, identities| {
            competition_execution_fence(tx, id, generation, identities)
        })
    }

    /// Revalidate the exact binding while holding the business transaction
    /// and all current grants through the supplied synchronous DB operation.
    #[allow(dead_code)]
    pub(crate) fn with_competition_execution_fence<R>(
        &self,
        service: &std::sync::Arc<crate::collaboration::CollaborationService>,
        fence: &CompetitionExecutionFence,
        apply: impl FnOnce(
            &rusqlite::Transaction<'_>,
            &[crate::collaboration::ExecutionIdentity],
        ) -> Result<R, DatabaseError>,
    ) -> Result<R, DatabaseError> {
        self.with_competition_delegation(service, &fence.id, fence.generation, |tx, identities| {
            if competition_execution_fence(tx, &fence.id, fence.generation, identities)? != *fence {
                return Err(DatabaseError::CommandAuthorization(
                    crate::collaboration::Error::Forbidden,
                ));
            }
            apply(tx, identities)
        })
    }

    /// Admit saved automation authors together with competition operators.
    /// The additional subjects must be obtained from a trusted saved candidate.
    #[allow(dead_code)]
    pub(crate) fn with_competition_execution_fence_and_subjects<R>(
        &self,
        service: &std::sync::Arc<crate::collaboration::CollaborationService>,
        fence: &CompetitionExecutionFence,
        additional: &[(String, String)],
        apply: impl FnOnce(
            &rusqlite::Transaction<'_>,
            &[crate::collaboration::ExecutionIdentity],
            &CompetitionExecutionAdmission<'_>,
        ) -> Result<R, DatabaseError>,
    ) -> Result<R, DatabaseError> {
        self.with_competition_delegation_and_subjects(
            service,
            &fence.id,
            fence.generation,
            additional,
            |tx, identities, operator_count| {
                if competition_execution_fence(
                    tx,
                    &fence.id,
                    fence.generation,
                    &identities[..operator_count],
                )? != *fence
                {
                    return Err(DatabaseError::CommandAuthorization(
                        crate::collaboration::Error::Forbidden,
                    ));
                }
                apply(
                    tx,
                    identities,
                    &CompetitionExecutionAdmission {
                        fence,
                        identities,
                        winner: None,
                    },
                )
            },
        )
    }

    /// Hold the competition, operator, author and immutable winner bindings
    /// together while admitting the saved winner action.
    #[allow(dead_code)]
    pub(crate) fn with_competition_winner_admission<R>(
        &self,
        service: &std::sync::Arc<crate::collaboration::CollaborationService>,
        fence: &CompetitionExecutionFence,
        claim: &ActionClaim,
        additional: &[(String, String)],
        apply: impl FnOnce(
            &rusqlite::Transaction<'_>,
            &[crate::collaboration::ExecutionIdentity],
            &CompetitionExecutionAdmission<'_>,
            &WinnerAction,
        ) -> Result<R, DatabaseError>,
    ) -> Result<R, DatabaseError> {
        if claim.competition_id != fence.id || claim.generation != fence.generation {
            return Err(DatabaseError::CommandAuthorization(
                crate::collaboration::Error::Forbidden,
            ));
        }
        self.with_competition_execution_fence_and_subjects(
            service,
            fence,
            additional,
            |tx, identities, admission| {
                validate_competition_winner_claim(tx, claim)?;
                let winner_admission = CompetitionExecutionAdmission {
                    fence: admission.fence,
                    identities,
                    winner: Some(claim),
                };
                apply(tx, identities, &winner_admission, &claim.action)
            },
        )
    }

    /// Only call from an external-effect journal admission callback, which
    /// already owns the automation SQLite writer until its commit callback.
    /// This reader must not start a transaction or retain a statement across
    /// `apply`, because the journal must be able to commit its writer.
    pub(crate) fn with_competition_journal_admission<R>(
        &self,
        service: &std::sync::Arc<crate::collaboration::CollaborationService>,
        fence: &CompetitionExecutionFence,
        claim: Option<&ActionClaim>,
        additional: &[(String, String)],
        apply: impl FnOnce(
            &[crate::collaboration::ExecutionIdentity],
            &CompetitionExecutionAdmission<'_>,
        ) -> Result<R, DatabaseError>,
    ) -> Result<R, DatabaseError> {
        use crate::collaboration::{Capability, Error, ResourceKind};
        let denied = || DatabaseError::CommandAuthorization(Error::Forbidden);
        if claim.is_some_and(|claim| {
            claim.competition_id != fence.id || claim.generation != fence.generation
        }) {
            return Err(denied());
        }
        let generation = i64::try_from(fence.generation)
            .ok()
            .filter(|n| *n > 0)
            .ok_or_else(denied)?;
        let db = self.open(&self.automation_path())?;
        let origin: Option<String> = db.query_row("SELECT origin FROM competition_execution_generations WHERE competition_id=?1 AND generation=?2", params![fence.id,generation], |r| r.get(0)).optional()?;
        if origin.as_deref() != Some("delegated") {
            return Err(denied());
        }
        let mut statement = db.prepare("SELECT principal_id,session_id,grant_revision FROM competition_execution_subjects WHERE competition_id=?1 AND generation=?2 ORDER BY principal_id,session_id LIMIT 17")?;
        let entries = statement
            .query_map(params![fence.id, generation], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        drop(statement);
        if entries.is_empty()
            || entries.len() > 16
            || entries.iter().any(|(_, _, r)| {
                r.parse::<i64>()
                    .ok()
                    .is_none_or(|n| n <= 0 || n.to_string() != *r)
            })
        {
            return Err(denied());
        }
        let mut subjects = entries
            .iter()
            .map(|(p, s, _)| (p.clone(), s.clone()))
            .collect::<Vec<_>>();
        for subject in additional {
            if !subjects.contains(subject) {
                subjects.push(subject.clone());
            }
        }
        let authorization = service
            .execution_authorizations(&subjects)
            .map_err(DatabaseError::CommandAuthorization)?;
        authorization
            .with_current(|identities| {
                if identities.len() != subjects.len()
                    || identities.iter().zip(&entries).any(
                        |(id, (principal, session, revision))| {
                            id.principal_id() != principal
                                || id.session_id() != session
                                || id.grant_revision().parse::<i64>().ok()
                                    < revision.parse::<i64>().ok()
                                || !id
                                    .grant()
                                    .capabilities
                                    .contains(&Capability::CompetitionsRead)
                                || !id
                                    .grant()
                                    .capabilities
                                    .contains(&Capability::CompetitionsManage)
                                || id.grant().resources.contains_key(&ResourceKind::Live)
                                || !id
                                    .grant()
                                    .allows_resource(ResourceKind::Competition, &fence.id)
                                || id.check_deadline().is_err()
                        },
                    )
                    || competition_execution_fence(
                        &db,
                        &fence.id,
                        fence.generation,
                        &identities[..entries.len()],
                    )? != *fence
                {
                    return Err(denied());
                }
                if let Some(claim) = claim {
                    validate_competition_winner_claim(&db, claim)?;
                }
                let admission = CompetitionExecutionAdmission {
                    fence,
                    identities,
                    winner: claim,
                };
                apply(identities, &admission)
            })
            .map_err(DatabaseError::CommandAuthorization)?
    }
}
